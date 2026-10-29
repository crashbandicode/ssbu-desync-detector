//! The 2026-10-29 Eden proof as a regression test. Two instances played a
//! Quickplay match; on instance b an Eden-only probe moved a fighter below
//! the blast zone, so b left vanilla and the match ended in a desync at
//! frame 1128. Instance a ran only this plugin. Both recordings were then
//! played from the Vault on a. The files are the plugin's `frames-N.bin`:
//! `*-online.bin` from each console's match, `*-vanilla.bin` from the
//! playback of that console's recording.
use ssbu_desync_detector::model::{
    referee, FrameRecord, RefereeReport, Verdict, KIND_ONLINE, KIND_PLAYBACK,
};

const FRAME: u32 = 1128;
/// The values each console compared at frame 1128 (both `events.log`s).
const A_VALUE: u32 = 0x4aec250b;
const B_VALUE: u32 = 0x720d5ecb;

fn records(bytes: &[u8], kind: u32) -> Vec<FrameRecord> {
    let mut records: Vec<_> = bytes
        .as_chunks::<20>()
        .0
        .iter()
        .filter_map(|record| FrameRecord::from_bytes(record))
        .filter(|r| r.kind() == kind)
        .collect();
    if kind == KIND_PLAYBACK {
        records.sort_by_key(|r| r.frame);
    }
    records
}

fn judge(console: &[u8], vanilla: &[u8], ours: u32, theirs: u32) -> RefereeReport {
    referee(
        &records(console, KIND_ONLINE),
        &records(vanilla, KIND_PLAYBACK),
        FRAME,
        ours,
        theirs,
    )
}

const A_ONLINE: &[u8] = include_bytes!("eden-probe-20261029/a-online.bin");
const A_VANILLA: &[u8] = include_bytes!("eden-probe-20261029/a-vanilla.bin");
const B_ONLINE: &[u8] = include_bytes!("eden-probe-20261029/b-online.bin");
const B_VANILLA: &[u8] = include_bytes!("eden-probe-20261029/b-vanilla.bin");

#[test]
fn the_clean_console_is_cleared() {
    let report = judge(A_ONLINE, A_VANILLA, A_VALUE, B_VALUE);
    assert_eq!(report.verdict, Verdict::OtherConsoleDiverged);
    assert_eq!(report.vanilla_at_desync, Some(A_VALUE));
    assert_eq!(report.offset, 0);
    assert_eq!(report.matched, 1128);
    assert_eq!(report.first_divergence, None);
    assert_eq!(report.first_position_mismatch, None);
}

#[test]
fn the_probed_console_is_named() {
    let report = judge(B_ONLINE, B_VANILLA, B_VALUE, A_VALUE);
    assert_eq!(report.verdict, Verdict::ThisConsoleDiverged);
    assert_eq!(report.vanilla_at_desync, Some(A_VALUE));
    assert_eq!(report.first_divergence, Some(FRAME));
    // The game rejected b's recording at the same frame.
    assert_eq!(report.first_position_mismatch, Some(FRAME));
}

#[test]
fn either_recording_can_referee_the_match() {
    // Both recordings hold the same inputs, so a's clean playback also
    // decides b's desync.
    let report = judge(B_ONLINE, A_VANILLA, B_VALUE, A_VALUE);
    assert_eq!(report.verdict, Verdict::ThisConsoleDiverged);
    assert_eq!(report.vanilla_at_desync, Some(A_VALUE));
}
