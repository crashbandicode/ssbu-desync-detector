//! Host-testable half: hook sites, the record and event formats, fixed-buffer
//! log lines, and the referee that decides which console diverged.
//!
//! Smash Ultimate 13.0.5 runs online matches in lockstep. Every input record
//! carries a check value, the XOR of the `+0xc` word of every game-RNG state
//! (`.text+0x16d6c3c`), and each console compares the peer's value with its own
//! (`.text+0x16cf1d0`). A mismatch aborts the match; the result is "No contest".
//! Both consoles see the same mismatch, so the values alone cannot say which
//! simulation left vanilla. A vanilla re-simulation can: the game records every
//! online match as a replay (inputs plus a per-frame position checksum), and
//! playing that recording in an unmodded game recomputes both the positions and
//! the RNG check value for every frame. Whichever console's values the vanilla
//! run reproduces is the one that stayed vanilla.

use core::fmt;

pub const SUPPORTED_VERSION: &str = "13.0.5";

/// An inline hook site and the 64 bytes that must be there (13.0.5 image).
pub struct Site {
    pub name: &'static str,
    pub offset: usize,
    pub guard: [u8; 64],
}

/// `stp w11, w8, [x9]` after the online check value is computed for the frame
/// about to run: w11 = check value, w8 = 1 << local station, x10 - 0xa = the
/// input record (u16 frame at +0), x19 = the lockstep session.
pub const FRAME_CHECK_OFFSET: usize = 0x16d6cdc;
pub const FRAME_CHECK: Site = Site {
    name: "frame_check",
    offset: FRAME_CHECK_OFFSET,
    guard: [
        0x2b, 0x21, 0x00, 0x29, 0x28, 0x99, 0x92, 0x52, 0x88, 0x00, 0xa0, 0x72, 0xf4, 0x03, 0x1f,
        0xaa, 0x68, 0x02, 0x08, 0x8b, 0x05, 0xfb, 0x87, 0x52, 0xff, 0x73, 0x00, 0xb9, 0x64, 0xec,
        0x92, 0x52, 0x06, 0x1f, 0x96, 0x52, 0xe8, 0x2f, 0x00, 0xf9, 0x07, 0x22, 0x96, 0x52, 0x16,
        0x20, 0x96, 0x52, 0x17, 0x1e, 0x96, 0x52, 0xda, 0x06, 0x80, 0x52, 0xf8, 0x03, 0x00, 0x32,
        0x19, 0xef, 0x92, 0x52,
    ],
};

/// `ldrb w8, [x0, #0x5f]`, reached only when a peer's check value differs from
/// ours: w8 = ours, w9 = the peer's, w1 = its station, x2 = our record (u16
/// frame at +0).
pub const DESYNC_OFFSET: usize = 0x16cf304;
pub const DESYNC: Site = Site {
    name: "desync",
    offset: DESYNC_OFFSET,
    guard: [
        0x08, 0x7c, 0x41, 0x39, 0x08, 0x01, 0x00, 0x32, 0x08, 0x7c, 0x01, 0x39, 0xc0, 0x03, 0x5f,
        0xd6, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0x43,
        0x03, 0xd1, 0xe8, 0x33, 0x00, 0xfd, 0xfc, 0x6f, 0x07, 0xa9, 0xfa, 0x67, 0x08, 0xa9, 0xf8,
        0x5f, 0x09, 0xa9, 0xf6, 0x57, 0x0a, 0xa9, 0xf4, 0x4f, 0x0b, 0xa9, 0xfd, 0x7b, 0x0c, 0xa9,
        0xfd, 0x03, 0x03, 0x91,
    ],
};

/// Entry of the lockstep session end `(S, kind)`; x30 names the trigger.
pub const SYNC_END_OFFSET: usize = 0x16ce7c0;
pub const SYNC_END: Site = Site {
    name: "sync_end",
    offset: SYNC_END_OFFSET,
    guard: [
        0xff, 0xc3, 0x02, 0xd1, 0xef, 0x3b, 0x01, 0x6d, 0xed, 0x33, 0x02, 0x6d, 0xeb, 0x2b, 0x03,
        0x6d, 0xe9, 0x23, 0x04, 0x6d, 0xfc, 0x6f, 0x05, 0xa9, 0xfa, 0x67, 0x06, 0xa9, 0xf8, 0x5f,
        0x07, 0xa9, 0xf6, 0x57, 0x08, 0xa9, 0xf4, 0x4f, 0x09, 0xa9, 0xfd, 0x7b, 0x0a, 0xa9, 0xfd,
        0x83, 0x02, 0x91, 0x28, 0x1c, 0x00, 0x12, 0xf3, 0x03, 0x00, 0xaa, 0x1f, 0x09, 0x00, 0x71,
        0x61, 0x00, 0x00, 0x54,
    ],
};

/// The type-7 (end/abort) case of the lockstep control-message dispatcher:
/// x19 = session, x20 = message (`[0] >> 4` sender, `[1..3]` big-endian reason).
pub const PEER_ABORT_OFFSET: usize = 0x16cc2c4;
pub const PEER_ABORT: Site = Site {
    name: "peer_abort",
    offset: PEER_ABORT_OFFSET,
    guard: [
        0x61, 0x06, 0x40, 0x39, 0x62, 0x0e, 0x40, 0x79, 0x23, 0x03, 0x80, 0x52, 0xe0, 0x03, 0x13,
        0xaa, 0xe4, 0x03, 0x1f, 0x2a, 0x16, 0x06, 0x00, 0x94, 0xc0, 0x00, 0x00, 0x34, 0xe3, 0x03,
        0x00, 0x2a, 0x62, 0x16, 0x40, 0xf9, 0x60, 0x32, 0x40, 0xb9, 0x21, 0x10, 0x80, 0x52, 0xd4,
        0x06, 0x00, 0x94, 0x88, 0x12, 0x40, 0x78, 0x61, 0xb6, 0x40, 0x79, 0x7f, 0x9e, 0x00, 0x79,
        0x7f, 0x42, 0x01, 0x79,
    ],
};

/// Replay playback's per-frame check, `ldr w9, [x22, #0x4]`: w8 = the position
/// checksum just recomputed, `[x22 + 4]` = the recorded one, x27 = the game
/// object whose `+0x590` is the replay player (frame index at `+0x200078`, one
/// past the current record).
pub const PLAYBACK_CHECK_OFFSET: usize = 0x135fd60;
pub const PLAYBACK_CHECK: Site = Site {
    name: "playback_check",
    offset: PLAYBACK_CHECK_OFFSET,
    guard: [
        0xc9, 0x06, 0x40, 0xb9, 0x1f, 0x01, 0x09, 0x6b, 0xc0, 0x13, 0x00, 0x54, 0xe8, 0x03, 0x00,
        0x32, 0x68, 0x4b, 0x00, 0x39, 0x9b, 0x00, 0x00, 0x14, 0x89, 0x10, 0x80, 0x52, 0x09, 0x04,
        0xa0, 0x72, 0x09, 0x01, 0x09, 0x8b, 0x2a, 0x01, 0x40, 0xb9, 0x5f, 0x05, 0x00, 0x71, 0xa0,
        0x12, 0x00, 0x54, 0x6a, 0x9b, 0x56, 0x39, 0x6a, 0x02, 0x00, 0x34, 0x7f, 0x9b, 0x16, 0x39,
        0x17, 0x00, 0x00, 0x14,
    ],
};

