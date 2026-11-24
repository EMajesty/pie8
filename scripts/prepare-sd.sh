#!/usr/bin/env bash
set -euo pipefail

firmware_rev=cd2a1f1b67266d85f8c00dd9c2190fb8f7b48465
base_url="https://raw.githubusercontent.com/raspberrypi/firmware/${firmware_rev}/boot"

if [[ $# -ne 1 ]]; then
  printf 'usage: %s <empty-mounted-FAT-boot-directory>\n' "$0" >&2
  exit 2
fi

destination="$(realpath -- "$1")"
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
kernel="$repo_root/build/kernel8.img"

if [[ ! -d "$destination" || ! -w "$destination" ]]; then
  printf 'destination must be an existing writable directory: %s\n' "$destination" >&2
  exit 1
fi
if [[ ! -f "$kernel" ]]; then
  printf 'missing %s; run ./scripts/build.sh first\n' "$kernel" >&2
  exit 1
fi
if find "$destination" -mindepth 1 -maxdepth 1 -print -quit | grep -q .; then
  printf 'refusing non-empty destination: %s\n' "$destination" >&2
  exit 1
fi

download_dir="$(mktemp -d)"
trap 'rm -rf -- "$download_dir"' EXIT

for file in bootcode.bin start.elf fixup.dat LICENCE.broadcom; do
  curl --fail --location --silent --show-error \
    --output "$download_dir/$file" "$base_url/$file"
done

(
  cd "$download_dir"
  sha256sum --check <<'CHECKSUMS'
3d04e3f6b3c62981c38a31429cae62aab14cfd99dedd56403c36216ac08bb16d  bootcode.bin
8fd31e816477c455b9ae313787e9a869cacc32e476eac6653768063477368102  start.elf
34d4908502236975519a976f802822bacff5a9246cfce9492b8034b8de491cc2  fixup.dat
c7283ff51f863d93a275c66e3b4cb08021a5dd4d8c1e7acc47d872fbe52d3d6b  LICENCE.broadcom
CHECKSUMS
)

install -m 0644 "$download_dir/bootcode.bin" "$destination/bootcode.bin"
install -m 0644 "$download_dir/start.elf" "$destination/start.elf"
install -m 0644 "$download_dir/fixup.dat" "$destination/fixup.dat"
install -m 0644 "$download_dir/LICENCE.broadcom" "$destination/LICENCE.broadcom"
install -m 0644 "$kernel" "$destination/kernel8.img"
install -m 0644 "$repo_root/boot/config.txt" "$destination/config.txt"

printf 'installed PIE8 boot files into %s\n' "$destination"
printf 'Raspberry Pi firmware revision: %s\n' "$firmware_rev"
