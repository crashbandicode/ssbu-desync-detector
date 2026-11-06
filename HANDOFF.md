# Popup-route worker — 2026-11-06 00:15Z (current; everything below is history)

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
- ab1 23:43-23:48Z (one instance, card's 18 plugins + Deluxe config): replay stops at 4151 exactly as
  with the detector alone.
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