/// `orr w8, wzr, #0x1e0000` on the pass that arms the 30-frame countdown,
/// after `.16acd30` returned nonzero and that byte was stored at session
/// `+0x5e`. The next three words store the countdown and the armed flag.
/// Straight line: a 16-byte far patch relocates no branch and no call.
pub const FAULT_ARM_OFFSET: usize = 0x16d5ee8;
pub const FAULT_ARM: Site = Site {
    name: "fault_arm",
    offset: FAULT_ARM_OFFSET,
    guard: [
        0xe8, 0x0f, 0x0f, 0x32, 0x68, 0x66, 0x00, 0xb9, 0xe8, 0x03, 0x00, 0x32, 0x68, 0x8e, 0x01,
        0x39, 0x68, 0x02, 0x40, 0x39, 0x1f, 0x05, 0x00, 0x71, 0xc1, 0xfc, 0xff, 0x54, 0x61, 0x06,
        0x40, 0x39, 0x3f, 0xfc, 0x03, 0xf1, 0x60, 0xfc, 0xff, 0x54, 0x08, 0x2f, 0x46, 0xf9, 0x16,
        0x05, 0x40, 0xf9, 0xc8, 0x46, 0x40, 0xf9, 0x09, 0x00, 0x94, 0x52, 0x09, 0x69, 0x69, 0xb8,
        0xea, 0x03, 0x00, 0x32,
    ],
};

/// `bl .16acd30`. Its return is the byte stored by `FAULT_STORE`.
pub const FAULT_CALL_OFFSET: usize = 0x16d5ec4;
pub const FAULT_CALL_WORD: u32 = 0x97ff_5b9b;
/// `strb w0, [x19, #0x5e]`.
pub const FAULT_STORE_OFFSET: usize = 0x16d5ecc;
pub const FAULT_STORE_WORD: u32 = 0x3901_7a60;
/// `mov x19, x0`, the session pointer the arming stores use.
pub const FAULT_SESSION_OFFSET: usize = 0x16d5e90;
pub const FAULT_SESSION_WORD: u32 = 0xaa00_03f3;
/// `.text` slot holding the fault-table wrapper. The object is `*(*slot)`.
pub const FAULT_OBJECT: usize = 0x52c3940;
/// Session byte written with `.16acd30`'s return on the arming pass.
pub const FAULT_CODE_AT: usize = 0x5e;
/// First of the ten words `.16acd30` walks. Word 0 suppresses; words 1..9
/// are the returns 1..9.
pub const FAULT_WORD_AT: usize = 0x18;
pub const FAULT_WORD_COUNT: usize = 10;

pub const SITES: [&Site; 6] = [
    &FRAME_CHECK,
    &DESYNC,
    &SYNC_END,
    &PEER_ABORT,
    &PLAYBACK_CHECK,
    &FAULT_ARM,
];

/// `.text` slot holding a pointer to the RNG manager pointer; the manager
/// starts with a `[begin, end)` vector of 20-byte RNG states.
pub const RNG_MANAGER: usize = 0x52c35b0;
pub const RNG_STATE_STRIDE: usize = 0x14;
pub const RNG_CHECK_WORD: usize = 0xc;
/// Game object `+0x590`: the replay player/recorder; its frame index.
pub const REPLAY_PLAYER: usize = 0x590;
pub const REPLAY_FRAME_INDEX: usize = 0x200078;

/// The match recorder. Every online match is recorded at StateMelee start
/// into an arena entry: a `CFMK` header kept current every frame, while the
/// records are staged in the player's buffer until the end-of-match commit
/// appends them as a `FRAM` chunk. A saved replay file is a 0x3b0-byte outer
/// header followed by that entry.
pub mod recorder {
    /// `.text` slot holding the game object pointer (the player is at `+0x590`).
    pub const GAME_OBJECT: usize = 0x52c50f0;
    pub const ARENA: usize = 0x18;
    pub const STAGING: usize = 0x68;
    pub const CURSOR: usize = 0x200068;
    pub const MODE: usize = 0x24;
    pub const MODE_RECORD: u32 = 2;
    /// Arena: entry count at `+0`, data at `+0x20`, descriptors `{u64 off,
    /// u64 size, u8 valid}` growing down from `+0x600008`, the outer-header
    /// map's root at `+0x600028` and the overflow flag at `+0x600038`.
    pub const ARENA_DATA: usize = 0x20;
    pub const ARENA_BYTES: usize = 0x600000;
    pub const DESCRIPTORS: usize = 0x600008;
    pub const DESCRIPTOR_STRIDE: usize = 0x18;
    pub const OUTER_MAP_ROOT: usize = 0x600028;
    pub const OVERFLOW: usize = 0x600038;
    /// Outer-header map node: key at `+0x20`, the shared_ptr's object (the
    /// 0x3b0-byte header itself) at `+0x28`. Reading it at object `+0x18`
    /// put "Smash" at file `+0x02` instead of `+0x1a` (Eden, 2026-10-29).
    pub const NODE_KEY: usize = 0x20;
    pub const NODE_OBJECT: usize = 0x28;
    pub const OBJECT_OUTER: usize = 0;
    pub const STAGING_BYTES: usize = 0x200000;
}

pub const OUTER_BYTES: usize = 0x3b0;
pub const CFMK_BYTES: usize = 0x1bb0;
pub const TAG_CFMK: u32 = u32::from_le_bytes(*b"CFMK");
pub const TAG_FRAM: u32 = u32::from_le_bytes(*b"FRAM");
pub const REPLAY_VERSION: u32 = 0xd005;
/// CFMK header: payload bytes at `+0xc`, record count at `+0x14`.
pub const CFMK_PAYLOAD_LEN: usize = 0xc;
pub const CFMK_RECORDS: usize = 0x14;

fn le32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn le16(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

/// XOR of the little-endian words in `bytes`; a trailing partial word is
/// ignored. Entry word 0 is the XOR of every later word, so it can be
/// computed piece by piece over word-aligned pieces.
pub fn xor_words(bytes: &[u8]) -> u32 {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .fold(0, |acc, word| acc ^ u32::from_le_bytes(*word))
}

/// Eden-only test switches the diag build reads from `diag.txt`; the
/// shipped NRO never reads them.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Diag {
    /// The online frame whose check value this console stores with its low
    /// bit flipped, once per boot (the first session that reaches it). Both
    /// consoles then record a mismatch at that frame and the game ends the
    /// session on its own desync path; the simulation itself is untouched.
    pub force_desync_frame: Option<u32>,
}

