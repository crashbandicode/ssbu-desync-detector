#![allow(dead_code)]
#[path = "../src/disconnect.rs"]
mod disconnect;
#[path = "../src/evidence.rs"]
mod evidence;
#[path = "../src/model.rs"]
mod model;

use disconnect::{Reason, TICKS_PER_SECOND as SECOND, VERDICT_WINDOW_TICKS};
use evidence::Evidence;
use model::{RawEvent, EV_DESYNC, EV_SYNC_END, KIND_ONLINE, KIND_PLAYBACK};

fn event(tick: u64, code: u8, a: u64, b: u64) -> RawEvent {
    RawEvent { tick, code, a, b }
}

#[test]
fn no_session_and_playback_leave_the_native_message_alone() {
    let evidence = Evidence::new();
    assert_eq!(evidence.reason(SECOND), None);
    evidence.frame(SECOND, KIND_PLAYBACK, 0x20000, 0);
    evidence.event(0x20000, &event(SECOND, EV_DESYNC, 0, 0));
    evidence.link(SECOND, 2, 0xd46e);
    assert_eq!(evidence.reason(SECOND), None);
}

#[test]
fn mismatch_is_available_before_a_writer_drain() {
    let evidence = Evidence::new();
    evidence.frame(SECOND, KIND_ONLINE, 0x10000, 100);
    evidence.event(0x10000, &event(SECOND + 1, EV_DESYNC, 0, 0));
    assert_eq!(evidence.reason(SECOND + 2), Some(Reason::Desync));
}

#[test]
fn other_session_objects_cannot_supply_a_reason() {
    let evidence = Evidence::new();
    evidence.frame(SECOND, KIND_ONLINE, 0x10000, 100);
    evidence.event(0x20000, &event(SECOND + 1, EV_DESYNC, 0, 0));
    assert_eq!(evidence.reason(SECOND + 2), Some(Reason::Unknown));
}

#[test]
fn a_new_object_or_a_reused_object_starting_a_match_discards_old_evidence() {
    let evidence = Evidence::new();
    evidence.frame(SECOND, KIND_ONLINE, 0x10000, 20);
    evidence.event(0x10000, &event(SECOND + 1, EV_DESYNC, 0, 0));
    evidence.frame(SECOND + 2, KIND_ONLINE, 0x10000, 0);
    assert_eq!(evidence.reason(SECOND + 3), Some(Reason::Unknown));
    evidence.event(0x10000, &event(SECOND + 4, EV_DESYNC, 0, 0));
    evidence.frame(SECOND + 5, KIND_ONLINE, 0x20000, 0);
    assert_eq!(evidence.reason(SECOND + 6), Some(Reason::Unknown));
}

#[test]
fn natural_u16_frame_wrap_retains_this_matchs_evidence() {
    let evidence = Evidence::new();
    evidence.frame(SECOND, KIND_ONLINE, 0x10000, 65535);
    evidence.event(0x10000, &event(SECOND + 1, EV_DESYNC, 0, 0));
    evidence.frame(SECOND + 2, KIND_ONLINE, 0x10000, 0);
    assert_eq!(evidence.reason(SECOND + 3), Some(Reason::Desync));
}

#[test]
fn normal_end_suppresses_the_reason_until_the_next_match() {
    let evidence = Evidence::new();
    evidence.frame(SECOND, KIND_ONLINE, 0x10000, 20);
    evidence.event(0x10000, &event(SECOND + 1, EV_SYNC_END, 1, 0));
    evidence.frame(SECOND + 2, KIND_ONLINE, 0x10000, 21);
    assert_eq!(evidence.reason(SECOND + 3), None);
    evidence.frame(SECOND + 4, KIND_ONLINE, 0x10000, 0);
    assert_eq!(evidence.reason(SECOND + 5), Some(Reason::Unknown));
}

#[test]
fn undocking_after_the_error_is_not_its_cause() {
    let evidence = Evidence::new();
    evidence.frame(SECOND, KIND_ONLINE, 0x10000, 20);
    evidence.link(SECOND + 1, 2, 4u64 << 32);
    assert_eq!(evidence.reason(SECOND + 2), Some(Reason::Unknown));
    evidence.link(SECOND + 3, 1, 0xd46e);
    assert_eq!(evidence.reason(SECOND + 4), Some(Reason::Unknown));
}

#[test]
fn a_link_fault_after_the_recorded_end_is_not_its_cause() {
    let evidence = Evidence::new();
    evidence.frame(SECOND, KIND_ONLINE, 0x10000, 20);
    evidence.event(0x10000, &event(SECOND + 1, EV_SYNC_END, 2, 0x16cfc18));
    evidence.link(SECOND + 2, 2, 0xd46e);
    assert_eq!(evidence.reason(SECOND + 3), Some(Reason::Timeout));
}

#[test]
fn no_error_event_is_synthesized_when_frames_stop() {
    let evidence = Evidence::new();
    evidence.frame(SECOND, KIND_ONLINE, 0x10000, 20);
    assert_eq!(evidence.reason(SECOND + VERDICT_WINDOW_TICKS + 1), None);
}

#[test]
fn playback_then_online_starts_a_fresh_episode() {
    let evidence = Evidence::new();
    evidence.frame(SECOND, KIND_ONLINE, 0x10000, 20);
    evidence.event(0x10000, &event(SECOND + 1, EV_DESYNC, 0, 0));
    evidence.frame(SECOND + 2, KIND_PLAYBACK, 0x20000, 0);
    assert_eq!(evidence.reason(SECOND + 3), None);
    evidence.frame(SECOND + 4, KIND_ONLINE, 0x10000, 0);
    assert_eq!(evidence.reason(SECOND + 5), Some(Reason::Unknown));
}
