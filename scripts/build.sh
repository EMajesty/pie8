#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

build_id="${PIE8_BUILD_ID:-$(git rev-parse --short=12 HEAD 2>/dev/null || printf unknown)}"
export PIE8_BUILD_ID="$build_id"

cargo build --locked --release --package pie8-firmware --target aarch64-unknown-none
mkdir -p build
llvm-objcopy -O binary \
  target/aarch64-unknown-none/release/pie8-firmware \
  build/kernel8.img

cargo run --locked --quiet --package pie8-image-inspect -- build/kernel8.img
printf 'built build/kernel8.img (build id %s)\n' "$build_id"
