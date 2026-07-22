#!/usr/bin/env bash
# Print the wasm-bindgen version pinned in Cargo.lock (the CLI must match).
set -euo pipefail
cd "$(dirname "$0")/.."
awk '/^name = "wasm-bindgen"$/{getline; gsub(/version = |"/, ""); print; exit}' Cargo.lock