/// `key=value` lines; unknown keys, bad numbers and other lines are ignored.
pub fn parse_diag(text: &[u8]) -> Diag {
    let mut diag = Diag::default();
    for line in text.split(|byte| *byte == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        let Some(split) = line.iter().position(|byte| *byte == b'=') else {
            continue;
        };
        let (key, value) = (line[..split].trim_ascii(), line[split + 1..].trim_ascii());
        if key == b"force_desync_frame" {
            diag.force_desync_frame = core::str::from_utf8(value)
                .ok()
                .and_then(|value| value.parse().ok());
        }
    }
    diag
}

/// The check value a diag build stores for `frame` (unchanged unless forced).
pub fn diag_check_value(value: u32, frame: u32, diag: &Diag) -> u32 {
    if diag.force_desync_frame == Some(frame) {
        value ^ 1
    } else {
        value
    }
}

/// The packed local time at outer `+0`: year, month, day, hour and minute
/// in the low word, the second in the high word (other high bits as saved
/// replays have them).
pub fn pack_time(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> u64 {
    let low = u64::from(year & 0xfff)
        | u64::from(month & 0xf) << 12
        | u64::from(day & 0x1f) << 16
        | u64::from(hour & 0x1f) << 21
        | u64::from(minute & 0x3f) << 26;
    low | (0x8001_0000 | u64::from(second & 0x3f)) << 32
}

/// The outer header written when the game has not built one (a match that
/// was not finalised): the time and the version, everything else zero. The
/// game does not check the outer header; `examples/replay.rs` can swap in a
/// saved replay's header for the full Vault display.
pub fn minimal_outer(time: u64) -> [u8; OUTER_BYTES] {
    let mut outer = [0; OUTER_BYTES];
    outer[..8].copy_from_slice(&time.to_le_bytes());
    outer[8..12].copy_from_slice(&REPLAY_VERSION.to_le_bytes());
    outer
}

/// Mirror of the game's replay validator (`.text+0x175be20`), which gates
/// both the Vault's play button and playback.
pub fn validate_entry(entry: &[u8]) -> Result<(), &'static str> {
    let size = entry.len();
    if size <= CFMK_BYTES {
        return Err("entry too short");
    }
    if le32(entry, 8) < 0x9000 {
        return Err("version below 0x9000");
    }
    if le32(entry, 0) != xor_words(&entry[4..]) {
        return Err("xor word");
    }
    if usize::from(le16(entry, 0x10)) != CFMK_BYTES {
        return Err("header size");
    }
    if le32(entry, 4) != TAG_CFMK {
        return Err("not CFMK");
    }
    if !(1..=8).contains(&le16(entry, 0x12)) {
        return Err("segment count");
    }
    if le16(entry, CFMK_RECORDS) == 0 {
        return Err("no records");
    }
    if le32(entry, CFMK_PAYLOAD_LEN) as usize > size - CFMK_BYTES {
        return Err("payload length");
    }
    let mut at = CFMK_BYTES;
    while at + 8 <= size {
        if le32(entry, at) == TAG_FRAM {
            return Ok(());
        }
        at += 8 + le32(entry, at + 4) as usize;
    }
    Err("no FRAM chunk")
}

/// The online check value: XOR of every RNG state's `+0xc` word.
pub fn rng_check(words: impl IntoIterator<Item = u32>) -> u32 {
    words.into_iter().fold(0, |acc, word| acc ^ word)
}

/// What ended a lockstep session, from the caller of `.text+0x16ce7c0`.
pub fn sync_end_trigger(caller: u64) -> &'static str {
    match caller {
        0x16d6b08 => "desync",
        0x16cfc18 => "peer_silent",
        0x16cfb8c => "session_timeout",
        0x16cf4f4 => "station_lost_before_start",
        0x16d7908 | 0x16d7edc | 0x16d9190 => "participant_missing_at_start",
        0x16d5ec0 => "referee",
        0x1563258 => "end_judge",
        0x1569718 | 0x1568824 => "normal",
        _ => "other",
    }
}

/// One frame in `frames-N.bin`, 20 bytes little-endian. Online: the frame
/// number, our check value and `KIND_ONLINE | local station << 8`. Playback:
/// the replay frame index, the check value recomputed by the vanilla game, and
/// the recomputed and recorded position checksums.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FrameRecord {
    pub frame: u32,
    pub rng: u32,
    pub position: u32,
    pub recorded: u32,
    pub flags: u32,
}

pub const KIND_ONLINE: u32 = 1;
pub const KIND_PLAYBACK: u32 = 2;
pub const RECORD_BYTES: usize = 20;

impl FrameRecord {
    pub fn kind(&self) -> u32 {
        self.flags & 0xff
    }

