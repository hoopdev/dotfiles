"""Session isolation tests; run with python3 -B -m unittest discover -s tests."""

import os
from pathlib import Path
import shutil
import socket
import subprocess
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "home/mac/cli/ssh-agent.sh"


class AgentInitTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="za-", dir="/tmp")
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name).resolve()
        self.local = self.root / "Library/Group Containers/2BUA8C4S2C.com.1password/t/agent.sock"
        self.env = dict(os.environ, HOME=str(self.root))
        for key in ("SSH_CONNECTION", "SSH_AUTH_SOCK", "DEV_SSH_AGENT_SOCK",
                    "_DOTFILES_SSH_AGENT_INITIALIZED"):
            self.env.pop(key, None)
        self.shells = [shutil.which(name) for name in ("zsh", "bash")]
        self.assertTrue(all(self.shells), "Both zsh and bash must be on PATH")

    def make_socket(self, name):
        path = self.root / name
        sock = socket.socket(socket.AF_UNIX)
        sock.bind(str(path))
        self.addCleanup(sock.close)
        return str(path)

    def run_shell(self, shell, extra_env):
        command = '. "$1"; . "$1"; printf "%s\\n%s\\n" "${SSH_AUTH_SOCK-}" "$DEV_SSH_AGENT_SOCK"'
        result = subprocess.run(
            [shell, "-fc", command, "agent-test", str(SCRIPT)],
            env=self.env | extra_env, capture_output=True, text=True, timeout=3,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout.splitlines()

    def test_local_ignores_mux_agent_and_legacy_shared_link(self):
        other = self.make_socket("other.sock")
        stable = self.root / ".ssh/agent/current"
        stable.parent.mkdir(parents=True)
        stable.symlink_to(other)
        for shell in self.shells:
            with self.subTest(shell=shell):
                self.assertEqual(self.run_shell(shell, {"SSH_AUTH_SOCK": other}),
                                 [str(self.local), str(self.local)])
                self.assertEqual(str(stable.resolve()), other)

    def test_forwarded_and_unforwarded_sessions(self):
        forwarded = self.make_socket("forwarded.sock")
        for shell in self.shells:
            for sock in (forwarded, ""):
                with self.subTest(shell=shell, sock=sock):
                    self.assertEqual(self.run_shell(shell, {
                        "SSH_CONNECTION": "client server",
                        "SSH_AUTH_SOCK": sock,
                        "DEV_SSH_AGENT_SOCK": str(self.local),
                    }), [sock, sock])

    def test_child_mux_panes_keep_parent_agent_even_when_disconnected(self):
        live = self.make_socket("live.sock")
        dead = str(self.root / "disconnected.sock")
        for shell in self.shells:
            for sock in (live, dead, ""):
                with self.subTest(shell=shell, sock=sock):
                    self.assertEqual(self.run_shell(shell, {
                        "DEV_SSH_AGENT_SOCK": sock,
                        "SSH_AUTH_SOCK": "mux-owned-socket",
                    }), [sock, sock])

    def test_second_connection_does_not_change_first_session(self):
        a = self.make_socket("a.sock")
        b = self.make_socket("b.sock")
        for shell in self.shells:
            with self.subTest(shell=shell):
                env_a = dict(self.env, SSH_CONNECTION="client-a", SSH_AUTH_SOCK=a)
                # Keep A alive while B starts, then ask A to resolve its socket.
                process = subprocess.Popen(
                    [shell, "-fc", '. "$1"; printf "ready\\n"; read -r reply; '
                     'printf "%s\\n" "$SSH_AUTH_SOCK"', "agent-test", str(SCRIPT)],
                    env=env_a, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE, text=True,
                )
                try:
                    self.assertEqual(process.stdout.readline(), "ready\n")
                    self.assertEqual(self.run_shell(shell, {
                        "SSH_CONNECTION": "client-b", "SSH_AUTH_SOCK": b,
                    }), [b, b])
                    out, err = process.communicate("continue\n", timeout=3)
                    self.assertEqual(process.returncode, 0, err)
                    self.assertEqual(str(Path(out.strip()).resolve()), a)
                    self.assertFalse((self.root / ".ssh/agent/current").exists())
                finally:
                    if process.poll() is None:
                        process.kill()
                        process.communicate()


if __name__ == "__main__":
    unittest.main()
