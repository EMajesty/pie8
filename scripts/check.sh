#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

cargo fmt --all --check
cargo test --locked --workspace --exclude pie8-firmware --exclude pie8-rpi
cargo clippy --locked --workspace --exclude pie8-firmware --exclude pie8-rpi --all-targets -- -D warnings
cargo clippy --locked --package pie8-rpi --package pie8-firmware \
  --target aarch64-unknown-none -- -D warnings
./scripts/build.sh

elf=target/aarch64-unknown-none/release/pie8-firmware
llvm-readelf --file-header --sections --symbols "$elf"
entry="$(llvm-readelf --file-header "$elf" | awk '/Entry point address:/ { print $4 }')"
if [[ "$entry" != "0x80000" ]]; then
  printf 'unexpected ELF entry point: %s\n' "$entry" >&2
  exit 1
fi
printf 'verified AArch64 ELF entry point at %s\n' "$entry"
