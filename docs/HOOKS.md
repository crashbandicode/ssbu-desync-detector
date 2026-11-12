# Hook sites and files

## Hook sites (13.0.5 `.text`)

Simulation hooks read registers and memory the game is already reading.
The text hooks change only the native formatted text buffer and its length.
Each site is checked against its 64 instruction bytes at boot;
nothing is installed on another game version.

| Offset | What it reads |
|---|---|
| `0x16d6cdc` | this frame's check value and frame number |
| `0x16cf304` | a mismatch: both values, the station and the frame |
| `0x16ce7c0` | the lockstep session end and its caller |
| `0x16cc2c4` | the other console's end message |
| `0x135fd60` | a replay playback frame and its position check |
| `0x16d5ee8` | the fault-table number that arms the match countdown, the ten words behind it, and the mask |
| `0x37ad8e8` | append a reason at the formatter's return, only for the guarded communication-text caller |
| `0x377a250` | append a reason to label text set without arguments, before the pane setter |
| `0x14d96a0` | 0.5.0: match setup copying the fault table's limits; the setup block's first 64 bytes |
| `0x1363420`, `0x1363998` | 0.5.0: a KO the fault table counts, the table before and after |
| `0x16cc4c0`, `0x16ccae8` | 0.5.0: the lockstep's two peer-end paths into phase 6 |
| `0x16cced0`, `0x16d5cd0` | 0.5.0: phase 6 (reason, fault code, caller) and the session stop (summary) |

The 0.5.0 sites are described in [MATCH-TELEMETRY.md](MATCH-TELEMETRY.md).

The game sets a label's text two ways, and each has one route in
`src/dialog.rs` (`Route`): with arguments through `.37a2660` and the
formatter `.37ab750` (`args`), and without through `.377a120` and the plain
formatter `.37ae170` (`plain`). The pop-up manager's body text (`.1eef640`)
uses `plain` for a sentence with no placeholders. One function,
`append_in_frame`, is the hook body for both; the event names the route
(`communication_text … route=plain`).

`plain` is hooked inside `.377a120` at `mov w2, w0`, after the formatter
returned the count in w0 and before `bl .37a2270` at `0x377a260`. The site is
8-byte aligned: the far patch relocates MOV/ADD (sp-relative)/MOV/MOV, the
near patch only the first MOV, and the setter call stays in place. No direct
branch targets `0x377a254..0x377a25f`. The frame is 0x40 bytes of saves above
0x1120 bytes, so FP = SP+0x1150 and the zeroed 0x1000-byte buffer is
SP+0x120. Writing w0 makes the relocated `mov w2, w0` pass the new count to
the setter; `mov x0, x19` then replaces x0. Four 64-byte guards: the
prologue (`377a120`), lookup/memset/formatter call (`377a210`), the hooked
consumer (`377a250`) and the formatter's bound/terminator/return
(`37ae580`). The dated 2026-11-05 copy of the 18 card NROs has no reference
to these sites; ARCropolis calls `.377a120` (its own UI text then passes
the phrase filter like any other) and patches nothing there
(`card-scan-20261105.log` in the popup-route work folder).

The `args` hook runs before the formatter's epilogue, after its work. Its aligned
far patch relocates four instructions: MOV, ADD sp, LDP, LDP. The near
patch relocates only MOV. Neither span includes a branch or call.
The formatter has a 528-byte frame, with saved LR at SP+520 and FP at
SP+512. Only saved LR equal to `.text+37a272c` is accepted; that caller's
zeroed 0x1000-byte UTF-16 buffer is SP+528. `w20` holds the formatted unit
count. Updating saved w20 makes the relocated MOV return the new count;
the epilogue restores the caller's original x20 from its stack slot.

Six 64-byte guards cover the caller stack allocation (`37a2660`),
lookup/zeroing/capacity/call (`37a26f0`), consumer (`37a272c`),
termination/bounding (`37ad828`), formatter return (`37ad8e8`) and
formatter frame (`37ab750`). Each route is installed only when all of its
own guards pass; a failed guard is named in the install line
(`dialog_guard_failed=<site>`), which then reads `dialog=partial` or
`dialog=not_installed`, and `text_routes=` lists the installed routes. No
pointer is retained and no scratch buffer is shared. The original formatter
and UI call execute.

The dated 2026-11-03 copy of all 17 card NROs has no reference to this site
or the ABI guard offsets (`card-dialog-scan-20261103.log`,
`card-dialog-abi-scan-20261103.log`, and `card-dialog-v2-scan-20261103.log`
in the work folder). The rejected
lookup-entry design would conflict with `libelite_rematch` at `37a4450`;
this hook is in its caller and does not overlap that replacement.
The dumped Skyline inline bridge stores the original game SP at context
byte 248, matching Skyline 0.6.0's `InlineCtx.sp`. Static evidence is in
`ui-review-entry.log`, `formatter-prologue.log`, and
`skyline-patch-width.log` in the work folder. The earlier hook at
`37a272c` was rejected: an unaligned far patch replaces five instructions,
including the following BL. Its build was never installed.
Runtime coverage and text fit still require acceptance. See
`DISCONNECT-DIALOG.md`.

## Files (`sd:/ultimate/ssbu-desync-detector/`)

Sessions rotate through eight slots (`next-slot.txt` holds the last one); a
new session empties its slot first.

| File | Contents |
|---|---|
| `events.log` | Install line; per session `session`, `desync` (frame, this console's value, the other console's value, its station), `sync_end` (what ended the session), `peer_end`, `link`, `recording` (or `not_kept reason=normal_end`), `playback_mismatch`, `fault` (the number that armed the countdown, the frame, the mask, and the ten table words); `communication_text` records preparation time and reason, not proof of a rendered frame; `notice_text` records a Canceled notice or the reconnect warning, the function that asked for it (`caller`), and whether a verdict was withheld. 0.5.0 adds `match_start`, `ko`, `peer_end_path`, `session_end` and `session_summary` ([MATCH-TELEMETRY.md](MATCH-TELEMETRY.md)). At 1 MiB it becomes `events.previous.log`. |
| `frames-N.bin` | 20-byte records. Online: frame number and this console's check value. Playback: playback frame, the computed check value, computed and recorded position checksums. |
| `match-N.bin` | Only for a match that did not end normally: the game's recording as a replay file, saved one second after the session ends and replaced by the finalised entry if the game finalises it. |

About 20 bytes per frame for each file (a 5-minute match is about 350 KiB),
a replay never passes the game's 2 MB recording buffer, and the log stays
under 2 MiB. The plugin makes no heap allocations: static rings, stack
buffers, `nn::fs` files and a static 32 KiB writer stack
(`heap_allocations=0` in the log).

## Verdicts

| Verdict | Meaning |
|---|---|
| the other console left vanilla | the vanilla value at the desync frame equals this console's value |
| this console left vanilla | it equals the other console's value, or the game stopped the playback earlier because it could not reproduce this console's recording |
| both consoles left vanilla | it equals neither |
| inconclusive | the playback did not line up, or ended before the desync without disagreeing |