    pub fn to_bytes(self) -> [u8; RECORD_BYTES] {
        let mut out = [0; RECORD_BYTES];
        for (index, word) in [
            self.frame,
            self.rng,
            self.position,
            self.recorded,
            self.flags,
        ]
        .into_iter()
        .enumerate()
        {
            out[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let word = |index: usize| {
            bytes
                .get(index * 4..index * 4 + 4)
                .map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
        };
        Some(Self {
            frame: word(0)?,
            rng: word(1)?,
            position: word(2)?,
            recorded: word(3)?,
            flags: word(4)?,
        })
    }
}

/// A text line built in a fixed buffer; anything past `N` bytes is dropped.
pub struct Line<const N: usize> {
    buf: [u8; N],
    len: usize,
}

impl<const N: usize> Line<N> {
    pub const fn new() -> Self {
        Self {
            buf: [0; N],
            len: 0,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// The text plus a NUL, for `nn::fs` paths; `None` when there is no room.
    pub fn with_nul(&mut self) -> Option<&[u8]> {
        if self.len >= N {
            return None;
        }
        self.buf[self.len] = 0;
        Some(&self.buf[..=self.len])
    }
}

impl<const N: usize> Default for Line<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> fmt::Write for Line<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let take = s.len().min(N - self.len);
        self.buf[self.len..self.len + take].copy_from_slice(&s.as_bytes()[..take]);
        self.len += take;
        Ok(())
    }
}

/// Events the hooks queue for the log, as plain integers.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RawEvent {
    pub tick: u64,
    pub code: u8,
    pub a: u64,
    pub b: u64,
}

pub const EV_SYNC_END: u8 = 1;
pub const EV_DESYNC: u8 = 2;
pub const EV_PEER_ABORT: u8 = 3;
pub const EV_LINK: u8 = 4;
pub const EV_PLAYBACK_MISMATCH: u8 = 5;
pub const EV_SESSION: u8 = 6;
pub const EV_RECORDING: u8 = 7;
/// A native communication-error text was prepared, not proof of a rendered frame.
pub const EV_COMMUNICATION_TEXT: u8 = 8;
/// Frames stopped arriving; an observation of coverage, never a native end.
pub const EV_FRAME_GAP: u8 = 9;
/// A Canceled notice or the reconnect warning was prepared. Not proof it was
/// drawn, and not a claim about why the game chose it.
pub const EV_NOTICE_TEXT: u8 = 10;
/// `.16acd30` returned nonzero and the match armed its countdown.
pub const EV_FAULT: u8 = 11;
/// `dialog` notice kinds, in `EV_NOTICE_TEXT` bits 0..8.
pub const NOTICE_KINDS: [&str; 3] = ["canceled", "canceled_period", "reconnect_warning"];
/// `EV_NOTICE_TEXT` bits 16..24 when the session had no verdict.
pub const NOTICE_VERDICT_WITHHELD: u64 = 0xff;
pub const COMMUNICATION_REASONS: [&str; 6] = [
    "desync",
    "local_link",
    "local_status",
    "peer_ended",
    "timeout",
    "unknown",
];
/// `dialog::Route::code` names, in `EV_COMMUNICATION_TEXT` bits 8..16.
pub const COMMUNICATION_ROUTES: [&str; 2] = ["args", "plain"];

/// Why a match recording was not saved (`EV_RECORDING` with no bytes).
pub const RECORDING_REASONS: [&str; 9] = [
    "none",
    "no_recorder",
    "no_entry",
    "overflow",
    "invalid_entry",
    "not_cfmk",
    "length_mismatch",
    "write_failed",
    "normal_end",
];
/// The recording was not kept because the match ended normally.
pub const RECORDING_NORMAL_END: u8 = 8;

/// Session end kind 1 is a normal match end at the results; kind 2 is an
/// abort (desync, a silent peer, the session timeouts, the referee).
pub const END_KIND_NORMAL: u32 = 1;
/// The reason an opponent's end message carries on a normal end.
pub const PEER_END_NORMAL_REASON: u64 = 1;

/// How an online session ended, as far as a recording is concerned: only
/// abnormal ends keep their recording (the referee needs it; a normal match
/// does not).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EndState {
    pub normal_end_seen: bool,
    pub abnormal: bool,
}

impl EndState {
    /// A lockstep session end (`EV_SYNC_END`).
    pub fn sync_end(&mut self, kind: u32, desync_seen: bool) {
        if kind == END_KIND_NORMAL && !desync_seen {
            self.normal_end_seen = true;
        } else {
            self.abnormal = true;
        }
    }

    /// A mismatch row (`EV_DESYNC`).
    pub fn desync(&mut self) {
        self.abnormal = true;
    }

    /// The opponent's end message (`EV_PEER_ABORT`).
    pub fn peer_end(&mut self, reason: u64) {
        if reason != PEER_END_NORMAL_REASON {
            self.abnormal = true;
        }
    }

    /// Keep the recording unless a normal end was seen and nothing abnormal
    /// was: a session that just stopped (no end seen) is kept too.
    pub fn keep_recording(&self) -> bool {
        self.abnormal || !self.normal_end_seen
    }
}

/// A zero return does not arm the countdown, so it is not a log line.
pub fn fault_code_is_logged(code: u8) -> bool {
    code != 0
}

/// One arming of the fault table. `words`, when read, are `+0x18` through
/// `+0x3c` in that order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FaultView {
    pub code: u8,
    pub frame: Option<u32>,
    pub mask: Option<u16>,
    pub words: Option<[u32; FAULT_WORD_COUNT]>,
}

/// `code`, `frame` and `mask` packed into an event payload. The ten words
/// travel beside it; they do not fit here.
pub fn pack_fault(view: &FaultView) -> u64 {
    let mut packed = u64::from(view.code);
    if let Some(frame) = view.frame {
        packed |= u64::from(frame) << 8;
        packed |= 1 << 40;
    }
    if let Some(mask) = view.mask {
        packed |= u64::from(mask & 0x3ff) << 41;
        packed |= 1 << 51;
    }
    packed
}

pub fn unpack_fault(packed: u64) -> FaultView {
    FaultView {
        code: packed as u8,
        frame: (packed & 1 << 40 != 0).then_some((packed >> 8) as u32),
        mask: (packed & 1 << 51 != 0).then_some((packed >> 41) as u16 & 0x3ff),
        words: None,
    }
}

/// One `events.log` line (without the newline) for a queued event.
pub fn describe(event: &RawEvent, out: &mut impl fmt::Write) -> fmt::Result {
    let (a, b) = (event.a, event.b);
    write!(out, "{} ", event.tick)?;
    match event.code {
        EV_SYNC_END => write!(
            out,
            "sync_end trigger={} kind={} frame={} local={} desync_seen={} already_ending={} caller=0x{:x}",
            sync_end_trigger(b),
            a & 0xff,
            a >> 32 & 0xffff,
            a >> 16 & 0xff,
            a >> 8 & 1,
            a >> 9 & 1,
            b
        ),
        EV_DESYNC => write!(
            out,
            "desync frame={} ours=0x{:08x} theirs=0x{:08x} peer={}",
            b >> 32 & 0xffff,
            a & 0xffff_ffff,
            b & 0xffff_ffff,
            a >> 32 & 0xff
        ),
        EV_PEER_ABORT => write!(
            out,
            "peer_end sender={} reason={} frame={}",
            a & 0xff,
            a >> 8 & 0xffff,
            b
        ),
        EV_LINK => write!(
            out,
            "link result=0x{:x} interface={} strength={} state={}",
            b & 0xffff_ffff,
            a & 0xff,
            a >> 32 & 0xff,
            b >> 32
        ),
        EV_COMMUNICATION_TEXT => write!(
            out,
            "communication_text reason={} text_units={b} route={}",
            COMMUNICATION_REASONS
                .get((a & 0xff) as usize)
                .copied()
                .unwrap_or("unknown"),
            COMMUNICATION_ROUTES
                .get((a >> 8 & 0xff) as usize)
                .copied()
                .unwrap_or("unknown")
        ),
        EV_NOTICE_TEXT => {
            let verdict = a >> 16 & 0xff;
            write!(
                out,
                "notice_text kind={} text_units={} route={} caller=0x{:x} verdict={}",
                NOTICE_KINDS
                    .get((a & 0xff) as usize)
                    .copied()
                    .unwrap_or("unknown"),
                a >> 32 & 0xffff,
                COMMUNICATION_ROUTES
                    .get((a >> 8 & 0xff) as usize)
                    .copied()
                    .unwrap_or("unknown"),
                b,
                if verdict == NOTICE_VERDICT_WITHHELD {
                    "withheld"
                } else {
                    COMMUNICATION_REASONS
                        .get(verdict as usize)
                        .copied()
                        .unwrap_or("unknown")
                }
            )
        }
        EV_FRAME_GAP => write!(
            out,
            "frame_gap kind={} slot={} frame={} last_frame_tick={b}",
            if a >> 32 & 0xff == u64::from(KIND_PLAYBACK) { "playback" } else { "online" },
            a >> 40 & 0xff,
            a & 0xffff_ffff
        ),
        EV_PLAYBACK_MISMATCH => write!(
            out,
            "playback_mismatch frame={} computed=0x{:08x} recorded=0x{:08x}",
            b,
            a & 0xffff_ffff,
            a >> 32
        ),
        EV_SESSION => write!(
            out,
            "session slot={} kind={} heap_allocations={}",
            a,
            if b & 0xff == u64::from(KIND_PLAYBACK) { "playback" } else { "online" },
            b >> 8
        ),
        EV_RECORDING if a & 0xffff_ffff == 0 && b >> 32 == u64::from(RECORDING_NORMAL_END) => {
            write!(
                out,
                "recording slot={} entry={} not_kept reason=normal_end",
                a >> 48 & 0xff,
                b & 0xffff_ffff
            )
        }
        EV_RECORDING if a & 0xffff_ffff == 0 => write!(
            out,
            "recording slot={} entry={} unavailable reason={}",
            a >> 48 & 0xff,
            b & 0xffff_ffff,
            RECORDING_REASONS[(b >> 32) as usize % RECORDING_REASONS.len()]
        ),
        EV_RECORDING => write!(
            out,
            "recording slot={} entry={} bytes={} records={} source={} outer={}",
            a >> 48 & 0xff,
            b & 0xffff_ffff,
            a & 0xffff_ffff,
            a >> 32 & 0xffff,
            if a >> 56 & 1 != 0 { "committed" } else { "staged" },
            if a >> 57 & 1 != 0 { "game" } else { "minimal" }
        ),
        other => write!(out, "unknown code={other} a=0x{a:x} b=0x{b:x}"),
    }
}

