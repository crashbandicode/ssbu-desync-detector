# Match telemetry (0.5.0)

2026-11-12. Static design and host tests only. No line below has been written by a match yet; the example text is
what the host tests print (`src/matchlog.rs`).

0.5.0 adds read-only `events.log` lines for online matches: the match's rules and fault-table limits at the start,
each KO the fault table counts, which of the lockstep's peer-end paths ran, the phase-6 end with its network result,
and one summary per session. When `ssbu-stall-press-carry` 0.2.0 or later is loaded, the summary also carries its
counters. The 0.4.0 lines are unchanged.

## Lines

```text
5 match_start melee=0x0e mode=0x07010101 fault_mask=0x3ee watched=0x03 watched_n=2 limits=+0x40:window_s=30,n=6,l=7 rules=0e00…00ab
9 ko frame=4842 entry=1 kind=3 stamps=2->3 fault_word2=0->1
1 peer_end_path path=A frame=8444 station=1 peer_mask=0x3 end_reason=2 msg_type=7 msg_station=0 msg_word=7
1 peer_end_path path=B frame=none station=0 peer_mask=0x1 caller=0x16cfa38
3 session_end reason=1 fault_code=2 net_result=-993 phase_before=4 already_ending=1 frame=100 caller=0x16d8d3c via=countdown
7 session_summary frame=5707 kos=1,0,0,0,0,0,0,0 fault_words=0,0,0,0,0,0,0,0,0,0 ended=sync_end_normal last_session=normal penalty_flag=unknown stall_press rescued=10 (jump=9 attack=1 special=0 shield=0 grab=0 other=0) dropped=123 committed=45 stalled=123 flick_carried=3 flick_lost=1 (reversal=1 superseded=0)
```

The first number is the system tick, as on every line.

- **`match_start`**: written while match setup copies the fault table's limits, only when the table is armed
  (`fault_mask` nonzero, the online melee modes).
  - `melee` is the setup block's first byte (`main+0x52c51b0`) and `mode` the word at `main+0x53050f0`.
  - `watched` and `watched_n` are the entries the fault table watches (`+0xc`, `+0x10`).
  - `limits` names the block the game chose: `+0x38` for one watched entry, `+0x40` for more, then its window in
    seconds and its two counts. Read from a running game (Eden, 2026-11-12): `+0x38` is 30 s, 3, 4 and `+0x40` is
    30 s, 6, 7. In an Eden Quickplay 1v1 (`proof-dc8ee988-r2`) each console watched only its own fighter
    (`watched=0x02 watched_n=1` on one, `0x01` on the other), so the `+0x38` block applied.
  - The fault table looks like the game's own penalty evaluator (the string `GamePenaltyEvaluatorManager::process` is
    referenced at `.1354a94`). In that run, the idle console (a shield tap every 2 s for about 54 s) armed code 1
    (`fault code=1 … words=0,1,0,…`) and was removed with "Canceled." and `net_result=-993`. Code 2 is the KO
    pattern counted by `.1363420`.
  - `rules` is the setup block's first 64 bytes, raw. The stock, time and team fields in it are not named yet; the
    raw bytes are kept so they can be decoded later.
- **`ko`**: one per call of the KO method the fault table counts (`.1363420`), while the table is armed. `entry` and
  `kind` are its arguments; kind 8 is not counted and its meaning is unknown. `stamps` is the stamp list's length and
  `fault_word2` the table's word 2, before and after the call. `frame` is the lockstep session's frame.
- **`peer_end_path`**: path A is the receive handler `.16cc140`'s tail into `.16cced0` from `.16cc4d4`. Two of its
  jump-table cases reach it: case 7, the other console's end message (`end_reason=2`; 0.4.0's `peer_end` line comes
  from the same case), and case 8, a message naming this station (`end_reason=1`). `msg_type` and `msg_station` are
  the message's first byte (low and high nibble) and `msg_word` its big-endian half-word at +1, which case 7 stores at
  `session+0x64` (0.4.0's `peer_end` reason); `message=unread` without a message pointer. Path B is the abort function `.16ccad0` (it ends in
  `.16cced0` from `.16ccc94`); `caller` is its return address in `.text`.
