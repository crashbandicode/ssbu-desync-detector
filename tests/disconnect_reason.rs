//! Host tests of the disconnect-reason latch (`src/disconnect.rs`) against the
//! real model API: events are built with `model::pack_*`, their field layout
//! is pinned to `model::describe`'s output (the lines the Eden runs logged),
//! and the callers come from `model::sync_end_trigger`. The expectations are
//! the requirements, not the implementation: precedence, scoping, boundaries
//! on every window, normal and repeated ends, resets, and no allocation.
#[path = "../src/dialog.rs"]
#[allow(dead_code)]
mod dialog;
#[path = "../src/disconnect.rs"]
#[allow(dead_code)]
mod disconnect;
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use core::mem::size_of;
use disconnect::{
    classify, Facts, Latch, LocalFault, Observed, Reason, Rejection, Verdict, Withheld,
    END_GRACE_TICKS, LINK_WINDOW_TICKS, SKEW_TICKS, TICKS_PER_SECOND, VERDICT_WINDOW_TICKS,
};
use model::{
    describe, pack_desync, pack_recording, pack_sync_end, sync_end_trigger, Line, RawEvent,
    EV_DESYNC, EV_LINK, EV_PEER_ABORT, EV_PLAYBACK_MISMATCH, EV_RECORDING, EV_SESSION, EV_SYNC_END,
    KIND_ONLINE, RECORDING_NORMAL_END,
};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

// ---- a counting allocator, per thread, to prove the latch never allocates ----

thread_local! {
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
}

struct Counting;

