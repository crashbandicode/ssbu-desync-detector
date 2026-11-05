//! Append evidence to the game's formatted communication-error text.
//!
//! The game sets a label's text in two ways. With arguments, `.37a2660`
//! formats into a zeroed 0x1000-byte UTF-16 stack buffer through `.37ab750`;
//! without, `.377a120` does the same through `.37ae170`. Either way the
//! finished text and its count sit in the caller's buffer before the pane
//! setter `.37a2270` copies them. A [`Route`] describes one such point: the
//! hooked instruction, the register with the count, the guarded frame that
//! locates the buffer and, for a hook inside a shared callee, the saved
//! return address of the one accepted caller. [`append_in_frame`] is the hook
//! itself minus register access, the same for every route. Editing that
//! buffer avoids replacing an applet, guessing an error code, retaining a
//! pointer or sharing scratch storage between calls/threads.
use crate::disconnect::Reason;
use crate::model::Site;

pub const CAPACITY: usize = 0x800;
/// Formatter `.37ab750` epilogue (`mov w0, w20`), for caller `.37a2660`.
pub const ARGS_HOOK: usize = 0x37ad8e8;
/// `.377a120` after its formatter returned (`mov w2, w0`), before the setter.
pub const PLAIN_HOOK: usize = 0x377a250;
const PREFIX: &str = "\n\nReason: ";

// The argument route. All bytes generated from the hash-verified 13.0.5 NSO.
// Only FORMAT_RETURN is hooked. The other guards cover the native stack,
// capacity and count ABI.
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
    offset: 0x37a272c,
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
    offset: ARGS_HOOK,
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

// The no-argument route, from the same hash-verified 13.0.5 NSO
// (popup-route/route-guards.log in the work folder). PLAIN_READY is hooked.
/// `.377a120` prologue: 0x40 bytes of saves, then a 0x1120-byte frame.
pub const PLAIN_STACK: Site = Site {
    name: "plain_stack",
    offset: 0x377a120,
    guard: [
        0xfc, 0x0f, 0x1c, 0xf8, 0xf6, 0x57, 0x01, 0xa9, 0xf4, 0x4f, 0x02, 0xa9, 0xfd, 0x7b, 0x03,
        0xa9, 0xfd, 0xc3, 0x00, 0x91, 0xff, 0x07, 0x40, 0xd1, 0xff, 0x83, 0x04, 0xd1, 0xe6, 0x1f,
        0x03, 0xad, 0xe6, 0x9f, 0x0a, 0xa9, 0xe4, 0x97, 0x09, 0xa9, 0xe2, 0x8f, 0x08, 0xa9, 0xe4,
        0x17, 0x02, 0xad, 0xe2, 0x0f, 0x01, 0xad, 0xe0, 0x07, 0x00, 0xad, 0x09, 0x04, 0x40, 0xf9,
        0x29, 0x09, 0x40, 0xf9,
    ],
};
/// Lookup, `memset(sp+0x120, 0, 0x1000)`, then `.37ae170(sp+0x120, text, attr)`.
pub const PLAIN_FORMAT: Site = Site {
    name: "plain_format",
    offset: 0x377a210,
    guard: [
        0x08, 0x0e, 0x01, 0xb0, 0x08, 0x31, 0x46, 0xf9, 0x00, 0x01, 0x40, 0xf9, 0xe1, 0x03, 0x14,
        0xaa, 0x8c, 0xa8, 0x00, 0x94, 0x68, 0x5e, 0x42, 0xb9, 0xf4, 0x03, 0x00, 0xaa, 0xe0, 0x83,
        0x04, 0x91, 0xe2, 0x03, 0x14, 0x32, 0xe1, 0x03, 0x1f, 0x2a, 0xa8, 0x83, 0x1d, 0xb8, 0x19,
        0x18, 0x09, 0x94, 0xe0, 0x83, 0x04, 0x91, 0xa2, 0xa3, 0x00, 0xd1, 0xe1, 0x03, 0x14, 0xaa,
        0xc9, 0xcf, 0x00, 0x94,
    ],
};
/// Hooked: `mov w2,w0; add x1,sp,#0x120; mov x0,x19; mov w3,wzr; bl .37a2270`.
pub const PLAIN_READY: Site = Site {
    name: "plain_ready",
    offset: PLAIN_HOOK,
    guard: [
        0xe2, 0x03, 0x00, 0x2a, 0xe1, 0x83, 0x04, 0x91, 0xe0, 0x03, 0x13, 0xaa, 0xe3, 0x03, 0x1f,
        0x2a, 0x04, 0xa0, 0x00, 0x94, 0xff, 0x07, 0x40, 0x91, 0xff, 0x83, 0x04, 0x91, 0xfd, 0x7b,
        0x43, 0xa9, 0xf4, 0x4f, 0x42, 0xa9, 0xf6, 0x57, 0x41, 0xa9, 0xfc, 0x07, 0x44, 0xf8, 0xc0,
        0x03, 0x5f, 0xd6, 0xff, 0x43, 0x04, 0xd1, 0xfc, 0x7b, 0x00, 0xf9, 0xfd, 0x7b, 0x10, 0xa9,
        0xfd, 0x03, 0x04, 0x91,
    ],
};
/// `.37ae170` end: count bounded to 0x7ff, NUL at the count and at unit
/// 0x7ff, `mov w0, w22`, epilogue.
pub const PLAIN_BOUND: Site = Site {
    name: "plain_bound",
    offset: 0x37ae580,
    guard: [
        0xf8, 0x03, 0x1f, 0x2a, 0x16, 0x03, 0x16, 0x0b, 0x69, 0x03, 0x19, 0x0b, 0x3f, 0x01, 0x17,
        0x6b, 0xcb, 0xe6, 0xff, 0x54, 0xdf, 0xfa, 0x1f, 0x71, 0x8d, 0x00, 0x00, 0x54, 0xf6, 0x2b,
        0x00, 0x32, 0x02, 0x00, 0x00, 0x14, 0xf6, 0x03, 0x1f, 0x2a, 0x7f, 0xda, 0x36, 0x78, 0x7f,
        0xfe, 0x1f, 0x79, 0xe0, 0x03, 0x16, 0x2a, 0xfd, 0x7b, 0x47, 0xa9, 0xf4, 0x4f, 0x46, 0xa9,
        0xf6, 0x57, 0x45, 0xa9,
    ],
};

