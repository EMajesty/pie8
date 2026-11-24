# Screen specifications and mockups

Mockups are editable 320×240 SVG sources generated from screen data and an
original 5×7 pixel font in [`mockups/render.py`](mockups/render.py). Matching
lossless PNG files are the rendered previews. Regenerate both with:

```sh
docs/ui/mockups/render-previews.sh
```

The script rejects any output that is not exactly 320×240. SVG contains only
integer-coordinate rectangles and lines, so it is directly inspectable and can
also serve as a renderer geometry reference. The mockups show realistic but
non-normative musical data.

## Core screens

| Preview | Purpose and key columns |
|---|---|
| [Song](mockups/song.png) | Arrangement rows × tracks; Chain IDs, mute/cue/play state |
| [Chain](mockups/chain.png) | 16 Phrase references and signed transpose; shared marker |
| [Phrase](mockups/phrase.png) | 16 rows of note, instrument, velocity and three FX slots |
| [Sampler instrument](mockups/sampler-instrument.png) | Sample reference, pitch, range, loop, envelope, gain/pan |
| [Table](mockups/table.png) | Transpose and three command/value pairs over 16 rows |
| [Groove](mockups/groove.png) | Tick counts over 16 rows plus timing visualization |

Phrase guarantees all sixteen rows simultaneously. Song and Phrase keep data
columns visually dominant; help text stays in the status line. A shared-object
asterisk and status explanation make reference semantics discoverable.

## Additional screens

| Preview | Purpose and key controls |
|---|---|
| [Sample browser](mockups/sample-browser.png) | Path, directories/WAV metadata, memory and load/audition status |
| [Sample editor](mockups/sample-editor.png) | Waveform, start/end/loop markers, zoom and nondestructive parameters |
| [Mixer](mockups/mixer.png) | Per-track meters, faders, pan, mute/solo and master headroom |
| [Project settings](mockups/project-settings.png) | Project name/save/load, tempo, live quantize and dirty state |

Effects follows the parameter-list grammar of Instrument. Settings follows
Project but contains device-wide theme, input, display and future MIDI choices.
Live reuses Song geometry with large cue/playing/stopped state blocks and hides
editing affordances. Diagnostics retains its existing renderer until the shared
UI renderer is implemented, but is entered only through the map's diagnostic
edge.

## Screen-specific behavior

### Song and Live

Song scrolls vertically with track columns fixed. OPTION+UP/DOWN jumps sixteen
rows. PLAY toggles song playback; LEFT+PLAY cues the selected row. EDIT on `--`
creates/reuses a Chain ID. Live is a performance projection: PLAY queues the
focused row/track, SHIFT+PLAY stops it, and OPTION cancels an armed cue.

### Chain, Phrase, Table and Groove

All have exactly sixteen logical rows (`0–F`) even if future objects support
other lengths. Chain maps Phrase IDs and transpose. Phrase stores notes,
instrument references and FX. Table stores modulation/FX independent of notes.
Groove values are ticks per step and show total/average timing. Common selection
and clipboard gestures apply.

### Instrument and effects

Instrument presents a type-neutral header and a type-specific body. Sampler is
specified now; future instruments plug into the same bounded parameter schema.
EDIT changes values, EDIT+PLAY auditions, OPTION+UP/DOWN changes instrument.
Effects is project routing/return configuration, not per-row phrase FX help.

### Samples

Browser enumeration is bounded and incremental to preserve input/audio work.
PLAY auditions a supported WAV; EDIT loads or enters; OPTION returns. Editor
changes playback metadata initially: start/end, loop, gain, pan and root note.
No write or destructive waveform operation is implied by these mockups.

### Mixer, Project and Settings

Mixer faders use fine/coarse editing and show peak hold/clipping. Project owns
song-level tempo and live quantization plus explicit load/save actions. Settings
owns device defaults. Save is unavailable until filesystem writes exist; its
mockup state is a future interaction specification, not current firmware ability.
