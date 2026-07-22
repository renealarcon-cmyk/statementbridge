#!/usr/bin/env bash
# Reproducible benchmarks for StatementBridge.
#
# Generates synthetic CSV files (1k / 10k / 100k rows), runs the release CLI
# on them, and prints wall-clock time, throughput and peak memory (RSS).
# Nothing here fabricates numbers: run it on your machine and read the output.
#
# Usage: scripts/benchmark.sh [rows ...]   (default: 1000 10000 100000)
set -euo pipefail
cd "$(dirname "$0")/.."

ROWS=("${@:-1000 10000 100000}")
if [[ $# -eq 0 ]]; then ROWS=(1000 10000 100000); fi

echo "==> building release CLI"
cargo build --release -p statementbridge-cli >/dev/null
BIN=target/release/statementbridge

echo "==> environment"
echo "    date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "    rustc: $(rustc --version)"
echo "    cpu: $(grep -m1 'model name' /proc/cpuinfo 2>/dev/null | cut -d: -f2- | xargs || sysctl -n machdep.cpu.brand_string 2>/dev/null || echo unknown)"
echo "    cores: $(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo unknown)"

echo "==> binary size"
ls -l "$BIN" | awk '{printf "    %s bytes (%.1f MiB)\n", $5, $5/1048576}'
if [[ -f apps/web/src/lib/wasm/pkg/statementbridge_wasm_bg.wasm ]]; then
  ls -l apps/web/src/lib/wasm/pkg/statementbridge_wasm_bg.wasm | \
    awk '{printf "    wasm: %s bytes (%.1f MiB)\n", $5, $5/1048576}'
fi

mkdir -p bench-out
for n in ${ROWS[@]}; do
  f="fixtures/generated/large_${n}.csv"
  [[ -f "$f" ]] || python3 scripts/generate_fixtures.py --large "$n" >/dev/null
  echo "==> convert ${n} rows ($(du -h "$f" | cut -f1 | xargs))"
  # Portable measurement (Linux/macOS): wall clock + child peak RSS via python.
  BIN="$BIN" F="$f" N="$n" python3 - <<'PY'
import os, resource, subprocess, sys, time
bin_, f, n = os.environ["BIN"], os.environ["F"], int(os.environ["N"])
t0 = time.monotonic()
r = subprocess.run([bin_, "convert", f, "--output", f"bench-out/out_{n}.json",
                    "--imported-at", "2026-01-01T00:00:00Z", "-q"])
wall = time.monotonic() - t0
if r.returncode != 0:
    sys.exit(r.returncode)
ru = resource.getrusage(resource.RUSAGE_CHILDREN)
rss_kib = ru.ru_maxrss  # KiB on Linux, bytes on macOS
if sys.platform == "darwin":
    rss_kib //= 1024
print(f"    wall: {wall:.2f}s  ·  peak RSS: {rss_kib // 1024} MiB  ·  {n / max(wall, 0.001):,.0f} rows/s")
PY
done
echo "==> outputs in bench-out/ (delete freely)"
