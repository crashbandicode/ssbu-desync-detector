#![allow(dead_code)]
#[path = "../src/dialog.rs"]
mod dialog;
#[path = "../src/disconnect.rs"]
mod disconnect;
#[path = "../src/model.rs"]
mod model;

use dialog::{append_in_frame, append_reason, is_communication_error, Frame, Route, CAPACITY};
use disconnect::Reason;
use std::cell::Cell;

#[test]
fn formatter_frame_identifies_the_caller_buffer_without_x17() {
    let caller_sp = 0x20000;
    let formatter_sp = caller_sp - 528;
    let args = &dialog::ARGS;
    assert_eq!(
        args.frame(formatter_sp, caller_sp - 16),
        Some(Frame {
            buffer: caller_sp,
            saved_lr: Some(caller_sp - 8)
        })
    );
    assert_eq!(args.frame(formatter_sp, caller_sp), None);
    assert_eq!(args.frame(0x1000, 0x1200), None);
    assert_eq!(args.frame(formatter_sp + 1, caller_sp - 15), None);
    assert_eq!(args.frame(u64::MAX - 15, 496), None);
    assert_eq!(args.caller, Some((520, 0x37a272c)));
}

#[test]
fn plain_route_frame_finds_the_caller_buffer() {
    // `.377a120`: str x28,[sp,#-0x40]!; ...; add x29,sp,#0x30; then
    // sub sp,#0x1000 and sub sp,#0x120. Its buffer is memset at sp+0x120.
    let saves = 0x40000;
    let sp = saves - 0x1120;
    let fp = saves + 0x30;
    let plain = &dialog::PLAIN;
    assert_eq!(
        plain.frame(sp, fp),
        Some(Frame {
            buffer: sp + 0x120,
            saved_lr: None
        })
    );
    // The buffer's last unit ends exactly where the register saves begin.
    assert_eq!(sp + 0x120 + 2 * CAPACITY as u64, saves);
    assert_eq!(plain.frame(sp, fp - 16), None);
    assert_eq!(plain.frame(sp + 8, fp + 8), None);
    assert_eq!(plain.frame(0x8000, 0x8000 + 0x1150), None);
    assert_eq!(plain.frame(u64::MAX - 0x10f, u64::MAX), None);
    // The guarded bytes say the same: the frame and the buffer offset.
    let word = |site: &model::Site, at: usize| {
        u32::from_le_bytes(site.guard[at * 4..at * 4 + 4].try_into().unwrap())
    };
    assert_eq!(word(&dialog::PLAIN_STACK, 0), 0xf81c0ffc); // str x28,[sp,#-0x40]!
    assert_eq!(word(&dialog::PLAIN_STACK, 4), 0x9100c3fd); // add x29,sp,#0x30
    assert_eq!(word(&dialog::PLAIN_STACK, 5), 0xd14007ff); // sub sp,sp,#0x1000
    assert_eq!(word(&dialog::PLAIN_STACK, 6), 0xd10483ff); // sub sp,sp,#0x120
    assert_eq!(word(&dialog::PLAIN_FORMAT, 7), 0x910483e0); // add x0,sp,#0x120 (memset)
    assert_eq!(word(&dialog::PLAIN_FORMAT, 8), 0x321403e2); // mov w2,#0x1000
    assert_eq!(word(&dialog::PLAIN_FORMAT, 12), 0x910483e0); // add x0,sp,#0x120 (format)
    assert_eq!(word(&dialog::PLAIN_READY, 1), 0x910483e1); // add x1,sp,#0x120 (setter)
}

/// Is `word` a branch, call or return of any kind (B/BL, B.cond, CBZ/CBNZ,
/// TBZ/TBNZ, BR/BLR/RET)?
fn is_control(word: u32) -> bool {
    word & 0x7c00_0000 == 0x1400_0000
        || word & 0xff00_0010 == 0x5400_0000
        || word & 0x7e00_0000 == 0x3400_0000
        || word & 0x7e00_0000 == 0x3600_0000
        || word & 0xfe00_0000 == 0xd600_0000
}

