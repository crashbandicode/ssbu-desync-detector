//! Compares two playbacks of the same replay frame by frame.
//!
//! ```text
//! cargo run --example compare -- <baseline frames-N.bin> <candidate frames-N.bin>
//! ```
//!
//! Prints the first frame whose check value differs and the first frame whose
//! recomputed position checksum differs, and whether either run stopped
//! because its positions no longer matched the recording.
use ssbu_desync_detector::model::{FrameRecord, KIND_PLAYBACK};
use std::collections::BTreeMap;
use std::process::ExitCode;

fn load(path: &str) -> Result<BTreeMap<u32, FrameRecord>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let mut frames = BTreeMap::new();
    for record in bytes.as_chunks::<20>().0.iter() {
        if let Some(record) = FrameRecord::from_bytes(record).filter(|r| r.kind() == KIND_PLAYBACK)
        {
            frames.entry(record.frame).or_insert(record);
        }
    }
    Ok(frames)
}

fn run(args: &[String]) -> Result<bool, String> {
    let [baseline, candidate] = args else {
        return Err("usage: compare <baseline frames-N.bin> <candidate frames-N.bin>".into());
    };
    let (base, cand) = (load(baseline)?, load(candidate)?);
    let common: Vec<u32> = base
        .keys()
        .filter(|f| cand.contains_key(f))
        .copied()
        .collect();
    println!(
        "baseline {} frames, candidate {} frames, {} in common",
        base.len(),
        cand.len(),
        common.len()
    );
    let check = common.iter().find(|f| base[f].rng != cand[f].rng);
    let position = common.iter().find(|f| base[f].position != cand[f].position);
    match check {
        Some(f) => println!(
            "check value first differs at frame {f} ({:.2} s): 0x{:08x} vs 0x{:08x}",
            f64::from(*f) / 60.0,
            base[f].rng,
            cand[f].rng
        ),
        None => println!("check value identical on every common frame"),
    }
    match position {
        Some(f) => println!("positions first differ at frame {f}"),
        None => println!("positions identical on every common frame"),
    }
    for (name, frames) in [("baseline", &base), ("candidate", &cand)] {
        if let Some(r) = frames.values().find(|r| r.position != r.recorded) {
            println!(
                "{name}: the recording was not reproduced from frame {}",
                r.frame
            );
        }
    }
    Ok(check.is_none() && position.is_none() && base.len() == cand.len())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}
