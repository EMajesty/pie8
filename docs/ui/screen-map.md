# Spatial screen map

`SHIFT+direction` moves one edge in this graph. There is no wrap. If an edge is
absent, the chord does nothing and the mini-map edge flashes. Plain directions
always remain cursor movement.

```text
                         [PROJECT]──R──[SETTINGS]
                             │
                             D
                             │
 [LIVE]──R──[SONG]──R──[CHAIN]──R──[PHRASE]──R──[INSTRUMENT]
             │          │          │               │
             D          D          D               D
             │          │          │               │
          [MIXER]    [GROOVE]    [TABLE]──R──[EFFECTS]
             │                                     │
             D                                     D
             │                                     │
        [DIAGNOSTICS]                    [SAMPLE BROWSER]──R──[SAMPLE EDITOR]
```

Opposite directions traverse the same edge. The Project edge is reached from
Chain with SHIFT+UP, deliberately echoing the central sequencing hierarchy of
LSDJ and M8 without copying either complete map. Diagnostics is below Mixer,
outside the musical path; it is omitted from release builds unless explicitly
enabled.

## Explicit transitions

| From | SHIFT+UP | SHIFT+DOWN | SHIFT+LEFT | SHIFT+RIGHT |
|---|---|---|---|---|
| Live | — | — | — | Song |
| Song | — | Mixer | Live | Chain |
| Chain | Project | Groove | Song | Phrase |
| Phrase | — | Table | Chain | Instrument |
| Instrument | — | Sample browser | Phrase | — |
| Table | Phrase | — | — | Effects |
| Groove | Chain | — | — | — |
| Mixer | Song | Diagnostics | — | — |
| Effects | — | Sample browser | Table | — |
| Project | — | Chain | — | Settings |
| Settings | — | — | Project | — |
| Sample browser | Effects | — | — | Sample editor |
| Sample editor | — | — | Sample browser | — |
| Diagnostics | Mixer | — | — | — |

`EDIT` may also enter the object under the cursor: Song chain → Chain, Chain
phrase → Phrase, Phrase instrument → Instrument, Instrument sample → Sample
browser, browser file → Sample editor. OPTION returns through that exact entry
edge rather than consulting the spatial map.

## Context preservation

Each screen owns a persistent cursor, scroll origin and selection anchor per
project. Object screens additionally keep a navigation frame:

```text
parent screen + parent object ID + parent row/column + child object ID
```

Entering Chain from Song therefore restores the chain row last used for that
chain; OPTION returns to the exact Song cell. SHIFT navigation preserves this
frame but does not push another. Changing object with OPTION+UP/DOWN replaces
only the child ID and keeps the parent. Project load resets frames to valid
defaults. Deleted IDs invalidate frames deterministically and return to the
nearest valid parent.

## Screen responsibilities

- **Song:** vertical arrangement of chain references per track; track cue state.
- **Chain:** sixteen phrase references with transpose.
- **Phrase:** sixteen note/instrument/FX rows.
- **Instrument:** engine-independent parameters; Sampler is the first type.
- **Table:** sixteen modulation/FX rows.
- **Groove:** sixteen timing steps and derived subdivision preview.
- **Sample browser/editor:** read-only filesystem navigation; nondestructive
  playback range, loop, gain and metadata editing.
- **Mixer/Effects:** track levels/pan/mute/solo and send/return parameters.
- **Project/Settings:** song persistence metadata versus device-wide behavior.
- **Live:** quantized track cue/stop and safe performance macros.
- **Diagnostics:** current hardware pages, isolated from song state.

Track count and polyphony are represented by runtime model capabilities; the
map and input contract do not fix either value.
