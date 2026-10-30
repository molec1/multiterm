#!/usr/bin/env python3
"""Compare the Python app with the Rust/GTK 3 port.

Each run uses its own data directory, so the saved session is left alone.
The clock starts when the process is spawned and stops at the line ``BENCH ready``.
The process then reports how long the shells took to start and how long a
typed command took to appear. Memory is read from /proc after that line.
"""

from __future__ import annotations

import json
import os
import select
import statistics
import subprocess
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RUNS = 3


def source_stats(glob: str) -> tuple[int, int]:
    files = [path for path in ROOT.glob(glob) if path.is_file()]
    lines = 0
    size = 0
    for path in files:
        text = path.read_text(encoding="utf-8")
        size += path.stat().st_size
        lines += text.count("\n") + (0 if text.endswith("\n") or not text else 1)
    return lines, size


def median(values: list[float]) -> float:
    return statistics.median(values) if values else float("nan")


def memory(pid: int) -> tuple[int, int]:
    rss = pss = 0
    text = Path(f"/proc/{pid}/smaps_rollup").read_text(encoding="utf-8")
    for line in text.splitlines():
        if line.startswith("Rss:"):
            rss = int(line.split()[1])
        elif line.startswith("Pss:"):
            pss = int(line.split()[1])
    return rss, pss


def once(command: list[str], env: dict[str, str], tabs: int) -> dict[str, float]:
    data = tempfile.mkdtemp(prefix=f"multiterm-bench-{tabs}-")
    run_env = os.environ.copy()
    run_env.update(env)
    run_env["PYTHONUNBUFFERED"] = "1"
    run_env["MULTITERM_BENCH"] = "1"
    run_env["MULTITERM_BENCH_TABS"] = str(tabs)
    run_env["MULTITERM_DATA"] = data
    run_env.setdefault("DISPLAY", ":1")
    started = time.perf_counter()
    error_log = tempfile.NamedTemporaryFile(prefix="multiterm-bench-err-", delete=False)
    process = subprocess.Popen(
        command,
        cwd=ROOT,
        env=run_env,
        stdout=subprocess.PIPE,
        stderr=error_log,
        text=True,
    )
    startup = None
    metrics: dict[str, float] = {}
    assert process.stdout is not None
    output = process.stdout.fileno()
    os.set_blocking(output, False)
    try:
        deadline = time.monotonic() + 25
        pending = ""
        while time.monotonic() < deadline and not metrics:
            ready, _, _ = select.select([output], [], [], 0.5)
            if not ready:
                if process.poll() is not None:
                    break
                continue
            try:
                chunk = os.read(output, 4096)
            except BlockingIOError:
                continue
            if not chunk:
                break
            pending += chunk.decode("utf-8", errors="replace")
            while "\n" in pending:
                line, pending = pending.split("\n", 1)
                if line.startswith("BENCH ready") and startup is None:
                    startup = (time.perf_counter() - started) * 1000
                elif line.startswith("BENCH metrics"):
                    metrics = json.loads(line.split(" ", 2)[2])
        time.sleep(0.2)
        rss, pss = memory(process.pid)
    finally:
        process.kill()
        process.wait(timeout=5)
        error_log.close()
    if startup is None or not metrics:
        err = Path(error_log.name).read_text(encoding="utf-8", errors="replace")[-800:]
        raise RuntimeError(f"no benchmark output from {command[0]}: {err}")
    return {
        "startup_ms": startup,
        "shells_ms": float(metrics["shells_ms"]),
        "echo_ms": float(metrics["echo_ms"]),
        "rss_kib": rss,
        "pss_kib": pss,
    }


def sample(command: list[str], env: dict[str, str], tabs: int) -> dict[str, float]:
    rows = []
    for index in range(RUNS):
        row = once(command, env, tabs)
        print(f"run {tabs} {index} {command[0]} {json.dumps(row)}", flush=True)
        rows.append(row)
    return {key: median([row[key] for row in rows]) for key in rows[0]}


def main() -> None:
    py_lines, py_bytes = source_stats("multiterm/*.py")
    rs_lines, rs_bytes = source_stats("rust/src/*.rs")
    binary = ROOT / "rust/target/release/multiterm"
    python = [str(ROOT / ".venv/bin/python"), str(ROOT / "bin/multiterm")]
    rust = [str(binary)]
    rust_env = {}
    print(f"python lines {py_lines} bytes {py_bytes}")
    print(f"rust lines {rs_lines} bytes {rs_bytes}")
    print(f"rust binary bytes {binary.stat().st_size}")
    for tabs in (1, 8):
        py = sample(python, {}, tabs)
        rs = sample(rust, rust_env, tabs)
        print(f"tabs {tabs} python {json.dumps(py)}")
        print(f"tabs {tabs} rust {json.dumps(rs)}")


if __name__ == "__main__":
    main()
