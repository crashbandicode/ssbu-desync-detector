#![allow(dead_code)]
#[path = "../src/dialog.rs"]
mod dialog;
#[path = "../src/disconnect.rs"]
mod disconnect;
#[path = "../src/model.rs"]
mod model;

use dialog::{append_reason, is_communication_error, CAPACITY};
use disconnect::Reason;

#[test]
fn formatter_frame_identifies_the_caller_buffer_without_x17() {
    let caller_sp = 0x20000;
    let formatter_sp = caller_sp - 528;
    assert_eq!(
        dialog::frame_addresses(formatter_sp, caller_sp - 16),
        Some((caller_sp, caller_sp - 8))
    );
    assert_eq!(dialog::frame_addresses(formatter_sp, caller_sp), None);
    assert_eq!(dialog::frame_addresses(0x1000, 0x1200), None);
    assert_eq!(
        dialog::frame_addresses(formatter_sp + 1, caller_sp - 15),
        None
    );
    assert_eq!(dialog::frame_addresses(u64::MAX - 15, 496), None);
    // Both near (one instruction) and aligned far (four instructions) spans
    // finish before the next epilogue load, with no relocated branch/call.
    assert_eq!(dialog::FORMAT_RETURN_OFFSET % 8, 0);
    assert_eq!(&dialog::FORMAT_RETURN.guard[..4], &[0xe0, 0x03, 0x14, 0x2a]);
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
                "123 communication_text reason={} text_units=120",
                reason.name()
            )
        );
    }
}
