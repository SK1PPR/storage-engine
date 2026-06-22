#!/usr/bin/env python3
import argparse
import json
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description="Run pocket-lsm performance and RSS benchmarks.")
    parser.add_argument("--keys", type=int, default=200_000)
    parser.add_argument("--ops", type=int, default=200_000)
    parser.add_argument("--value-bytes", type=int, default=256)
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--sample-interval", type=float, default=0.02)
    parser.add_argument("--json-out", type=Path)
    parser.add_argument("--keep-dirs", action="store_true")
    parser.add_argument(
        "--profile",
        choices=("default", "large"),
        default="default",
        help="large raises defaults to make small RSS differences easier to see",
    )
    args = parser.parse_args()

    if args.profile == "large":
        args.keys = max(args.keys, 500_000)
        args.ops = max(args.ops, 500_000)
        args.value_bytes = max(args.value_bytes, 512)

    repo = Path(__file__).resolve().parents[1]
    binary = repo / "target" / "release" / "perf_harness"
    print("building release perf harness...", flush=True)
    build(repo)
    print(
        f"starting benchmark: keys={args.keys} ops={args.ops} value_bytes={args.value_bytes} repeats={args.repeats}",
        flush=True,
    )

    results = []
    temp_roots = []
    try:
        for repeat in range(args.repeats):
            results.extend(run_suite(binary, args, repeat, temp_roots))
    finally:
        if args.keep_dirs:
            for root in temp_roots:
                print(f"kept data dir: {root}")
        else:
            for root in temp_roots:
                shutil.rmtree(root, ignore_errors=True)

    print_table(results)
    if args.json_out:
        args.json_out.write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
        print(f"\nwrote JSON results to {args.json_out}")

    return 0


def build(repo: Path) -> None:
    subprocess.run(
        ["cargo", "build", "--release", "-p", "storage_engine_runner", "--bin", "perf_harness"],
        cwd=repo,
        check=True,
    )


