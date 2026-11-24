#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
python3 render.py
for source in *.svg; do
    magick -background none "$source" "${source%.svg}.png"
done
echo "rendered 10 native PNG previews"
