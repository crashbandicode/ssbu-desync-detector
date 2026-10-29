# Eden proof (2026-10-29)

The question: after a desync, can the detector say which console left
vanilla? Tested with a forced desync between two Eden instances.

## Setup

- Eden LAN Online (Quickplay between two instances on two hosts), Smash
  Ultimate 13.0.5.
- Instance a: only this plugin, build `b15e929` (NRO sha256
  `7603ad747733b5e55a52b8e69fc63e3a698bb4258c7e728ca6c845ca41d2113f`).
- Instance b: the same plugin plus an Eden-only probe, a scratch build of
  Online Deluxe that moves a fighter below the blast zone about 20 seconds
  into a match on that instance only. The probe is not in any repository
  or release.
- Both picked Lucario. Inputs came from a keepalive script (crouch,
  shield, short hop).

## The match

Both consoles logged the mismatch at the same frame, each with its own
value as `ours`:

```text
b: desync frame=1128 ours=0x720d5ecb theirs=0x4aec250b peer=1
b: sync_end trigger=desync kind=2 frame=1132 local=0 desync_seen=1
a: desync frame=1128 ours=0x4aec250b theirs=0x720d5ecb peer=0
a: sync_end trigger=desync kind=2 frame=1129 local=1 desync_seen=1
```

Each saved its recording one second after the lockstep end and again when
the game finalised the match (`source=committed outer=game`). Both files
passed the game's replay validator and were listed in the Vault as
Quickplay replays with the match date.

## The referee

Both `match-0.bin` files were copied to instance a's
`save_data/replay/` and played from the Vault.

| Recording | Playback on a | Vanilla value at frame 1128 | Referee |
|---|---|---|---|
| a's | all 1631 records, no position mismatch | `0x4aec250b` = a's value | a: the other console left vanilla |
| b's | stopped at frame 1128: `playback_mismatch computed=0x160baefe recorded=0x5ddfd77f`, game dialog "This replay can't be played" | `0x4aec250b` = a's value | b: this console left vanilla |

The vanilla playback reproduced the online check value of every frame
before the desync at frame offset 0 (1128 of 1128). Judging b's frames
against the playback of a's recording gives the same verdict, since both
recordings hold the same inputs.

An earlier run with a development build gave the same verdicts (desync at
frame 1130, values `0xb38828e6` on the clean instance and `0x40b1d96a` on
the probed one).

## Observations

- After the recorded records end, the replay viewer keeps running the
  scene without its per-frame check. Only frames up to the desync count.
- Eden's `FlushFile` does not reach the host file: lines written to an open
  file appear when Eden closes it. The plugin appends `events.log` by
  open, write and close for that reason.
- `heap_allocations=0` in every install and session line.

## Not yet tested

- A Switch, and a real "No contest" on Nintendo's servers (Eden LAN ends a
  desynced match with "A connection error has occurred").
- Matches longer than about 30 seconds, doubles, and Arena.
