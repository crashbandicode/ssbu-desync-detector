# Installed telemetry candidate compliance — 2026-11-03

Developmentcandidate02488cdc installed16:13:34Z under explicit operator order.
Readbacks/rollback independently verified; all device operations closed. Console run pending.
Playbook03e23218af1eaa007793bbc21718bd594f4999c7; no new code/build/Eden work this turn.

2026-11-13 note: rows below were true on 2026-11-03. Since then `docs/media/referee.gif` was rescaled to
1280×720, 9.6 s (`ec7d0fe`, 2026-11-06); `tools/summarize_disconnect.py` and its 12 tests are committed (`316f18d`,
2026-11-05); and release 0.3.0 replaced `02488cdc` on the card on 2026-11-06 ([VERSIONS.md](VERSIONS.md)).


| Requirement | Tag | Result | Evidence |
| --- | --- | --- | --- |
| crate-type is ["cdylib"] only (no rlib) | M | pass | Cargo.toml crate-type = ['cdylib'] |
| release profile opt-level "s", lto, codegen-units 1, panic abort | M | pass | Cargo.toml [profile.release] as required |
| skyline pinned to =0.6.0 | M | pass | Cargo.toml |
| diag cargo feature declared | M | pass | Cargo.toml [features] |
| toolchain 1.98.1 | M | pass | rust-toolchain.toml |
| runtime only under cfg(target_os = "switch") | M | pass | src/lib.rs |
| supported version 13.0.5 in model | M | pass | src/model.rs |
| every site guard is 64 bytes | M | pass | 11 guard(s) in src/, sizes [64] |
| counting allocator reports heap_allocations | M | pass | src/**/*.rs |
| host tests in model (cargo test) | M | pass | 14 #[test] in src/** |
| README carries the AI-assistance disclaimer | M | pass | README.md |
| README has a GIF, build commands and a license line | M | pass | README.md |
| GIF referee.gif: 1280 wide, under 10 s | M | fail | 480 px, 11.1 s |
| dated evidence doc present | M | pass | docs/COMPLIANCE-DISCONNECT.md, docs/DISCONNECT-DIALOG.md, docs/EDEN-PROOF.md, docs/HOOKS.md, docs/WHAT-IS-CHECKED.md |
| numbers in the evidence come from tested scripts over the logs | C | fail | Twelve parser tests pass and candidate manifest asserts input/artifact SHAs; scripts remain uncommitted, so the committed-script requirement is pending |
| handoff file present | M | pass | HANDOFF.md |
| remote is Forgejo, not GitHub | M | pass | https://git.syntaxbyexample.com/intpa/ssbu-desync-detector.git |
| working tree clean | M | fail | git status: dirty |
| no script mounts, unmounts or ejects the card (the user connects and disconnects it) | M | pass | none in the tracked scripts or the playbook's tools/switch |
| build identity | M | pass | HEAD 475b0cf; no tag on HEAD |
| installed development NRO identity | M | pass | /home/intpa/ssbu-cpu-pilot-work/disconnect-dialog/artifacts/telemetry-candidate/release/libssbu_desync_detector.nro sha256 02488cdc8e0e4ab2 resident 135168 B (132 KiB) |
| no diag strings in the shipped NRO | M | pass | libssbu_desync_detector.nro: no diag markers |
| Gates run: test, clippy -D warnings, no-default-features, skyline release, diag build | M | pass | telemetry-{test,clippy,nodefault,release,diag}.json and .log all exit0 on final source; warnings denied for all-target Clippy, no Switch warnings. Twelve parser tests pass, telemetry-parser-final.log. Initial u64 mismatch and unsafe warning corrected; raw diagnostics retained in telemetry-build-corrections.md |
| Fixes: root cause fixed, or each workaround approved by the operator before it shipped and listed with its date | M | pass | Telemetry addresses missing periodic/clock/coverage observations without changing classifier precedence or any hook. Parser preserves clock uncertainty and boot scope. This is an authorized diagnostic addition, not a claimed hardware fix; original physical disconnect root cause remains unresolved |
| Card scan for the site (and vtable slot) over a dated card copy | M | pass | Original dated17NRO site/ABI scans retained; current-card-site-scan.log repeats37ad8e8/37ab750 scan against all17freshpreflight NROs, no references. Runtime guards remain authoritative; scan alone is not universal hook-sharing proof |
| Hook sharing or chaining; both stand-in modes and load orders in Eden | C | N/A: no other owner found at the selected site | Dated scans show no current-site reference; elite-rematch owns different lookup37a4450 |
| Hook after the prologue or the function called instead | M | pass | Formatter epilogue37ad8e8 after full prologue; aligned near1/far4 position-independent instructions; formatter-prologue.log, skyline-patch-width.log, ui-v2-review.md |
| Shipped NRO writes only its install line (plus a logging plugin's data files) | M | pass | InstalleddevelopmentNRO02488cdc, not a publishedrelease. Existing logging data files/installline and5s/event fixed-buffer telemetry only; no new hooks/per-frame formatting/diag feature strings |
| Online: read-only rules reviewed hook by hook | C | pass | Existing per-hook source review retained. Telemetry adds existing SDK read queries on the writer, no network sends, native RNG calls or simulation writes. UI still only changes its caller text/count. New runtime query behavior has not run on console |
| Online: replay A/B with the desync detector identical | C | fail | Unverified for candidate02488cdc. Operator selected bounded host/build checks and next normal console run; no new replay campaign performed |
| Online: two consoles past the changed path, desync_seen=0 at the same frame | C | fail | Unverified for candidate02488cdc. Pair s08 never launched/staged and both requests withdrawn15:46Z. No console installation or two-peer proof |
| Eden: claim written and released, also after a failed run | M | N/A: no new telemetry Eden run | Each Yoga compiler claim was released in finally; per-command receipts and full ledger retained. Historical e07 released15:30Z with runtime-release.json |
| Eden: one instance per host, MemAvailable >= 10 GiB, not profile a or :61 | M | N/A: no new telemetry Eden run | No Eden launched. Compiler admission reads whole ledger with no expiry, both queues and current processes/resources; second compiler slot allowed only by operator two-compile rule; no local Eden |
| Eden: profile copy with hard-linked game content; only the copy changed | M | N/A: no new emulator profile | Pair s08 never staged. Historical e07 full603-file original restored and SHA-verified twice; no profile changed by telemetry work |
| Copies: byte-identical files linked, differing or written-in-place files copied | M | N/A: no new game-content copy | NRO/ELF retained independently with exact SHA receipts; inherited game/profile directories untouched |
| Disk: one target dir per source copy (never shared between copies), cargo home shared per host and toolchain, df checked (>= 40 GB), finished build products deleted with a manifest | M | pass | Every telemetry compiler receipt records>=40GiB disk and>=10GiB available; sole owned target-model, shared cargo home/incremental0. Four artifacts/source archive retained and SHA-verified; telemetry-target-deletion.json deletes373119848B of owned cache, inherited target untouched |
| Eden: waits gated on a log line or screenshot check | M | N/A: no new telemetry Eden run | Historical e07 screenshot/log-gated actions retained; no game actions in this telemetry pass |
| Eden: failed precondition aborted and retried under the next label | M | N/A: no new telemetry Eden run | Compiler admissions blocked on actual foreign claims; no failed admission was treated as a compile. Historical e07 fullscreen capture rejected and windowed capture used |
| Eden: game-side facts from guest tracepoints on a run already happening; diag rebuild or extra trace boot only with a stated reason | M | N/A: no new telemetry Eden run | Historical e07 used tracepoint in existing run, no hit. This rebuild implements requested new telemetry, not merely a trace observation. No synthetic events or guest writes |
| Eden: profile restored, temp files removed, host released | M | N/A: no new telemetry Eden run | Historical e07 full603files/CLAIM independently verified twice, test clone removed, Job/task gone, host released. No new profile/host mutation |
| Eden: console.log searched for panic/abort/crash; run.json and memory.peak kept | M | N/A: no new telemetry Eden run | Historical e07 current log0Unmapped/0panic-abort-crash,2482unclassified permission errors and peak7257878528B retained. That is not new-source runtime evidence |
| Every local command under the memory and time guard | M | fail | Early bootstrap/metadata guard omissions recorded in progress.md; subsequent shells use pwsh7 under systemd memory limit and timeout. Cannot repair prior omissions retroactively |
| Evidence doc states run labels, build, what was done, what the log showed | M | pass | docs/DISCONNECT-DIALOG.md identifies installed02488cdc, pre-update37filecapture,17plugin/3protected readbacks and unresolved UI/runtime/boot-cause questions. deployment.json, independent-verification.json and captured raw reports retained |
| GIF caption gives date and build; clip shows start, action and result | M | fail | Existing referee.gif480px/11.1s is unrelated; no telemetry/dialog GIF. Not expanded into a capture campaign for this next-run candidate |
| No operator quotes, no claims beyond the tests | M | pass | Product docs paraphrase requirements and distinguish observations/inferences/untested runtime. No credentials in docs; separate privacy failure below |
| Handoff file current (branch, commit, evidence, live state, next commands) | M | pass | Detector/CPU-pilot/memory-guard handoffs record explicit install authorization, verified installed hash, closed device operations, rollback/receipts/capture, unchangedotherplugins and local crash review state. No runtime host claim |
| Release: version bump, annotated tag, assets verified by anonymous download | C | not authorized | No version bump/tag/release/push requested or performed |
| Release: rebuild of the tag byte-identical to the released NRO | C | not authorized | No version bump/tag/release/push requested or performed |
| Card: MTP preflight, rollback copy, readbacks, receipts not re-run | C | pass | Explicit current operator install order in operator-approval.json; fresh preflight_verified16:13:02Z and installed_verified16:13:34Z, phases each ran once. Old29067bb6 archive/stage/new02488cdc readbacks verified;17plugins+3protected files verified; independent-verification.json rehashes37captured files/all20before-after files/rollback/stage |
| Card: the user connected and disconnected it; the agent never mounted, unmounted, ejected or remounted it, nor started or stopped DBI | M | pass | Only already exposed local DBI MTP was used. No agent mount/unmount/eject/remount or DBI/UMS/connect/disconnect operation. All device operations closed16:13:34Z; user told they can closeDBI/disconnect/play. No further device accesses |
| Nothing pushed, tagged or written to the card without the operator's order | M | pass | Only explicitly ordered detector replacement and its rollback/stage outside loader were written. Other16plugins and3protected settings unchanged. No commit/push/tag/release or other card edits |
| Model/effort gate and watcher per the routing authority | M | fail | Current primary exactSol/xhigh verified/no change; sole modelservice reaped16:19:04Z inactive/dead/MainPID0/success, watchers-stopped.json. GrokHIGH watcher3641530 stopped/verified absent and wrapper stopped. Historical initial classifier effort mismatch/temporary duplicate watcher remain recorded; no unexpected pair change this install turn |

| Target dialog/layout, runtime allocations and next console run | M | fail | Unverified for telemetry02488cdc. Host tests and build are complete; natural next run supplies runtime evidence |
| Every worker completed own checklist | M | fail | Historical partial worker reviews remain recorded; latest parser worker resolved30individual manual rows, its11tests passed, watcher/wrapper stopped. Primary corrected clock precision and passed12tests |
| Secret handling | M | fail | Historical worker diagnostic-token exposure retained without reproducing it. No secret needed or used by telemetry work; no new remote/card/device access |
| One logical shell operation | M | fail | Historical violations remain. Some independent parser/size and metadata reads were also combined this pass. Each Cargo subcommand was separate, sequential, bounded and receipted |

Unverified: natural console dialog/layout/runtime allocations, replay/two-console behavior and boot-crash root causes. Latest boot report is BCATCacheStorage DataAbort inmemset, before the telemetry install; no hardware or plugin culprit inferred.
Existing/historical failures retained: GIF480px/11.1s and caption, dirty/uncommittedtree/scripts, earlier model/guard/shell/privacy/partial-worker discipline. Installation is verified, not full runtime certification.
Not performed/authorized: new Eden/GIF campaign, release/version bump/tag/commit/push. The user alone handles card/DBI/USB connections.
