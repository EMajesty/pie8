# UI reference study

Status: UI-001 research, 2026-11-24. “Documented” below means the cited manual
states the behavior. “PIE8” means a proposal, not a claim about either product.

## Primary sources

- Little Sound Dj, *Operation Manual*, version 9.2.6, 79 pages. The PDF is the
  newest English manual in the [official documentation index](https://www.littlesounddj.com/latest/documentation/):
  [LSDj_9_2_6.pdf](https://www.littlesounddj.com/latest/documentation/LSDj_9_2_6.pdf).
- Dirtywave M8, *Operation Manual*, version 6.5.2, 2026-04-21, 98 pages. This is
  the manual linked by Dirtywave's [official resources page](https://dirtywave.com/pages/resources-downloads):
  [m8_operation_manual_v20260421.pdf](https://cdn.shopify.com/s/files/1/0455/0485/6229/files/m8_operation_manual_v20260421.pdf?v=1776791699).

Page references use the printed page number when present. No graphical assets,
fonts, firmware, or manual text are copied into PIE8; names and short control
notations are used only to describe behavior.

## Comparative summary

| Topic | LSDJ 9.2.6 (documented) | M8 6.5.2 (documented) | PIE8 proposal |
|---|---|---|---|
| Controls | Game Boy D-pad, A, B, SELECT, START | Directions, SHIFT, EDIT, OPTION, PLAY | M8-labelled eight-button set exactly |
| View movement | SELECT+direction over a 5×3 map (pp. 15–16) | SHIFT+direction over a spatial view map with mini-map (pp. 6–7) | SHIFT+direction; stable map in `screen-map.md` |
| Hierarchy | Song → chain → phrase; SELECT+RIGHT drills in (pp. 11, 15–18) | Song → chain → phrase; eight independent tracks (pp. 5, 10–15) | Same object hierarchy; remember parent and cursor |
| Cell edit | A creates; A+direction changes; B+A deletes (pp. 11, 16–18) | EDIT inserts; EDIT+L/R fine; EDIT+U/D coarse; EDIT+OPTION deletes/defaults (pp. 7, 68) | M8 semantics; explicit empty/default rules |
| Selection | Manual documents LSDJ's row marking and button gestures, not a rectangular M8-style selection model | SHIFT+OPTION selects; OPTION copies; EDIT+OPTION cuts; SHIFT+EDIT pastes (pp. 7, 68–69) | M8 selection/copy/cut/paste conventions |
| Clone/reference | A,A creates new objects; SELECT+(B,A) clones some referenced objects; deep/slim chain cloning is a project option (pp. 12, 20, 32) | Sequencer objects are shared; SHIFT+OPTION then EDIT clones; double EDIT deep-clones chain plus phrases (pp. 11–20, 68) | Shared by default; explicit shallow/deep clone feedback |
| Transport | START plays/stops contextual phrase/chain/song; SELECT+START starts song (pp. 16–18) | PLAY contextual; SHIFT+PLAY continues all tracks from song context (p. 7 and view shortcuts) | M8-first contextual PLAY and global SHIFT+PLAY |
| Live use | Song screen row marking/jumps and channel mutes; no M8-style Live view is documented | Live mode cues tracks, quantizes transitions, mutes/solos, supports queued song loading (pp. 9–11) | Dedicated Live screen, quantized cue/stop; no destructive edits there |
| Tables | Per-note transpose, commands and amplitude sequences; table loops (pp. 26–27, 49) | Per-step transpose and three FX columns; phrase/table integration (pp. 24–25) | M8-shaped rows/FX, with an explicit rate and loop marker |
| Groove | Sixteen-step tick sequence in Groove screen (pp. 30–31) | Sixteen step tick sequence and interpolation workflow (pp. 26–27) | Sixteen steps, ticks plus derived timing preview |
| Samples | KIT instruments use ROM sample kits and two note columns (pp. 22–24, appendix A) | SD sample browser, Sampler instrument and destructive Sample Editor (pp. 54–57) | RAM-loaded WAV library, sampler parameters, separate editor; edits are initially nondestructive metadata |
| Undo | No general undo/redo contract documented | Paste can undo some fill/paste actions; song snapshots exist in Mixer/Project (pp. 15, 30, 38) | Bounded project edit history; transport excluded |

## Important documented details

LSDJ uses nine principal screens. Its central row is Song, Chain, Phrase,
Instrument and Table; Project/Synth/Wave are above and Groove below (p. 15).
Song contains chains, chains contain phrases, and phrases contain notes. Empty
Song/Chain cells can create a new object with two A taps (p. 12). Phrase has
note, instrument, command and value columns, and objects may be reused across
channels (pp. 17–18). The manual distinguishes deep chain cloning (clone its
phrases) from slim cloning (reuse them) in Project settings (p. 32).

M8 similarly uses Song, Chain and Phrase, but Song has eight independent tracks;
each stops/loops when it reaches an empty cell and track positions may diverge
(p. 10). An asterisk warns when a phrase is referenced elsewhere (p. 14).
Selection, clipboard, clone and transport gestures are largely global. Live mode
cues a selected chain at a configurable quantization boundary, with per-track
stop, mute and solo (pp. 10–11). Queued project loading waits for a chain
boundary (p. 9). The browser previews songs, samples or instruments with PLAY.

## Priority and intentional differences

When references conflict, PIE8 follows M8 because its physical control names
and eight-button count are identical. PIE8 does not reproduce the M8 screen
layout pixel-for-pixel, its theme, icons, fonts, product-specific synths, effects,
touchscreen actions, or fixed eight-track engine. LSDJ's compact hierarchy and
sixteen-row rhythm remain influential, but Game Boy A/B/SELECT/START chords are
translated to M8 semantics instead of emulated.

PIE8 does not promise LSDJ or M8 project-file compatibility. Track count,
polyphony, streaming policy, effect inventory, sample editing operations, undo
depth and live quantization choices remain product decisions.
