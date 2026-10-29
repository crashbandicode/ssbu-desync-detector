# ssbu-desync-detector

Skyline plugin for Super Smash Bros. Ultimate 13.0.5 that records what ended
an online match and keeps the per-frame evidence a vanilla replay needs to
show which console desynced.

Online matches run in lockstep. Every frame each console sends a check value
(the XOR of the game's RNG states) and compares the other console's value
with its own; a mismatch ends the match as "No contest". Both consoles see
the same two values, so the values alone cannot say whose simulation left
vanilla. A replay can: the game records every online match, and an unmodded
game playing that recording recomputes the check value for every frame.
Whichever console's value the vanilla run reproduces stayed vanilla.

## What it writes

Everything goes to `sd:/ultimate/ssbu-desync-detector/`. Sessions rotate
through eight slots (`next-slot.txt` holds the last one).

| File | Contents |
|---|---|
| `events.log` | The install line, then per session: `session`, `desync` (frame, this console's value, the other console's value, its station), `sync_end` (what ended the lockstep session), `peer_end` (the other console's end message), `link` (connection status changes), `recording` (the saved match), `playback_mismatch`. |
| `frames-N.bin` | 20-byte records. Online: the frame number and this console's check value. Replay playback: the playback frame, the check value the game computed, and the recomputed and recorded position checksums. |
| `match-N.bin` | The game's recording of that online match as a replay file. It is saved one second after the lockstep session ends and replaced by the finalised entry if the game finalises the match. |

## Who desynced

1. Copy the folder off the console that saw the No contest.
2. In an unmodded game (Eden with only this plugin works), copy
   `match-N.bin` to `save_data/replay/replayYYYYMMDDhhmmss.bin`, open the
   Vault's replays and play it. The plugin records the playback in a new
   slot. (If the Vault lists the match without names, run
   `cargo run --example replay -- outer match-N.bin <a saved replay> out.bin`
   and copy `out.bin` instead.)
3. Copy that game's folder too and run

   ```text
   cargo run --example referee -- <console-dir> N <vanilla-dir> M
   ```

The referee lines the console's frames up with the playback, then looks up
the vanilla value at the desync frame:

| Verdict | Meaning |
|---|---|
| the other console left vanilla | the vanilla value equals this console's value |
| this console left vanilla | the vanilla value equals the other console's value, or the game stopped the playback earlier because it could not reproduce this console's recorded positions |
| both consoles left vanilla | the vanilla value equals neither |
| inconclusive | the playback did not line up, or ended before the desync without disagreeing |

## Tested

In Eden on 2026-10-29: two instances in Quickplay, one running only this
plugin and one with an Eden-only probe that forced a desync. Both consoles
logged the same frame with mirrored values, and playing each recording on
the clean instance named the probed console. Details are in
[docs/EDEN-PROOF.md](docs/EDEN-PROOF.md); `tests/eden_probe.rs` replays
those frames as a regression test. It has not run on a Switch yet.

## What the check covers

[docs/WHAT-IS-CHECKED.md](docs/WHAT-IS-CHECKED.md) explains what the online
check compares, which plugin changes can and cannot cause a No contest, and
how to test a plugin by playing the same replay with and without it
(`cargo run --example compare`).

## Safe online

The hooks only read registers and memory the game is already reading at that
instruction. The plugin writes no game memory, changes no game state and
sends nothing. Each hook site is checked against its 13.0.5 instruction bytes
first; the plugin installs nothing on other versions.

## Memory

The plugin does not allocate. Frame and event rings are static, lines and
paths are built in fixed stack buffers, files go through `nn::fs`, and the
writer thread runs on a static 32 KiB stack. A counting allocator reports
the plugin's heap allocations in the install line and every `session` line
(`heap_allocations=0`).

## Hook sites (13.0.5 `.text`)

| Offset | What it reads |
|---|---|
| `0x16d6cdc` | this frame's check value and frame number |
| `0x16cf304` | a mismatch: both values, the station and the frame |
| `0x16ce7c0` | the lockstep session end and its caller |
| `0x16cc2c4` | the other console's end message |
| `0x135fd60` | a replay playback frame and its position check |

## Build

```text
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --no-default-features
cargo skyline build --release
```

Install `target/aarch64-skyline-switch/release/libssbu_desync_detector.nro`
to `atmosphere/contents/01006A800016E000/romfs/skyline/plugins/`.
