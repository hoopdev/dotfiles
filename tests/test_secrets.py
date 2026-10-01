"""Exercise the Zsh functions embedded in the Home Manager module using dummy secrets."""

import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


class SecretsLoadTests(unittest.TestCase):
    def setUp(self):
        self.zsh = shutil.which("zsh")
        self.assertTrue(self.zsh, "zsh must be on PATH")
        self.tmp = tempfile.TemporaryDirectory(prefix="secrets-review-")
        self.addCleanup(self.tmp.cleanup)
        root = Path(self.tmp.name)
        self.managed = root / "managed.env"
        self.local = root / "local.env"
        self.managed.write_text("REVIEW_FIRST=new\nREVIEW_SECOND=managed\n")
        # Test the actual embedded functions, with only Nix interpolation rendered.
        source = (Path(__file__).resolve().parents[1] /
                  "home/common/cli/onepassword.nix").read_text()
        env_files = source.split("  envFilesSh = ''\n", 1)[1].split("\n  '';", 1)[0]
        functions = source.split("    programs.zsh.initContent = ''\n", 1)[1].rsplit("\n    '';", 1)[0]
        self.functions = (functions.replace("${envFilesSh}", env_files)
                          .replace("${envFile}", str(self.managed))
                          .replace("${localEnvFile}", str(self.local))
                          .replace("''${", "${"))
        self.env = dict(os.environ)
        self.env.pop("OP_SERVICE_ACCOUNT_TOKEN", None)
        for key in list(self.env):
            if key.startswith("REVIEW_"):
                self.env.pop(key)

    def run_load(self, *, fail_local=False, prefix=""):
        # No real 1Password invocation or secret material is used.
        mock = '''
op() {
  [[ "$1" == whoami ]] && return 0
  [[ "$1" == inject && "$2" == -i ]] || return 2
  if [[ "$3" == "$REVIEW_FAIL_FILE" ]]; then return 1; fi
  command cat "$3"
}
'''
        command = prefix + "\n" + mock + self.functions + '''
secrets-load
result=$?
printf '%s\\n' "$result" "${REVIEW_FIRST-unset}" "${REVIEW_SECOND-unset}" "${REVIEW_THIRD-unset}"
'''
        result = subprocess.run(
            [self.zsh, "-fc", command],
            env=self.env | {"REVIEW_FAIL_FILE": str(self.local) if fail_local else ""},
            capture_output=True, text=True, timeout=3,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout.splitlines(), result.stderr

    def test_managed_file_without_optional_local_file(self):
        values, _ = self.run_load()
        self.assertEqual(values, ["0", "new", "managed", "unset"])

    def test_local_overrides_and_literal_values(self):
        self.local.write_text('# comment\nREVIEW_SECOND=local value=with equals\n'
                             'REVIEW_THIRD=$(echo should-not-run)\n')
        values, _ = self.run_load()
        self.assertEqual(values, ["0", "new", "local value=with equals", "$(echo should-not-run)"])

    def test_failure_in_second_file_preserves_original_environment(self):
        self.local.write_text("REVIEW_SECOND=local\n")
        values, _ = self.run_load(fail_local=True, prefix="export REVIEW_FIRST=original")
        self.assertEqual(values, ["1", "original", "unset", "unset"])

    def test_invalid_assignment_does_not_export_anything(self):
        for invalid in ("INVALID-NAME=sensitive", "missing-equals-sensitive"):
            with self.subTest(invalid=invalid):
                self.local.write_text("REVIEW_THIRD=valid\n" + invalid + "\n")
                values, err = self.run_load()
                self.assertEqual(values, ["1", "unset", "unset", "unset"])
                self.assertNotIn("sensitive", err)

    def test_readonly_assignment_does_not_export_anything(self):
        self.local.write_text("REVIEW_THIRD=sensitive\n")
        values, err = self.run_load(prefix="readonly REVIEW_THIRD=original")
        self.assertEqual(values, ["1", "unset", "unset", "original"])
        self.assertNotIn("sensitive", err)


if __name__ == "__main__":
    unittest.main()
