# ssbu-desync-detector

> **Disclaimer:** this project was developed with the assistance of AI
> (Claude, Codex and other coding agents) under the operator's direction and
> review. Nothing is claimed beyond the dated tests cited below.

A Skyline plugin for Super Smash Bros. Ultimate 13.0.5. When an online match
ends because the two consoles stopped agreeing (a desync), it notes the frame
and keeps the match's recording, so an unmodded game can replay it and show
which console left vanilla. It also adds the reason to the game's own
connection-error pop-up. It reads the match and never changes it.

![Referee](docs/media/referee.gif)

*Eden, 2026-10-29, detector build `b15e929`: an unmodded game plays back the
recording of the console that left vanilla and stops at the desync ("This
replay can't be played"). The run is in [docs/EDEN-PROOF.md](docs/EDEN-PROOF.md).*

## How it works

1. **Play online.** Each frame both consoles compare one check value. When
   they disagree, the plugin writes the frame and both values to its log and
   saves the match's recording as `match-N.bin`. Both consoles see the same
   two values, so the values alone can't say which one left vanilla.
2. **Replay it unmodded.** Copy `match-N.bin` to
   `save_data/replay/replayYYYYMMDDhhmmss.bin` in a game with only this plugin
   (Eden works) and play it from the Vault. If this console left vanilla, the
   game stops where it can no longer reproduce the recording (above).
3. **Ask the referee.** With both detector folders:

   ```text
   cargo run --example referee -- <console-dir> <slot> <vanilla-dir> <slot>
   ```

   It ends with a verdict: this console left vanilla, the other console left
   vanilla, both did, or inconclusive.

## The reason in the pop-up

| | Without the reason | With 0.3.0 |
|---|---|---|
| **Desync** | ![Desync, without the reason: the game's connection-error pop-up and no Reason line](docs/media/desync-before.gif) | ![Desync, with 0.3.0: the same pop-up plus Reason: Desync detected](docs/media/desync-after.gif) |

*Eden, 2026-11-06, the same Quickplay route with TrainingRooms 0.2.0-rc.2
(`23b89786`) on one console, so the first death desyncs. Without: release
0.2.0 (`29067bb6`, commit `1daf9a2`), run `dd-gif-b1`. With: release 0.3.0
(`0ac90bc6`, commit `49214c1`), run `dd-gif-a1`: "Reason: Desync detected.
The consoles stopped agreeing on the match."*

## Where it works

- **Online matches:** tested in Eden (Quickplay pairs) and seen on a Switch:
  the 2026-11-05 desync there was recorded and refereed
  ([docs/DISCONNECT-DIALOG.md](docs/DISCONNECT-DIALOG.md)).
- **Local wireless:** the session and how it ended are logged (Eden,
  2026-11-12); the match-telemetry lines that need the online penalty table
  don't appear there.
- **Offline matches:** not recorded. Replays played from the Vault are.
- **Text:** the reason is added to English pop-ups only.

## Install

Copy `libssbu_desync_detector.nro` to
`sd:/atmosphere/contents/01006A800016E000/romfs/skyline/plugins/`. The latest
release is
[v0.4.0](https://git.syntaxbyexample.com/intpa/ssbu-desync-detector/releases/tag/v0.4.0)
(2026-11-07); `main` builds 0.5.1, which is not released. Output goes to
`sd:/ultimate/ssbu-desync-detector/`. Recordings are kept only for matches
that did not end normally, and the log is capped at two 1 MiB files.

## More

- [docs/VERSIONS.md](docs/VERSIONS.md): what each version added and how it
  was tested.
- [docs/EDEN-PROOF.md](docs/EDEN-PROOF.md): the forced-desync test in Eden.
- [docs/WHAT-IS-CHECKED.md](docs/WHAT-IS-CHECKED.md): what the online check
  compares, and how to test a plugin for desyncs (`cargo run --example compare`).
- [docs/HOOKS.md](docs/HOOKS.md): hook sites, files and verdicts.
- [docs/MATCH-TELEMETRY.md](docs/MATCH-TELEMETRY.md): the 0.5.x match lines.
- [docs/DISCONNECT-DIALOG.md](docs/DISCONNECT-DIALOG.md): the pop-up reason
  and the 2026-11-05 desync, traced to TrainingRooms' waiting-room flag.

## Build

```text
cargo test
cargo skyline build --release
```

The NRO is `target/aarch64-skyline-switch/release/libssbu_desync_detector.nro`.

## License

MIT
