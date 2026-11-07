# Compliance — fault-table log

2026-11-07. Playbook `3f96569`.
`compliance.py --base 49214c13267cc1f813ca86d606c6bd9f2232de76 --trace docs/FAULT-CODE.md --nro target/aarch64-skyline-switch/release/libssbu_desync_detector.nro`.

Machine rows passed except `working tree clean` (fail: the fault log, the notice matcher, and the GIF leftovers are uncommitted; no commit was ordered). Release NRO sha256 prefix `82539499150a1e89`, 143360 bytes, no diag markers. The requirement-trace row passed: one clause, `fault_line_names_the_code_and_every_word` exists.

| Requirement | Result | Evidence |
| --- | --- | --- |
| Gates | pass | `cargo test` (all four harnesses), `cargo clippy --all-targets -- -D warnings`, `cargo build --no-default-features`, `cargo skyline build --release` with `-D warnings`, and the diag Skyline build in `target/diag-skyline` with `-D warnings` |
| Fixes | pass | No workaround. The image does not name eight of the nine entries; the log records the number and the ten words on the next arming |
| Card scan | pass | 2026-11-06 snapshot, 18 NROs. `16d5ee8`, `16d5ec4`, `16acd30`: no other plugin. This plugin's `0x16d5ec0` is the referee caller id. `docs/FAULT-CODE.md` |
| Hook sharing | N/A | That scan showed nobody else at the site |
| Hook after the prologue | pass | `.16d5ee8` is inside `.16d5e50`, after its prologue, on the four countdown stores. The 16-byte patch has no branch and no call |
| Shipped NRO writes | pass | One `fault` line per arming, plus the install line (`fault=installed` or `fault=skipped`). Not per frame. Not copied to the card |
| Online read-only | pass | `docs/WHAT-IS-CHECKED.md`. The hook takes `&InlineCtx`, reads the stored byte and the table, and the original four instructions still run |
| Replay A/B | not authorized | No Eden boot was ordered |
| Two consoles | not authorized | No online session with this build was ordered |
| Eden rows (claim, hosts, profile, copies, waits, runner, grouping, retry, tracing, restore, console.log) | not authorized | No Eden boot was ordered. A local boot cannot raise this fault without writing game state |
| Disk | pass | `df -h /` before the builds: 213G free. Release and diag use separate target dirs. The two NROs stay local as the build evidence |
| Commands under a guard | pass | Each Cargo invocation was `systemd-run --user --scope -p MemoryMax=8G` with `timeout` |
| Evidence doc run labels | N/A | No boot. `docs/FAULT-CODE.md` states the line the next match writes |
| GIF caption | N/A | This change adds no clip. The existing README GIFs passed the size check |
| No operator quotes, no claims beyond the tests | pass | README and the shipped docs do not quote the request. The host test proves the line format. Nothing here claims the hook ran in a match |
| Handoff current | pass | `HANDOFF.md`: branch `readme-gifs`, HEAD `ec7d0fe`, dirty, not installed |
| Release version and tag | pass, pending the tag | Cargo.toml is 0.4.0. The annotated tag and Forgejo release follow this commit; the card install waits until the Switch is connected |
| Tag rebuild | N/A | No tag |
| Card MTP | N/A | No card write was ordered |
| Card connect | pass | This change did not mount, unmount, eject, or start or stop DBI |
| Nothing pushed or written to the card | pass | No push, no tag, no card write |
| Model and watcher | pass | `cursor-model-watcher-f774f0f3.service` is active. No pair-change warning |