/// The fault-table line. `words` absent means the object could not be read,
/// which is not the same as ten zeros.
pub fn describe_fault(tick: u64, view: &FaultView, out: &mut impl fmt::Write) -> fmt::Result {
    write!(out, "{tick} fault code={} ", view.code)?;
    match view.frame {
        Some(frame) => write!(out, "frame={frame} ")?,
        None => write!(out, "frame=none ")?,
    }
    match view.mask {
        Some(mask) => write!(out, "mask=0x{mask:x} ")?,
        None => write!(out, "mask=none ")?,
    }
    match view.words {
        Some(words) => {
            write!(out, "words=")?;
            for (index, word) in words.iter().enumerate() {
                if index != 0 {
                    write!(out, ",")?;
                }
                write!(out, "{word}")?;
            }
            Ok(())
        }
        None => write!(out, "words=unread"),
    }
}

/// A bounded writer-thread sample. Raw network enums are retained rather
/// than interpreted as a particular dock, adapter or router failure.
#[derive(Clone, Copy, Debug, Default)]
pub struct Telemetry {
    pub tick: u64,
    pub boot_tick: u64,
    pub unix_seconds: u64,
    pub clock_initialized: bool,
    pub clock_result: u32,
    pub clock_query_ticks: u64,
    pub poll: u64,
    pub poll_gap_ticks: u64,
    pub link: (u64, u64),
    pub active: bool,
    pub kind: u32,
    pub slot: usize,
    pub frame: u32,
    pub last_frame_tick: u64,
    pub event_drops: u64,
    pub frame_drops: u64,
    pub evidence_losses: u64,
    pub heap_allocations: u64,
}

/// Wall-clock seconds are usable only when initialized and result is zero.
/// The tick span bounds where the seconds-resolution clock was read; it
/// cannot establish that the console clock agrees with the router clock.
pub fn describe_telemetry(sample: &Telemetry, out: &mut impl fmt::Write) -> fmt::Result {
    let ticks_per_ms = 19_200;
    let frame_age = if sample.last_frame_tick == 0 {
        u64::MAX
    } else {
        sample.tick.saturating_sub(sample.last_frame_tick) / ticks_per_ms
    };
    let kind = match sample.kind {
        KIND_ONLINE => "online",
        KIND_PLAYBACK => "playback",
        _ => "idle",
    };
    write!(out,
        "{} telemetry boot_tick={} unix_s={} clock_initialized={} clock_result=0x{:x} clock_query_ticks={} poll={} poll_gap_ms={} result=0x{:x} interface={} strength={} state={} active={} kind={} slot={} frame={} frame_age_ms={} event_drops={} frame_drops={} evidence_losses={} heap_allocations={}",
        sample.tick, sample.boot_tick, sample.unix_seconds,
        u8::from(sample.clock_initialized), sample.clock_result, sample.clock_query_ticks,
        sample.poll, sample.poll_gap_ticks / ticks_per_ms,
        sample.link.1 & 0xffff_ffff, sample.link.0 & 0xff,
        sample.link.0 >> 32 & 0xff, sample.link.1 >> 32,
        u8::from(sample.active), kind, sample.slot, sample.frame, frame_age,
        sample.event_drops, sample.frame_drops, sample.evidence_losses, sample.heap_allocations)
}

/// `EV_RECORDING` payload: a saved file (`bytes > 0`) or a failure reason.
pub fn pack_recording(
    slot: u8,
    entry: u32,
    bytes: u32,
    records: u16,
    committed: bool,
    game_outer: bool,
    reason: u8,
) -> (u64, u64) {
    (
        u64::from(bytes)
            | u64::from(records) << 32
            | u64::from(slot) << 48
            | u64::from(committed) << 56
            | u64::from(game_outer) << 57,
        u64::from(entry) | u64::from(reason) << 32,
    )
}

/// Pack helpers shared by the hooks (kept here so they are tested).
pub fn pack_sync_end(kind: u32, desync_seen: bool, ending: bool, local: u8, frame: u16) -> u64 {
    u64::from(kind & 0xff)
        | u64::from(desync_seen) << 8
        | u64::from(ending) << 9
        | u64::from(local) << 16
        | u64::from(frame) << 32
}

pub fn pack_desync(ours: u32, theirs: u32, peer: u8, frame: u16) -> (u64, u64) {
    (
        u64::from(ours) | u64::from(peer) << 32,
        u64::from(theirs) | u64::from(frame) << 32,
    )
}

/// The referee's answer for one desync.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Verdict {
    /// The vanilla run matched this console's value at the desync frame.
    OtherConsoleDiverged,
    /// The vanilla run matched the other console's value.
    ThisConsoleDiverged,
    /// The vanilla run matched neither value.
    BothDiverged,
    /// The vanilla run did not line up with this console's frames, or it
    /// ended before the desync frame without disagreeing with them.
    Inconclusive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RefereeReport {
    pub verdict: Verdict,
    /// Vanilla frame index = console frame + offset.
    pub offset: i64,
    /// Console frames whose check value the vanilla run reproduced.
    pub matched: usize,
    /// First console frame whose check value differs from vanilla.
    pub first_divergence: Option<u32>,
    pub vanilla_at_desync: Option<u32>,
    /// First playback frame whose recomputed positions differ from the recording.
    pub first_position_mismatch: Option<u32>,
}

