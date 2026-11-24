#!/usr/bin/env bash
set -euo pipefail
if [[ $# -ne 1 ]]; then echo "usage: $0 OUTPUT_DIRECTORY" >&2; exit 2; fi
output=$1
mkdir -p "$output/samples/drums"
python3 - "$output/samples/drums/kick.wav" "$output/samples/Long Sample Name.wav" <<'PY'
import math, sys, wave
for path, frequency, width in ((sys.argv[1], 80, 2), (sys.argv[2], 440, 3)):
    with wave.open(path, "wb") as wav:
        wav.setnchannels(2); wav.setsampwidth(width); wav.setframerate(48000)
        frames=bytearray()
        for n in range(4800):
            value=int(math.sin(2*math.pi*frequency*n/48000)*((1 << (width*8-1))-1)*0.25)
            packed=value.to_bytes(width,"little",signed=True); frames.extend(packed+packed)
        wav.writeframes(frames)
PY
image="$output/pie8-fat32.img"
truncate -s 66M "$image"
printf 'label: dos\nstart=2048, size=131072, type=c\n' | sfdisk "$image" >/dev/null
mkfs.fat -F 32 -n PIE8TEST --offset 2048 "$image" 65536 >/dev/null
mmd -i "$image@@1048576" ::samples ::samples/drums
mcopy -i "$image@@1048576" "$output/samples/drums/kick.wav" ::samples/drums/
mcopy -i "$image@@1048576" "$output/samples/Long Sample Name.wav" ::samples/
echo "generated $image and WAV fixtures under $output/samples"
