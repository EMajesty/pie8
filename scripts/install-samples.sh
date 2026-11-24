#!/usr/bin/env bash
set -euo pipefail
if [[ $# -lt 2 ]]; then echo "usage: $0 MOUNTED_BOOT_PARTITION WAV_OR_DIRECTORY..." >&2; exit 2; fi
destination=$1
shift
if [[ ! -d "$destination" ]]; then echo "not a directory: $destination" >&2; exit 1; fi
mkdir -p "$destination/samples"
for source in "$@"; do cp -R -- "$source" "$destination/samples/"; done
echo "copied samples to $destination/samples (no formatting performed)"