#[test]
fn routes_hook_aligned_moves_and_relocate_no_call() {
    for (index, route) in dialog::ROUTES.iter().enumerate() {
        assert_eq!(route.code as usize, index);
        assert_eq!(model::COMMUNICATION_ROUTES[index], route.name);
        // An 8-byte aligned site: Skyline's far patch relocates four
        // instructions, its near patch one.
        assert_eq!(route.hook % 8, 0, "{}", route.name);
        let site = route
            .guards
            .iter()
            .find(|site| site.offset == route.hook)
            .expect("the hooked site is guarded");
        let words: Vec<u32> = site.guard[..16]
            .chunks(4)
            .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
            .collect();
        // The first is `mov w0|w2, w<count>` (ORR Wd, WZR, Wm), the count register.
        assert_eq!(words[0] & 0xffe0_ffe0, 0x2a00_03e0, "{}", route.name);
        assert_eq!(
            (words[0] >> 16 & 31) as usize,
            route.count,
            "{}",
            route.name
        );
        assert!(words.iter().all(|w| !is_control(*w)), "{}", route.name);
    }
    // The plain hook leaves the setter call, its fifth word, in place.
    let fifth = u32::from_le_bytes(dialog::PLAIN_READY.guard[16..20].try_into().unwrap());
    assert!(is_control(fifth));
}

/// A host stand-in for a route's native frame: real memory laid out as the
/// guards fix it, so `append_in_frame` runs exactly as in the hook.
struct NativeFrame {
    memory: Vec<u64>,
    sp: u64,
}

const TEXT_BASE: u64 = 0x80_0000_0000;

impl NativeFrame {
    fn new(route: &Route) -> Self {
        let mut memory = vec![0u64; 0x1400 / 8 + 2];
        let start = memory.as_mut_ptr() as u64;
        let sp = (start + 15) & !15;
        let mut frame = Self { memory, sp };
        if let Some((slot, offset)) = route.caller {
            frame.write_u64(slot, TEXT_BASE + offset);
        }
        frame
    }

    fn write_u64(&mut self, at: u64, value: u64) {
        let index = ((self.sp + at) - self.memory.as_ptr() as u64) / 8;
        self.memory[index as usize] = value;
    }