/// One native point where a caller's finished UTF-16 text and its count are
/// ready and the pane setter has not run yet.
pub struct Route {
    /// Install-line and event name.
    pub name: &'static str,
    /// Event payload (`communication_text route=`).
    pub code: u64,
    /// The hooked instruction, a `mov` that hands the count on.
    pub hook: usize,
    /// Register holding the native UTF-16 count at the hook. The relocated
    /// `mov` reads it, so updating it changes the count the setter receives.
    pub count: usize,
    /// The guarded frame: FP == SP + `fp`.
    pub fp: u64,
    /// The caller's zeroed 0x800-unit buffer at SP + `buffer`.
    pub buffer: u64,
    /// For a hook inside a shared callee: the saved LR at SP + `.0` must be
    /// `.text` + `.1`, the one caller whose frame holds that buffer.
    pub caller: Option<(u64, u64)>,
    /// Every site that must hold its 13.0.5 bytes before the hook goes in.
    pub guards: &'static [&'static Site],
}

/// Label text with arguments: the formatter epilogue, for caller `.37a2660`
/// only. The formatter frame is 96 bytes of saves plus 432 locals, so the
/// caller's buffer (its SP) is at SP+528 and its return address at SP+520.
pub const ARGS: Route = Route {
    name: "args",
    code: 0,
    hook: ARGS_HOOK,
    count: 20,
    fp: 512,
    buffer: 528,
    caller: Some((520, 0x37a272c)),
    guards: &[
        &TEXT_STACK,
        &TEXT_FORMAT,
        &TEXT_READY,
        &FORMAT_TERMINATOR,
        &FORMAT_RETURN,
        &FORMAT_STACK,
    ],
};

/// Label text without arguments (the pop-up body's route for a sentence with
/// no placeholders): inside `.377a120`, which owns the buffer, so any caller.
/// Its frame is 0x40 bytes of saves (FP at +0x30) above 0x1120 bytes.
pub const PLAIN: Route = Route {
    name: "plain",
    code: 1,
    hook: PLAIN_HOOK,
    count: 0,
    fp: 0x1150,
    buffer: 0x120,
    caller: None,
    guards: &[&PLAIN_STACK, &PLAIN_FORMAT, &PLAIN_READY, &PLAIN_BOUND],
};

pub const ROUTES: [&Route; 2] = [&ARGS, &PLAIN];

/// Addresses inside a route's guarded frame, before any of them is read.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Frame {
    pub buffer: u64,
    pub saved_lr: Option<u64>,
}

impl Route {
    /// The caller buffer and saved-LR slot for SP/FP at the hook, or `None`
    /// when the frame is not the guarded one.
    pub fn frame(&self, sp: u64, fp: u64) -> Option<Frame> {
        if sp < 0x10000 || !sp.is_multiple_of(16) || fp != sp.checked_add(self.fp)? {
            return None;
        }
        let buffer = sp.checked_add(self.buffer)?;
        buffer.checked_add(2 * CAPACITY as u64)?;
        let saved_lr = match self.caller {
            Some((slot, _)) => Some(sp.checked_add(slot)?),
            None => None,
        };
        Some(Frame { buffer, saved_lr })
    }
}

/// The hook of every route, minus register access: checks the frame and the
/// caller, then appends `reason()` to a communication-error text. `reason`
/// runs only for such a text. Returns the reason and the new count.
///
/// # Safety
/// `sp`/`fp` are the native SP and FP at `route.hook` with the guards of
/// `route` matched, so the frame (and the saved LR slot, if any) is live.
pub unsafe fn append_in_frame(
    route: &Route,
    text_base: u64,
    sp: u64,
    fp: u64,
    len: usize,
    reason: impl FnOnce() -> Reason,
) -> Option<(Reason, usize)> {
    if len >= CAPACITY {
        return None;
    }
    let frame = route.frame(sp, fp)?;
    if let (Some(slot), Some((_, offset))) = (frame.saved_lr, route.caller) {
        if (slot as *const u64).read_unaligned() != text_base.checked_add(offset)? {
            return None;
        }
    }
    // The guards fix this caller's zeroed 0x1000-byte buffer at this frame
    // offset; it belongs to this call and is not kept.
    let buffer = core::slice::from_raw_parts_mut(frame.buffer as *mut u16, CAPACITY);
    if !is_communication_error(&buffer[..len]) {
        return None;
    }
    let reason = reason();
    append_reason(buffer, len, reason).map(|len| (reason, len))
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
