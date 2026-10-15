"""Offline regression tests: no SSH/SCP connection or remote commands execute."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "build-remote.sh"


class ReleaseScriptTests(unittest.TestCase):
    def run_mocked(self, ssh_status, scp_status):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            trace = root / "trace"
            # The actual build script runs, but SSH only drains the heredoc. Never
            # run its contents: that would install packages and alter the host.
            for tool, status in [("ssh", ssh_status), ("scp", scp_status)]:
                drain = "/bin/cat >/dev/null\n" if tool == "ssh" else ""
                executable = root / tool
                executable.write_text(
                    "#!/bin/bash\n"
                    f"printf '{tool}\\n' >> \"$TRACE\"\n"
                    + drain
                    + f"exit {status}\n"
                )
                executable.chmod(0o700)
            env = os.environ | {"PATH": str(root), "TRACE": str(trace)}
            result = subprocess.run(
                ["/bin/bash", str(SCRIPT)], cwd=root, env=env,
                capture_output=True, text=True, timeout=5,
            )
            return result, trace.read_text().splitlines()

    def test_failed_remote_build_stops_before_copying_artifacts(self):
        result, trace = self.run_mocked(42, 0)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(trace, ["ssh"])

    def test_failed_copy_stops_at_first_artifact(self):
        result, trace = self.run_mocked(0, 43)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(trace, ["ssh", "scp"])

    def test_success_copies_all_nine_artifacts(self):
        result, trace = self.run_mocked(0, 0)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(trace, ["ssh"] + ["scp"] * 9)


if __name__ == "__main__":
    unittest.main()
