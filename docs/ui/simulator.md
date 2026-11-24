# Desktop simulator architecture

The simulator is a future host executable around the same portable renderer and
input reducer used by firmware. It is not a second UI implementation.

```text
keyboard/replay ──> logical button edges ──> debounce/chord/repeat reducer
                                                   │
project model ──> screen controller ───────────────┤
                                                   v
                                      portable RGB565 renderer
                                                   │
                                320×240 framebuffer + frame hash
                                                   │
                          desktop texture, PNG capture, regression oracle
```

## Boundaries

`pie8-core` should own logical buttons, timestamped events, chord resolution,
screen state reducers, geometry, palette roles and a `Surface` trait accepting
clipped RGB565 primitives. It must not know GPIO, SDL, files or wall-clock time.
Firmware adapts existing button events and framebuffer. The host supplies a
manual monotonic clock and an image/presentation adapter.

Proposed host presentation library: **minifb** first, because it has a small API
for a fixed pixel buffer, keyboard state and integer-sized window. Evaluate
**winit + pixels** only if robust controller/hidpi handling is needed; it brings a
larger dependency graph. SDL2 is mature but introduces a native library. None is
linked into firmware, and library selection is deferred until implementation.

## Keyboard map

| Logical | Primary | Alternate |
|---|---|---|
| UP/DOWN/LEFT/RIGHT | Arrow keys | W/S/A/D |
| SHIFT | Left Shift | J |
| EDIT | X | K |
| OPTION | Z | L |
| PLAY | Space | Enter |

The host reports key edges, not characters. OS key repeat is disabled; the
portable reducer generates repeats from replay time. Multiple keys remain down
simultaneously, so chords match hardware.

## Deterministic replay and regression

Replay is a UTF-8 text fixture with integer microseconds and stable button names:

```text
0 PRESS SHIFT
10000 PRESS RIGHT
30000 RELEASE RIGHT
40000 RELEASE SHIFT
100000 PRESS EDIT
120000 RELEASE EDIT
```

The runner starts from a versioned model fixture, advances a fake clock, renders
after each event and at requested checkpoints, then records model digest,
RGB565 framebuffer hash and optional PNG. No sleep, host locale, font service or
filesystem enumeration may influence results. Tests cover chord precedence,
repeat boundaries, navigation/context restoration, selection/clipboard, shared
clone confirmation and exact screenshots.

Screenshot capture writes the native 320×240 frame; presentation scaling is
never captured. The desktop window allows integer 1×–6× scaling with black
letterboxing and nearest-neighbor sampling. A debug overlay may show held keys
outside the native framebuffer so it cannot affect golden images.

## Proposed crate split

- `pie8-core`: reusable input semantics and tracker domain types.
- future `pie8-ui`: no_std controller/renderer, mock project fixtures, no MMIO.
- future `tools/pie8-sim`: desktop event/presentation and PNG/replay CLI.
- `firmware`: GPIO/timer adapters and the existing RGB565 framebuffer surface.

Do not create these crates until production UI implementation begins; UI-001 is
specification only.

## Open simulator decisions

- minifb versus winit/pixels after NixOS and macOS input-latency prototypes;
- committed PNG goldens versus lossless RGB565 dumps plus generated PNGs;
- whether MIDI input belongs in the simulator or a separate host adapter;
- accessibility alternatives for color and small bitmap text.