def run_suite(binary: Path, args: argparse.Namespace, repeat: int, temp_roots: list[Path]) -> list[dict]:
    results = []
    print(f"\nrepeat {repeat + 1}/{args.repeats}", flush=True)

    write_memtable_dir = Path(tempfile.mkdtemp(prefix=f"pocket-lsm-write-memtable-{repeat}-"))
    temp_roots.append(write_memtable_dir)
    high_threshold = max(args.keys * (args.value_bytes + 96), 1024 * 1024 * 1024)
    results.append(
        run_harness(
            binary,
            "write_memtable",
            repeat,
            write_memtable_dir,
            workload="write",
            keys=args.keys,
            ops=args.ops,
            value_bytes=args.value_bytes,
            memtable_threshold=high_threshold,
            maximum_memtables=64,
            sample_interval=args.sample_interval,
        )
    )

    write_flush_dir = Path(tempfile.mkdtemp(prefix=f"pocket-lsm-write-flush-{repeat}-"))
    temp_roots.append(write_flush_dir)
    results.append(
        run_harness(
            binary,
            "write_flush",
            repeat,
            write_flush_dir,
            workload="write",
            keys=args.keys,
            ops=args.ops,
            value_bytes=args.value_bytes,
            memtable_threshold=1024 * 1024,
            maximum_memtables=0,
            sample_interval=args.sample_interval,
        )
    )

    read_dir = Path(tempfile.mkdtemp(prefix=f"pocket-lsm-read-{repeat}-"))
    temp_roots.append(read_dir)
    run_harness(
        binary,
        "setup_read",
        repeat,
        read_dir,
        workload="populate",
        keys=args.keys,
        ops=args.ops,
        value_bytes=args.value_bytes,
        memtable_threshold=1024 * 1024,
        maximum_memtables=0,
        sample_interval=args.sample_interval,
    )
    results.append(
        run_harness(
            binary,
            "read_sstable",
            repeat,
            read_dir,
            workload="read",
            keys=args.keys,
            ops=args.ops,
            value_bytes=args.value_bytes,
            memtable_threshold=1024 * 1024,
            maximum_memtables=0,
            sample_interval=args.sample_interval,
        )
    )

    mixed_dir = Path(tempfile.mkdtemp(prefix=f"pocket-lsm-mixed-{repeat}-"))
    temp_roots.append(mixed_dir)
    setup_keys = max(args.keys // 2, 1)
    run_harness(
        binary,
        "setup_mixed",
        repeat,
        mixed_dir,
        workload="populate",
        keys=setup_keys,
        ops=args.ops,
        value_bytes=args.value_bytes,
        memtable_threshold=1024 * 1024,
        maximum_memtables=0,
        sample_interval=args.sample_interval,
    )
    results.append(
        run_harness(
            binary,
            "mixed_50_50",
            repeat,
            mixed_dir,
            workload="mixed",
            keys=setup_keys,
            ops=args.ops,
            value_bytes=args.value_bytes,
            memtable_threshold=1024 * 1024,
            maximum_memtables=4,
            sample_interval=args.sample_interval,
        )
    )

    return results


def run_harness(
    binary: Path,
    label: str,
    repeat: int,
    data_dir: Path,
    *,
    workload: str,
    keys: int,
    ops: int,
    value_bytes: int,
    memtable_threshold: int,
    maximum_memtables: int,
    sample_interval: float,
) -> dict:
    command = [
        str(binary),
        "--workload",
        workload,
        "--data-dir",
        str(data_dir),
        "--keys",
        str(keys),
        "--ops",
        str(ops),
        "--value-bytes",
        str(value_bytes),
        "--memtable-threshold",
        str(memtable_threshold),
        "--maximum-memtables",
        str(maximum_memtables),
    ]
    proc = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    max_rss_kib = 0
    started = time.perf_counter()
    last_report = started

    print(
        f"  start {label}: workload={workload} keys={keys} ops={ops} value_bytes={value_bytes}",
        flush=True,
    )

    try:
        while proc.poll() is None:
            max_rss_kib = max(max_rss_kib, rss_kib(proc.pid))
            now = time.perf_counter()
            if now - last_report >= 5.0:
                print(
                    f"    running {label}: elapsed={now - started:.1f}s max_rss={max_rss_kib / 1024:.2f} MiB",
                    flush=True,
                )
                last_report = now
            time.sleep(sample_interval)

        max_rss_kib = max(max_rss_kib, rss_kib(proc.pid))
        stdout, stderr = proc.communicate()
    except KeyboardInterrupt:
        proc.kill()
        proc.wait()
        raise

    wall_ms = int((time.perf_counter() - started) * 1000)

    if proc.returncode != 0:
        sys.stderr.write(stdout)
        sys.stderr.write(stderr)
        raise subprocess.CalledProcessError(proc.returncode, command)

    try:
        payload = json.loads(stdout.strip().splitlines()[-1])
    except (IndexError, json.JSONDecodeError) as exc:
        raise RuntimeError(f"perf_harness did not emit JSON for {label}: {stdout!r}") from exc

    payload.update(
        {
            "label": label,
            "repeat": repeat,
            "wall_ms": wall_ms,
            "max_rss_bytes": max_rss_kib * 1024,
            "max_rss_mib": round(max_rss_kib / 1024, 2),
        }
    )
    print(
        f"  done  {label}: elapsed={wall_ms / 1000:.1f}s ops/s={payload['ops_per_sec']:.0f} max_rss={payload['max_rss_mib']:.2f} MiB",
        flush=True,
    )
    return payload


def rss_kib(pid: int) -> int:
    try:
        output = subprocess.check_output(["ps", "-o", "rss=", "-p", str(pid)], text=True)
    except subprocess.SubprocessError:
        return 0

    output = output.strip()
    return int(output) if output else 0


def print_table(results: list[dict]) -> None:
    visible = [result for result in results if not result["label"].startswith("setup_")]
    headers = ["label", "rep", "ops", "ops/s", "rss MiB", "sst", "memtable", "immut"]
    rows = [
        [
            result["label"],
            result["repeat"],
            result["ops"],
            f"{result['ops_per_sec']:.0f}",
            f"{result['max_rss_mib']:.2f}",
            result["sstables"],
            result["memtable_size"],
            result["immutable_memtables"],
        ]
        for result in visible
    ]
    widths = [
        max(len(str(row[index])) for row in [headers, *rows])
        for index in range(len(headers))
    ]

    print()
    print(" ".join(str(value).ljust(widths[index]) for index, value in enumerate(headers)))
    print(" ".join("-" * width for width in widths))
    for row in rows:
        print(" ".join(str(value).ljust(widths[index]) for index, value in enumerate(row)))


if __name__ == "__main__":
    raise SystemExit(main())
