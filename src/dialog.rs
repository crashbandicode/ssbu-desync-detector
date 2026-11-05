//! Append evidence to the game's formatted communication-error text.
//!
//! The native caller owns a zeroed 0x1000-byte UTF-16 stack buffer. Just
//! before its formatter returns, w20 contains the UTF-16 count. The saved
//! return address identifies this caller and the guarded frame identifies
//! its output buffer, without relying on the trampoline-clobbered x17.
//! Editing that buffer avoids replacing an applet, guessing an error code,
//! retaining a pointer or sharing scratch storage between calls/threads.
use crate::disconnect::Reason;
use crate::model::Site;

pub const CAPACITY: usize = 0x800;
pub const FORMAT_RETURN_OFFSET: usize = 0x37ad8e8;
pub const CALLER_RETURN_OFFSET: u64 = 0x37a272c;
pub const FRAME_BYTES: u64 = 528;
pub const SAVED_LR_OFFSET: u64 = 520;
const PREFIX: &str = "\n\nReason: ";

// All bytes generated from the hash-verified 13.0.5 NSO. Only FORMAT_RETURN is
// hooked. The other guards cover the native stack, capacity and count ABI.
pub const TEXT_STACK: Site = Site {
    name: "text_stack",
    offset: 0x37a2660,
    guard: [
        0xfc, 0x0f, 0x1d, 0xf8, 0xf4, 0x4f, 0x01, 0xa9, 0xfd, 0x7b, 0x02, 0xa9, 0xfd, 0x83, 0x00,
        0x91, 0xff, 0x07, 0x40, 0xd1, 0xff, 0x03, 0x02, 0xd1, 0xbf, 0x7f, 0x3d, 0xa9, 0xf3, 0x03,
        0x00, 0xaa, 0xbf, 0x7f, 0x3c, 0xa9, 0xbf, 0x7f, 0x3b, 0xa9, 0xbf, 0x7f, 0x3a, 0xa9, 0xbf,
        0x7f, 0x39, 0xa9, 0xbf, 0x7f, 0x38, 0xa9, 0xbf, 0x7f, 0x37, 0xa9, 0xbf, 0x7f, 0x36, 0xa9,
        0x48, 0x24, 0x40, 0xa9,
    ],
};
pub const TEXT_FORMAT: Site = Site {
    name: "text_format",
    offset: 0x37a26f0,
    guard: [
        0xc8, 0x0c, 0x01, 0xb0, 0x08, 0x31, 0x46, 0xf9, 0x00, 0x01, 0x40, 0xf9, 0x55, 0x07, 0x00,
        0x94, 0xf4, 0x03, 0x00, 0xaa, 0xe0, 0x03, 0x00, 0x91, 0xe2, 0x03, 0x14, 0x32, 0xe1, 0x03,
        0x1f, 0x2a, 0xe4, 0x76, 0x08, 0x94, 0xe0, 0x03, 0x00, 0x91, 0xe1, 0x03, 0x15, 0x32, 0xa3,
        0x63, 0x00, 0xd1, 0xa4, 0x83, 0x02, 0xd1, 0xe2, 0x03, 0x14, 0xaa, 0x0a, 0x24, 0x00, 0x94,
        0xe2, 0x03, 0x00, 0x2a,
    ],
};
pub const TEXT_READY: Site = Site {
    name: "text_ready",
    offset: CALLER_RETURN_OFFSET as usize,
    guard: [
        0xe2, 0x03, 0x00, 0x2a, 0xe1, 0x03, 0x00, 0x91, 0xe0, 0x03, 0x13, 0xaa, 0xe3, 0x03, 0x1f,
        0x2a, 0xcd, 0xfe, 0xff, 0x97, 0xff, 0x07, 0x40, 0x91, 0xff, 0x03, 0x02, 0x91, 0xfd, 0x7b,
        0x42, 0xa9, 0xf4, 0x4f, 0x41, 0xa9, 0xfc, 0x07, 0x43, 0xf8, 0xc0, 0x03, 0x5f, 0xd6, 0xe0,
        0x03, 0x02, 0xaa, 0xf5, 0x7a, 0x08, 0x94, 0xfc, 0x0f, 0x1b, 0xf8, 0xf8, 0x5f, 0x01, 0xa9,
        0xf6, 0x57, 0x02, 0xa9,
    ],
};
pub const FORMAT_TERMINATOR: Site = Site {
    name: "format_terminator",
    offset: 0x37ad828,
    guard: [
        0x49, 0x07, 0x00, 0x11, 0x28, 0x04, 0x00, 0x51, 0x3f, 0x01, 0x01, 0x6b, 0x54, 0xb3, 0x88,
        0x1a, 0x3f, 0xda, 0x34, 0x78, 0x4a, 0x00, 0x00, 0x36, 0x3f, 0xda, 0x28, 0x78, 0x08, 0x03,
        0x1b, 0xeb, 0x40, 0x02, 0x00, 0x54, 0x4a, 0xfe, 0x46, 0xd3, 0x4a, 0xd9, 0x7d, 0x92, 0x69,
        0x03, 0x40, 0xf9, 0x6a, 0x6b, 0x6a, 0xf8, 0x4b, 0x22, 0x40, 0x92, 0x4a, 0x0d, 0x0b, 0x8b,
        0x5f, 0x01, 0x09, 0xeb,
    ],
};
pub const FORMAT_RETURN: Site = Site {
    name: "format_return",
    offset: FORMAT_RETURN_OFFSET,
    guard: [
        0xe0, 0x03, 0x14, 0x2a, 0xff, 0xc3, 0x06, 0x91, 0xfd, 0x7b, 0x45, 0xa9, 0xf4, 0x4f, 0x44,
        0xa9, 0xf6, 0x57, 0x43, 0xa9, 0xf8, 0x5f, 0x42, 0xa9, 0xfa, 0x67, 0x41, 0xa9, 0xfc, 0x6f,
        0xc6, 0xa8, 0xc0, 0x03, 0x5f, 0xd6, 0xf2, 0x03, 0x1f, 0xaa, 0xf8, 0x03, 0x1f, 0xaa, 0xfb,
        0x03, 0x1f, 0xaa, 0xf5, 0x03, 0x1f, 0xaa, 0xea, 0x03, 0x1f, 0x2a, 0xfa, 0x03, 0x1f, 0x2a,
        0xc1, 0xff, 0xff, 0x17,
    ],
};
pub const FORMAT_STACK: Site = Site {
    name: "format_stack",
    offset: 0x37ab750,
    guard: [
        0xfc, 0x6f, 0xba, 0xa9, 0xfa, 0x67, 0x01, 0xa9, 0xf8, 0x5f, 0x02, 0xa9, 0xf6, 0x57, 0x03,
        0xa9, 0xf4, 0x4f, 0x04, 0xa9, 0xfd, 0x7b, 0x05, 0xa9, 0xfd, 0x43, 0x01, 0x91, 0xff, 0xc3,
        0x06, 0xd1, 0xf1, 0x03, 0x00, 0xaa, 0x02, 0x04, 0x01, 0xb4, 0x4a, 0x00, 0x40, 0x79, 0xf7,
        0x03, 0x02, 0xaa, 0xaa, 0x03, 0x01, 0x34, 0xe8, 0x03, 0x1f, 0x2a, 0xfc, 0x03, 0x1f, 0x2a,
        0xe9, 0x07, 0x00, 0x32,
    ],
};
pub const GUARDS: [&Site; 6] = [
    &TEXT_STACK,
    &TEXT_FORMAT,
    &TEXT_READY,
    &FORMAT_TERMINATOR,
    &FORMAT_RETURN,
    &FORMAT_STACK,
];

