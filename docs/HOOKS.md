# Hook sites and files

## Hook sites (13.0.5 `.text`)

Simulation hooks read registers and memory the game is already reading.
The unreleased UI hook changes only the native formatted text buffer and
its length. Each site is checked against its 64 instruction bytes at boot;
nothing is installed on another game version.

| Offset | What it reads |
|---|---|
| `0x16d6cdc` | this frame's check value and frame number |
| `0x16cf304` | a mismatch: both values, the station and the frame |
| `0x16ce7c0` | the lockstep session end and its caller |
| `0x16cc2c4` | the other console's end message |
| `0x135fd60` | a replay playback frame and its position check |
| `0x37ad8e8` | unreleased: append a reason at the formatter's return, only for the guarded communication-text caller |

The hook runs before the formatter's epilogue, after its work. Its aligned
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
formatter frame (`37ab750`). All must pass before installing the hook.
A failed guard is named in the install line. No pointer is retained and
no scratch buffer is shared. The original formatter and UI call execute.

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
| `events.log` | Install line; per session `session`, `desync` (frame, this console's value, the other console's value, its station), `sync_end` (what ended the session), `peer_end`, `link`, `recording` (or `not_kept reason=normal_end`), `playback_mismatch`; unreleased `communication_text` records preparation time and reason, not proof of a rendered frame. At 1 MiB it becomes `events.previous.log`. |
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
