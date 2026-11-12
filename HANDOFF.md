# 0.5.0 match telemetry — 2026-11-12 (current)

- **Work clone:** `/home/intpa/ssbu-desync-detector-050` on the Surface (`ubuntu-surface`), from the Yoga clone's
  `readme-gifs` at `ca0c149` (0.4.0). The Yoga clone's own uncommitted `HANDOFF.md` and `BRIEF-GIFS.md` were left
  alone.
- **Branch:** `match-telemetry-050`, to be pushed with no tag.
- **Change:** `src/matchlog.rs` (sites, events 12–16, packing, line text, host tests) and the hooks and writer in
  `src/runtime.rs`. The lines and sites are in `docs/MATCH-TELEMETRY.md`.
- **Design and trace:** `smash-training-lobby/artifacts/phase23/double-input/DESIGN-desync-detector-0.5.0.md`.
- **Builds:** release `dc8ee988…` (73728 bytes), diag `a42d6c51…`. No Eden run and no match yet.
- **Dialog wording, the operator 2026-11-12 (via the primary 19:06Z): option 1.** The dialogs stay exactly 0.4.0's
  (`PeerEnded` keeps "The other console ended the session."; the reconnect warning keeps the game's text plus the
  reason). `2f76d36` (an evidence build that said "Your opponent disconnected" and replaced the warning) is superseded
  and must not be run or shipped. Why: the peer's end message carries kind 2 for every abnormal end, including the
  peer timing this console out, and a peer that quits sends nothing.
- **Kept:** `last-session.txt` (a record only) and `last_session=… penalty_flag=unknown` in the summary.
- **Next:**
  - the Eden proof pair with `ssbu-stall-press-carry` 0.1.0 and 0.2.0;
  - staging the card install over 0.4.0 `a31ad657…` (the primary installs it).

# Why frame 8444 was the last one — 2026-11-07

The frame the plugin records is the halfword stored by `stp w11, w8, [x9]`
at `.16d6cdc`, inside `.16d5e50`. The next call did not reach that store.
The prologue returns at `.16d6c8c` when session byte `+0x62` is 6
(`cmp`/`b.eq` at `.16d5e7c`). That byte is written with 6 at `.16cd544` by
`.16cced0`. During a match the call that does it without passing the
peer-abort hook is `.16d8d38`, and it runs when the halfword at session
`+0x66` decrements to 0.

That halfword is set to 30 at `.16d5eec` on the pass where `.16acd30`
returns nonzero. `.16acd30` reads the object at `[main+0x52c3940]`. It
returns 0 while the word at `+0x18` is nonzero. It returns 1..9 for the
first nonzero word at `+0x1c`, `+0x20`, `+0x24`, `+0x28`, `+0x2c`, `+0x30`,
`+0x34`, `+0x38`, `+0x3c`, if the low 10 bits of the object's first word are
set. Each later pass that still stores a frame decrements `+0x66`. The pass
that stored 8444 is the one that reached 0, so it stored the frame and then
set the phase to 6. The next pass returned at the prologue. This path does
not call `.16ce7c0`, so there is no `sync_end`. The plugin's peer-end hook
is the receive side (`.16cc2c4`); this decision is local, so there is no
`peer_end`.

Which of the nine words was set is not in the Switch log. The release hooks
do not write `+0x62` and do not call `.16acd30`. `heap_allocations=0`,
`evidence_losses=0`. A boot cannot recover the word from a match that
already ended.

2026-11-07: the fourteen functions that load `main+0x52c3940` were followed
with the object pointer kept live. The only store into the nine fault words
is entry 2, the word at `+0x20`, incremented at `.1363884` and `.13638b0`
inside `.1363420`. That function takes a station index and a second code.
When the second code is 8 it skips the increment. Otherwise, if this
station's bit is set in the word at `+0xc`, the low 10 bits of the first
word are nonzero, byte `+8` is nonzero, and bit 2 of that first word is set,
it appends a timestamp and increments `+0x20` when the gaps in that list are
all within the limit copied to `+0x218`/`+0x21c`, or the list length reaches
the limit at `+0x220`. The counter saturates at 999999999. Any nonzero value
makes `.16acd30` return 2 when `+0x18` and `+0x1c` are still zero.

Match setup at `.14d9414` writes that mask and copies the two limits from one
of two config blocks. It does not publish the object pointer anywhere else.
The other eight fault words are not stored by any of those fourteen
functions. The handler sits in an unnamed table of (function, zero pointer,
0x403) records at `0x3c8ade0`, `0x3c8b9b0`, and `0x3c8c4c0`.

Tangent `f774f0f3-t1` finished. The card's `libarcropolis.nro` is stock
4.1.1 (`92171b2f…`, 2076672 bytes). There is no ARCropolis checkout here.
The panic is an out-of-bounds read in the MSBT patch handler
(`arcropolis+0x1ea98`); `+0x171258` is the shared `brk #1`. No fork edit.

2026-11-07 17:16Z: DBI was already open (`mtp://-_DBI_XAW10006803734/`).
Deleted the 14 generated files under `ultimate/arcropolis/cache`
(13.0.3, 13.0.4, 13.0.5, including the two patched bins). Config, resources,
and mods were left. The newest Smash crash before that delete is Atmosphere
0xA8, BRK at `arcropolis+0x171258` (caller `+0x171914`), report
`01794071226_01006a800016e000.log`. Tangent `f774f0f3-t1`, Cursor chat
`4f3d6879-87e4-45c6-b398-2ba6b3f35149`, owns the fork fix. The operator
later said to leave that tangent alone. No NRO, no commit.

# Fault-table log — 2026-11-07

The detector now stages the stored return of `.16acd30` at `.16d5ee8`
(session `+0x5e`), the low 10 bits of the object's first word, and the ten
words at `+0x18` through `+0x3c`. The frame hook claims that snapshot while
it is at most 50 ms old. After 100 ms the writer emits the line with
`frame=none`. One `events.log` line per arming:

