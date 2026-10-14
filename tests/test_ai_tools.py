"""Ensure downloaded installer content is never executed after a hash mismatch."""

import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import unittest


class InstallerIntegrityTests(unittest.TestCase):
    def test_changed_installers_fail_before_execution(self):
        source = (Path(__file__).resolve().parents[1] /
                  "home/common/cli/ai-tools.nix").read_text()
        script = source.split("            if ! (\n", 1)[1].split("\n            ); then", 1)[0]
        for app in ("claude", "codex", "antigravity"):
            digest = re.search(rf'{app} = "([a-f0-9]{{64}})";', source).group(1)
            script = script.replace("${installerHashes." + app + "}", digest)

        with tempfile.TemporaryDirectory(prefix="installer-integrity-") as directory:
            root = Path(directory)
            payload = root / "changed-installer.sh"
            payload.write_text('#!/bin/sh\nprintf executed > "$TEST_INSTALLER_MARKER"\n')
            curl = root / "curl"
            curl.write_text('''#!/bin/sh
while [ "$#" -gt 0 ]; do
  if [ "$1" = -o ]; then
    cp "$TEST_INSTALLER_PAYLOAD" "$2"
    exit 0
  fi
  shift
done
exit 2
''')
            curl.chmod(0o700)
            sha256sum = shutil.which("sha256sum")
            self.assertTrue(sha256sum, "run in the Nix development shell")
            (root / "sha256sum").symlink_to(sha256sum)
            for app in ("claude", "codex", "antigravity"):
                with self.subTest(app=app):
                    marker = root / f"{app}-executed"
                    result = subprocess.run(
                        ["bash", "-c", "set -euo pipefail\n" + script],
                        env=dict(os.environ) | {
                            "PATH": f"{root}:{os.environ['PATH']}",
                            "app": app,
                            "TEST_INSTALLER_PAYLOAD": str(payload),
                            "TEST_INSTALLER_MARKER": str(marker),
                        },
                        capture_output=True, text=True, timeout=3,
                    )
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn("installer checksum mismatch", result.stderr)
                    self.assertFalse(marker.exists())


if __name__ == "__main__":
    unittest.main()
