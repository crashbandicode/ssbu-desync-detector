# Versions

2026-11-13. What each version added, where it is, and what tested it. Moved here from the README and checked against
the Forgejo releases, the card-install receipts and the run logs named below.

| Version | Build | Released | On the card |
|---|---|---|---|
| 0.5.1 | `baf8387` (in `main`), NRO `36027bb7…` | no tag or release | since 2026-11-12 22:26Z |
| 0.5.0 | `ac75be2`, NRO `dc8ee988…` | no tag or release | 2026-11-12 20:29Z to 22:26Z |
| 0.4.0 | tag `v0.4.0`, `ca0c149`, NRO `a31ad657…` | 2026-11-07 | 2026-11-07 18:43Z to 2026-11-12 20:29Z |
| 0.3.0 | tag `v0.3.0`, `49214c1`, NRO `0ac90bc6…` | 2026-11-06 | 2026-11-06 11:55Z to 2026-11-07 18:43Z |
| 0.2.0 | tag `v0.2.0`, `1daf9a2`, NRO `29067bb6…` | 2026-10-30 | until 2026-11-03 16:13Z |
| 0.1.0 | tag `v0.1.0`, `977fb5b`, NRO `95e7def9…` | 2026-10-29 | installed (its `manifest.json`; dates not recorded) |

Card times are the install receipts' finish times: `smash-training-lobby/artifacts/card-install-detector-0.5.1/` and
`card-install-detector-0.5.0/` on the Surface; `/home/intpa/ssbu-desync-detector-release/v0.4.0-install/` and
`/home/intpa/ssbu-desync-detector-card-20261106-v030/` on the Yoga. From 2026-11-03 16:13Z to the 0.3.0 install the
card ran the unreleased telemetry build `02488cdc` ([DISCONNECT-DIALOG.md](DISCONNECT-DIALOG.md)).

## 0.5.1

Logs every stock a fighter loses (`stock_lost`, marked when it was a self-destruct), read from the game's own
per-fighter counters, and the summary's `kos=` and new `self_destructs=` count them
([MATCH-TELEMETRY.md](MATCH-TELEMETRY.md)). Dialogs are unchanged from 0.4.0.

- Host tests: 168 pass (`cargo test` on `846814c`, 2026-11-13).
- Eden, Quickplay pairs on the Yoga: `proof-36027bb7-r8` (two falls after a hit, `self_destruct=no` on both
  consoles at the same frame) and `proof-36027bb7-r10` (a run-off at GO, `self_destruct=yes`, `self_destructs=1`).
- Local wireless: not tested. `a-ldn-sd051-0c628724` (Surface, 2026-11-12) failed before the match.
- Switch: no card log since the install has been read.

## 0.5.0

Read-only match lines in `events.log`: `match_start`, `ko`, `peer_end_path`, `session_end` and one
`session_summary` per session (with `ssbu-stall-press-carry` 0.2.0's counters when it is loaded), and the last
online session's verdict in `last-session.txt`. Dialogs are unchanged from 0.4.0.

- Eden, Quickplay pairs on the Yoga: `proof-dc8ee988-r2` and `-r4` to `-r7`. Every match in r4 to r7 ended
  `sync_end … desync_seen=0` on both consoles. In r2's second match the idle console was removed by the game
  (`fault code=1`, `session_end … net_result=-993`) and the other ended normally.
- Local wireless, `a-ldn-fix020t-0c628724` (Surface, 2026-11-12, diag build `a42d6c51`): both consoles logged
  `session slot=2 kind=online`, `sync_end … desync_seen=0`, `peer_end_path`, `session_end` and `session_summary`,
  but no `match_start` or `ko`, and `kos=0` although the host walked off 4 times.

## 0.4.0

One `events.log` line when Smash arms the countdown that stops the frame counter (`fault`: the fault-table number,
the frame, the mask and the ten table words, [FAULT-CODE.md](FAULT-CODE.md)). The same reason as 0.3.0 is appended
to the English "Canceled", "Canceled." and "You disconnected in the middle of your last online battle." texts, and
`notice_text` logs which function asked for the text ([CANCELED-NOTICE.md](CANCELED-NOTICE.md)).

- Eden, 2026-11-12 (Yoga, release `a31ad657`): `fault code=1 frame=3215 … words=0,1,0,…` in `dbl-ga-02420ee`,
  `dbl-gb-02420ee` and `dbl-gb2-02420ee`; on console a of `dbl-ga` and `dbl-gb2` also
  `notice_text kind=canceled_period … verdict=unknown`.
- Switch: the card log read on 2026-11-11 (`smash-training-lobby/artifacts/phase23/hang-add-new/console-sd-2026-11-11/`,
  0.4.0 boots only) has 25 `sync_end trigger=normal` and no `fault`, `notice_text` or `communication_text` line.

## 0.3.0

Appends the reason to the game's English connection-error pop-ups, on both ways the game builds that text, and
records telemetry around disconnects ([DISCONNECT-DIALOG.md](DISCONNECT-DIALOG.md)).

- Eden: `dd-popup-y03` and `dd-popup-y05` (2026-11-05, pre-release builds `b5fa80f8` and `92fbcf84` of the same
  text routes) showed the reason under "A connection error has occurred." on both consoles. The README GIFs
  (2026-11-06) are release 0.3.0 in `dd-gif-a1` against 0.2.0 in `dd-gif-b1`; those run folders are no longer on
  the Surface or the Yoga, so their record is `HANDOFF.md`.
- Switch: no card log from the 0.3.0 days has been read.

## 0.2.0

0.1.0's code built with link-time optimization: 124 KiB of memory instead of 552 KiB (text + ro + data + bss in
each release's `manifest.json`).

## 0.1.0

The lockstep and playback hooks, the frame files, the recordings and the referee. The Eden proof
([EDEN-PROOF.md](EDEN-PROOF.md)) ran the earlier build `b15e929`.
