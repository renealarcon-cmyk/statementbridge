# Benchmarks

This page defines *how* StatementBridge performance is measured, so that every published number is reproducible by anyone. It intentionally contains **no results yet**: numbers only enter this document when produced by the procedure below on a described machine. Fabricated or unattributed benchmarks are worse than none.

## Targets (not results)

These are the initial performance *targets* the project aims for. They are goals to verify, not claims:

- Convert a 100,000-row CSV in under 5 seconds on a modern reference machine.
- Keep peak memory proportional to input size, with a documented ceiling for the 100,000-row case.
- Produce identical output for identical input and configuration on every run (this one is enforced by tests, independent of speed).

If measurements miss a target, the real numbers get recorded here and an optimization item goes on the roadmap. The target is not edited to match the result.

## How to run

Everything is scripted:

```sh
scripts/benchmark.sh                # default: 1,000 / 10,000 / 100,000 rows
scripts/benchmark.sh 500000         # custom row counts
```

The script:

1. Builds the release CLI (`cargo build --release -p statementbridge-cli`).
2. Prints the environment (UTC date, `rustc --version`, CPU model, core count) — record this alongside any numbers.
3. Prints the release binary size, and the WASM bundle size if `apps/web/src/lib/wasm/pkg/` has been built (`scripts/build-wasm.sh`).
4. Generates deterministic synthetic CSV fixtures of the requested sizes via `scripts/generate_fixtures.py --large N` (fully invented data; byte-identical on regeneration).
5. Runs `statementbridge convert` on each file with a fixed `--imported-at` (so outputs are comparable), measuring wall-clock time and the child process's peak resident set size portably (Python `time.monotonic` + `resource.getrusage(RUSAGE_CHILDREN)`; works on Linux and macOS).
6. Prints wall time, peak RSS and derived rows/second per size. Outputs land in `bench-out/` (safe to delete).

Requirements: Python 3 (used for the fixture generator and the measurement wrapper). Linux and macOS are supported directly; on Windows, run the same conversions under an equivalent measurement tool and say so when recording results.

## What is measured

| Metric | Definition |
|---|---|
| Wall time | Elapsed time of the whole `convert` invocation: read + decode + detect + normalize + validate + serialize + write. Not engine-only time. |
| Rows/s | Input data rows divided by wall time. |
| Peak RSS | Maximum resident set size of the `convert` process, from `getrusage(RUSAGE_CHILDREN)`. |
| Binary size | `target/release/statementbridge` on the build machine (stripped; see `[profile.release]`). |
| WASM size | `statementbridge_wasm_bg.wasm` as produced by `scripts/build-wasm.sh` (size-optimized `wasm-release` profile; smaller again after `wasm-opt` when available). |

Methodology notes:

- Run each size at least three times and record the median; the first run pays filesystem cache costs.
- Do not benchmark debug builds; the script always builds release.
- The synthetic fixture has a fixed shape (six columns, ISO dates, signed amounts, balances). Real files with wider rows, worse encodings or heavy warning volume will differ; treat these numbers as a controlled baseline, not a promise about your file.

## Results

Numbers only enter this section when produced by the procedure above, together with the environment block. To add a run: execute `scripts/benchmark.sh` three times and record the medians.

### Reference run — 2026-07-22 (v0.1.0 initial tree)

Small shared-cloud VM; treat as a conservative baseline, not best-case hardware.

| Field | Value |
|---|---|
| Date (UTC) | 2026-07-22 |
| CPU | Intel(R) Xeon(R) Processor @ 2.80GHz (shared cloud VM) |
| Cores | 2 |
| RAM | 7 GiB |
| OS / kernel | Ubuntu 24.04, Linux 6.18.5 |
| rustc | 1.95.0 |
| StatementBridge commit | v0.1.0 initial tree |

| Rows | Wall time (median of 3) | Rows/s (median) | Peak RSS | Binary size | WASM size |
|---:|---:|---:|---:|---:|---:|
| 1,000 | 0.01 s | ≈67,000 | 10 MiB | 4,255,008 B (4.1 MiB) | 1,932,470 B (1.8 MiB) |
| 10,000 | 0.11 s | ≈94,600 | 18 MiB | ″ | ″ |
| 100,000 | 1.06 s | ≈94,600 | 139 MiB | ″ | ″ |

Observations, honestly stated:

- The 100,000-row target (< 5 s) is met on this modest 2-core VM: 1.06 s wall, ≈94,600 rows/s. Full-invocation time includes process startup, reading, hashing, validation and writing a 44 MB pretty-printed JSON output.
- The 1,000-row figure is dominated by process startup (~10 ms total); its rows/s is not meaningful for extrapolation.
- Peak RSS grows roughly linearly with input (18 MiB → 139 MiB from 10k → 100k rows). The whole table and the whole output are held in memory by design in v0.1; a streaming mode is a possible future optimization if very large files matter to you.
- Determinism was re-verified on the 100,000-row output: two consecutive runs produced byte-identical 44,389,098-byte files.
- WASM size is before `wasm-opt` (not installed on the reference machine); binaryen typically shrinks it further.

## Known cost factors

Understanding where time goes (from the pipeline structure; profile before optimizing):

- Per-cell work dominates: decimal parsing, date parsing and text normalization run once per populated cell of a mapped column.
- Fingerprinting is two SHA-256 computations per transaction (fingerprint + id).
- Warning volume is bounded by design (100 per code) so pathological files cannot degrade into report generation.
- Serializing pretty JSON for very large outputs is extra work the CSV export path does not do; measure both if export time matters to you.
