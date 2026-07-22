#!/usr/bin/env bash
# Run every test in the repository (Rust + web).
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> cargo test (workspace)"
cargo test --workspace

if [[ -d apps/web/node_modules ]]; then
  echo "==> web tests (vitest)"
  pnpm --dir apps/web test -- --run
else
  echo "==> skipping web tests (run 'pnpm --dir apps/web install' first)"
fi

echo "==> all tests passed"
