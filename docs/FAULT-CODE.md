# Fault-table log

2026-11-07. Design, written before the code. Playbook `3f96569`.

The online match stops storing frames after Smash reads a fault table and
arms a 30-frame countdown. The reader is `.16acd30`. It returns 0 while the
word at `+0x18` of `[[main+0x52c3940]]` is nonzero. It returns 1 through 9
for the first nonzero word at `+0x1c` through `+0x3c`, when the low 10 bits
of the object's first word are set. The return is stored at session `+0x5e`
(`strb w0, [x19, #0x5e]` at `.16d5ecc`). The countdown is armed at
`.16d5ee8`, and only on that pass.

## What the log records

One `events.log` line, once, on that arming pass. Not every frame.

```text
<tick> fault code=<n> frame=<frame|none> mask=<0xhhh|none> words=<10 integers|unread>
```

`code` is the byte the game stored, the number `.16acd30` returned.
`words` are the ten words at `+0x18` through `+0x3c`, in that order. Word 0
is the suppress word. Words 1 through 9 are what the reader would return as
1 through 9. `mask` is the low 10 bits of the object's first word. `frame`
is the frame number the existing frame hook reads on the same pass. That
hook claims the staged snapshot only while it is at most 50 ms old. After
100 ms the line still comes out with `frame=none`.

The hook reads registers and those words. It does not write them, does not
draw a random number, and does not skip the game's own stores. The original
four instructions still run.

## Site

| | |
|---|---|
| Hook | `.16d5ee8`, `orr w8, wzr, #0x1e0000`, 8-byte aligned |
| Patch | 16 bytes: `orr`, `str w8, [x19, #0x64]`, `orr`, `strb w8, [x19, #0x63]`. No branch and no call in that span |
| Relied on | `.16d5ec4` `bl .16acd30` (`0x97ff5b9b`); `.16d5ecc` `strb w0, [x19, #0x5e]` (`0x39017a60`); `.16d5e90` `mov x19, x0` (`0xaa0003f3`) |
| Object | `[[main+0x52c3940]]`, rejected under `0x10000` or not 4-byte aligned |

A mismatch of the 64-byte guard or any relied-on word leaves the site
unhooked and the install line says `fault=skipped`.

## Card scan

2026-11-06 snapshot
`ssbu-desync-detector-card-20261106-v030/after/.../skyline/plugins`,
18 NROs, the same set that was on the card. `scan-offsets.py` for
`16d5ee8`, `16d5ec4` and `16acd30`: no reference in any other plugin.
`libssbu_desync_detector.nro` has `movz/movk` of `0x16d5ec0`, which is
`sync_end_trigger`'s `referee` caller, the instruction after the ready-word
`bl .16ce7c0`. It is not a hook of this site. Sharing does not apply.

The snapshot is on the Yoga at `/home/intpa/ssbu-desync-detector-card-20261106-v030/` (the 0.3.0 install receipt).

## Seen since (2026-11-13 note)

- Eden, 2026-11-12, release 0.4.0 `a31ad657`: `fault code=1 frame=3215 mask=0x3ee words=0,1,0,0,0,0,0,0,0,0` on
  both consoles of `dbl-ga-02420ee`, `dbl-gb-02420ee` and `dbl-gb2-02420ee` (Yoga).
- 0.5.0 `proof-dc8ee988-r2` (console a) and 0.5.1 `proof-36027bb7-r9` (console b, `frame=2015`, no input after a
  refused runner call): `fault code=1`, then `session_end … net_result=-993 via=countdown`.
- Code 2 has not been seen. No Switch log shows a `fault` line yet ([VERSIONS.md](VERSIONS.md)).

## Requirement trace

| Requirement (paraphrased) | Design | Test |
|---|---|---|
| Extend the detector to log what is needed at that point, including the number | `fault` line: the stored return (`code`), the ten words, the mask, and the frame of the arming pass | `fault_line_names_the_code_and_every_word` |
