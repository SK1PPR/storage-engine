#!/usr/bin/env python3
import argparse
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description="Kill/restart crash recovery smoke test.")
    parser.add_argument("--writes", type=int, default=80)
    parser.add_argument("--threshold", type=int, default=64)
    parser.add_argument("--keep-dir", action="store_true")
    args = parser.parse_args()

    repo = Path(__file__).resolve().parents[1]
    data_dir = Path(tempfile.mkdtemp(prefix="storage-engine-crash-"))
    expected_file = data_dir / "expected.tsv"
    expected = []
    proc = None

    try:
        proc = subprocess.Popen(
            [
                "cargo",
                "run",
                "--quiet",
                "-p",
                "storage_engine_runner",
                "--bin",
                "crash_harness",
                "--",
                "write-loop",
                str(data_dir),
                str(args.threshold),
            ],
            cwd=repo,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )

        assert proc.stdout is not None
        while len(expected) < args.writes:
            line = proc.stdout.readline()
            if line == "":
                stderr = proc.stderr.read() if proc.stderr else ""
                raise RuntimeError(f"writer exited before enough writes\n{stderr}")
            parts = line.rstrip("\n").split("\t")
            if len(parts) == 3 and parts[0] == "PUT":
                expected.append((parts[1], parts[2]))

        expected_file.write_text(
            "".join(f"{key}\t{value}\n" for key, value in expected),
            encoding="utf-8",
        )

        proc.kill()
        proc.wait(timeout=10)

        verify = subprocess.run(
            [
                "cargo",
                "run",
                "--quiet",
                "-p",
                "storage_engine_runner",
                "--bin",
                "crash_harness",
                "--",
                "verify",
                str(data_dir),
                str(expected_file),
            ],
            cwd=repo,
            text=True,
            capture_output=True,
        )
        if verify.returncode != 0:
            sys.stderr.write(verify.stdout)
            sys.stderr.write(verify.stderr)
            return verify.returncode

        print(verify.stdout.strip())
        print(f"crash recovery smoke passed: {len(expected)} acknowledged writes")
        return 0
    finally:
        if proc is not None and proc.poll() is None:
            proc.kill()
        if args.keep_dir:
            print(f"kept data dir: {data_dir}")
        else:
            shutil.rmtree(data_dir, ignore_errors=True)


if __name__ == "__main__":
    raise SystemExit(main())
