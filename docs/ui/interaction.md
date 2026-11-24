# PIE8 interaction specification

This file is normative for the proposed UI. Reference behavior is identified and
cited in `reference-study.md`; everything else is a PIE8 design decision.

## Logical controls and event model

The only logical controls are `UP DOWN LEFT RIGHT SHIFT EDIT OPTION PLAY`.
They are independent of BCM GPIO and keyboard mappings. Every physical edge is
timestamped and debounced by the existing input layer. UI input receives:

- `Press(button, time)`, once on stable down;
- `Release(button, time, held_duration)`, once on stable up;
- `Repeat(button, time)`, after 350 ms, then every 70 ms, accelerating to 35 ms
  after 1.5 s. Repeat applies only to directions and only where the focused
  action declares it safe.
- `Chord(button_set, time)`, recognized when presses fall within a 50 ms window.

Pressing modifiers does not itself alter data. A chord action fires once on the
last required press; its component presses are consumed. Release order does not
change the result. Directions held before EDIT perform navigation, not editing;
EDIT must already be down when a direction is pressed. This precedence prevents
an intended cursor move from changing a value.

Precedence, highest first: `SHIFT+OPTION+PLAY`, `SHIFT+modifier+action`,
`EDIT+OPTION`, `modifier+PLAY`, `modifier+direction`, plain buttons. A longer
valid chord suppresses its shorter subset. Opposite directions cancel movement;
perpendicular directions are processed in timestamp order. PLAY never repeats.

## Global controls

| Gesture | Action |
|---|---|
| Direction | Move cursor; repeat when held |
| SHIFT+direction | Move to spatially adjacent screen |
| EDIT on empty | Insert last-used value for that field, otherwise field default |
| EDIT+LEFT/RIGHT | Fine decrement/increment; repeat allowed |
| EDIT+UP/DOWN | Coarse increment/decrement; repeat allowed |
| EDIT+OPTION | Clear a cell; on parameters restore default |
| SHIFT+OPTION | Enter/extend rectangular selection |
| OPTION in selection | Copy selection and leave selection mode |
| EDIT+OPTION in selection | Cut selection and leave selection mode |
| SHIFT+EDIT | Paste at cursor; preview footprint before committing if it clips |
| OPTION | Cancel modal, leave child/browser, or collapse selection |
| PLAY | Contextual play/stop or audition |
| SHIFT+PLAY | Start/stop all tracks from preserved Song context |
| OPTION+PLAY | Solo focused track while held; release restores previous mask |
| OPTION+SHIFT | Mute focused track while held; release restores previous mask |
| OPTION+SHIFT+PLAY | Clear latched mute/solo state |

These intentionally track M8's global shortcuts (M8 manual pp. 7 and 68–69).
SHIFT+direction navigation and cursor state are active even during playback.

## Data entry

- Empty cells render `--`, notes `---`, and empty FX commands `--- ----`.
- Two-digit identifiers and byte-like values use uppercase hexadecimal `00–FF`.
  Decimal is used for BPM, milliseconds, percentages, dB and filesystem sizes.
- Notes are `C-0` through `B-9`; sharps use `C#4`, not flats. `OFF` is a note-off
  event and `---` is no event.
- EDIT on an empty note recalls the last note in that phrase; instrument and FX
  fields recall their own last value. A newly referenced Chain/Phrase allocates
  the lowest free ID. Allocation is shown before it becomes dirty state.
- EDIT+horizontal is the smallest semantic step; EDIT+vertical is one musical
  or display-scale step (octave for notes, 0x10 for hex, 10 for percentages).
- Values clamp unless a field explicitly declares wrapping. NaN and invalid
  object IDs are never displayable states.

## References, clone, clipboard and history

Song cells reference chains; chain cells reference phrases; phrase instrument
cells reference instruments; instruments may reference samples, tables and
grooves. Editing a referenced object edits all its uses. A `*` after an object ID
means it has more than one owner, following M8's shared-phrase warning.

On a Chain/Phrase/Instrument/Table reference, `SHIFT+OPTION`, then `EDIT`
performs a shallow clone into the lowest free ID and rewrites only the focused
reference. On a chain, double-tapping EDIT within 250 ms performs a deep clone
of the chain and every phrase it references. The status bar must say `CLONE
CHAIN ONLY?` or `DEEP CLONE CHAIN+PHRASES?`; a second EDIT confirms and OPTION
cancels. This inherits M8's clone distinction while adding confirmation.

The clipboard stores typed grid data plus source schema, never raw pointers.
Paste validates dimensions and referenced IDs. Project edits enter a bounded
undo journal; `SHIFT+LEFT+OPTION` undoes and `SHIFT+RIGHT+OPTION` redoes. These
three-button PIE8-only chords fire on OPTION and never navigate. Saving clears
redo, not undo. Loading a project clears both. Transport, audition, cursor,
screen navigation and live cue state are not journaled.

## Notes, audition and playback edits

EDIT on an empty Phrase note enters the recalled note. EDIT+horizontal changes
semitone; EDIT+vertical changes octave. PLAY while stopped plays/stops the
current Phrase/Chain/Instrument according to context. `EDIT+PLAY` momentarily
auditions the focused note or instrument and releases on either button release.
During song playback PLAY stops contextual playback; SHIFT+PLAY owns global
transport.

Edits are allowed while playing and become visible immediately. Sequencer data
uses immutable snapshots per processing block: a cell change affects the next
not-yet-scheduled event, never an event already emitted. Sample unload, sample
range mutation and project load are deferred until all voices using old data are
released or explicitly stopped.

## Context actions

- Song: PLAY toggles all tracks; LEFT+PLAY cues the row; Live mode makes PLAY
  cue the focused track at its quantization boundary.
- Chain/Phrase/Table/Groove: PLAY toggles the focused structure; SHIFT+PLAY
  returns to global song playback at the stored parent position.
- Instrument/Sample browser: EDIT enters; PLAY auditions; OPTION returns.
- Sample editor: EDIT+direction changes the selected marker/parameter; PLAY
  auditions selection. Destructive PCM operations are outside UI-001 and must
  require a named action plus confirmation.
- Project: load/overwrite/delete require a confirmation screen. Delete requires
  holding EDIT for 750 ms; no OPTION+EDIT shortcut deletes files.
- Live: grid editing and paste are disabled. EDIT arms a cue; PLAY commits it;
  OPTION cancels. This keeps performance gestures reversible.

## Unresolved design decisions

- Track count, voice count and per-track monophony await representative DSP and
  future storage-streaming measurements.
- Whether note preview defaults on, and whether EDIT+PLAY should latch.
- Undo journal depth, memory budget and whether sample metadata joins the same
  transaction history.
- Live quantization choices and whether queued project loading belongs in the
  first writable-filesystem release.
- Exact effect columns, table rate semantics and groove limits.
- Whether deep clone deduplicates repeated Phrase IDs within one Chain.
- Long-press duration after physical usability testing; 750 ms is provisional.
- Accessibility palette/font variants and localized help text scope.
- Sample editor operations once safe filesystem writes exist; UI-001 only
  specifies nondestructive playback metadata.