fn count() {
    let _ = ALLOCATIONS.try_with(|n| n.set(n.get() + 1));
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        System.alloc(layout)
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        System.alloc_zeroed(layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count();
        System.realloc(ptr, layout, size)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

// ---- fixtures ----

const S: u64 = TICKS_PER_SECOND;
const START: u64 = 100 * S;
/// Link observations as `(result, state)`: the Eden logs' failed call at boot,
/// a working call that reports a state other than connected, and connected.
const STATUS_ERROR: (u32, u32) = (0xca6e, 0);
const NOT_CONNECTED: (u32, u32) = (0, 1);
const CONNECTED: (u32, u32) = (0, 4);
/// Real `.text` caller offsets from `model::sync_end_trigger`.
const C_DESYNC: u64 = 0x16d6b08;
const C_SILENT: u64 = 0x16cfc18;
const C_TIMEOUT: u64 = 0x16cfb8c;
const C_NORMAL: u64 = 0x1569718;
const C_NORMAL_2: u64 = 0x1568824;
const C_REFEREE: u64 = 0x16d5ec0;

fn live() -> Latch {
    let mut latch = Latch::new();
    assert_eq!(latch.begin_online(START, START), Observed::Recorded);
    latch
}

fn link(latch: &mut Latch, at: u64, (result, state): (u32, u32)) -> Observed {
    latch.link(at, at, result, state)
}

fn end(latch: &mut Latch, at: u64, kind: u32, caller: u64) -> Observed {
    latch.sync_end(at, at, kind, false, caller)
}

fn ev_sync_end(tick: u64, kind: u32, desync_seen: bool, caller: u64) -> RawEvent {
    RawEvent {
        tick,
        code: EV_SYNC_END,
        a: pack_sync_end(kind, desync_seen, false, 0, 1132),
        b: caller,
    }
}

fn ev_desync(tick: u64) -> RawEvent {
    let (a, b) = pack_desync(0x720d5ecb, 0x4aec250b, 1, 1128);
    RawEvent {
        tick,
        code: EV_DESYNC,
        a,
        b,
    }
}

/// Sender 0, as the runtime hook stores it: `sender | reason << 8`.
fn ev_peer_end(tick: u64, reason: u64) -> RawEvent {
    RawEvent {
        tick,
        code: EV_PEER_ABORT,
        a: reason << 8,
        b: 992,
    }
}

fn ev_link(tick: u64, result: u32, interface: u8, strength: u8, state: u32) -> RawEvent {
    RawEvent {
        tick,
        code: EV_LINK,
        a: u64::from(interface) | u64::from(strength) << 32,
        b: u64::from(result) | u64::from(state) << 32,
    }
}

fn line(event: &RawEvent) -> String {
    let mut out = Line::<192>::new();
    describe(event, &mut out).unwrap();
    String::from_utf8(out.as_bytes().to_vec()).unwrap()
}

fn show(reason: Reason) -> Verdict {
    Verdict::Show(reason)
}

// ---- the texts ----

#[test]
fn texts_are_stable_static_complete_sentences() {
    const GOLDEN: [(Reason, &str, &str); 6] = [
        (
            Reason::Desync,
            "desync",
            "Desync detected. The consoles stopped agreeing on the match.",
        ),
        (
            Reason::LocalLink,
            "local_link",
            "Local connection problem. This console did not report a connected network.",
        ),
        (
            Reason::LocalStatus,
            "local_status",
            "Local network status error. This console could not read its network status.",
        ),
        (
            Reason::PeerEnded,
            "peer_ended",
            "The other console ended the session.",
        ),
        (
            Reason::Timeout,
            "timeout",
            "Connection timed out. Nothing arrived from the other console in time.",
        ),
        (
            Reason::Unknown,
            "unknown",
            "The cause of the disconnection could not be determined.",
        ),
    ];
    // The text is a constant: nothing is formatted at run time.
    const DESYNC_TEXT: &str = Reason::Desync.text();
    assert_eq!(DESYNC_TEXT, GOLDEN[0].2);
    assert_eq!(Reason::ALL.len(), GOLDEN.len());
    for (reason, name, text) in GOLDEN {
        assert!(Reason::ALL.contains(&reason));
        assert_eq!(reason.name(), name);
        assert_eq!(reason.text(), text);
    }
    // Properties that must survive any rewording.
    let forbidden = [
        "dock",
        "router",
        "cable",
        "modem",
        "wifi",
        "wi-fi",
        "quit",
        "left",
        "intentional",
        "purpose",
        "your",
        "fault",
        "{",
        "}",
        "0x",
    ];
    for reason in Reason::ALL {
        let text = reason.text();
        assert!(text.is_ascii() && !text.contains('\n'), "{text}");
        assert!(text.chars().next().unwrap().is_ascii_uppercase(), "{text}");
        assert!(text.ends_with('.'), "{text}");
        assert!(!text.chars().any(|c| c.is_ascii_digit()), "{text}");
        assert!(text.len() <= 100, "{text}");
        for word in forbidden {
            assert!(!text.to_lowercase().contains(word), "{text} has {word}");
        }
        assert!(reason
            .name()
            .chars()
            .all(|c| c.is_ascii_lowercase() || c == '_'));
    }
    for (i, a) in Reason::ALL.iter().enumerate() {
        for b in &Reason::ALL[i + 1..] {
            assert_ne!(a.text(), b.text());
            assert_ne!(a.name(), b.name());
        }
    }
    // A mismatch names no console; a timeout names no departure.
    for word in ["this console", "other console", "opponent"] {
        assert!(!Reason::Desync.text().to_lowercase().contains(word));
    }
    assert!(Reason::Timeout.text().contains("timed out"));
    assert!(!Reason::Timeout.text().contains("disconnect"));
    // Only a shown reason has text.
    assert_eq!(show(Reason::Timeout).text(), Some(Reason::Timeout.text()));
    assert_eq!(show(Reason::Timeout).reason(), Some(Reason::Timeout));
    assert_eq!(Verdict::Withheld(Withheld::NormalEnd).text(), None);
    assert_eq!(Verdict::Withheld(Withheld::Stale).reason(), None);
}

// ---- precedence ----

#[test]
fn precedence_is_mismatch_local_peer_timeout_unknown() {
    let locals = [
        None,
        Some(LocalFault::NotConnected),
        Some(LocalFault::StatusError),
    ];
    for mismatch in [false, true] {
        for local in locals {
            for peer_ended in [false, true] {
                for timed_out in [false, true] {
                    let facts = Facts {
                        mismatch,
                        local,
                        peer_ended,
                        timed_out,
                    };
                    let expected = if mismatch {
                        Reason::Desync
                    } else if local == Some(LocalFault::NotConnected) {
                        Reason::LocalLink
                    } else if local == Some(LocalFault::StatusError) {
                        Reason::LocalStatus
                    } else if peer_ended {
                        Reason::PeerEnded
                    } else if timed_out {
                        Reason::Timeout
                    } else {
                        Reason::Unknown
                    };
                    assert_eq!(classify(facts), expected, "{facts:?}");
                }
            }
        }
    }
    assert_eq!(classify(Facts::default()), Reason::Unknown);
}

#[test]
fn every_kind_of_evidence_reaches_the_verdict_in_that_order() {
    // All four at once, then peeled one by one from the most specific.
    let build = |mismatch: bool, local: bool, peer: bool| {
        let mut latch = live();
        if mismatch {
            assert_eq!(
                latch.desync(START + 10 * S, START + 10 * S),
                Observed::Recorded
            );
        }
        if local {
            link(&mut latch, START + 11 * S, NOT_CONNECTED);
        }
        if peer {
            assert_eq!(
                latch.peer_end(START + 12 * S, START + 12 * S, 7),
                Observed::Recorded
            );
        }
        end(&mut latch, START + 13 * S, 2, C_SILENT);
        latch.verdict(START + 14 * S)
    };
    assert_eq!(build(true, true, true), show(Reason::Desync));
    assert_eq!(build(false, true, true), show(Reason::LocalLink));
    assert_eq!(build(false, false, true), show(Reason::PeerEnded));
    assert_eq!(build(false, false, false), show(Reason::Timeout));
}

#[test]
fn arrival_order_before_the_end_does_not_change_the_reason() {
    const PERMUTATIONS: [[usize; 3]; 6] = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    for order in PERMUTATIONS {
        for present in [
            [true, true, true],
            [false, true, true],
            [false, false, true],
        ] {
            let mut latch = live();
            for (slot, kind) in order.into_iter().enumerate() {
                let at = START + (10 + slot as u64) * S;
                if !present[kind] {
                    continue;
                }
                match kind {
                    0 => latch.desync(at, at),
                    1 => link(&mut latch, at, NOT_CONNECTED),
                    _ => latch.peer_end(at, at, 7),
                };
            }
            end(&mut latch, START + 13 * S, 2, C_SILENT);
            let expected = match present {
                [true, _, _] => Reason::Desync,
                [false, true, _] => Reason::LocalLink,
                _ => Reason::PeerEnded,
            };
            assert_eq!(
                latch.verdict(START + 14 * S),
                show(expected),
                "{order:?} {present:?}"
            );
        }
    }
}

// ---- desync ----

#[test]
fn a_recorded_mismatch_is_a_desync_from_any_of_its_three_sources() {
    // A mismatch row alone, before any end is seen.
    let mut rows = live();
    assert_eq!(
        rows.desync(START + 5 * S, START + 5 * S),
        Observed::Recorded
    );
    assert_eq!(rows.verdict(START + 6 * S), show(Reason::Desync));
    // The session's own desync flag on an end, with no row logged.
    let mut flag = live();
    assert_eq!(
        flag.sync_end(START + 5 * S, START + 5 * S, 2, true, 7),
        Observed::Recorded
    );
    assert_eq!(flag.verdict(START + 6 * S), show(Reason::Desync));
    // The flag on a kind-1 end is still abnormal: not a normal end.
    let mut flag_kind_1 = live();
    flag_kind_1.sync_end(START + 5 * S, START + 5 * S, 1, true, C_NORMAL);
    assert_eq!(flag_kind_1.verdict(START + 6 * S), show(Reason::Desync));
    // Only the trigger says desync (the end's caller), kind 2, no flag, no row.
    let mut trigger = live();
    end(&mut trigger, START + 5 * S, 2, C_DESYNC);
    assert_eq!(sync_end_trigger(C_DESYNC), "desync");
    assert_eq!(trigger.verdict(START + 6 * S), show(Reason::Desync));
    // Even a kind-1 end whose caller is the mismatch path is not normal.
    let mut odd = live();
    end(&mut odd, START + 5 * S, 1, C_DESYNC);
    assert_eq!(odd.verdict(START + 6 * S), show(Reason::Desync));
}

#[test]
fn the_eden_forced_desync_is_replayed_from_its_logged_lines() {
    // docs/EDEN-PROOF.md, console b: the two lines the plugin logged.
    let row = ev_desync(START + 18 * S);
    assert_eq!(
        line(&row),
        format!(
            "{} desync frame=1128 ours=0x720d5ecb theirs=0x4aec250b peer=1",
            row.tick
        )
    );
    let ended = ev_sync_end(START + 18 * S + S / 15, 2, true, C_DESYNC);
    assert!(line(&ended)
        .ends_with("sync_end trigger=desync kind=2 frame=1132 local=0 desync_seen=1 already_ending=0 caller=0x16d6b08"));
    let mut latch = live();
    assert_eq!(latch.observe(row.tick, &row), Observed::Recorded);
    assert_eq!(latch.observe(ended.tick, &ended), Observed::Recorded);
    assert_eq!(latch.verdict(ended.tick + S), show(Reason::Desync));
}

// ---- local link ----

#[test]
fn a_fresh_failed_link_is_a_local_fault_and_its_text_names_no_hardware() {
    for (observation, reason) in [
        (NOT_CONNECTED, Reason::LocalLink),
        // The call worked but reports "connecting" states too.
        ((0, 0), Reason::LocalLink),
        ((0, 3), Reason::LocalLink),
        // A failed status call is only a status error, whatever the state says.
        (STATUS_ERROR, Reason::LocalStatus),
        ((0xca6e, 4), Reason::LocalStatus),
        ((1, 2), Reason::LocalStatus),
    ] {
        let mut latch = live();
        link(&mut latch, START + 20 * S, observation);
        end(&mut latch, START + 25 * S, 2, C_SILENT);
        assert_eq!(
            latch.verdict(START + 26 * S),
            show(reason),
            "{observation:?}"
        );
        // The raw result and state stay available for the log only.
        assert_eq!(latch.failed_link(), Some(observation));
    }
}

#[test]
fn a_connected_link_is_never_a_fault_even_across_a_handover() {
    let mut latch = live();
    // Wi-Fi (interface 1) then Ethernet (interface 2), both connected.
    let wifi = ev_link(START + 10 * S, 0, 1, 2, 4);
    let ethernet = ev_link(START + 20 * S, 0, 2, 3, 4);
    assert_eq!(latch.observe(wifi.tick, &wifi), Observed::Recorded);
    assert_eq!(latch.observe(ethernet.tick, &ethernet), Observed::Recorded);
    end(&mut latch, START + 25 * S, 2, C_SILENT);
    assert_eq!(latch.failed_link(), None);
    assert_eq!(latch.verdict(START + 26 * S), show(Reason::Timeout));
}

#[test]
fn a_recovered_link_supersedes_an_earlier_failure() {
    // Undock, then Ethernet comes up: the latest observation is connected.
    let mut latch = live();
    link(&mut latch, START + 10 * S, STATUS_ERROR);
    link(&mut latch, START + 12 * S, CONNECTED);
    end(&mut latch, START + 20 * S, 2, C_SILENT);
    assert_eq!(latch.verdict(START + 21 * S), show(Reason::Timeout));
    // And the other way round: connected, then failed, is a fault.
    let mut down = live();
    link(&mut down, START + 10 * S, CONNECTED);
    link(&mut down, START + 12 * S, NOT_CONNECTED);
    end(&mut down, START + 20 * S, 2, C_SILENT);
    assert_eq!(down.verdict(START + 21 * S), show(Reason::LocalLink));
}

#[test]
fn a_link_fault_is_fresh_for_exactly_the_window_before_the_end() {
    let ended = START + 100 * S;
    for (before, expected) in [
        (0, Reason::LocalLink),
        (LINK_WINDOW_TICKS - 1, Reason::LocalLink),
        (LINK_WINDOW_TICKS, Reason::LocalLink),
        (LINK_WINDOW_TICKS + 1, Reason::Timeout),
    ] {
        let mut latch = live();
        link(&mut latch, ended - before, NOT_CONNECTED);
        end(&mut latch, ended, 2, C_TIMEOUT);
        assert_eq!(
            latch.verdict(ended + S),
            show(expected),
            "{before} before the end"
        );
    }
    // Without an end the window runs to the dialog's own tick.
    let at = START + 10 * S;
    for (now, expected) in [
        (at, Reason::LocalLink),
        (at + LINK_WINDOW_TICKS, Reason::LocalLink),
        (at + LINK_WINDOW_TICKS + 1, Reason::Unknown),
    ] {
        let mut latch = live();
        link(&mut latch, at, NOT_CONNECTED);
        assert_eq!(
            latch.verdict(now),
            show(expected),
            "now = at + {}",
            now - at
        );
    }
}

#[test]
fn a_link_failure_after_the_end_never_becomes_its_cause() {
    // The game errors out on a timeout, then the player undocks.
    let ended = START + 60 * S;
    for later in [1, SKEW_TICKS, END_GRACE_TICKS, END_GRACE_TICKS + 1, 5 * S] {
        let mut latch = live();
        end(&mut latch, ended, 2, C_SILENT);
        assert_eq!(
            link(&mut latch, ended + later, STATUS_ERROR),
            Observed::Rejected(Rejection::AfterEnd),
            "{later} ticks after the end"
        );
        assert_eq!(latch.failed_link(), None);
        assert_eq!(latch.verdict(ended + 2 * S), show(Reason::Timeout));
        assert_eq!(latch.verdict(ended + 20 * S), show(Reason::Timeout));
    }
    // A fault stamped exactly at the end is part of it.
    let mut latch = live();
    end(&mut latch, ended, 2, C_SILENT);
    assert_eq!(link(&mut latch, ended, NOT_CONNECTED), Observed::Recorded);
    assert_eq!(latch.verdict(ended + S), show(Reason::LocalLink));
}

#[test]
fn a_link_stamp_later_than_the_end_is_never_counted_even_if_it_arrived_first() {
    // Cross-thread skew: the link poll's stamp is recorded before the end
    // whose own stamp is slightly earlier. It still came after the end.
    let mut latch = live();
    link(&mut latch, START + 50 * S + SKEW_TICKS / 2, NOT_CONNECTED);
    end(&mut latch, START + 50 * S, 2, C_SILENT);
    assert_eq!(latch.verdict(START + 51 * S), show(Reason::Timeout));
    // A live session does not count a stamp ahead of the dialog's tick either.
    let mut live_latch = live();
    let at = START + 30 * S;
    assert_eq!(live_latch.link(at - 1, at, 0, 1), Observed::Recorded);
    assert_eq!(live_latch.verdict(at - 1), show(Reason::Unknown));
    assert_eq!(live_latch.verdict(at), show(Reason::LocalLink));
}

#[test]
fn the_boot_time_link_failure_of_the_real_logs_is_not_blamed() {
    // events.log of the Eden runs: a failed call at boot, connected a second later.
    let (boot_fail, boot_up) = (11_649_467_243_578u64, 11_649_486_086_459u64);
    let failed = ev_link(boot_fail, 0xca6e, 0, 0, 0);
    let connected = ev_link(boot_up, 0, 2, 3, 4);
    assert_eq!(
        line(&failed),
        format!("{boot_fail} link result=0xca6e interface=0 strength=0 state=0")
    );
    assert_eq!(
        line(&connected),
        format!("{boot_up} link result=0x0 interface=2 strength=3 state=4")
    );
    // No online session yet: neither is taken.
    let mut latch = Latch::new();
    assert_eq!(
        latch.observe(boot_fail, &failed),
        Observed::Rejected(Rejection::NotOnline)
    );
    assert_eq!(
        latch.observe(boot_up, &connected),
        Observed::Rejected(Rejection::NotOnline)
    );
    // A late feed of the failure into a session begun after it is the previous one's.
    let begun = boot_up + 100 * S;
    assert_eq!(latch.begin_online(begun, begun), Observed::Recorded);
    assert_eq!(
        latch.observe(begun, &failed),
        Observed::Rejected(Rejection::PreviousSession)
    );
    end(&mut latch, begun + 40 * S, 2, C_SILENT);
    assert_eq!(latch.verdict(begun + 41 * S), show(Reason::Timeout));
}

// ---- the other console ----

#[test]
fn an_abnormal_peer_end_is_reported_and_the_normal_reason_is_not() {
    for reason in [0, 2, 7, 0xffff] {
        let mut latch = live();
        assert_eq!(
            latch.peer_end(START + 5 * S, START + 5 * S, reason),
            Observed::Recorded
        );
        end(&mut latch, START + 5 * S + S / 15, 2, 7);
        assert_eq!(
            latch.verdict(START + 6 * S),
            show(Reason::PeerEnded),
            "reason {reason}"
        );
    }
    // Reason 1 is the normal end message: it must not count, for an abnormal
    // end that is otherwise unexplained either.
    let mut latch = live();
    latch.peer_end(START + 5 * S, START + 5 * S, 1);
    end(&mut latch, START + 5 * S + S / 15, 2, C_REFEREE);
    assert_eq!(latch.verdict(START + 6 * S), show(Reason::Unknown));
    // The text claims an ended session, not a departure.
    assert_eq!(
        Reason::PeerEnded.text(),
        "The other console ended the session."
    );
}

#[test]
fn the_peer_end_message_may_follow_the_local_end_by_exactly_the_grace() {
    let ended = START + 40 * S;
    let message = |after: u64| {
        let mut latch = live();
        end(&mut latch, ended, 2, C_REFEREE);
        let seen = latch.peer_end(ended + after, ended + after, 7);
        (seen, latch.verdict(ended + 2 * S))
    };
    assert_eq!(message(0), (Observed::Recorded, show(Reason::PeerEnded)));
    assert_eq!(
        message(S / 15),
        (Observed::Recorded, show(Reason::PeerEnded))
    );
    assert_eq!(
        message(END_GRACE_TICKS),
        (Observed::Recorded, show(Reason::PeerEnded))
    );
    assert_eq!(
        message(END_GRACE_TICKS + 1),
        (
            Observed::Rejected(Rejection::AfterEnd),
            show(Reason::Unknown)
        )
    );
}

#[test]
fn a_peer_message_before_or_after_the_end_gives_the_same_reason() {
    let mut before = live();
    before.peer_end(START + 10 * S, START + 10 * S, 7);
    end(&mut before, START + 11 * S, 2, C_SILENT);
    let mut after = live();
    end(&mut after, START + 11 * S, 2, C_SILENT);
    after.peer_end(START + 11 * S + S / 15, START + 11 * S + S / 15, 7);
    assert_eq!(before.verdict(START + 12 * S), show(Reason::PeerEnded));
    assert_eq!(after.verdict(START + 12 * S), show(Reason::PeerEnded));
}

// ---- timeouts and unknown ----

#[test]
fn only_the_two_silence_ends_are_a_timeout_and_the_rest_stay_unknown() {
    // Every caller the model names, with what the model calls it.
    let callers: [(u64, &str); 12] = [
        (0x16d6b08, "desync"),
        (0x16cfc18, "peer_silent"),
        (0x16cfb8c, "session_timeout"),
        (0x16cf4f4, "station_lost_before_start"),
        (0x16d7908, "participant_missing_at_start"),
        (0x16d7edc, "participant_missing_at_start"),
        (0x16d9190, "participant_missing_at_start"),
        (0x16d5ec0, "referee"),
        (0x1563258, "end_judge"),
        (0x1569718, "normal"),
        (0x1568824, "normal"),
        (7, "other"),
    ];
    for (caller, name) in callers {
        assert_eq!(sync_end_trigger(caller), name);
        let expected = match name {
            "desync" => Reason::Desync,
            "peer_silent" | "session_timeout" => Reason::Timeout,
            "station_lost_before_start"
            | "participant_missing_at_start"
            | "referee"
            | "end_judge"
            | "normal"
            | "other" => Reason::Unknown,
            other => panic!("trigger {other} has no expected reason"),
        };
        let mut latch = live();
        end(&mut latch, START + 20 * S, 2, caller);
        assert_eq!(latch.verdict(START + 21 * S), show(expected), "{name}");
    }
}

#[test]
fn silence_is_a_timeout_with_no_claim_about_the_opponent() {
    let mut latch = live();
    end(&mut latch, START + 20 * S, 2, C_SILENT);
    let verdict = latch.verdict(START + 21 * S);
    assert_eq!(verdict, show(Reason::Timeout));
    assert_ne!(verdict.text(), Some(Reason::PeerEnded.text()));
}

#[test]
fn missing_or_ambiguous_evidence_is_unknown_and_invents_no_cause() {
    // An online session with nothing seen at all.
    assert_eq!(live().verdict(START + 300 * S), show(Reason::Unknown));
    // An unexplained abort with a connected link and a normal peer message.
    let mut latch = live();
    link(&mut latch, START + 10 * S, CONNECTED);
    latch.peer_end(START + 20 * S, START + 20 * S, 1);
    end(&mut latch, START + 20 * S, 2, C_REFEREE);
    assert_eq!(latch.verdict(START + 21 * S), show(Reason::Unknown));
    // An abort whose only link fault is too old to be fresh.
    let mut old = live();
    link(&mut old, START + 5 * S, STATUS_ERROR);
    end(
        &mut old,
        START + 5 * S + LINK_WINDOW_TICKS + 1,
        2,
        C_REFEREE,
    );
    assert_eq!(
        old.verdict(START + 6 * S + LINK_WINDOW_TICKS),
        show(Reason::Unknown)
    );
}

// ---- normal ends ----

#[test]
fn a_normal_end_suppresses_the_reason_and_discards_link_evidence() {
    for caller in [C_NORMAL, C_NORMAL_2] {
        let mut latch = live();
        link(&mut latch, START + 50 * S, STATUS_ERROR);
        assert_eq!(
            end(&mut latch, START + 60 * S, 1, caller),
            Observed::Recorded
        );
        // The peer's own end message follows with the normal reason.
        latch.peer_end(START + 60 * S + S / 15, START + 60 * S + S / 15, 1);
        assert_eq!(
            latch.verdict(START + 61 * S),
            Verdict::Withheld(Withheld::NormalEnd)
        );
        assert_eq!(latch.verdict(START + 61 * S).text(), None);
        assert_eq!(latch.failed_link(), None);
        // A link observation stamped at the end, fed late, keeps nothing.
        link(&mut latch, START + 60 * S, STATUS_ERROR);
        assert_eq!(latch.failed_link(), None);
    }
    // Whatever the caller says, a kind-1 end with no flag is normal.
    let mut silent_caller = live();
    end(&mut silent_caller, START + 60 * S, 1, C_SILENT);
    assert_eq!(
        silent_caller.verdict(START + 61 * S),
        Verdict::Withheld(Withheld::NormalEnd)
    );
}

#[test]
fn a_normal_end_is_still_overridden_by_abnormal_evidence() {
    // An abnormal peer message with the normal end: the model's EndState keeps
    // such a session abnormal, so a real disconnection is reported. The link
    // failure before the normal end is discarded, so it is not the reason.
    for peer_first in [false, true] {
        let mut latch = live();
        link(&mut latch, START + 50 * S, NOT_CONNECTED);
        if peer_first {
            latch.peer_end(START + 59 * S, START + 59 * S, 7);
        }
        end(&mut latch, START + 60 * S, 1, C_NORMAL);
        if !peer_first {
            latch.peer_end(START + 60 * S + S / 15, START + 60 * S + S / 15, 7);
        }
        assert_eq!(
            latch.verdict(START + 61 * S),
            show(Reason::PeerEnded),
            "{peer_first}"
        );
    }
}

#[test]
fn repeated_endings_and_repeated_questions_give_the_same_answer() {
    let mut latch = live();
    end(&mut latch, START + 60 * S, 1, C_NORMAL);
    // Another normal end on the same stamp, and a later one past the end.
    assert_eq!(
        end(&mut latch, START + 60 * S, 1, C_NORMAL_2),
        Observed::Recorded
    );
    assert_eq!(
        end(&mut latch, START + 62 * S, 2, C_SILENT),
        Observed::Rejected(Rejection::AfterEnd)
    );
    for now in [
        START + 61 * S,
        START + 70 * S,
        START + 60 * S + VERDICT_WINDOW_TICKS,
    ] {
        assert_eq!(latch.verdict(now), Verdict::Withheld(Withheld::NormalEnd));
    }
    // An abnormal end repeated on the same stamp does not change its cause.
    let mut abnormal = live();
    end(&mut abnormal, START + 60 * S, 2, C_SILENT);
    assert_eq!(
        end(&mut abnormal, START + 60 * S, 2, C_REFEREE),
        Observed::Recorded
    );
    let first = abnormal.verdict(START + 61 * S);
    assert_eq!(first, show(Reason::Timeout));
    assert_eq!(abnormal.verdict(START + 61 * S), first);
    assert_eq!(abnormal.verdict(START + 61 * S), first);
}

#[test]
fn a_real_normal_match_is_left_alone() {
    // events.log of the Eden runs: `sync_end kind=1 frame=988`, then
    // `peer_end sender=0 reason=1 frame=992`, then the recording is not kept.
    let local = ev_sync_end(START + 90 * S, 1, false, C_NORMAL);
    let peer = ev_peer_end(START + 90 * S + 4 * S / 60, 1);
    assert!(
        line(&local).contains("sync_end trigger=normal kind=1 frame=1132 local=0 desync_seen=0")
    );
    assert_eq!(
        line(&peer),
        format!("{} peer_end sender=0 reason=1 frame=992", peer.tick)
    );
    let mut latch = live();
    assert_eq!(latch.observe(local.tick, &local), Observed::Recorded);
    assert_eq!(latch.observe(peer.tick, &peer), Observed::Recorded);
    let (a, b) = pack_recording(2, 0, 0, 0, false, false, RECORDING_NORMAL_END);
    let kept = RawEvent {
        tick: peer.tick + S,
        code: EV_RECORDING,
        a,
        b,
    };
    assert_eq!(latch.observe(kept.tick, &kept), Observed::Irrelevant);
    assert_eq!(
        latch.verdict(peer.tick + 2 * S),
        Verdict::Withheld(Withheld::NormalEnd)
    );
}

#[test]
fn a_rematch_after_a_normal_end_starts_clean_and_a_failed_rematch_is_not_blamed() {
    let mut latch = live();
    latch.desync(START + 10 * S, START + 10 * S);
    end(&mut latch, START + 11 * S, 2, C_DESYNC);
    assert_eq!(latch.verdict(START + 12 * S), show(Reason::Desync));
    // The next match begins: nothing leaks from the previous one.
    let next = START + 200 * S;
    assert_eq!(latch.begin_online(next, next), Observed::Recorded);
    assert_eq!(latch.verdict(next + S), show(Reason::Unknown));
    end(&mut latch, next + 60 * S, 1, C_NORMAL);
    // The rematch negotiation then fails: the game shows an error, the latch
    // knows only a normal end, so the dialog is left alone.
    assert_eq!(
        latch.verdict(next + 70 * S),
        Verdict::Withheld(Withheld::NormalEnd)
    );
}

// ---- sessions and scope ----

#[test]
fn nothing_is_online_until_a_session_begins() {
    let mut latch = Latch::new();
    assert_eq!(latch.verdict(START), Verdict::Withheld(Withheld::NoSession));
    assert_eq!(latch.generation(), 0);
    for observed in [
        latch.desync(START, START),
        latch.sync_end(START, START, 2, true, C_DESYNC),
        latch.peer_end(START, START, 7),
        latch.link(START, START, 0, 1),
        latch.observe(START, &ev_desync(START)),
    ] {
        assert_eq!(observed, Observed::Rejected(Rejection::NotOnline));
    }
    assert_eq!(latch.verdict(START), Verdict::Withheld(Withheld::NoSession));
    assert_eq!(Latch::default(), Latch::new());
}

#[test]
fn playback_cannot_produce_an_online_reason() {
    let mut latch = live();
    latch.desync(START + 5 * S, START + 5 * S);
    link(&mut latch, START + 6 * S, STATUS_ERROR);
    assert_eq!(
        latch.begin_playback(START + 10 * S, START + 10 * S),
        Observed::Recorded
    );
    assert_eq!(
        latch.verdict(START + 11 * S),
        Verdict::Withheld(Withheld::Playback)
    );
    for observed in [
        latch.desync(START + 11 * S, START + 11 * S),
        end(&mut latch, START + 12 * S, 2, C_SILENT),
        latch.peer_end(START + 12 * S, START + 12 * S, 7),
        link(&mut latch, START + 12 * S, NOT_CONNECTED),
    ] {
        assert_eq!(observed, Observed::Rejected(Rejection::NotOnline));
    }
    assert_eq!(
        latch.verdict(START + 13 * S),
        Verdict::Withheld(Withheld::Playback)
    );
    // The next online session works and carries nothing from before.
    let next = START + 100 * S;
    assert_eq!(latch.begin_online(next, next), Observed::Recorded);
    assert_eq!(latch.verdict(next + S), show(Reason::Unknown));
}

#[test]
fn a_session_begin_or_a_reset_drops_every_earlier_observation() {
    let mut latch = live();
    let first = latch.generation();
    latch.desync(START + 5 * S, START + 5 * S);
    link(&mut latch, START + 6 * S, STATUS_ERROR);
    latch.peer_end(START + 7 * S, START + 7 * S, 7);
    end(&mut latch, START + 8 * S, 2, C_SILENT);
    assert_eq!(latch.verdict(START + 9 * S), show(Reason::Desync));
    // Observations never change the generation; a begin does.
    assert_eq!(latch.generation(), first);
    let next = START + 50 * S;
    latch.begin_online(next, next);
    assert_eq!(latch.generation(), first.wrapping_add(1));
    assert_eq!(latch.failed_link(), None);
    end(&mut latch, next + 5 * S, 2, C_SILENT);
    assert_eq!(latch.verdict(next + 6 * S), show(Reason::Timeout));
    latch.reset();
    assert_eq!(latch.generation(), first.wrapping_add(2));
    assert_eq!(
        latch.verdict(next + 7 * S),
        Verdict::Withheld(Withheld::NoSession)
    );
    assert_eq!(
        latch.desync(next + 7 * S, next + 7 * S),
        Observed::Rejected(Rejection::NotOnline)
    );
}

#[test]
fn the_previous_sessions_late_observations_are_refused() {
    let mut latch = live();
    // Exactly the skew before the begin still belongs to it, one tick more does not.
    assert_eq!(
        link(&mut latch, START - SKEW_TICKS, CONNECTED),
        Observed::Recorded
    );
    let mut other = live();
    assert_eq!(
        link(&mut other, START - SKEW_TICKS - 1, CONNECTED),
        Observed::Rejected(Rejection::PreviousSession)
    );
    // The previous match's end, arriving after the next one began.
    let mut next = live();
    let old_end = ev_sync_end(START - 30 * S, 2, true, C_DESYNC);
    assert_eq!(
        next.observe(START, &old_end),
        Observed::Rejected(Rejection::PreviousSession)
    );
    assert_eq!(next.verdict(START + S), show(Reason::Unknown));
}

#[test]
fn a_session_start_stamped_in_the_future_leaves_no_session_in_scope() {
    let now = START;
    let mut latch = live();
    latch.desync(now, now);
    let before = latch.generation();
    // One tick beyond the skew: refused, and the old evidence is not kept.
    assert_eq!(
        latch.begin_online(now, now + SKEW_TICKS + 1),
        Observed::Rejected(Rejection::Future)
    );
    assert_eq!(latch.generation(), before.wrapping_add(1));
    assert_eq!(latch.verdict(now), Verdict::Withheld(Withheld::NoSession));
    assert_eq!(
        latch.desync(now, now),
        Observed::Rejected(Rejection::NotOnline)
    );
    // The same for a playback begin.
    let mut playback = live();
    assert_eq!(
        playback.begin_playback(now, now + SKEW_TICKS + 1),
        Observed::Rejected(Rejection::Future)
    );
    assert_eq!(
        playback.verdict(now),
        Verdict::Withheld(Withheld::NoSession)
    );
    // Exactly the skew ahead is accepted.
    assert_eq!(
        latch.begin_online(now, now + SKEW_TICKS),
        Observed::Recorded
    );
    assert_eq!(latch.verdict(now), show(Reason::Unknown));
}

// ---- clock and boundaries ----

#[test]
fn observations_stamped_in_the_future_are_refused_beyond_the_skew() {
    let now = START + 5 * S;
    let mut latch = live();
    assert_eq!(latch.link(now, now + SKEW_TICKS, 0, 4), Observed::Recorded);
    assert_eq!(
        latch.link(now, now + SKEW_TICKS + 1, 0, 4),
        Observed::Rejected(Rejection::Future)
    );
    assert_eq!(
        latch.desync(now, now + 3600 * S),
        Observed::Rejected(Rejection::Future)
    );
    assert_eq!(latch.verdict(now + S), show(Reason::Unknown));
}

#[test]
fn a_stamp_behind_a_newer_observation_is_a_clock_regression_beyond_the_skew() {
    let newest = START + 10 * S;
    let mut latch = live();
    assert_eq!(link(&mut latch, newest, CONNECTED), Observed::Recorded);
    // Within the skew, and an older link stamp does not replace the newer one.
    assert_eq!(
        link(&mut latch, newest - SKEW_TICKS, STATUS_ERROR),
        Observed::Recorded
    );
    assert_eq!(latch.failed_link(), None);
    assert_eq!(
        link(&mut latch, newest - SKEW_TICKS - 1, STATUS_ERROR),
        Observed::Rejected(Rejection::ClockRegression)
    );
    assert_eq!(latch.failed_link(), None);
}

#[test]
fn a_dialog_tick_behind_the_recorded_evidence_claims_nothing_precise() {
    let at = START + 10 * S;
    let mut latch = live();
    latch.desync(at, at);
    assert_eq!(latch.verdict(at), show(Reason::Desync));
    assert_eq!(latch.verdict(at - SKEW_TICKS), show(Reason::Desync));
    assert_eq!(latch.verdict(at - SKEW_TICKS - 1), show(Reason::Unknown));
}

#[test]
fn a_verdict_is_about_its_session_only_for_the_verdict_window() {
    let ended = START + 40 * S;
    let mut latch = live();
    end(&mut latch, ended, 2, C_TIMEOUT);
    assert_eq!(
        latch.verdict(ended + VERDICT_WINDOW_TICKS),
        show(Reason::Timeout)
    );
    assert_eq!(
        latch.verdict(ended + VERDICT_WINDOW_TICKS + 1),
        Verdict::Withheld(Withheld::Stale)
    );
    // A session that never reported an end has no such limit.
    let mut open = live();
    open.desync(START + S, START + S);
    assert_eq!(open.verdict(START + 3600 * S), show(Reason::Desync));
}

#[test]
fn extreme_stamps_do_not_overflow() {
    let mut latch = Latch::new();
    assert_eq!(latch.begin_online(u64::MAX, u64::MAX), Observed::Recorded);
    assert_eq!(latch.link(u64::MAX, u64::MAX, 0, 1), Observed::Recorded);
    assert_eq!(
        latch.sync_end(u64::MAX, u64::MAX, 2, false, C_SILENT),
        Observed::Recorded
    );
    assert_eq!(latch.peer_end(u64::MAX, u64::MAX, 7), Observed::Recorded);
    assert_eq!(latch.verdict(u64::MAX), show(Reason::LocalLink));
    // A begin at zero with a far-future observation, and a verdict at zero.
    let mut zero = Latch::new();
    assert_eq!(zero.begin_online(0, 0), Observed::Recorded);
    assert_eq!(
        zero.link(0, u64::MAX, 0, 1),
        Observed::Rejected(Rejection::Future)
    );
    assert_eq!(zero.verdict(0), show(Reason::Unknown));
}

// ---- how the runtime feeds it ----

#[test]
fn queued_events_are_read_exactly_as_describe_prints_them() {
    let mut latch = live();
    // The four kinds are read; the session, recording and playback events are not.
    let session = RawEvent {
        tick: START,
        code: EV_SESSION,
        a: 3,
        b: u64::from(KIND_ONLINE),
    };
    let playback = RawEvent {
        tick: START,
        code: EV_PLAYBACK_MISMATCH,
        a: 1,
        b: 2,
    };
    assert_eq!(latch.observe(START, &session), Observed::Irrelevant);
    assert_eq!(latch.observe(START, &playback), Observed::Irrelevant);
    assert_eq!(latch.generation(), 1);
    // Link: the fields of `link result=0xca6e interface=0 strength=0 state=0`.
    let failed = ev_link(START + 5 * S, 0xca6e, 0, 0, 0);
    assert_eq!(
        line(&failed),
        format!(
            "{} link result=0xca6e interface=0 strength=0 state=0",
            failed.tick
        )
    );
    latch.observe(failed.tick, &failed);
    assert_eq!(latch.failed_link(), Some((0xca6e, 0)));
    let up = ev_link(START + 6 * S, 0, 2, 3, 4);
    assert_eq!(
        line(&up),
        format!("{} link result=0x0 interface=2 strength=3 state=4", up.tick)
    );
    latch.observe(up.tick, &up);
    assert_eq!(latch.failed_link(), None);
    // Peer end: `peer_end sender=0 reason=7 frame=992`.
    let abnormal = ev_peer_end(START + 7 * S, 7);
    assert_eq!(
        line(&abnormal),
        format!("{} peer_end sender=0 reason=7 frame=992", abnormal.tick)
    );
    latch.observe(abnormal.tick, &abnormal);
    // Sync end: kind and caller as printed.
    let silent = ev_sync_end(START + 8 * S, 2, false, C_SILENT);
    assert!(line(&silent)
        .contains("sync_end trigger=peer_silent kind=2 frame=1132 local=0 desync_seen=0"));
    latch.observe(silent.tick, &silent);
    assert_eq!(latch.verdict(START + 9 * S), show(Reason::PeerEnded));
    // A mismatch row.
    let mut rows = live();
    rows.observe(START + 5 * S, &ev_desync(START + 5 * S));
    assert_eq!(rows.verdict(START + 6 * S), show(Reason::Desync));
}

#[test]
fn a_copy_taken_for_the_ui_is_a_stable_snapshot() {
    // The runtime copies the latch out under a try-lock and asks the copy.
    let mut shared = live();
    shared.desync(START + 5 * S, START + 5 * S);
    let snapshot = shared;
    shared.reset();
    assert_eq!(snapshot.verdict(START + 6 * S), show(Reason::Desync));
    assert_eq!(
        shared.verdict(START + 6 * S),
        Verdict::Withheld(Withheld::NoSession)
    );
    // Small enough to sit in a static behind a try-lock.
    const _: () = assert!(size_of::<Latch>() <= 128);
    const STATIC_START: Latch = Latch::new();
    assert_eq!(STATIC_START, Latch::default());
}

#[test]
fn feeding_and_asking_never_allocate() {
    let events = [
        ev_desync(START + 5 * S),
        ev_link(START + 6 * S, 0xca6e, 0, 0, 0),
        ev_peer_end(START + 7 * S, 7),
        ev_sync_end(START + 8 * S, 2, true, C_DESYNC),
    ];
    let before = ALLOCATIONS.with(Cell::get);
    let mut latch = Latch::new();
    let mut shown = 0usize;
    for round in 0..4u64 {
        let base = round * 1000 * S;
        latch.begin_online(base + START, base + START);
        for event in &events {
            let moved = RawEvent {
                tick: event.tick + base,
                ..*event
            };
            latch.observe(moved.tick, &moved);
            if let Some(text) = latch.verdict(moved.tick).text() {
                shown += text.len();
            }
        }
        latch.link(base + START + 9 * S, base + START + 9 * S, 0, 4);
        latch.begin_playback(base + START + 20 * S, base + START + 20 * S);
        latch.reset();
    }
    for reason in Reason::ALL {
        shown += reason.text().len() + reason.name().len();
        let mut text = [0u16; dialog::CAPACITY];
        let mut len = 0;
        for unit in "A communication error has occurred.".encode_utf16() {
            text[len] = unit;
            len += 1;
        }
        shown += dialog::append_reason(&mut text, len, reason).unwrap();
    }
    let after = ALLOCATIONS.with(Cell::get);
    assert!(shown > 0);
    assert_eq!(after - before, 0);
}
