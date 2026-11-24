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

symbol_address() {
  llvm-nm --defined-only "$elf" | awk -v symbol="$1" '$3 == symbol { print $1 }'
}

vectors_start="$(symbol_address __vectors_start)"
vectors_end="$(symbol_address __vectors_end)"
framebuffer_start="$(symbol_address __framebuffer_start)"
framebuffer_end="$(symbol_address __framebuffer_end)"
stack_bottom="$(symbol_address __stack_bottom)"
tables_start="$(symbol_address __translation_tables_start)"
tables_end="$(symbol_address __translation_tables_end)"

if (( (0x$vectors_start & 0x7ff) != 0 )); then
  printf 'exception vectors are not 2048-byte aligned: 0x%s\n' "$vectors_start" >&2
  exit 1
fi
if (( 0x$vectors_end - 0x$vectors_start != 2060 )); then
  printf 'unexpected vector section size\n' >&2
  exit 1
fi
if (( 0x$framebuffer_end - 0x$framebuffer_start != 320 * 240 * 2 )); then
  printf 'framebuffer is not exactly 320x240 RGB565\n' >&2
  exit 1
fi
if (( 0x$framebuffer_end > 0x$stack_bottom )); then
  printf 'framebuffer overlaps stack\n' >&2
  exit 1
fi
printf 'verified vectors, 153600-byte framebuffer, and non-overlapping stack\n'
if (( (0x$tables_start & 0xfff) != 0 || 0x$tables_end - 0x$tables_start != 8192 )); then
  printf 'translation tables are not two 4K-aligned pages\n' >&2
  exit 1
fi
if (( 0x$stack_bottom >= 16 * 1024 * 1024 )); then
  printf 'firmware runtime memory exceeds the mapped 16 MiB RAM window\n' >&2
  exit 1
fi
printf 'verified two aligned translation tables inside the 16 MiB RAM map\n'