/// Guarded formatter frame: 96 bytes of register saves, then 432 locals.
/// Returns the caller buffer and saved-LR slot, before any pointer is read.
pub fn frame_addresses(sp: u64, fp: u64) -> Option<(u64, u64)> {
    if sp < 0x10000 || !sp.is_multiple_of(16) || fp != sp.checked_add(512)? {
        return None;
    }
    Some((
        sp.checked_add(FRAME_BYTES)?,
        sp.checked_add(SAVED_LR_OFFSET)?,
    ))
}

/// Text tags have embedded zero words. Skip their payload for matching,
/// while retaining every original unit when the suffix is appended.
#[derive(Clone)]
struct Visible<'a> {
    text: &'a [u16],
    at: usize,
    space: bool,
}

impl Iterator for Visible<'_> {
    type Item = u16;

    fn next(&mut self) -> Option<u16> {
        while let Some(&unit) = self.text.get(self.at) {
            if unit == 0xe || unit == 0xf {
                let words = if unit == 0xe {
                    let bytes = usize::from(*self.text.get(self.at + 3)?);
                    if bytes % 2 != 0 {
                        return None;
                    }
                    4 + bytes / 2
                } else {
                    3
                };
                self.at = self.at.checked_add(words)?;
                if self.at > self.text.len() {
                    return None;
                }
                continue;
            }
            self.at += 1;
            let space = matches!(unit, 9 | 10 | 13 | 32 | 0xa0);
            if space && self.space {
                continue;
            }
            self.space = space;
            return Some(if space {
                32
            } else if (65..=90).contains(&unit) {
                unit + 32
            } else {
                unit
            });
        }
        None
    }
}

/// Recognizes the English communication/connection-error sentences,
/// including native line breaks and text tags. Other languages/messages
/// retain their original text until their labels are independently known.
pub fn is_communication_error(text: &[u16]) -> bool {
    const PHRASES: [&[u8]; 4] = [
        b"communication error has occurred",
        b"communication error occurred",
        b"connection error has occurred",
        b"connection error occurred",
    ];
    let mut visible = Visible {
        text,
        at: 0,
        space: false,
    };
    while let Some(unit) = visible.next() {
        if unit != u16::from(b'c') {
            continue;
        }
        for phrase in PHRASES {
            let mut rest = visible.clone();
            if phrase[1..]
                .iter()
                .all(|&byte| rest.next() == Some(u16::from(byte)))
            {
                return true;
            }
        }
    }
    false
}

fn ends_with_ascii(text: &[u16], suffix: &str) -> bool {
    text.len() >= suffix.len()
        && text[text.len() - suffix.len()..]
            .iter()
            .copied()
            .eq(suffix.encode_utf16())
}

/// Returns the new native UTF-16 count. No partial write on insufficient
/// room, invalid count/terminator, unrelated text or an existing suffix.
pub fn append_reason(buffer: &mut [u16], len: usize, reason: Reason) -> Option<usize> {
    if len >= buffer.len() || buffer[len] != 0 || !is_communication_error(&buffer[..len]) {
        return None;
    }
    for previous in Reason::ALL {
        if ends_with_ascii(&buffer[..len], previous.text()) {
            let prefix_end = len - previous.text().len();
            if ends_with_ascii(&buffer[..prefix_end], PREFIX) {
                return None;
            }
        }
    }
    // Both static strings are ASCII; byte count equals UTF-16 count.
    let new_len = len.checked_add(PREFIX.len() + reason.text().len())?;
    if new_len >= buffer.len() {
        return None;
    }
    for (slot, unit) in buffer[len..new_len]
        .iter_mut()
        .zip(PREFIX.encode_utf16().chain(reason.text().encode_utf16()))
    {
        *slot = unit;
    }
    buffer[new_len] = 0;
    Some(new_len)
}
