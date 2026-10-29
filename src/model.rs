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

pub const SITES: [&Site; 5] = [
    &FRAME_CHECK,
    &DESYNC,
    &SYNC_END,
    &PEER_ABORT,
    &PLAYBACK_CHECK,
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
    /// Outer-header map node: key at `+0x20`, object at `+0x28`, header at
    /// object `+0x18`.
    pub const NODE_KEY: usize = 0x20;
    pub const NODE_OBJECT: usize = 0x28;
    pub const OBJECT_OUTER: usize = 0x18;
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

/// Why a match recording could not be saved (`EV_RECORDING` with no bytes).
pub const RECORDING_REASONS: [&str; 8] = [
    "none",
    "no_recorder",
    "no_entry",
    "overflow",
    "invalid_entry",
    "not_cfmk",
    "length_mismatch",
    "write_failed",
];

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
    let first_divergence = console
        .iter()
        .filter(|r| r.kind() == KIND_ONLINE)
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
    fn guards_start_with_the_hooked_instructions() {
        // stp w11,w8,[x9] / ldrb w8,[x0,#0x5f] / sub sp,sp,#0xb0 /
        // ldrb w1,[x19,#1] / ldr w9,[x22,#4]
        let heads = [
            [0x2b, 0x21, 0x00, 0x29],
            [0x08, 0x7c, 0x41, 0x39],
            [0xff, 0xc3, 0x02, 0xd1],
            [0x61, 0x06, 0x40, 0x39],
            [0xc9, 0x06, 0x40, 0xb9],
        ];
        for (site, head) in SITES.iter().zip(heads) {
            assert_eq!(site.guard[..4], head, "{}", site.name);
        }
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
