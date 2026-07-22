#!/usr/bin/env bash
# Build the StatementBridge WebAssembly package for the web app.
#
# Output: apps/web/src/lib/wasm/pkg/ (gitignored; regenerate any time)
#
# Preferred path: wasm-pack (handles wasm-bindgen versioning automatically):
#   cargo install wasm-pack   # or: npm i -g wasm-pack
#
# Fallback path (no wasm-pack): cargo + a wasm-bindgen CLI matching the
# version in Cargo.lock:
#   cargo install wasm-bindgen-cli --version "$(scripts/wasm-bindgen-version.sh)"
#
# Environments that cannot download the rust-std for wasm32 (offline mirrors,
# sandboxes) can set SB_BUILD_STD=1 to compile std from source instead
# (requires the rust-src component or a vendored library/ tree in the sysroot).
set -euo pipefail
cd "$(dirname "$0")/.."

OUT="apps/web/src/lib/wasm/pkg"
PROFILE_DIR="target/wasm32-unknown-unknown/wasm-release"
BUILD_STD_ARGS=()
if [[ "${SB_BUILD_STD:-0}" == "1" ]]; then
  export RUSTC_BOOTSTRAP=1
  BUILD_STD_ARGS=(-Zbuild-std=std,panic_abort)
else
  rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
fi

if command -v wasm-pack >/dev/null 2>&1 && [[ "${SB_BUILD_STD:-0}" != "1" ]]; then
  echo "==> building with wasm-pack"
  wasm-pack build crates/statementbridge-wasm \
    --release --target web --out-dir "../../$OUT" --no-pack
else
  echo "==> building with cargo + wasm-bindgen"
  cargo build --profile wasm-release --target wasm32-unknown-unknown \
    -p statementbridge-wasm "${BUILD_STD_ARGS[@]}"
  command -v wasm-bindgen >/dev/null 2>&1 || {
    echo "wasm-bindgen CLI not found. Install the version matching Cargo.lock:" >&2
    echo "  cargo install wasm-bindgen-cli --version \"\$(scripts/wasm-bindgen-version.sh)\"" >&2
    exit 1
  }
  mkdir -p "$OUT"
  wasm-bindgen "$PROFILE_DIR/statementbridge_wasm.wasm" \
    --target web --out-dir "$OUT" --no-typescript-eslint 2>/dev/null || \
  wasm-bindgen "$PROFILE_DIR/statementbridge_wasm.wasm" \
    --target web --out-dir "$OUT"
fi

# Optional size optimization when binaryen is available.
if command -v wasm-opt >/dev/null 2>&1; then
  echo "==> optimizing with wasm-opt -Os"
  wasm-opt -Os -o "$OUT/statementbridge_wasm_bg.wasm" "$OUT/statementbridge_wasm_bg.wasm"
fi

ls -la "$OUT" | sed -n '1,12p'
echo "==> wasm package ready in $OUT"
