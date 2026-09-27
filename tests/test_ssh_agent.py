"""Run with SSH_AGENT_TIMEOUT=/path/to/coreutils/bin/timeout python3 -m unittest discover -s tests."""

import os
from pathlib import Path
import shutil
import socket
import subprocess
import tempfile
import unittest


class AgentInitTests(unittest.TestCase):
    def test_agent_selection(self):
        timeout = os.environ.get("SSH_AGENT_TIMEOUT") or shutil.which("timeout")
        self.assertTrue(timeout, "Set SSH_AGENT_TIMEOUT to GNU coreutils timeout")
        template = (
            Path(__file__).resolve().parents[1] / "home/mac/cli/ssh-agent.sh"
        ).read_text()
        for shell in ("/bin/zsh", "/bin/bash"):
            for case in ("local", "missing", "dead", "empty", "hung", "forwarded", "no-forwarding"):
                with self.subTest(shell=shell, case=case), tempfile.TemporaryDirectory(
                    prefix="za-", dir="/tmp"
                ) as tmp:
                    root = Path(tmp).resolve()
                    local = root / "Library/Group Containers/2BUA8C4S2C.com.1password/t/agent.sock"
                    local.parent.mkdir(parents=True)
                    local.touch()
                    stable = root / ".ssh/agent/current"
                    forwarded = root / "forwarded.sock"
                    with socket.socket(socket.AF_UNIX) as sock:
                        sock.bind(str(forwarded))
                        if case != "missing":
                            stable.parent.mkdir(parents=True)
                            stable.symlink_to(local if case in ("local", "forwarded", "no-forwarding") else forwarded)
                        marker = root / "probe-called"
                        mock = root / "ssh-add"
                        behavior = {
                            "hung": "trap '' TERM; exec /bin/sleep 10",
                            "empty": "exit 1",
                        }.get(case, "exit 2")
                        mock.write_text('#!/bin/sh\necho called >> "$HOME/probe-called"\n' + behavior + "\n")
                        mock.chmod(0o700)
                        script = root / "agent.sh"
                        script.write_text(template.replace("@timeout@", str(timeout)).replace("@sshAdd@", str(mock)))
                        env = dict(os.environ, HOME=str(root), SSH_CONNECTION="", SSH_AUTH_SOCK="")
                        env.pop("_DOTFILES_SSH_AGENT_INITIALIZED", None)
                        if case in ("forwarded", "no-forwarding"):
                            env["SSH_CONNECTION"] = "test connection"
                        if case == "forwarded":
                            env["SSH_AUTH_SOCK"] = str(forwarded)
                        command = '. "$1"; . "$1"; printf "%s" "$SSH_AUTH_SOCK"'
                        result = subprocess.run(
                            [shell, "-fc", command, "agent-test", str(script)],
                            env=env, capture_output=True, text=True, timeout=3,
                        )
                        self.assertEqual(result.returncode, 0, result.stderr)
                        self.assertEqual(result.stdout, "" if case == "no-forwarding" else str(stable))
                        expected = forwarded if case in ("empty", "hung", "forwarded") else local
                        self.assertEqual(stable.resolve(), expected)
                        self.assertEqual(marker.exists(), case in ("missing", "dead", "empty", "hung"))
                        if marker.exists():
                            self.assertEqual(marker.read_text(), "called\n")


if __name__ == "__main__":
    unittest.main()
