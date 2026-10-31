//! Decides which console left vanilla in a desync.
//!
//! ```text
//! cargo run --example referee -- <console-dir> <slot> <vanilla-dir> <slot> [FRAME,OURS,THEIRS]
//! ```
//!
//! `<console-dir>` is the detector folder from the console that played the
//! match (`events.log`, `frames-N.bin`); `<vanilla-dir>` is the folder from an
//! unmodded game that played that console's `match-N.bin` from the Vault. The
//! desync values come from the console's `events.log` unless given.
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::{referee, FrameRecord, Verdict, KIND_ONLINE, KIND_PLAYBACK};
use std::path::Path;
use std::process::ExitCode;

fn frames(dir: &Path, slot: &str, kind: u32) -> Result<Vec<FrameRecord>, String> {
    let path = dir.join(format!("frames-{slot}.bin"));
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(bytes
        .as_chunks::<20>()
        .0
        .iter()
        .filter_map(|record| FrameRecord::from_bytes(record))
        .filter(|r| r.kind() == kind)
        .collect())
}

fn hex(field: &str) -> Option<u32> {
    u32::from_str_radix(field.trim_start_matches("0x"), 16).ok()
}

/// The first desync line of the newest online session in `slot`:
/// (frame, ours, theirs, peer).
fn desync_from_log(dir: &Path, slot: &str) -> Result<(u32, u32, u32, u32), String> {
    let path = dir.join("events.log");
    let log = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let start = format!("session slot={slot} kind=online");
    let lines: Vec<&str> = log.lines().collect();
    let begin = lines
        .iter()
        .rposition(|line| line.contains(&start))
        .ok_or(format!(
            "no online session in slot {slot} in {}",
            path.display()
        ))?;
    for line in &lines[begin + 1..] {
        if line.contains(" session slot=") {
            break;
        }
        if !line.contains(" desync frame=") {
            continue;
        }
        let field = |name: &str| {
            line.split_whitespace()
                .find_map(|word| word.strip_prefix(name))
                .map(str::to_owned)
        };
        let parsed = (|| {
            Some((
                field("frame=")?.parse().ok()?,
                hex(&field("ours=")?)?,
                hex(&field("theirs=")?)?,
                field("peer=")?.parse().ok()?,
            ))
        })();
        return parsed.ok_or(format!("unreadable desync line: {line}"));
    }
    Err(format!("no desync recorded in slot {slot}'s session"))
}

fn run(args: &[String]) -> Result<(), String> {
    let [console_dir, console_slot, vanilla_dir, vanilla_slot, rest @ ..] = args else {
        return Err(
            "usage: referee <console-dir> <slot> <vanilla-dir> <slot> [FRAME,OURS,THEIRS]".into(),
        );
    };
    let (console_dir, vanilla_dir) = (Path::new(console_dir), Path::new(vanilla_dir));
    let (frame, ours, theirs, peer) = match rest.first() {
        Some(given) => {
            let parts: Vec<&str> = given.split(',').collect();
            match parts[..] {
                [f, o, t] => (
                    f.parse().map_err(|_| "bad FRAME")?,
                    hex(o).ok_or("bad OURS")?,
                    hex(t).ok_or("bad THEIRS")?,
                    u32::MAX,
                ),
                _ => return Err("expected FRAME,OURS,THEIRS".into()),
            }
        }
        None => desync_from_log(console_dir, console_slot)?,
    };
    let console = frames(console_dir, console_slot, KIND_ONLINE)?;
    let mut vanilla = frames(vanilla_dir, vanilla_slot, KIND_PLAYBACK)?;
    vanilla.sort_by_key(|r| r.frame);
    if console.is_empty() || vanilla.is_empty() {
        return Err("no online frames or no playback frames in those slots".into());
    }
    let report = referee(&console, &vanilla, frame, ours, theirs);

    let span = |records: &[FrameRecord]| {
        let low = records.iter().map(|r| r.frame).min().unwrap_or(0);
        let high = records.iter().map(|r| r.frame).max().unwrap_or(0);
        format!("{} frames ({low}..{high})", records.len())
    };
    let or_none = |value: Option<u32>| value.map_or("none".to_owned(), |v| v.to_string());
    println!("console:  slot {console_slot}, {}", span(&console));
    match peer {
        u32::MAX => println!("desync:   frame {frame}"),
        peer => println!("desync:   frame {frame}, from station {peer}"),
    }
    println!("          this console  0x{ours:08x}");
    println!("          other console 0x{theirs:08x}");
    println!("vanilla:  slot {vanilla_slot}, {}", span(&vanilla));
    println!(
        "          aligned at offset {} on {} frames before the desync",
        report.offset, report.matched
    );
    match report.vanilla_at_desync {
        Some(value) => {
            let whose = if value == ours {
                "this console's value"
            } else if value == theirs {
                "the other console's value"
            } else {
                "neither value"
            };
            println!("          at the desync frame 0x{value:08x} = {whose}");
        }
        None => println!("          did not reach the desync frame"),
    }
    println!(
        "          first frame where this console differs: {}",
        or_none(report.first_divergence)
    );
    println!(
        "          first playback frame whose positions it could not reproduce: {}",
        or_none(report.first_position_mismatch)
    );
    let verdict = match report.verdict {
        Verdict::OtherConsoleDiverged => "the other console left vanilla",
        Verdict::ThisConsoleDiverged => "this console left vanilla",
        Verdict::BothDiverged => "both consoles left vanilla",
        Verdict::Inconclusive => "inconclusive (the vanilla run does not decide it)",
    };
    println!("verdict:  {verdict}");
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
