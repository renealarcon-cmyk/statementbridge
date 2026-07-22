#!/usr/bin/env bash
# Run every linter/formatter check (Rust + web).
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> rustfmt"
cargo fmt --all --check

echo "==> clippy"
cargo clippy --workspace --all-targets -- -D warnings

if [[ -d apps/web/node_modules ]]; then
  echo "==> eslint + prettier + tsc"
  pnpm --dir apps/web lint
  pnpm --dir apps/web format:check
  pnpm --dir apps/web typecheck
else
  echo "==> skipping web lint (run 'pnpm --dir apps/web install' first)"
fi

echo "==> lint clean"