`<tick> fault code=<n> frame=<frame|none> mask=<0xhhh|none> words=<10 integers|unread>`

Install is independent of the lockstep set. The install line says
`fault=installed` or `fault=skipped`. A guard or relied-on word mismatch
does not install the hook. Design: `docs/FAULT-CODE.md`. Compliance:
`docs/COMPLIANCE-FAULT.md`.

The operator ordered minor version 0.4.0 and a card install once the
Switch is connected. This commit is that version: the fault line and the
Canceled-notice matcher. Branch `readme-gifs`. The card still has 0.3.0
until the install receipt exists. Eden was not ordered. The next real
match after the install is what records `code=`.

# Canceled notice — 2026-11-07

The plain route `.377a120` and the argument route `.37a2660` now append a
reason to the English labels `Canceled`, `Canceled.`, and the reconnect
sentence "You disconnected in the middle of your last online battle." A
notice with no session verdict is left unchanged. The same plain hook logs
`notice_text` with the kind, the caller of `.377a120` (saved LR at FP+8),
and `verdict=withheld` when there is no session verdict. No new hook. Host
tests and clippy pass. Not built as an NRO, not installed, not committed.
Design: `docs/CANCELED-NOTICE.md`. Playbook `3f96569`.

The battle-end popup openers at `.15657e8` and `.15665a8` pass a message id
into `.32f1bb0` only after `.3291150` accepts it. This incident had no
`sync_end` and no `communication_text`, so that opener did not run. The
orange "Canceled" word is the matching-status label
`mnu_pallet_matching_canceled` (and two other labels that display the same
word). Which caller set it is what `notice_text` is for, on the next boot
of this source.

# README GIF worker — 2026-11-06

Worker Cursor chat f774f0f3, live pair grok-4.7 high. Watcher unit
`cursor-model-watcher-f774f0f3.service` (stop file `/tmp/cursor-model-watcher-f774f0f3/stop`,
stopped with the report). Playbook `cd605af` on both hosts. Worktree
`/home/intpa/ssbu-desync-detector-gifs`, branch `readme-gifs` from `49214c1`. No card, no push.

Clips, both 1280×720, frames checked:

- Desync after: `docs/media/desync-after.gif`, 7.4 s, run `dd-gif-a1` (Surface). Released 0.3.0
  `0ac90bc6`, commit `49214c1`. TrainingRooms `23b89786` on station 1 only. Both logs:
  `desync frame=732`, `communication_text reason=desync text_units=102 route=plain`. The row gate
  `reason-desync` failed (pattern required `route=plain` with nothing between, and it ran after
  the pop-up had closed). The raw recording already held the pop-up, so it was trimmed
  (4700–12050 ms) and encoded. No second boot. Claim released 12:26:35Z. `RESULTS.md` records
  r05 FAILED. Both console.logs: no panic/abort/crash/Unmapped.
- Desync before: `docs/media/desync-before.gif`, 6.8 s, run `dd-gif-b1` (Yoga, 41G free at Up).
  Released 0.2.0 `29067bb6`, tag commit `550cae6`. Same route. `desync frame=908`, no
  `communication_text`. Pop-up is only "A connection error has occurred." 6/6 PASS. Claim
  released 12:38:23Z, profile restored and verified. console.logs clean. Down's `game.stop`
  hit its deadline and the lifecycle SIGKILLed both; RESULTS marks that as not a crash.

Not reproduced, after two no-TrainingRooms pairs (`dd-gif-b2`/`dd-gif-a2`, then `dd-gif-b3`/`dd-gif-a3`, claims released). Home, then SIGKILL of the other instance, showed "You have been disconnected due to inactivity." Holding a stick on the recorded console did not change that. Neither 0.2.0 nor 0.3.0 logged `peer_end`, `sync_end`, or `communication_text`. That dialog is not the communication-error pop-up. Local link and local status need a NIFM fault; host networking was not touched. Unknown was not forced on a release build. Ally has one slot, so it cannot host a pair. ubuntu-xps was not ready for this worker.

`docs/media/referee.gif` is still the 2026-10-29 unmodded replay refusal. It was scaled from
480×270 / 11.1 s to 1280×720 / 9.6 s. It was not re-recorded.

Hosts released 13:11Z. No Eden process left by this worker.

XPS (2026-11-06 ~13:14Z): machine is free (175G disk, ~26 GiB, no Eden, displays :116 and :117 free). Its playbook checkout is `618cc82`, which admits `host=xps` and sets `LINUX_MAX_EDEN` to 1 until two instances have been tested on that machine. A pair was not started. The 0.3.0 NRO and the tile-cursor NRO are not on that host. Next: none unless the playbook's XPS limit is raised.

# Popup-route worker — 2026-11-06 ~05:00Z addendum detector-stage-pilot-1 DONE (history)

Result: TrainingRooms 0.2.0-rc.2 is necessary and sufficient. F (card set on both, pilots both) desync at
frame 1499, early rebirth of the station-1 fighter (pos6: 1482 vs 1541); C (same minus TrainingRooms) 5
matches, 10.45 match-minutes, 0 desyncs (r07 stopped by STOP after the pair stopped rematching: a's Eden
control socket silent since 04:32Z, game still running); ty1 (TR + deps on station 1 only) desync at 837.
Evidence: evidence/dd-popup-{bF1,pos6,bC1} (+tgz), taskb2/{bF1-check,bC1-check,shift-test-bF1-tr}.log,
manifest.json 2211 entries. No claim/Eden/job running; run copies removed with manifests.

# Popup-route worker — 2026-11-06 ~04:45Z addendum live bisection (history)

