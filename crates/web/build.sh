#!/usr/bin/env bash
# Build the static web page into crates/web/dist/.
#
# Needs: the wasm32-unknown-unknown target, `wasm-bindgen` (the CLI version must
# match the wasm-bindgen crate in Cargo.lock), and optionally `wasm-opt`.
#
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version "$(crates/web/build.sh --bindgen-version)"
#   crates/web/build.sh && python3 -m http.server -d crates/web/dist
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"

if [[ "${1:-}" == "--bindgen-version" ]]; then
  grep -A1 '^name = "wasm-bindgen"$' "$root/Cargo.lock" | sed -n 's/^version = "\(.*\)"/\1/p'
  exit 0
fi

dist="$here/dist"
target="$root/target/wasm32-unknown-unknown/release"

# Size-tuned release build; set via env so native builds are unaffected.
CARGO_PROFILE_RELEASE_OPT_LEVEL=z \
CARGO_PROFILE_RELEASE_LTO=true \
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
CARGO_PROFILE_RELEASE_PANIC=abort \
  cargo build --manifest-path "$root/Cargo.toml" -p mnemonic-web \
    --release --target wasm32-unknown-unknown

rm -rf "$dist"
mkdir -p "$dist"
cp -R "$here/www/." "$dist/"
wasm-bindgen --target web --no-typescript --out-dir "$dist/pkg" "$target/mnemonic_web.wasm"

if command -v wasm-opt >/dev/null; then
  wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int \
    -o "$dist/pkg/mnemonic_web_bg.wasm" "$dist/pkg/mnemonic_web_bg.wasm"
else
  echo "note: wasm-opt not found; skipping size optimization" >&2
fi

# Cache-bust the page's assets with a content hash.
hash="$(cat "$dist/app.js" "$dist/style.css" "$dist/pkg/"* | sha256sum | cut -c1-10)"
sed -i.bak "s/__BUILD__/$hash/g" "$dist/index.html" "$dist/app.js" && rm -f "$dist"/*.bak

ls -l "$dist" "$dist/pkg"
