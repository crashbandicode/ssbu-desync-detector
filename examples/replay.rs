//! Checks a saved `match-N.bin` and prepares it for the Vault.
//!
//! ```text
//! cargo run --example replay -- check <match.bin>
//! cargo run --example replay -- outer <match.bin> <saved-replay.bin> <out.bin>
//! ```
//!
//! `check` runs the game's replay validator. `outer` replaces a minimal outer
//! header with the one from a replay the game saved (the Vault shows names
//! and stages from it), keeping the match's time. Copy the result to
//! `save_data/replay/replayYYYYMMDDhhmmss.bin` and reopen the Vault.
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::{validate_entry, CFMK_BYTES, CFMK_RECORDS, OUTER_BYTES};
use std::process::ExitCode;

fn read(path: &str) -> Result<Vec<u8>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    if bytes.len() <= OUTER_BYTES + CFMK_BYTES {
        return Err(format!("{path}: too short for a replay"));
    }
    Ok(bytes)
}

fn run(args: &[String]) -> Result<(), String> {
    match args {
        [command, file] if command == "check" => {
            let bytes = read(file)?;
            let entry = &bytes[OUTER_BYTES..];
            let records = u16::from_le_bytes([entry[CFMK_RECORDS], entry[CFMK_RECORDS + 1]]);
            let time = u64::from_le_bytes(bytes[..8].try_into().unwrap_or_default());
            println!("{file}: {} bytes, {records} records", bytes.len());
            println!(
                "outer header: {}",
                if bytes[12..OUTER_BYTES].iter().all(|b| *b == 0) {
                    "minimal"
                } else {
                    "from the game"
                }
            );
            println!("time: 0x{time:016x}");
            validate_entry(entry).map_err(|why| format!("the game would reject it: {why}"))?;
            println!("the game's validator accepts it");
            Ok(())
        }
        [command, file, saved, out] if command == "outer" => {
            let mut bytes = read(file)?;
            let saved = read(saved)?;
            let time = bytes[..8].to_vec();
            bytes[..OUTER_BYTES].copy_from_slice(&saved[..OUTER_BYTES]);
            if time.iter().any(|b| *b != 0) {
                bytes[..8].copy_from_slice(&time);
            }
            validate_entry(&bytes[OUTER_BYTES..])
                .map_err(|why| format!("the game would reject it: {why}"))?;
            std::fs::write(out, &bytes).map_err(|e| format!("{out}: {e}"))?;
            println!("wrote {out} ({} bytes)", bytes.len());
            Ok(())
        }
        _ => {
            Err("usage: replay check <match.bin> | outer <match.bin> <saved.bin> <out.bin>".into())
        }
    }
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
