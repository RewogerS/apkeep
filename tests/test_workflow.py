"""Validate rustup workflow flags against the real local CLI without downloads."""
from pathlib import Path
import shlex
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[1]


class RustupWorkflowTests(unittest.TestCase):
    def test_installer_flags_are_supported_by_selected_rustup_subcommand(self):
        lines = (ROOT / '.github/workflows/tests.yml').read_text().splitlines()
        commands = [shlex.split(line.strip()[len('run: '):]) for line in lines
                    if line.strip().startswith('run: rustup ')]
        self.assertEqual(len(commands), 1, 'expected one Rust toolchain setup command')
        command = commands[0]
        prefix = command[:command.index('stable')]
        result = subprocess.run(prefix + ['--help'], capture_output=True, text=True, timeout=15)
        self.assertEqual(result.returncode, 0, result.stderr)
        for flag in (arg for arg in command if arg.startswith('--')):
            self.assertIn(flag, result.stdout, f'{flag} not supported by {prefix}')
        self.assertIn('clippy', command)
        self.assertIn('minimal', command)


if __name__ == '__main__':
    unittest.main()
