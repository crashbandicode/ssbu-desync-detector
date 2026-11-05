# Design: the reason on the no-argument pop-up route — 2026-11-05

Working file beside `HANDOFF.md` (not a shipped doc). Branch
`feat/disconnect-popup-route`, baseline commit `316f18d` (inherited work).

## What failed on the Switch

Telemetry build `02488cdc` logged `dialog=installed` at boot and a real desync
at 15:32:57 (`events.log` lines 3380-3387), then no `communication_text`
event. The only text hook was the formatter epilogue `.37ad8e8`, accepted only
when the saved LR is `.37a272c` (caller `.37a2660`, label + arguments).

## Static route (13.0.5 main NSO, hash-verified)

The common pop-up manager's dispatch calls `.1eef640` (`.32f1cc4`), which sets
the body text pane `set_txt_0N`:

- arguments present: `bl .37a2660` at `.1eef84c` (the hooked route);
- no arguments: `bl .377a120` at `.1eef834`.

`.377a120` (1172 callers: every label text without arguments) builds the
label, looks it up (`.37a4450`), zeroes a 0x1000-byte stack buffer at
`sp+0x120`, calls the plain formatter `.37ae170(buf, text, attr)` (bounded to
0x7ff units, terminator at the count and at unit 0x7ff), then
`.377a250: mov w2,w0; add x1,sp,#0x120; mov x0,x19; mov w3,wzr;
.377a260: bl .37a2270` (the pane setter, which sizes the pane's string buffer
to the count it is given). A sentence without placeholders such as
"A connection error occurred." takes this route, never `.37ab750`.
Dynamic proof is pending (Eden pair, guest tracing).

## Change

One route abstraction in `src/dialog.rs`: a `Route` names its hooked
instruction, the register with the native UTF-16 count, the frame check
(FP = SP + n), the caller buffer (SP + n, 0x800 units), an optional saved-LR
caller check, and its 64-byte guards. `append_in_frame` is the whole hook
minus register access: frame check, caller check, phrase filter, reason,
append. Both hooks in `runtime.rs` call one function with their route; the
event names the route.

| Route | Hook | Count | Frame | Buffer | Caller check | Guards |
| --- | --- | --- | --- | --- | --- | --- |
| `args` (existing) | `.37ad8e8` formatter epilogue | w20 | FP = SP+512 | SP+528 | saved LR at SP+520 = `.37a272c` | 6 existing |
| `plain` (new) | `.377a250` in `.377a120`, after the formatter returns, before the setter | w0 | FP = SP+0x1150 | SP+0x120 | none (the hook is in the buffer's owner) | `.377a120`, `.377a210`, `.377a250`, `.37ae580` |

`.377a250` is 8-byte aligned: the far patch relocates four instructions
(MOV, ADD sp-relative, MOV, MOV), the near patch one; the `bl` at `.377a260`
stays in place. No direct branch in `.text` targets `.377a254..0x377a25f`
(`route-guards.log`). Updating w0 makes the relocated `mov w2, w0` pass the
new count to the setter; x0 is then overwritten by `mov x0, x19`.

Same guarantees as `args`: 64-byte guards on every hooked, relied-on and
called site; hook after the prologue on the most specific instruction (the
one point where this function holds the finished text and has not yet set
it); no heap; original units kept; reason appended only when the whole
suffix and its terminator fit in 0x800 units; no pointer kept. Install line:
`dialog=installed text_routes=args,plain` (`dialog=partial` /
`not_installed` and `dialog_guard_failed=<site>` otherwise).

## Requirement trace

| Operator clause | Design | Test |
| --- | --- | --- |
| "an online match ended with the orange-and-white pop-up "A connection error occurred" and our disconnect reason was NOT added to it" | `plain` route at `.377a250`, the no-argument text route of the pop-up body | `plain_route_frame_finds_the_caller_buffer`, `routes_hook_aligned_moves_and_relocate_no_call` |
| "Make the ssbu-desync-detector add its reason to that pop-up" | `append_in_frame` shared by both routes, same reason and append | `both_routes_append_the_same_way_for_one_table_of_cases`, `communication_text_event_names_the_route` |
| "investigate why that match desynced" | Vault playback of `match-0.bin` with the detector, `referee`/`compare` against the console's `frames-0.bin` | `the_referee_names_the_console_that_left_vanilla` |