    fn buffer(&mut self, route: &Route) -> &mut [u16] {
        let address = self.sp + route.buffer;
        // SAFETY: inside `memory` (0x1400 bytes past sp cover every route).
        unsafe { core::slice::from_raw_parts_mut(address as *mut u16, CAPACITY) }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Outcome {
    /// Not a text this hook may touch: no reason asked, nothing written.
    Untouched,
    /// A communication-error text: the reason was asked, the append refused.
    Asked,
    /// The reason was appended.
    Appended,
}

#[derive(Clone, Copy)]
enum Tamper {
    None,
    Fp,
    Sp,
    Caller,
    Terminator,
}

/// What one route did with one case: the result, the text afterwards and
/// whether the reason was asked for.
fn run_case(
    route: &Route,
    text: &str,
    len_override: Option<usize>,
    tamper: Tamper,
) -> (Option<(Reason, usize)>, Vec<u16>, bool) {
    let mut frame = NativeFrame::new(route);
    let units: Vec<u16> = text.encode_utf16().collect();
    let buffer = frame.buffer(route);
    buffer[..units.len()].copy_from_slice(&units);
    if let Tamper::Terminator = tamper {
        buffer[units.len()] = u16::from(b'!');
    }
    if let Tamper::Caller = tamper {
        if let Some((slot, offset)) = route.caller {
            frame.write_u64(slot, TEXT_BASE + offset + 4);
        }
    }
    let (sp, fp) = match tamper {
        Tamper::Fp => (frame.sp, frame.sp + route.fp + 16),
        Tamper::Sp => (frame.sp + 8, frame.sp + 8 + route.fp),
        _ => (frame.sp, frame.sp + route.fp),
    };
    let asked = Cell::new(false);
    // SAFETY: sp/fp point into `frame.memory`, laid out as the route's guards.
    let result = unsafe {
        append_in_frame(
            route,
            TEXT_BASE,
            sp,
            fp,
            len_override.unwrap_or(units.len()),
            || {
                asked.set(true);
                Reason::Desync
            },
        )
    };
    (result, frame.buffer(route).to_vec(), asked.get())
}

#[test]
fn both_routes_append_the_same_way_for_one_table_of_cases() {
    let long = format!("A connection error occurred.{}", " ".repeat(0x7d0));
    // (case, text, native count override, tamper, expected outcome). `Asked`:
    // a communication-error text whose append is refused after the reason was
    // taken, as the original hook did (the dialog's freeze point).
    let cases: [(&str, &str, Option<usize>, Tamper, Outcome); 10] = [
        (
            "pop-up body",
            "A connection error occurred.",
            None,
            Tamper::None,
            Outcome::Appended,
        ),
        (
            "with support code",
            "A communication error has occurred.\nError Code: 2618-0201",
            None,
            Tamper::None,
            Outcome::Appended,
        ),
        (
            "unrelated text",
            "Rematch?",
            None,
            Tamper::None,
            Outcome::Untouched,
        ),
        (
            "already augmented",
            "A connection error occurred.\n\nReason: The other console ended the session.",
            None,
            Tamper::None,
            Outcome::Asked,
        ),
        ("does not fit", &long, None, Tamper::None, Outcome::Asked),
        (
            "count at capacity",
            "A connection error occurred.",
            Some(CAPACITY),
            Tamper::None,
            Outcome::Untouched,
        ),
        (
            "no terminator",
            "A connection error occurred.",
            None,
            Tamper::Terminator,
            Outcome::Asked,
        ),
        (
            "foreign frame pointer",
            "A connection error occurred.",
            None,
            Tamper::Fp,
            Outcome::Untouched,
        ),
        (
            "unaligned stack",
            "A connection error occurred.",
            None,
            Tamper::Sp,
            Outcome::Untouched,
        ),
        (
            "other caller (args only)",
            "A connection error occurred.",
            None,
            Tamper::Caller,
            Outcome::Untouched,
        ),
    ];
    for (name, text, len, tamper, outcome) in cases {
        let original: Vec<u16> = text.encode_utf16().collect();
        let mut outcomes = Vec::new();
        for route in dialog::ROUTES {
            let (result, after, asked) = run_case(route, text, len, tamper);
            // PLAIN has no caller to check: its hook sits in the buffer's owner.
            let expect = if matches!(tamper, Tamper::Caller) && route.caller.is_none() {
                Outcome::Appended
            } else {
                outcome
            };
            assert_eq!(
                result.is_some(),
                expect == Outcome::Appended,
                "{name} on {}",
                route.name
            );
            assert_eq!(
                asked,
                expect != Outcome::Untouched,
                "{name} on {}: reason asked",
                route.name
            );
            assert_eq!(
                &after[..original.len()],
                &original[..],
                "{name} on {}",
                route.name
            );
            if let Some((reason, new_len)) = result {
                assert_eq!(reason, Reason::Desync);
                assert_eq!(
                    String::from_utf16(&after[..new_len]).unwrap(),
                    format!("{text}\n\nReason: {}", Reason::Desync.text())
                );
                assert_eq!(after[new_len], 0);
            } else if !matches!(tamper, Tamper::Terminator) {
                assert!(after[original.len()..].iter().all(|u| *u == 0), "{name}");
            }
            outcomes.push((result, after));
        }
        if !matches!(tamper, Tamper::Caller) {
            assert_eq!(outcomes[0], outcomes[1], "{name}: routes differ");
        }
    }
}

#[test]
fn communication_text_event_names_the_route() {
    for route in dialog::ROUTES {
        let mut line = model::Line::<160>::new();
        model::describe(
            &model::RawEvent {
                tick: 7,
                code: model::EV_COMMUNICATION_TEXT,
                a: Reason::PeerEnded.code() | route.code << 8,
                b: 99,
            },
            &mut line,
        )
        .unwrap();
        assert_eq!(
            core::str::from_utf8(line.as_bytes()).unwrap(),
            format!(
                "7 communication_text reason=peer_ended text_units=99 route={}",
                route.name
            )
        );
    }
}

fn native(text: &str) -> ([u16; CAPACITY], usize) {
    let mut output = [0; CAPACITY];
    let mut len = 0;
    for unit in text.encode_utf16() {
        output[len] = unit;
        len += 1;
    }
    (output, len)
}

#[test]
fn native_message_and_support_details_are_preserved_for_every_reason() {
    let original = "A communication error has occurred.\nPlease try again.\nError Code: 2618-0201";
    for reason in Reason::ALL {
        let (mut text, len) = native(original);
        let prefix = text[..len].to_vec();
        let new_len = append_reason(&mut text, len, reason).unwrap();
        assert_eq!(&text[..len], prefix);
        assert_eq!(text[new_len], 0);
        assert_eq!(
            String::from_utf16(&text[..new_len]).unwrap(),
            format!("{original}\n\nReason: {}", reason.text())
        );
    }
}

#[test]
fn connection_error_variant_and_native_line_breaks_are_recognized() {
    for message in [
        "A connection error has occurred.",
        "A COMMUNICATION\n  ERROR HAS\r\nOCCURRED.",
        "Communication error occurred.",
        "Connection error occurred.",
    ] {
        let (text, len) = native(message);
        assert!(is_communication_error(&text[..len]), "{message}");
    }
}

#[test]
fn unrelated_messages_and_unknown_translations_are_unchanged() {
    for message in [
        "Rematch?",
        "A system error has occurred.",
        "An error has occurred while checking the SD card.",
        "Check your connection.",
        "通信エラーが発生しました。",
    ] {
        let (mut text, len) = native(message);
        let before = text;
        assert_eq!(append_reason(&mut text, len, Reason::Desync), None);
        assert_eq!(text, before);
    }
}

#[test]
fn text_tags_with_embedded_zero_words_and_surrogates_survive() {
    let (source, len) = native("A communication error has occurred. 🎮");
    let tag = [0xe, 0, 3, 4, 0, 0x1234];
    let mut text = [0; CAPACITY];
    text[..2].copy_from_slice(&source[..2]);
    text[2..2 + tag.len()].copy_from_slice(&tag);
    text[2 + tag.len()..len + tag.len()].copy_from_slice(&source[2..len]);
    let len = len + tag.len();
    let before = text;
    assert!(is_communication_error(&text[..len]));
    let end = append_reason(&mut text, len, Reason::Unknown).unwrap();
    assert_eq!(&text[..len], &before[..len]);
    assert_eq!(text[end], 0);
}

#[test]
fn a_tag_between_words_does_not_hide_the_error_sentence() {
    let (source, len) = native("A communication error has occurred.");
    let split = "A communication ".len();
    let tag = [0xe, 0, 3, 0, 0xf, 0, 3];
    let mut text = [0; CAPACITY];
    text[..split].copy_from_slice(&source[..split]);
    text[split..split + tag.len()].copy_from_slice(&tag);
    text[split + tag.len()..len + tag.len()].copy_from_slice(&source[split..len]);
    assert!(is_communication_error(&text[..len + tag.len()]));
}

#[test]
fn exact_fit_uses_the_terminator_slot_and_overflow_changes_nothing() {
    let (source, len) = native("A communication error has occurred.");
    let extra = "\n\nReason: ".len() + Reason::Unknown.text().len();
    let mut exact = vec![0; len + extra + 1];
    exact[..len].copy_from_slice(&source[..len]);
    assert_eq!(
        append_reason(&mut exact, len, Reason::Unknown),
        Some(len + extra)
    );
    let mut short = vec![0; len + extra];
    short[..len].copy_from_slice(&source[..len]);
    let before = short.clone();
    assert_eq!(append_reason(&mut short, len, Reason::Unknown), None);
    assert_eq!(short, before);
}

#[test]
fn bad_native_counts_terminators_and_tag_lengths_do_not_write() {
    let (mut text, len) = native("A communication error has occurred.");
    let before = text;
    assert_eq!(append_reason(&mut text, CAPACITY, Reason::Unknown), None);
    assert_eq!(append_reason(&mut text, usize::MAX, Reason::Unknown), None);
    assert_eq!(text, before);
    text[len] = 1;
    let before = text;
    assert_eq!(append_reason(&mut text, len, Reason::Unknown), None);
    assert_eq!(text, before);
    for bad in [&[0xe, 0, 0, 65534][..], &[0xe, 0, 0, 3], &[0xf, 0]] {
        assert!(!is_communication_error(bad));
    }
}

#[test]
fn repeated_setting_of_an_augmented_message_adds_no_second_reason() {
    let (mut text, len) = native("A communication error has occurred.");
    let len = append_reason(&mut text, len, Reason::Desync).unwrap();
    let before = text;
    assert_eq!(append_reason(&mut text, len, Reason::LocalLink), None);
    assert_eq!(text, before);
}

#[test]
fn each_call_owns_its_buffer_and_log_reason_matches_the_ui_reason() {
    let (mut first, len) = native("A communication error has occurred.");
    let (mut second, _) = native("A communication error has occurred.");
    let first_len = append_reason(&mut first, len, Reason::Desync).unwrap();
    append_reason(&mut second, len, Reason::Unknown).unwrap();
    assert!(String::from_utf16(&first[..first_len])
        .unwrap()
        .ends_with(Reason::Desync.text()));
    for reason in Reason::ALL {
        let mut line = model::Line::<160>::new();
        model::describe(
            &model::RawEvent {
                tick: 123,
                code: model::EV_COMMUNICATION_TEXT,
                a: reason.code(),
                b: 120,
            },
            &mut line,
        )
        .unwrap();
        assert_eq!(
            core::str::from_utf8(line.as_bytes()).unwrap(),
            format!(
                "123 communication_text reason={} text_units=120 route=args",
                reason.name()
            )
        );
    }
}
