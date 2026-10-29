# Hook sites and files

## Hook sites (13.0.5 `.text`)

Every hook only reads registers and memory the game is already reading at
that instruction. Each site is checked against its 64 instruction bytes at
boot; nothing is installed on another game version.

| Offset | What it reads |
|---|---|
| `0x16d6cdc` | this frame's check value and frame number |
| `0x16cf304` | a mismatch: both values, the station and the frame |
| `0x16ce7c0` | the lockstep session end and its caller |
| `0x16cc2c4` | the other console's end message |
| `0x135fd60` | a replay playback frame and its position check |

## Files (`sd:/ultimate/ssbu-desync-detector/`)

Sessions rotate through eight slots (`next-slot.txt` holds the last one); a
new session empties its slot first.

| File | Contents |
|---|---|
| `events.log` | Install line; per session `session`, `desync` (frame, this console's value, the other console's value, its station), `sync_end` (what ended the session), `peer_end`, `link`, `recording` (or `not_kept reason=normal_end`), `playback_mismatch`. At 1 MiB it becomes `events.previous.log`. |
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