- **`session_end`**: `.16cced0`'s entry, where the session goes to phase 6.
  - `reason` is its argument and `fault_code` the session's stored fault-table number.
  - `net_result=-993` when `fault_code` is nonzero: `.16cced0` then hands the transport `0xfffffc1f`.
  - `phase_before` and `already_ending` are the session bytes `+0x62` and `+0x63`.
  - `via` is `countdown` (the fault countdown's end, `.16d8d3c`), `start` (a failed session start, `.16d9328`) or
    `tail_call` (a tail call left the caller's caller in `caller`).
- **`session_summary`**: when the session stops (`.16d5cd0`), one per session.
  - `frame`; `kos`, the counted KOs per entry since `match_start`; and the fault table's ten words.
  - `ended` is the first end seen in the match: `sync_end_normal`, `sync_end_abnormal`, `local_fault`, `peer_end_a`,
    `peer_end_b`, `start_failure`, `phase6_other`, or `none_seen` (the session stopped with no end, such as a lost
    connection). For the phase-6 ends it adds `end_reason`, `end_fault_code` and `end_caller`. A phase-6 end within
    1 s of a peer-end path is attributed to that path.
  - `last_session` is the session's verdict as written to `last-session.txt`: `normal`, a 0.3.0 reason name
    (`desync`, `local_link`, `local_status`, `peer_ended`, `timeout`, `unknown`), or `none` when no verdict was in
    scope. The writer writes `in_progress` there when an online session starts, so a console that stops mid-match
    leaves `in_progress`; the next boot's first line after the install line is `last_session_loaded=<token>`. A record
    only: no dialog text depends on it, and the dialogs are exactly 0.4.0's.
  - `penalty_flag` is the game's own disconnect-penalty state for this console. No field of the game's is pinned yet,
    so it is `unknown` and nothing is read.
  - `stall_press …` is appended only when `ssbu_stall_press_carry_stats_v1` is found (`nn::ro::LookupSymbol`, by the
    writer thread) and answers with version 1 or later. `rescued` counts commits that received carried presses, by
    button group; `committed` the press edges the game received; `dropped` the samples the lockstep discarded.
    `stalled` is the same number as `dropped`: each drop is one stalled pass, and stalled passes that still commit
    are not visible to that plugin.

## Sites (13.0.5 `.text`)

Every site is 8-byte aligned, so the patch relocates at most four straight-line instructions (no branch, no call).
No direct branch targets the three instructions after any site (`xref.py` over `main-13.0.5.elf`). The disassembled
range around each site has no indirect branch, except in `.16cc140`, whose ten jump-table targets (table `0x44f37d4`)
are all outside `peer_path_a`'s span. Each site has a 64-byte guard; the seven are installed together or not at all, and the install line
says `match_telemetry=installed` or `match_telemetry=skipped guard_failed=<site>`.

| Site | Offset | Instruction | Reads |
|---|---|---|---|
| `match_limits` | `0x14d96a0` | `str w9, [x19, #0x218]` | x19 the fault object, x8 the chosen limit block |
| `ko_entry` | `0x1363420` | `sub sp, sp, #0x70` | w1 entry, w2 kind; the table before |
| `ko_exit` | `0x1363998` | `ldr x27, [sp, #0x10]` | the table after (the shared epilogue) |
| `peer_path_a` | `0x16cc4c0` | `ldp x29, x30, [sp, #0x50]` | x19 the session, x20 the message (`.16cc140` keeps x2 there), w1 the reason |
| `peer_path_b` | `0x16ccae8` | `ldrh w1, [x0, #0x5a]` | x0 the session, x30 the caller |
| `session_end` | `0x16cced0` | `sub sp, sp, #0x80` | x0 the session, w1 the reason, x30 the caller |
| `session_stop` | `0x16d5cd0` | `sub sp, sp, #0x60` | x0 the session |

`peer_path_a` sits after `mov x0, x19` rather than on the case-8 `orr w1, wzr, #1` at `.16cc4b8`: case 7 branches to
`.16cc4bc`, inside a patch there.

The hooks write only this plugin's statics. Wide records (`match_start`, `session_summary`) go through an
eight-slot side ring; the rest are ordinary events. The writer's line buffer is 512 bytes.

**Card scan** (`scan-offsets.py` over the 20-NRO card readback of 2026-11-12, for all seven offsets): no other
plugin references them. Online Deluxe hooks `0x16ccab8` (`ONLINE_LATENCY`), 0x30 bytes before `peer_path_b`'s patch;
its install check (`preflight`) reads only the word at its own site, so the two patches do not overlap or disturb
each other.

## Not covered

- Offline matches: the table is not armed and there is no lockstep session, so none of these lines.
- The meaning of the stock, time and team bytes in `rules`, of KO kind 8, and of the param pair 30.0 / 50.0 at
  `+0x48`/`+0x4c`.
