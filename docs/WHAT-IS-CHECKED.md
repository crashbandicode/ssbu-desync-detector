# What the online check compares (13.0.5)

Static study of the 13.0.5 image plus Eden runs on 2026-10-29. "Verified" means
checked in the binary or measured; anything else is marked as inferred.

## The check

- Every frame each console compares one u32: the XOR of the `w` word (`+0xc`)
  of the nine xorshift128 streams in the game RNG manager `*(*(main+0x52c35b0))`
  (computed at `.text+0x16d6c3c..0x16d6cdc`, compared at `.16cf1d0`). Nothing
  else is compared: not inputs, positions, damage or the streams' draw
  counters. Verified.
- The nine streams are created once at boot and all re-seeded with the match
  seed at match setup (`.1342830`), so both consoles start equal. Verified.
- In practice the value is a per-frame count of how many times each stream
  has been drawn: from the recorded seed and integer draw counts alone, every
  recorded check value of the Eden proof runs and the Town replays is
  reproduced. Verified.

| Stream | Category (`sv_math::rand` hash) | Main users |
|---|---|---|
| 0 | any other hash | `IS_RANDOM`, wait/squat motion switches, star-KO camera offsets, item `IT_IS_RND_PERCENT` |
| 1 | `fighter` | fighter common code and specializers, hit-list tie-break, damage reactions |
| 2 | `weapon` | weapon (projectile/article) specializers |
| 3 | `stage` | stage gimmicks and actors, the stage-manager seed |
| 4 | `ai` | CPU AI, `app::ai_random` |
| 5 | `enemy` | enemies and bosses (not used fighter-vs-fighter) |
| 6 | `item` | items, `app::random::*` |
| 7 | `sound` | some voice/SE helpers |
| 8 | `camera` | a camera-mode helper |

Not checked: the wall-clock "lib" RNG `.52391c0` (effect `*_RND` commands, UI),
the effect library's emitter RNG `.4f49cb4`, libc `rand()`, and per-object
streams such as a stage's local xorshift. They matter only if their results
reach game logic that draws a checked stream.

## What one match draws

An Eden-only diagnostic build (not released) hooked all 632 draw instructions
and logged every draw during a 12,977-frame Final Destination replay between
two CPUs. Verified:

| Stream | Draws | Seen by a hook | Sites that fired |
|---|---:|---:|---:|
| other | 14 | 14 | 5 of 27 |
| fighter | 13,101 | 13,101 | 14 of 142 |
| weapon | 2 | 2 | 2 of 61 |
| stage | 0 | 0 | 0 of 21 |
| ai | 111,481 | 111,479 | 33 of 248 |
| enemy | 0 | 0 | 0 of 51 |
| item | 18 | 18 | 3 of 69 |
| sound | 198 | 198 | 1 of 8 |
| camera | 0 | 0 | 0 of 5 |

## Damage trace

The same replay with 30 % added to the first fighter at frame 600 (the build's
damage read-back logged 0, so the amount is unconfirmed; the positions show it
took effect). Verified:

| Frame | First difference |
|---|---|
| 1571 | positions: the next hit's knockback |
| 1577 | the check: `sound` drawn 24 times instead of 25. The missing draw (`.23f623c`) is the launch voice in `.6e6260`: above the common param `damage_fly_se_threshold_voice` it picks one of two voices with a random variant, then waits 274 frames before the next. |
| 1606, 1651, 1814, 2013 | `ai`, `fighter`, `other`, `item` |

Weapon, stage, enemy and camera never differed.

## Not understood yet

- Most draw sites never ran: no stage hazards, few items and projectiles, no
  enemies. Two CPUs make `ai` dominate; a match between two people should
  draw it rarely, if at all (inferred).
- The busiest sites are in unexported functions and are not identified: 78 %
  of `fighter` draws come from `.4a9a00` (about one per frame), most `ai`
  draws from `.34eb4c`, `.34eb84` and `.2db4c8`.
- Two `ai` draws came from code outside the 632 hooked instructions.
- Of the unchecked RNGs, only Town and City's path into a checked stream has
  been traced.

## Rules for plugins

A No contest needs the two simulations to differ and the difference to change
how often some checked stream is drawn. Plugins installed identically on both
consoles and behaving deterministically are fine; the risk is asymmetry (one
console only, different settings, or decisions based on local data such as
the clock, files, frame pacing, menus, effect handles or the unchecked RNGs).

**Cannot move it**
- Reading memory or registers, logging, file I/O on the plugin's own thread.
- Render-only resources: models, textures, materials, audio waveforms, and
  motions that drive nothing the simulation reads.
- Visibility and draw flags.
- Changing an effect's look while the native request still runs.
- UI and HUD that write no battle state.
- Remapping the local controller before the game packs the input (both
  consoles then run the same inputs). Inferred.

**Moves it on the same frame**
- Calling any checked draw (`sv_math::rand/randf`, `app::random::*`,
  `app::ai_random::*`, Lua `sv_math.*`, `IS_RANDOM`).
- Skipping, adding or reordering native code that draws: status and ACMD
  branches, AI, items, stage actors. Some effect- or sound-looking helpers draw
  checked streams too (Jigglypuff dash smoke on stream 1, Pokémon Trainer
  voices and a sound helper on stream 7), so returning early from them moves
  the check.

**Moves it later**
- Any change to simulated state: damage, position, velocity, knockback,
  hit/hurtboxes, shields, timers, status, stage or item state, inputs after
  unpacking. Nothing of this is compared, but from that moment the consoles
  play different matches and the next draw that depends on it ends the match
  (a KO within a frame; +30 % damage after 67 s in an Eden online test, and
  in the damage trace above 6 frames after the next hit).
- Changes to an unchecked source that game logic consumes. Verified case:
  nulling Town and City's bubble effect changes its lifetime, which feeds the
  stage's local RNG, whose actors then make one fewer `stage` draw (61 s into
  the test replay).

"Positions are not compared" does not make position changes safe; it only
delays the No contest.

## Testing a plugin

Play the same replay twice in Eden with only this plugin installed, then with
the plugin under test (same configuration users will run), and compare the
`frames-N.bin` files:

```text
cargo run --example compare -- <baseline frames-N.bin> <candidate frames-N.bin>
```

Identical check values and positions over the whole replay mean the plugin
changed no checked draw for that content. Any check difference means it would
end an online match against a console without it. A position difference with
an identical check is a desync that has not been noticed yet: treat it as a
failure. A replay test cannot show behaviour gated on online state, content
the replay does not exercise, or divergence that never reaches a draw or a
position within the replay.

Results so far (Eden, 2026-10-29):

| Change | Replay | Result |
|---|---|---|
| FX plugin, Town and City effects Off | Town and City, 14,732 frames | check differs from frame 3658 (61 s); positions identical |
| FX plugin, Town and City effects On | same | identical |
| Cosmic Destination, Less Distracting Northern Cave, Less Distracting Umbra Clock Tower (ARCropolis) | Final Destination, 12,977 frames | identical |
| same three mods | Umbra Clock Tower (Battlefield form), 15,374 frames | identical |

Northern Cave was not played (no replay on that stage); its mod replaces only
models, motions and render files, none of which name a collision bone.
