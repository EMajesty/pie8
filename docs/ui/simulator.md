# Desktop simulator architecture and operation

UI-002 implements `tools/pie8-sim` around the exact portable model, controller,
input reducer and renderer intended for firmware. It is not a second UI.

```text
keyboard/replay ──> logical button edges ──> held modifier/repeat reducer
                                                   │
project model ──> screen controller ───────────────┤
                                                   v
                                      portable RGB565 renderer
                                                   │
                                320×240 framebuffer + frame hash
                                                   │
                          minifb texture, PNG capture, regression oracle
```

## Launch

minifb's X11 backend is the initial presentation layer. It has a small fixed-
framebuffer API; its dependency and the host PNG encoder do not enter portable
or firmware crates. Launch on NixOS:

```sh
nix develop path:. --command cargo run -p pie8-sim
```

The flake sets `LD_LIBRARY_PATH` with `pkgs.lib.makeLibraryPath` for X11,
Xcursor, Xext, Xfixes, Xi, Xrandr, Xrender and XCB. This lets minifb/x11-dl load
its versioned shared objects directly from the development shell; no `nix-ld`
or system-wide NixOS configuration is required. A graphical window still
requires a working X11 session and `DISPLAY`.

Escape closes the window. Native RGB565 is converted only for presentation.
Scaling is nearest-neighbor and integer; `--scale 1|2|4|8` selects a supported
window scale (default 4).

## Keyboard map

| Logical | Primary | Alternate |
|---|---|---|
| UP/DOWN/LEFT/RIGHT | Arrow keys | W/S/A/D |
| SHIFT | Left Shift | J |
| EDIT | X | K |
| OPTION | Z | L |
| PLAY | Space | Enter |

The host reports key edges, not characters. OS repeat is disabled; `pie8-ui`
generates repeat after 350 ms, every 70 ms, accelerating to 35 ms after 1.5 s.
Multiple keys remain down simultaneously, preserving hardware chord semantics.

## Deterministic replay and screenshots

Replay is UTF-8 text with monotonically increasing integer microseconds:

```text
0 PRESS SHIFT
10000 PRESS RIGHT
20000 RELEASE RIGHT
30000 RELEASE SHIFT
```

Run the committed editing session and save its final native frame:

```sh
nix develop path:. --command cargo run -p pie8-sim -- \
  --headless --replay tools/pie8-sim/replays/edit-session.txt \
  --screenshot /tmp/pie8-replay.png
```

Generate captures of every implemented core screen:

```sh
nix develop path:. --command cargo run -p pie8-sim -- \
  --headless --screenshot-dir docs/ui/screenshots
```

`--screen song|chain|phrase|instrument` chooses the initial screen. Screenshot
capture is always native 320×240 regardless of window scale. The committed
replay's RGB565 FNV-1a hash is `9ab647b19df200e0` and is asserted by tests. The
runner uses a fake clock; sleep, locale, fonts and directory enumeration cannot
affect the result.

## Crate and hardware boundaries

- `pie8-core`: clipped RGB565 surface and existing portable facilities.
- `pie8-ui`: no_std domain model, input state, controller, renderer and example.
- `tools/pie8-sim`: minifb, keyboard edges, replay parser and PNG output.
- `firmware`: unchanged in UI-002.

`pie8_ui::adapter::Platform` is the minimal next-step firmware adapter. It
provides monotonic microseconds, timestamped logical button events, a native
`[u16; 320*240]` framebuffer and `present()`. The working generic timer, button
state machine, static framebuffer and display region upload can implement it
without placing MMIO in `pie8-ui` or replacing diagnostics.

## Open simulator decisions

- committed PNG goldens versus lossless RGB565 dumps plus generated PNGs;
- whether MIDI input belongs here or in a separate host adapter;
- accessibility alternatives for color and small bitmap text;
- input latency comparison with winit/pixels if minifb becomes limiting.