/// Line this console's online frames up with a vanilla playback of its own
/// recording and decide the desync at `frame` (`ours` vs `theirs`). `vanilla`
/// holds playback records sorted by frame index.
pub fn referee(
    console: &[FrameRecord],
    vanilla: &[FrameRecord],
    frame: u32,
    ours: u32,
    theirs: u32,
) -> RefereeReport {
    let first_position_mismatch = vanilla
        .iter()
        .find(|r| r.kind() == KIND_PLAYBACK && r.position != r.recorded)
        .map(|r| r.frame);
    let vanilla_rng = |index: i64| {
        let at = vanilla.partition_point(|r| i64::from(r.frame) < index);
        vanilla
            .get(at)
            .filter(|r| i64::from(r.frame) == index && r.kind() == KIND_PLAYBACK)
            .map(|r| r.rng)
    };
    // The offset with the most agreeing frames before the desync wins; up to
    // 512 evenly spaced frames are enough to find it.
    let before = console
        .iter()
        .filter(|r| r.kind() == KIND_ONLINE && r.frame < frame);
    let step = (before.clone().count() / 512).max(1);
    let mut best = (0usize, 0i64);
    for offset in -900..=900i64 {
        let matched = before
            .clone()
            .step_by(step)
            .filter(|r| vanilla_rng(i64::from(r.frame) + offset) == Some(r.rng))
            .count();
        if matched > best.0 {
            best = (matched, offset);
        }
    }
    let offset = best.1;
    let matched = before
        .filter(|r| vanilla_rng(i64::from(r.frame) + offset) == Some(r.rng))
        .count();
    if matched < 30 {
        return RefereeReport {
            verdict: Verdict::Inconclusive,
            offset,
            matched,
            first_divergence: None,
            vanilla_at_desync: None,
            first_position_mismatch,
        };
    }
    // Frames after the desync are the forced end, not evidence.
    let first_divergence = console
        .iter()
        .filter(|r| r.kind() == KIND_ONLINE && r.frame <= frame)
        .find(|r| vanilla_rng(i64::from(r.frame) + offset).is_some_and(|v| v != r.rng))
        .map(|r| r.frame);
    let vanilla_at_desync = vanilla_rng(i64::from(frame) + offset);
    // The game stops a playback at the first frame whose recorded positions it
    // cannot reproduce, so a recording that left vanilla never reaches the
    // desync frame; leaving vanilla first is then the answer.
    let left_vanilla_first = first_divergence.is_some_and(|f| f < frame)
        || first_position_mismatch.is_some_and(|p| i64::from(p) - offset < i64::from(frame));
    let verdict = match vanilla_at_desync {
        Some(v) if v == ours => Verdict::OtherConsoleDiverged,
        Some(v) if v == theirs => Verdict::ThisConsoleDiverged,
        Some(_) => Verdict::BothDiverged,
        None if left_vanilla_first => Verdict::ThisConsoleDiverged,
        None => Verdict::Inconclusive,
    };
    RefereeReport {
        verdict,
        offset,
        matched,
        first_divergence,
        vanilla_at_desync,
        first_position_mismatch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::fmt::Write as _;

    #[test]
    fn diag_forces_one_frames_check_value_and_nothing_else() {
        let diag = parse_diag(b"# Eden only\r\nforce_desync_frame = 600\r\nother=1\n");
        assert_eq!(diag.force_desync_frame, Some(600));
        assert_eq!(diag_check_value(0x7774_d5d0, 600, &diag), 0x7774_d5d1);
        assert_eq!(diag_check_value(0x7774_d5d0, 599, &diag), 0x7774_d5d0);
        assert_eq!(diag_check_value(0x7774_d5d0, 601, &diag), 0x7774_d5d0);
        for text in [
            &b""[..],
            b"force_desync_frame=",
            b"force_desync_frame=-1",
            b"frame=600",
        ] {
            let none = parse_diag(text);
            assert_eq!(none, Diag::default());
            assert_eq!(diag_check_value(5, 600, &none), 5);
        }
    }

    #[test]
    fn guards_start_with_the_hooked_instructions() {
        // stp w11,w8,[x9] / ldrb w8,[x0,#0x5f] / sub sp,sp,#0xb0 /
        // ldrb w1,[x19,#1] / ldr w9,[x22,#4] / orr w8,wzr,#0x1e0000
        let heads = [
            [0x2b, 0x21, 0x00, 0x29],
            [0x08, 0x7c, 0x41, 0x39],
            [0xff, 0xc3, 0x02, 0xd1],
            [0x61, 0x06, 0x40, 0x39],
            [0xc9, 0x06, 0x40, 0xb9],
            [0xe8, 0x0f, 0x0f, 0x32],
        ];
        for (site, head) in SITES.iter().zip(heads) {
            assert_eq!(site.guard[..4], head, "{}", site.name);
        }
    }

    #[test]
    fn fault_line_names_the_code_and_every_word() {
        assert!(!fault_code_is_logged(0));
        assert!(fault_code_is_logged(2));
        let view = FaultView {
            code: 2,
            frame: Some(8414),
            mask: Some(0x3e6),
            words: Some([0, 0, 1, 0, 0, 0, 0, 0, 0, 0]),
        };
        assert_eq!(unpack_fault(pack_fault(&view)).code, 2);
        assert_eq!(unpack_fault(pack_fault(&view)).frame, Some(8414));
        assert_eq!(unpack_fault(pack_fault(&view)).mask, Some(0x3e6));
        let mut line = Line::<192>::new();
        describe_fault(5, &view, &mut line).unwrap();
        assert_eq!(
            core::str::from_utf8(line.as_bytes()).unwrap(),
            "5 fault code=2 frame=8414 mask=0x3e6 words=0,0,1,0,0,0,0,0,0,0"
        );
        let unread = FaultView {
            code: 2,
            frame: None,
            mask: None,
            words: None,
        };
        line.clear();
        describe_fault(5, &unread, &mut line).unwrap();
        assert_eq!(
            core::str::from_utf8(line.as_bytes()).unwrap(),
            "5 fault code=2 frame=none mask=none words=unread"
        );
        let longest = FaultView {
            code: 255,
            frame: Some(u32::MAX),
            mask: Some(0x3ff),
            words: Some([u32::MAX; FAULT_WORD_COUNT]),
        };
        line.clear();
        describe_fault(u64::MAX, &longest, &mut line).unwrap();
        assert!(line.as_bytes().len() <= 192);
        assert!(core::str::from_utf8(line.as_bytes())
            .unwrap()
            .contains("code=255"));
    }

    #[test]
    fn check_value_and_triggers() {
        assert_eq!(rng_check([0x0f0f, 0xff00, 0x1]), 0x0f0f ^ 0xff00 ^ 1);
        assert_eq!(rng_check([]), 0);
        assert_eq!(sync_end_trigger(0x16d6b08), "desync");
        assert_eq!(sync_end_trigger(0x16cfc18), "peer_silent");
        assert_eq!(sync_end_trigger(0x1569718), "normal");
        assert_eq!(sync_end_trigger(7), "other");
    }

    #[test]
    fn records_round_trip() {
        let record = FrameRecord {
            frame: 1131,
            rng: 0x3f64634c,
            position: 1,
            recorded: 2,
            flags: KIND_ONLINE | 1 << 8,
        };
        assert_eq!(FrameRecord::from_bytes(&record.to_bytes()), Some(record));
        assert_eq!(FrameRecord::from_bytes(&[0; 19]), None);
    }

    #[test]
    fn lines_are_fixed_and_events_read_back() {
        let mut line = Line::<64>::new();
        let (a, b) = pack_desync(0x3f64634c, 0xdbfd9659, 1, 1131);
        let event = RawEvent {
            tick: 5,
            code: EV_DESYNC,
            a,
            b,
        };
        describe(&event, &mut line).unwrap();
        assert_eq!(
            core::str::from_utf8(line.as_bytes()).unwrap(),
            "5 desync frame=1131 ours=0x3f64634c theirs=0xdbfd9659 peer=1"
        );
        let mut short = Line::<8>::new();
        write!(short, "0123456789").unwrap();
        assert_eq!(short.as_bytes(), b"01234567");
        let mut end = Line::<160>::new();
        let event = RawEvent {
            tick: 9,
            code: EV_SYNC_END,
            a: pack_sync_end(2, true, false, 0, 1132),
            b: 0x16d6b08,
        };
        describe(&event, &mut end).unwrap();
        assert!(core::str::from_utf8(end.as_bytes())
            .unwrap()
            .starts_with("9 sync_end trigger=desync kind=2 frame=1132 local=0 desync_seen=1"));
    }

    #[test]
    fn telemetry_retains_clock_status_network_values_and_capture_losses() {
        let sample = Telemetry {
            tick: 192_000_000,
            boot_tick: 42,
            unix_seconds: 1_793_721_600,
            clock_initialized: true,
            clock_result: 0x1234,
            clock_query_ticks: 96,
            poll: 20,
            poll_gap_ticks: 9_600_000,
            link: (2 | 77 << 32, 0x3456 | 3 << 32),
            active: true,
            kind: KIND_ONLINE,
            slot: 2,
            frame: 601,
            last_frame_tick: 182_400_000,
            event_drops: 4,
            frame_drops: 17,
            evidence_losses: 1,
            heap_allocations: 0,
        };
        let mut line = Line::<768>::new();
        describe_telemetry(&sample, &mut line).unwrap();
        let text = core::str::from_utf8(line.as_bytes()).unwrap();
        for field in [
            "clock_initialized=1 clock_result=0x1234 clock_query_ticks=96",
            "poll=20 poll_gap_ms=500",
            "result=0x3456 interface=2 strength=77 state=3",
            "active=1 kind=online slot=2 frame=601 frame_age_ms=500",
            "event_drops=4 frame_drops=17 evidence_losses=1 heap_allocations=0",
        ] {
            assert!(text.contains(field), "missing {field}");
        }
    }

    #[test]
    fn maximum_telemetry_line_fits_without_silently_losing_its_tail() {
        let sample = Telemetry {
            tick: u64::MAX,
            boot_tick: u64::MAX,
            unix_seconds: u64::MAX,
            clock_initialized: false,
            clock_result: u32::MAX,
            clock_query_ticks: u64::MAX,
            poll: u64::MAX,
            poll_gap_ticks: u64::MAX,
            link: (u64::MAX, u64::MAX),
            active: true,
            kind: KIND_PLAYBACK,
            slot: usize::MAX,
            frame: u32::MAX,
            last_frame_tick: 0,
            event_drops: u64::MAX,
            frame_drops: u64::MAX,
            evidence_losses: u64::MAX,
            heap_allocations: u64::MAX,
        };
        let mut full = String::new();
        describe_telemetry(&sample, &mut full).unwrap();
        full.push('\n');
        let mut bounded = Line::<768>::new();
        describe_telemetry(&sample, &mut bounded).unwrap();
        bounded.write_char('\n').unwrap();
        assert_eq!(bounded.as_bytes(), full.as_bytes());
        assert!(full.contains("frame_age_ms=18446744073709551615"));
    }

    #[test]
    fn frame_gap_preserves_session_context_without_claiming_an_end() {
        let mut line = Line::<192>::new();
        describe(
            &RawEvent {
                tick: 9,
                code: EV_FRAME_GAP,
                a: 601 | u64::from(KIND_PLAYBACK) << 32 | 3 << 40,
                b: 7,
            },
            &mut line,
        )
        .unwrap();
        assert_eq!(
            line.as_bytes(),
            b"9 frame_gap kind=playback slot=3 frame=601 last_frame_tick=7"
        );
    }

    fn online(frame: u32, rng: u32) -> FrameRecord {
        FrameRecord {
            frame,
            rng,
            flags: KIND_ONLINE,
            ..Default::default()
        }
    }

    fn playback(frame: u32, rng: u32, position: u32, recorded: u32) -> FrameRecord {
        FrameRecord {
            frame,
            rng,
            position,
            recorded,
            flags: KIND_PLAYBACK,
        }
    }

    #[test]
    fn the_referee_names_the_console_that_left_vanilla() {
        // Vanilla playback indices run 40 behind the online frame numbers.
        let vanilla: Vec<_> = (0..300).map(|i| playback(i, i * 7 + 1, i, i)).collect();
        let ours = |frame: u32| (frame - 40) * 7 + 1;
        // A clean console: its values match vanilla; the peer's differ at 200.
        let clean: Vec<_> = (40..240).map(|f| online(f, ours(f))).collect();
        let report = referee(&clean, &vanilla, 200, ours(200), 0xdead);
        assert_eq!(report.verdict, Verdict::OtherConsoleDiverged);
        assert_eq!(report.offset, -40);
        assert_eq!(report.first_divergence, None);
        // A console that diverged at 150: from there its values differ.
        let bad: Vec<_> = (40..240)
            .map(|f| online(f, if f < 150 { ours(f) } else { 0xbad0 + f }))
            .collect();
        let report = referee(&bad, &vanilla, 200, 0xbad0 + 200, ours(200));
        assert_eq!(report.verdict, Verdict::ThisConsoleDiverged);
        assert_eq!(report.first_divergence, Some(150));
        // Neither value is vanilla.
        assert_eq!(
            referee(&bad, &vanilla, 200, 1, 2).verdict,
            Verdict::BothDiverged
        );
        // Nothing lines up.
        let unrelated: Vec<_> = (40..240).map(|f| online(f, 0x5555_0000 + f)).collect();
        assert_eq!(
            referee(&unrelated, &vanilla, 200, 1, 2).verdict,
            Verdict::Inconclusive
        );
        // The playback's own position check is reported too.
        let mut moved = vanilla.clone();
        moved[120].position ^= 1;
        assert_eq!(
            referee(&clean, &moved, 200, ours(200), 0).first_position_mismatch,
            Some(120)
        );
        // The game stopped the playback at this console's first unreproducible
        // frame (index 110 = console frame 150), before the desync at 200.
        let stopped: Vec<_> = (0..=110)
            .map(|i| playback(i, i * 7 + 1, i, if i == 110 { i ^ 1 } else { i }))
            .collect();
        let report = referee(&bad, &stopped, 200, 0xbad0 + 200, ours(200));
        assert_eq!(report.vanilla_at_desync, None);
        assert_eq!(report.first_position_mismatch, Some(110));
        assert_eq!(report.verdict, Verdict::ThisConsoleDiverged);
        // A playback that simply ends early, matching throughout, decides nothing.
        let short: Vec<_> = (0..=110).map(|i| playback(i, i * 7 + 1, i, i)).collect();
        assert_eq!(
            referee(&clean, &short, 200, ours(200), 1).verdict,
            Verdict::Inconclusive
        );
    }

    #[test]
    fn only_abnormal_ends_keep_the_recording() {
        // The five 2026-10-29 console matches: kind 1, then the peer's reason 1.
        let mut normal = EndState::default();
        normal.sync_end(1, false);
        normal.peer_end(1);
        assert!(!normal.keep_recording());
        // The Eden desync: mismatch rows, then a kind 2 end with desync_seen.
        let mut desync = EndState::default();
        desync.desync();
        desync.sync_end(2, true);
        assert!(desync.keep_recording());
        // Kind 1 with the desync flag, an unknown kind, or an abnormal peer reason.
        for (kind, flag, reason) in [(1, true, 1), (0, false, 1), (3, false, 1), (1, false, 7)] {
            let mut end = EndState::default();
            end.sync_end(kind, flag);
            end.peer_end(reason);
            assert!(
                end.keep_recording(),
                "kind={kind} flag={flag} reason={reason}"
            );
        }
        // No end seen at all (the session just stopped): kept.
        assert!(EndState::default().keep_recording());
        let mut line = Line::<96>::new();
        let (a, b) = pack_recording(2, 0, 0, 0, false, false, RECORDING_NORMAL_END);
        describe(
            &RawEvent {
                tick: 4,
                code: EV_RECORDING,
                a,
                b,
            },
            &mut line,
        )
        .unwrap();
        assert_eq!(
            core::str::from_utf8(line.as_bytes()).unwrap(),
            "4 recording slot=2 entry=0 not_kept reason=normal_end"
        );
    }

    #[test]
    fn paths_end_in_nul() {
        let mut path = Line::<8>::new();
        write!(path, "sd:/a").unwrap();
        assert_eq!(path.with_nul(), Some(&b"sd:/a\0"[..]));
        write!(path, "bcdef").unwrap();
        assert_eq!(path.with_nul(), None);
    }

    fn entry_with(payload: &[u8]) -> Vec<u8> {
        let mut entry = vec![0u8; CFMK_BYTES];
        entry[4..8].copy_from_slice(&TAG_CFMK.to_le_bytes());
        entry[8..12].copy_from_slice(&REPLAY_VERSION.to_le_bytes());
        entry[0xc..0x10].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        entry[0x10..0x12].copy_from_slice(&(CFMK_BYTES as u16).to_le_bytes());
        entry[0x12..0x14].copy_from_slice(&1u16.to_le_bytes());
        entry[0x14..0x16].copy_from_slice(&3u16.to_le_bytes());
        entry.extend_from_slice(&TAG_FRAM.to_le_bytes());
        entry.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        entry.extend_from_slice(payload);
        entry
    }

    #[test]
    fn rebuilt_entries_pass_the_game_validator() {
        let payload: Vec<u8> = (0..96u8).collect();
        let mut entry = entry_with(&payload);
        assert_eq!(validate_entry(&entry), Err("xor word"));
        // Word 0 is resealed piece by piece, as the runtime writes the file.
        let word = xor_words(&entry[4..CFMK_BYTES])
            ^ xor_words(&entry[CFMK_BYTES..CFMK_BYTES + 8])
            ^ xor_words(&payload);
        entry[..4].copy_from_slice(&word.to_le_bytes());
        assert_eq!(validate_entry(&entry), Ok(()));
        entry[CFMK_BYTES + 20] ^= 1;
        assert_eq!(validate_entry(&entry), Err("xor word"));
        assert_eq!(validate_entry(&entry[..CFMK_BYTES]), Err("entry too short"));
    }

    #[test]
    fn outer_headers_carry_the_time_and_version() {
        // The sample replay's stamp: 2026-10-12 12:16:26.
        let time = pack_time(2026, 10, 12, 12, 16, 26);
        assert_eq!(time, 0x8001_001a_418c_a7ea);
        let outer = minimal_outer(time);
        assert_eq!(outer[..8], time.to_le_bytes());
        assert_eq!(le32(&outer, 8), REPLAY_VERSION);
        assert!(outer[12..].iter().all(|b| *b == 0));
    }

    #[test]
    fn recording_lines_read_back() {
        let mut line = Line::<160>::new();
        let (a, b) = pack_recording(3, 7, 40_000, 3676, false, false, 0);
        describe(
            &RawEvent {
                tick: 1,
                code: EV_RECORDING,
                a,
                b,
            },
            &mut line,
        )
        .unwrap();
        assert_eq!(
            core::str::from_utf8(line.as_bytes()).unwrap(),
            "1 recording slot=3 entry=7 bytes=40000 records=3676 source=staged outer=minimal"
        );
        line.clear();
        let (a, b) = pack_recording(4, 2, 0, 0, false, false, 3);
        describe(
            &RawEvent {
                tick: 2,
                code: EV_RECORDING,
                a,
                b,
            },
            &mut line,
        )
        .unwrap();
        assert_eq!(
            core::str::from_utf8(line.as_bytes()).unwrap(),
            "2 recording slot=4 entry=2 unavailable reason=overflow"
        );
        line.clear();
        let event = RawEvent {
            tick: 3,
            code: EV_SESSION,
            a: 5,
            b: u64::from(KIND_PLAYBACK),
        };
        describe(&event, &mut line).unwrap();
        assert_eq!(
            core::str::from_utf8(line.as_bytes()).unwrap(),
            "3 session slot=5 kind=playback heap_allocations=0"
        );
    }
}