Pairs: G&W (a, readies first, station 0) vs Fox (b, station 1), tile-cursor picks, each console's card
release pilot (ed8f308f) set to CPU (L9) via Deluxe overlay (L+R+Down to its page, ZR+Right; ZR also opens
the game's side menu: B, B, + recovers), auto rematch; 2:00 timed Quickplay rules with items (profile's).
- F dd-popup-bF1 (Yoga, 04:05-04:18Z): card 18 plugins + Deluxe config + TR settings on both: desync frame
  1499; a (st0) TR logged active=false at battle start, b (st1) did not. pos6 (Yoga): b's recording KO at
  1479, (0,170) at 1482 vs vanilla 1541 (+59 again); a's recording clean to 1544.
- C dd-popup-bC1 (Surface, from 04:20Z): same minus libtraining_rooms.nro on both. r06 watch stopped early
  (session-count bug, fixed in steps.py); r07-rewatch in the same boot up to 1500 s. 3 matches x 7521
  frames normal so far, 0 desyncs (live; finish: count sync_end frames in both detector logs).
- Then: Down bC1, pack evidence, delete copies, fill BC1-RESULT/BC1-MINUTES in docs, commit, compliance.

# Popup-route worker — 2026-11-06 ~04:00Z Task B cause FOUND (operator order ~02:25Z; history)

