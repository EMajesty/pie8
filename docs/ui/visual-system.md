# Visual system

The native canvas is 320×240 RGB565 landscape. Layout coordinates are integral;
the renderer has no subpixel positions, alpha requirement or heap dependency.
Desktop presentation scales by an integer and uses nearest-neighbor sampling.

## Grid and typography

PIE8 uses an original 5×7 bitmap glyph design stored as row masks in the mockup
generator. Production glyph cells are 6×8 pixels (5×7 ink plus one-pixel right
and lower spacing). Large labels use integer 2× glyph scaling, never smoothing.
ASCII uppercase, digits, tracker punctuation and a small icon set are required;
unsupported Unicode renders a boxed question mark.

The usable frame has 4 px outer margins. Header is y=4..19, column headings
y=22..31, content y=32..191, status y=208..223, and mini-map/button hints
y=226..235. Phrase rows are exactly 10 px high, giving sixteen visible rows.
Other lists may use 9 or 10 px rows but must align their baselines consistently.

## Palette roles

Render calls use semantic roles, not RGB literals:

| Role | Default RGB565 intent | Use |
|---|---|---|
| `background` | near-black blue | Canvas |
| `panel` | dark navy | Header/status/panels |
| `text` | warm light gray | Primary values |
| `muted` | slate | Empty cells, disabled fields |
| `accent` | cyan | Titles, active parameters |
| `secondary` | magenta | References and instrument IDs |
| `warning` | amber | Shared references, clipping |
| `danger` | coral red | Confirmations/errors only |
| `cursor_bg` / `cursor_fg` | light / dark | Focused cell, solid block |
| `selection_bg` | deep blue | Selected region |
| `playhead` | lime | Current playback row/marker |
| `beat_major` / `beat_minor` | two panel shades | Row subdivisions |
| `meter_low/mid/high` | green/amber/red | Level meters |
| `waveform` / `waveform_zero` | cyan/slate | Sample plots |

Theme data is one validated struct. Contrast pairs are tested; changing a theme
cannot change geometry. RGB565 conversion happens at the final framebuffer API.

## Tracker primitives

- Header: screen name left, object ID and shared `*` beside it, transport state,
  tempo and battery/storage flags right.
- Cursor: filled `cursor_bg` rectangle with `cursor_fg` ink. During value edit it
  gains a one-pixel accent underline.
- Selection: `selection_bg` cells plus an accent outline around the complete
  rectangle; cursor remains visible above it.
- Playback: one-pixel playhead bar at row left and `playhead` text. It never
  replaces cursor or selection color.
- Beats: rows 0,4,8,C use `beat_major`; other even rows use `beat_minor`.
- Empty values: muted `--`, `---`, or `....`; never blank, so focus is legible.
- Status: left side describes the focused field/value in decimal where useful;
  right side shows modifiers and pending confirmation.
- Mini-map: small nodes at bottom-right; current node accent, available edges
  text, unavailable edge muted.
- Waveforms: min/max vertical envelope per x pixel, a zero axis, range shading,
  loop markers and playhead. No misleading line interpolation at overview scale.
- Meters: 4 px segmented bars with peak-hold tick; red is reserved for actual
  clipping, not merely a high healthy level.

The mockups are original compositions. They do not use or trace LSDJ/M8 fonts,
icons, palettes, logos or screenshots.
