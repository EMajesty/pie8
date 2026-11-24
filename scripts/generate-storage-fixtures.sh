#!/usr/bin/env bash
set -euo pipefail
if [[ $# -ne 1 ]]; then echo "usage: $0 OUTPUT_DIRECTORY" >&2; exit 2; fi
output=$1
mkdir -p "$output/samples/drums" "$output/samples/long"
python3 - "$output/samples/drums/mono-pcm16.wav" \
  "$output/samples/drums/stereo-pcm16.wav" \
  "$output/samples/Long Stereo PCM24 Sample Name.wav" \
  "$output/samples/long/longer-sample.wav" \
  "$output/samples/unsupported-pcm8.wav" <<'PY'
import math, sys, wave
specs = (
    (sys.argv[1], 80, 2, 1, 48000, 4800),
    (sys.argv[2], 160, 2, 2, 48000, 4800),
    (sys.argv[3], 440, 3, 2, 48000, 4800),
    (sys.argv[4], 220, 2, 2, 48000, 48000 * 4),
    (sys.argv[5], 330, 1, 2, 48000, 4800),
)
for path, frequency, width, channels, rate, count in specs:
    with wave.open(path, "wb") as wav:
        wav.setnchannels(channels); wav.setsampwidth(width); wav.setframerate(rate)
        frames=bytearray()
        for n in range(count):
            value=int(math.sin(2*math.pi*frequency*n/rate)*((1 << (width*8-1))-1)*0.25)
            packed=value.to_bytes(width,"little",signed=True); frames.extend(packed*channels)
        wav.writeframes(frames)
PY
image="$output/pie8-fat32.img"
truncate -s 65M "$image"
printf 'label: dos\nstart=2048, size=131072, type=c\n' | sfdisk "$image" >/dev/null
mkfs.fat -F 32 -n PIE8TEST --offset 2048 "$image" 65536 >/dev/null
mmd -i "$image@@1048576" ::samples ::samples/drums ::samples/long
mcopy -i "$image@@1048576" "$output/samples/drums/mono-pcm16.wav" "$output/samples/drums/stereo-pcm16.wav" ::samples/drums/
mcopy -i "$image@@1048576" "$output/samples/Long Stereo PCM24 Sample Name.wav" "$output/samples/unsupported-pcm8.wav" ::samples/
mcopy -i "$image@@1048576" "$output/samples/long/longer-sample.wav" ::samples/long/
echo "generated $image and WAV fixtures under $output/samples"