Cause of the 15:32:57 desync: TrainingRooms 0.2.0-rc.2 (plugin, not a mod; not fixed, by order). Proven
in Eden: dd-popup-ty1 (Yoga pair, a vanilla station 0, b TrainingRooms rc.2 + nro_hook + param_hook + the
card's TR settings, station 1): b's practice active=true in the waiting room, never cleared in the battle;
b's fighter ran off -> desync frame=837 on both. dd-popup-pos5 (Yoga, vanilla playbacks with diag
playback_continue + position trace): vanilla console's recording 0 mismatches; TR console's recording
first fails at 536: its KO'd fighter at (0,170) at KO+3 (536) vs vanilla KO+62 (595), same as the card
(4151 vs 4210). Docs: docs/DISCONNECT-DIALOG.md "2026-11-06: what changed this console's positions".
Branches: feat/disconnect-popup-route (docs/eden rows; release still rebuilds 92fbcf84);
taskb/playback-continue 8def90b (diag switch only; release bytes 6493b38b by line numbers).
Evidence: work folder evidence/ (manifest.json 750 entries; dd-popup-pos1/pos2/ty1/pos5 + tgz, taskb2/).
Host state: no claim/Eden/job of this worker; run copies removed with manifests (pos1, pos2, pos3, pos4,
pos5, ty1, tr2 on the Surface); worktree target removed (target-deletion-r6.json). Surface leftovers:
popup-route-work (card copy, scripts, tgz), eden-runs dd-popup-{mods1,pos1,pos2,tr1,tr2} logs/out only.
Yoga leftovers: eden-runs dd-popup-{ty1,pos3,pos4,pos5} logs/out only.
Open (not established): whether the operator's TrainingRooms settings at 20:31Z still matched the 13:14Z
capture (TR's own match-time logs were not in the 15:38 card capture), and the other console's state
(only this console's recording exists; Eden shows a vanilla peer replays clean).

# Popup-route worker — 2026-11-06 ~02:55Z Task B cause search (history)

Order: find causes, fix nothing. Stage-mod branch NOT applicable (match on Yoshi's Island (Melee) BF form,
no mod touches it; mods1 playbacks identical). Established so far (work folder popup-route/taskb2/):
- Replay record format: FRAM records = u32 header (bits 8-15 player mask, bit 7 parity, bits 16-23 flag),
  u32 position checksum, 8-byte input per masked player (replay_inputs.py). Checksum = u32 sum of all 8
  slots' x/y/z float bit patterns (.135fc84-.135fd5c).
- Run dd-popup-pos1 (Surface, 02:44-02:53Z, released clean): vanilla playback with sp+0x70:128 trace.
  slot0 (Fox) KO'd at 4148, frozen at (240.6212, 74.0794) to 4151 in vanilla; the console's 4151 checksum
  = vanilla with slot0 at (0.0, 170.0, 0) exactly: the console moved the dead fighter to the rebirth point
  by 4151 (3 frames after the KO); vanilla had not. positions.py.
- CPU pilot NOT driving P1: release build forces P1=Player at boot (runtime.rs:735-741); Deluxe log shows
  only one overlay chord that boot (Hidden->Settings), never the pilot page (view 5).
- Deluxe events: HocGate applying=1 from ~59 s before the match to ~15 s after; ClockSample 1788 MHz at
  f~284; LatencyApplied 4/4 before the match; nothing else during it (deluxe_window.py).
- Horizon-OC Smash profile (card-2026-11-02-hoc): CPU 1785, GPU 921/460, MEM 1862, undervolt, uncapped.
- dd-popup-pos2 (diag 6bc2489a `playback_continue=1`, built r6 03:56Z; src change uncommitted, belongs on
  a separate branch: release bytes change from 92fbcf84 to 6493b38b by line numbers): vanilla moves the
  KO'd Fox to (0,170) at 4210 (KO+62) and descends; the console had it there at 4151 (KO+3), then not
  vanilla's descent. => this console ran Fox's dead/rebirth with ~1-frame waits.
- CAUSE (source): TrainingRooms 0.2.0-rc.2 (card NRO 23b89786 = Forgejo TrainingRoomsForUltimate
  v0.2.0-rc.2, commit 36d7f46; source copy taskb2/trainingrooms-36d7f46). Its widened is_training() =
  native Training || LOBBY_ACTIVE (runtime/scene.rs:35, model/scene.rs:21). LOBBY_ACTIVE is set/cleared
  only by lobby::update() (runtime/lobby/training/lobby.rs:75-121), called only from the input-mapping
  note hook for slot 0 (runtime/features/input_delay.rs:50-53). As station 1 (P2) nothing maps slot 0,
  so the waiting-room practice flag stays true into the battle; states.rs:1148-1171 + model/state/
  hooks.rs:29-45 then answer dead_rebirth_wait_frame=1 and rebirth_move/wait/invincible=0 (card settings
  13:14Z: lobby_practice On, save_state_enable On, respawn_invincibility Off). Card frames files: the
  desync match is the only one with local station 1 (flags 0x101); 7 others station 0, all normal ends.
  TrainingRooms' own morning log shows the normal clear when station 0 (#561/#562 active=false at melee
  0x0e). Deluxe audit (taskb2/deluxe-audit.md): no simulation writes; pilot inert (Stage::start).
- In progress: Eden proof run dd-popup-tr1 (Surface pair: a vanilla first/host, b TrainingRooms rc.2 +
  nro_hook + param_hook + card TR settings, second; b runs off stage; watch detectors). Up refused once
  ("Compiler already active on host": TrainingRooms owner's cargo on the Surface); waiting for it.
  pos1/pos2 evidence in evidence/dd-popup-pos{1,2} (+tgz), run copies removed with manifests.

# Popup-route worker — 2026-11-06 mods A/B, operation detector-mods-ab-1 (history)

Order: replay card match-0.bin with the card's ARCropolis (92171b2f) and the card's 7 mods (primary's
read-only copy switch/card-20261105-mods, 586 files, sha256-checked), alone and with all 18 card plugins.
Run dd-popup-mods1, Surface, Eden fefd2b5f, profile g, 01:54-02:18Z; Down released, cleanup_failures=[],
both forced stops (signal 9, not crashes); consoles unmapped 0, no guest fault (b: 3 Eden <Critical>
asserts, renderer/buffer queue). Rows as run: eden/popup-route-mods/rows-mods1.json (r02 and r04 rewritten
in the boot); clean re-run rows: rows.json (dry-run READY). Evidence: work folder evidence/dd-popup-mods1
(+ .tgz f04fb812), taskb-logs/, manifest.json (389 entries); run copies removed on the Surface
(dd-popup-mods1-deletion.json, 9,694 files).
Results: mods loaded on both (game's own Training SSS: FD tile = Cosmic Destination tile, MAD 21 vs 73
stock; ARCropolis discovery.cache names all 7). a (detector + ARCropolis + mods) and b (18 plugins +
Deluxe config + mods): playback_mismatch frame=4151 computed=0xa417d4d4 recorded=0x613d0d23, the same as
y01/ab1; referee and stream reconstruction identical (4348: two sound draws here, +1 sound there). Playback
frames file byte-identical in y01, ab1, mods1 a and b (33c2ab12...). No bisect: nothing changed the result.
Stage (replay details): Yoshi's Island (Melee), Battlefield form; Quickplay (Elite), Fox vs Mr. Game &
Watch, 3 stock 7:00, 1:28. Correction made in docs: ab1 carried 3 of the 7 mods (byte-identical) with
ARCropolis loaded, not "no mods". Still open: online-only plugin behaviour, the console, the other console.
Rows lessons: stock Training stage select ignores a B tap (the card's instant_back makes taps work; hold
B 2 s without it); X then B on a replay leaves focus on Details; idle main menu falls into the attract
movie (steps.py:to_vault handles movie/title/Smash/Games & More).
Surface leftovers: /home/intpa/popup-route-work (scripts, card/ copy of plugins+mods 96.6 MB, inputs/,
evidence tgz), /home/intpa/eden-runs/dd-popup-mods1 (logs/out only). No Eden/claim/job running.

# Popup-route worker — 2026-11-06 close-out (history)

Coordinator check ~23:45Z accepted dd-popup-y03 r05 (cite a-233408Z-613373-popup.png; a's later
popup-settled shot is black). Close-out: no further boots; compliance report
/home/intpa/ssbu-cpu-pilot-work/disconnect-dialog/popup-route/compliance-final.md (every agent row
disposed; fails: README GIF, Up deadline, unguarded quick commands). Playbook clone now cd605af
(compliance.py only since 0f0f533). No Eden, claim, watcher or job of this worker is running.


Brief: /home/intpa/ssbu-cpu-pilot-work/disconnect-dialog/popup-route-brief-20261105.md, plus the relayed
order to use any free Eden slot. Worker claude-opus-5-5/xhigh, session fa2f179b (no watcher running).
Playbook 0f0f533. Branch feat/disconnect-popup-route (worktree /home/intpa/ssbu-desync-detector-popup):
316f18d baseline, 3c6489a plain route, 8f1ce70 diag switch + tools, 77e48f0 docs, cce95c1 force-once
diag + A/B + normal-session rows, then this HANDOFF commit. Work folder:
/home/intpa/ssbu-cpu-pilot-work/disconnect-dialog/popup-route/ (evidence/ + manifest.json, gates/r5-*,
artifacts/r4/, compliance-resolved-r4.md, taskb-*.log).

Candidate: release 92fbcf849c196dd7e6e00b737eb6544b8ae09f56c153f0db7257b47fa64a3f1f (57344 B, 136 KiB),
rebuilt byte-identical on cce95c1; diag 45a6b424 (force_desync_frame from diag.txt, once per boot).
Route: pop-up body .1eef640 (dispatch .32f1cc8) -> no arguments -> .377a120 -> .37ae170 -> setter
.37a2270; new hook .377a250 (Route `plain`), 4 guards; `args` route .37ad8e8 unchanged.

Eden, Yoga only, all released clean, consoles 0 panic/abort/crash/Unmapped:
- y01 22:27-23:08Z: route traces (inactivity pop-ups), replay of card match-0.bin (Task B).
- y03 23:30-23:38Z and y05 23:52-00:10Z: forced desync -> pop-up "A connection error has occurred." +
  "Reason: Desync detected. ..." on both, communication_text route=plain; y05 then an unforced match
  ended normally on both at frame 7521, desync_seen=0.
- ab1 23:43-23:48Z (one instance, card's 18 plugins + Deluxe config; profile y also carried 3 of the
  card's mods, loaded by ARCropolis): replay stops at 4151 exactly as with the detector alone.
Task B: vanilla = this console 0-4151; positions not reproduced at 4151 (this console left vanilla);
first checked difference one extra `sound` draw on the other console at 4348. Card plugins in playback
don't reproduce it; open: card mods (ultimate/mods not captured), online-only plugin behaviour, console.

Not done: args route never ran in Eden. No push/install/card access. Surface leftovers:
/home/intpa/popup-route-work, /home/intpa/eden-runs/dd-popup-01 (never up). Yoga run folders
dd-popup-y01..y05, ab1 under /home/intpa/eden-runs (copies removed, manifests in the work folder).
# Disconnect-reason candidate — 2026-11-03

FINAL18:21Z read-only capture complete:140files15,201,850B in work/switch/
card-loader-20261103-1806, all hashes reverified; closure.json all jobs closed.
Telemetryboot47767659068 confirms telemetry=1/dialog=installed,13normal sync
ends/desync_seen0, zero mismatch/loss/allocation counters, no communication_text
event. Latest frame file contiguous; no rendered-error-dialog proof and no
newer crash. Captured package3 AND later supplemental stratosphere.romfs match
official1.12.1 archive byte-for-byte. Independent Grok review caught separate
ROfile missed in initialcopy; primary disclosed, read it once via still-exposed
DBI18:19:34Z, released user again. No further devicecalls. Source cleanup bug
strongly matches3priorcrashes, initial rejection/render writer still unknown.
Samechat6367efa1 review finished; native watcher/owner absent, wrapper stopped.
Primary watcher inactive/dead/MainPID0/success, Sol/xhigh unchanged. No live own
jobs/watchers/Eden/remote/device work; no code/build/install/commit/push. Existing
02488cdc detector install unchanged. Branchfeat/disconnect-reason-dialogHEAD
475b0cfccdf91d452fc1353b397b6a7ea251b883; pre-existing dirty work preserved.
Playbook49f81b1, capture compliance resolves manual rows with guard/provenance
exceptions. Next normal-play report; no immediate test or reconnect requested.
Before new work check model/rearm watcher and read capture README/root report.
All earlier operational states below are history.

CURRENT18:10Z: operator exposed DBI; primary captured loader/config/logs
READ-ONLY, complete139files/13,717,430B in work/switch/card-loader-20261103-1806.
capture_verified plus local-comparison.json; package3 independent double
read hash94c2e6f7e438571c51964b2d730c6cc7ab5557e19b2fd0315878c004dd8d56a6.
User released to closeDBI/disconnect/play; no more device actions. Fresh
detector log/frames changed since16:11Z; primary reviewing local copies.
Same Grok4.7 HIGH Cursorchat6367efa1 audits installed RO/config offline,
sendop pilot2-installed-loader-audit-send-1 deadline~20:06Z. Primary
Sol/xhigh verified, watcher3779610 active owner3150898. Playbook pulled/
read49f81b1. Finish offline review/compliance, then stop worker/watchers.
No code/build/reinstall/Eden/remote/git change;02488cdc remains installed.
Earlier operational states below are history.

CURRENT16:45Z offlinebootroot-causegoal, noEden/card/remote/codechange. Operator
confirmsAtmosphere1.12.1; lastLoadNro0x1216/cbb000 spansmatchbothBCATfaults
andpriorpage-releaseabort. UpstreamcleanupguardafterfallibleParseNroleaks
protectedsourcepages; game'sfailurebranchfrees/reusesthem. Source/binary
evidence /home/intpa/ssbu-cpu-pilot-work/disconnect-dialog/OFFLINE-BOOT-ROOT-CAUSE.md.
Stronginferencefor3reports, whyNROreject/bit42rendercrashunresolved. Telemetry
installunchangedverified02488cdc. Sol/xhighmodelwatcheractive; sameCursorGrokHIGH
chat6367efa1sourceauditactive, operationpilot2-offline-boot-source-send-1,
deadline18:35Z; finishreview/compliance/reapbeforeyield. Priorstateshistory.

FINAL16:19Z: authorized detector install and bounded boot-crash review
complete.02488cdc installed/readback-verified16:13:34Z, rollbackold29067bb6
verified, other16plugins/3protectedsettings unchanged.37files2707535B
captured; independent-verification.json under switch/install-telemetry-20261103-1611
rehashes all captured/readback/rollback/stage files. User already released
to closeDBI/disconnect/play; device operations closed, no further access.
No agent mount/connection/DBI control, no commit/push/tag/release.

Boot review identifies one new report01793721463 (Nov3 15:57Z): repeating
BCATCacheStorage DataAbort inmemset, LRmain+36fa0f4, matchingNov2report
01793619837. Older report has no memory_guard module, new has it; distinct
from prior FreeMemoryPages UserBreak, no common root or crash fix claimed.
No post-install runtime evidence yet. Detailed raw review/checklist:
work/disconnect-dialog/boot-crashes-1611-{review,compliance}.md.

Grok4.7HIGH worker finished, watcher3641530 stopped16:17:48Z and verified
absent; wrapper stopped. Primary Sol/xhigh reverified/no change; sole model
service stopped16:19:04Z inactive/dead/MainPID0/success. Receipt
switch/install-telemetry-20261103-1611/watchers-stopped.json. No live own
worker/watchers/compiler/Eden/queue/remote/device job. Current resolved
install checklist: sameinstallfolder/compliance-resolved.md and
docs/COMPLIANCE-DISCONNECT.md. Old GIF/dirtytree/historical process failures
remain; native UI/online/replay/allocation and root causes unverified.

Next: operator plays normally and reports any error/message/time, then
exposes DBI again for a fresh capture. Do not rerun completed install
phases. Safe exact local verification command:
/usr/bin/python3 -I /home/intpa/ssbu-cpu-pilot-work/switch/verify-telemetry-install.py
(through explicit pwsh7/resourceguard). Branchfeat/disconnect-reason-dialog,
HEAD475b0cfccdf91d452fc1353b397b6a7ea251b883, source unchanged/uncommitted.

INSTALL VERIFIED16:13:34Z: Telemetry candidate02488cdc is now on the
physical card under the explicit current operator order. Old detector
29067bb6 backed up locally and on-card at
ultimate/ssbu-desync-detector/backups/pre-02488cdc8e0e4ab2.nro, SHA-readback
verified. All17plugins/3protected files read back; other16plugins and all
protected settings unchanged. install-telemetry-20261103-1611/deployment.json
stateinstalled_verified; independent-verification.json asserts every local
capture/readback/rollback hash. Device operations closed16:13:34Z. User
told they can closeDBI/disconnect/play; no further USB/device access planned.
No agent mount/DBI/connection control. No publication/commit/push/tag.

37files/2707535B saved at work/switch/card-20261103-1611. Latest saved
crash01793721463 is DataAbort in BCATCacheStorage atnnSdk+50a380/memset+10,
LRmain+36fa0f4, with old detector still installed; not the previous
FreeMemoryPages/UserBreak signature. Local worker comparison/conclusion
pending, no crash fix claimed. New detector summary shows latestboot2online
sessions and0recordedmismatches; latest session normalend/desync_seen0,
peerreason1. These are pre-update logs, not new telemetry validation.

Same Grok4.7HIGH local crash worker active; no device permissions. Primary
watcher active until final cleanup. Earlier no-install/in-progress states
below are history. Detailed install report will be docs/DISCONNECT-DIALOG.md.

INSTALL IN PROGRESS16:13Z: Operator explicitly ordered installation over
the DBI USB connection on this machine and requested fresh boot-crash/log
capture so play can resume. Already exposed DBI endpoint read without any
mount/connection control.37files/2707535B captured read-only at
/home/intpa/ssbu-cpu-pilot-work/switch/card-20261103-1611 with SHA manifest.
Installed detector read as old29067bb6, not the telemetry candidate.
Preflight verified16:13:02Z under
/home/intpa/ssbu-cpu-pilot-work/switch/install-telemetry-20261103-1611:
all current plugins/protected files locally retained and target old hash
verified. Next exact operation: upgrade2-mtp.py install that directory.
Approved single target libssbu_desync_detector.nro ->02488cdc; rollback
ultimate/ssbu-desync-detector/backups/pre-02488cdc8e0e4ab2.nro and stage
outside loader. Require installed_verified and all readbacks before final.
No other plugins/settings/publication changes authorized or planned.

Same Cursor/Grok4.7HIGH chat6367efa1... resumed16:11:41Z for LOCAL captured
boot-crash analysis only; operationpilot2-boot-crashes-1611 timeout7200000ms
deadline18:12Z, bounded3min actual task. Owns boot-crashes-1611 reports
only; no USB/card/network/build/Eden. Primary owns device/transaction and
integration. Exact primary Sol/xhigh passed; sole512MiB owner-bound model
watcher rearmed after prior transient unit vanished. Reap before yielding.

FINAL TELEMETRY CHECKPOINT16:02:40Z supersedes earlier current paragraphs.
All15source inputs/fourartifacts/archive/ZIP verified; telemetry-verified.json.
Primary watcher stopped16:02:40Z (inactive/dead/MainPID0/success), no model
change, telemetry-watchers-stopped.json. All worker/watcher wrappers stopped.
Local candidate built for the operator's next normal Switch run:
SHA25602488cdc8e0e4ab2a3da0cd83cc2b2e363ef00c6e714227599675b7da43e82ee,
file53248B/resident135168B. Release/empty-feature diag NROs identical;
symbol-bearing ELFs, exact source inputs, manifest and install-layout ZIP
retained under /home/intpa/ssbu-cpu-pilot-work/disconnect-dialog/artifacts/telemetry-candidate.
No card install, version bump, commit, push, tag or release. This is an
unreleased0.2.0development artifact identified by SHA, not the card0.2.0.

Final source passed all-target Rust tests, all-target Clippy -D warnings,
no-default-features, Switch release and diag; twelve Python parser tests
pass. Logs/receipts telemetry-{test,clippy,nodefault,release,diag} and
telemetry-parser-final.log. Initial SDK u64 mismatch/unsafe warning fixed;
telemetry-build-corrections.md retains diagnostics. Source archive and
all four NRO/ELFs SHA-verified; owned target-model deleted373119848B with
telemetry-target-deletion.json, inherited target untouched.

New bounded telemetry:5s/event writer snapshots, console clock status and
query span, raw NIFM states/actual polling, session/frame context, frame-gap
observations and known loss/allocation counters; no new hooks or game-state
writes. Parser supports both rotated logs/latest boot, clock uncertainty
windows/discontinuities and loss totals. It makes no dock/router attribution.
Native dialog/layout/runtime-allocation/online/replay coverage remains
unverified. Full resolved checklist docs/COMPLIANCE-DISCONNECT.md records
those unknowns and existing/historical failures; no release certification.

Same Cursor/Grok4.7HIGH parser worker finished11tests; watcher3595170
stopped15:55:18Z, wrapper stopped. Primary corrected its clock precision
and passed12tests. Primary owns all files. Pair s08 never staged/launched;
requests removed15:46Z. No current own compiler/Eden/queue reservation or
remote session. Original603-file Ally profile/CLAIM restoration remains
verified; e07 released15:30Z. Primary watcher is reaped as recorded above.
Next: operator installs/plays normally per work/disconnect-dialog/NEXT-RUN.md,
then retain complete detector folder and correlate recurrence with router/
port records. User alone handles card connections/mounting/DBI/UMS.

CURRENT: Operator now requests practical telemetry for the next physical
run, with bounded testing. Runtime source adds periodic/event snapshots,
console clock anchors, raw network state/poll intervals, frame-gap and
capture-loss counters without adding hooks. Source is changed; old7d5d489f
is historical. New build/checks pending; no next-run artifact ready yet.
No s08 pair was launched/staged; both queue requests withdrawn15:46Z.
Same native Cursor chat6367efa1... now Grok4.7 HIGH implements parser/tests
only (pilot2-telemetry-parser-nov3,15:48Z,deadline17:48Z); primary owns
runtime/build/integration. Short Yoga compile claims only, one Cargo
subcommand each, telemetry-check.ps1 and per-command receipts. No current
Eden claim, card install, remote mutation, release or push. Full current
checkpoint: work/disconnect-dialog/runtime-validation.md.

CURRENT15:39Z: Unchanged candidate7d5d489f booted on Ally e07. Ordinary
Online navigation reached system applet2306-0520; main+37a272c trace never
hit, so no target-dialog or runtime allocation acceptance. Full23-file
evidence archive verified; current log0Unmapped/panic-abort-crash with2482
unclassified file-permission errors. Full603-file original profile/CLAIM
restored and independently verified twice; test clone removed. Job/task
gone and Ally RELEASED15:30:13Z, own queue rows removed. No current claim.
Same Cursor/Grok4.7xhigh chat6367efa1... now researches the existing private
two-host harness read-only (pilot2-pair-plan-nov3,15:32Z,7200000ms timeout,
deadline17:32Z). No Ally hosting permission or firewall changes. Exact
Sol/xhigh reverified; one512MiB owner-bound model watcher active after the
earlier128MiB watcher OOM. Reap before yielding. The older operational
paragraphs below are history; runtime-validation.md is current.

Runtime validation is now explicitly authorized: operator overrides the
earlier Eden stop for this candidate and requests an open host, updated
playbook and observational instrumentation without rebuilding. New
playbook03e23218af1eaa007793bbc21718bd594f4999c7 fully read. Current
runtime checklist/host/trace/worker state:
/home/intpa/ssbu-cpu-pilot-work/disconnect-dialog/runtime-validation.md.
That checkpoint supersedes the older authorization/watcher/worker states
below. No card install, push, tag or release is authorized.

Primary ssbu-cpu-pilot-2, Codex thread
01a19d4f-2312-71a2-8257-f68e795c1abe, verified gpt-6.1-sol/xhigh.
Repo /home/intpa/ssbu-desync-detector; branch
feat/disconnect-reason-dialog; baseline/HEAD
475b0cfccdf91d452fc1353b397b6a7ea251b883. Changes remain uncommitted.
No push, tag, version bump, release or card installation.

Scope: append a detected reason to actual communication-error messages,
preserving their text and codes. No usable session means Unknown. Current
English phrase filter and one formatter caller do not establish coverage
of every communication-error surface, language or system applet.
Network component attribution remains unresolved.

## Current candidate

Primary owns all source now. Classifier worker handed off; no source
ownership remains delegated. Evidence is scoped synchronously to the actual
online session before log draining. Playback, normal/stale/contended
episodes cannot supply a precise old reason. It uses no simulation writes,
native RNG calls or synthetic dialog on silence.

Hook v2: formatter epilogue .text+37ad8e8, aligned for a four-instruction
far patch (MOV/ADD/LDP/LDP), one instruction near. Six 64-byte guards.
Saved LR at formatter SP+520 must equal .text+37a272c; FP must equal
SP+512; caller buffer SP+528 is 0x800 UTF-16 units. Count is w20.
Native relocated MOV returns the new count, then native LDP restores
the caller's x20. Original formatter and setter still run.
Static evidence: formatter-prologue.log, skyline-patch-width.log,
ui-callback-v2-disasm.log, and dialog-guards-v2.log in the work folder.

The earlier v1 at37a272c was REJECTED: an unaligned far patch relocates
five instructions including the next BL. The four-MOV claim was wrong.
Old NROs are under artifacts/ui-v1-rejected; never install them.

Current release NRO SHA256:
7d5d489f2b6e993700681def8aa23eefdfa1925cde60cc3198e301788439f3bc.
File53248B/resident135168B (132KiB), v2-size.log.
Release and empty-feature diag NROs are byte-identical, retained with
separate ELF files under
/home/intpa/ssbu-cpu-pilot-work/disconnect-dialog/artifacts/ui-candidate.
manifest.json pins source hashes and artifacts, not a released commit.
source-inputs.tar.gz retains the exact Cargo/source/tests/parser inputs.

## Evidence and live operational state

Durable work ledger:
/home/intpa/ssbu-cpu-pilot-work/disconnect-dialog/progress.md.
Final current gates v2-test.log, v2-clippy.log, v2-no-default.log,
v2-release.log, v2-diag.log all pass without warnings. Allocation test
covers classifier and append, not installed-card runtime allocations.
Five tested Python summary tests passed earlier; parser is still uncommitted.

Dated17-plugin card snapshot switch/card-plugins-20261103-1306 verified
against receipt. No references at current hook/ABI offsets in
card-dialog-v2-scan-20261103.log and preceding ABI scan. Lookup37a4450
is owned by elite-rematch; the new site does not overlap it.

Cursor/Grok4.7xhigh SAME chat6367efa1-ec97-4dd7-a5c0-da774006aa37
finished v2 static review at14:31:44Z. ui-v2-review.md finds no static
blocker for the selected caller, not deployment acceptance. Its watcher
was reaped14:31:21Z and wrapper stopped by operation
pilot2-stop-ui-v2-review-nov3. All owned workers are stopped.
The review did not accept the final NRO: it was not supplied its hash and
the compliance tool discovered the inherited old target. Primary owns
the final retained-NRO/source receipt and ARM64 callback verification.
The worker's partial compliance report leaves manual rows unresolved;
the primary report preserves that failure and resolves overall task rows.
Network audit switched from Opus to Grok4.7high on the same chat as
requested. No new Opus turns.

## Network finding

HB0609 used only with Switch, no Bazzite sharing. Original Ethernet path:
HB0609 -> ArcherBE670 -> MS305E -> NanoPiR6S; mesh satellite via MoCA.
Console stayed docked until after the error. User moved dock directly
to MS305E port2. This bypasses Archer forwarding but retains shared LAN.
Read-only Netgear capture13:31:47Z: dock1G, R6S/AP2.5G, all CRC0,
loop prevention enabled, power saving disabled. Post-move counters with
unknown reset time do not establish incident-time health. No settings changed.

Raw card/router evidence is switch/card-20261103-1220; tested summary
incident-summary.json. No recorded mismatch; latest session lacks end
records and network state changed nearby. Operator now independently
confirms wired MAC00:E0:4C:69:2D:AD from Switch settings: configured
bazzite DHCP entry is the Switch's dock connection. Its repeated DHCP
discoveries belong to this wired path. Exact event ordering and an
independently measured clock offset remain missing.
No router reboot/interface teardown found. No component root cause
established. The MAC pairing is now confirmed; the fitted66.52s clock
offset is not independently confirmed. No need to ask for the wired MAC again.
No credential belongs in a file, prompt or report.

## Task checklist and limits

Playbook ae87443cbb801472e474b3f0c20b5f9c3f5f1f77 fully read by primary
and each worker. Full requirement table is docs/COMPLIANCE-DISCONNECT.md;
all29 primary manual rows are resolved, with failures retained.
M implementation/profile/toolchain/version/guards/allocator/static-writer:
source checked; current host/release/diag gates pass.
M after-prologue and dated card scan: current v2 static review passed for
one caller. Runtime and broader dialog coverage remain unverified.
M docs/handoff/evidence: current; existing GIF480px/11.1s fails its rule.
M disk: owned target-model deleted; target-deletion.json lists1840files/
561834735bytes removed. NRO/ELF/source archive retained; inherited target untouched.
M model/commands/delegation: primary Sol/xhigh verified; watcher stopped
and verified inactive/dead/MainPID0 before yield. Early command-guard
omissions, chained independent shell operations, incomplete worker
checklist and classifier initial xhigh/high gate remain failures.
Classifier also reported a local messaging
token printed in a diagnostic tool output; do not repeat it or claim
perfect privacy compliance.
C sharing: N/A, dated scan found no current-site owner; no vtable.
C online: simulation-read-only review applies; replay A/B, UI/layout,
runtime allocation and two-console path coverage NOT performed.
C numeric log evidence: tested parser exists but is uncommitted.
C release/tag/rebuild/card-write rows: not authorized, none done.
Eden rows: not authorized under recorded stop; no new run/profile changes.
No full playbook completion or deployment acceptance is claimed.

## Next exact work

Local implementation, static review, final fmt, whitespace check,
source/archive/artifact receipt verification and explicit-current-NRO
compliance are complete. Primary watcher is reaped and verified inactive.
No further worker turn or
build is needed absent a concrete source change. Owned cache cleanup is
complete; do not repeat its deletion or use inherited target for this task.

Next dependent work requires an explicit operator override of the Eden
stop below. If granted, read current coordination and playbook runtime
gates before any claim; verify actual matching dialog/layout, runtime
allocation and replay A/B with the retained candidate. Two-console
validation and any card installation require their own explicit order.
For a naturally recurring network incident, collect synchronized Switch,
router and port2 carrier/counter evidence on the current direct path.
Do not name a faulty dock/router/AP without discriminating evidence.

Read-only resume checks (each through guarded local PowerShell7):
`/usr/bin/python3 -I /tmp/ssbu-cpu-pilot-2-model.py check`
then verify/rearm one owner-bound watcher under the routing authority;
`/usr/bin/python3 -I /tmp/ssbu-pilot2-verify-receipt.py`;
`git -C /home/intpa/ssbu-desync-detector status --short`.
These commands do not authorize runtime/device/publication actions.

Binding: NO MORE EDEN by this primary absent an explicit operator override
of the Nov2 stop in /tmp/deluxe-surface-1007/OWNER-COORDINATION.md:3241
and3257. User alone connects/mounts/unmounts/ejects/remounts card, DBI/UMS.
No push without explicit order; Forgejo only; Claude/Cursor workers only.
Local Ubuntu commands explicit /usr/bin/pwsh with memory/time guard.
# Current primary checkpoint — 2026-11-03 17:47Z

Root-cause goal returned BLOCKED on absent failing-boot NRO/NRR bytes and actual
loader identity. Offline source/crash review supports an Atmosphere1.12.1
cleanup defect for three reports; separate render fault has no known writer.
Telemetry02488cdc installation stays verified and unchanged. No Eden/card/
build/commit/push/tag/release. PowerShell recovered without a shell exception;
primary model Sol/xhigh verified, watcher reaped17:46:05Z inactive/dead/MainPID0/
success/no pair change. Grok HIGH render review and watcher finished; wrapper
closed. No live own workers/watchers/remote/device jobs. Await operator input
or intended future capture, not another deployment. Evidence and exact next
inputs: work/disconnect-dialog/OFFLINE-BOOT-ROOT-CAUSE.md and NEXT-BOOT-EVIDENCE.md
under /home/intpa/ssbu-cpu-pilot-work. All states below are history.

## Earlier checkpoint — 17:30Z

Offline boot investigation only; detector source/build/card state unchanged.
Telemetry02488cdc remains installed/verified at16:13Z. No Eden/device access.
Primary root-cause report identifies an Atmosphère1.12.1 loader cleanup defect
matching three saved crashes; rejected image/authorization trigger and separate
render-pointer crash remain open. Worker/watchers for independent Grok HIGH
review stopped; wrapper closed. Primary watcher cleanup is unverified because
local PowerShell now exits134 at startup; direct-Python exception requested,
not assumed. Do not resume USB operations or repeat deployment receipts.
Full current state and error:
/home/intpa/ssbu-cpu-pilot-work/disconnect-dialog/OFFLINE-BOOT-ROOT-CAUSE.md
Older operational statements below are history. No own commit/push/tag/release.

