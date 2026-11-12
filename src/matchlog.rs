//! 0.5.0 match telemetry: read-only lines at an online match's start, at each KO the fault table counts, on both of
//! the lockstep's peer-end paths, when the session ends (phase 6) and when it stops, with a one-line summary. The summary carries `ssbu-stall-press-carry`'s counters
//! when that plugin is loaded (its export `ssbu_stall_press_carry_stats_v1`).
//!
//! Host-testable half: sites, addresses, payload packing, the end classification and every line's text. The hooks
//! are in `runtime.rs`.
use crate::model::Site;
use core::fmt;

/// `str w9, [x19, #0x218]` in match setup (`.14d9050`): the fault table's limits being copied. x19 = the fault object,
/// x8 = the chosen limit block (`param + 0x38` for one watched entry, `+ 0x40` for more), w9 = its seconds.
pub const MATCH_LIMITS_OFFSET: usize = 0x14d96a0;
pub const MATCH_LIMITS: Site = Site {
    name: "match_limits",
    offset: MATCH_LIMITS_OFFSET,
    guard: [
        0x69, 0x1a, 0x02, 0xb9, 0x09, 0x11, 0x40, 0x39, 0x69, 0x1e, 0x02, 0xb9, 0x08, 0x15, 0x40,
        0x39, 0x68, 0x22, 0x02, 0xb9, 0x48, 0xef, 0x01, 0xd0, 0x08, 0xc9, 0x43, 0xf9, 0xe8, 0x38,
        0x00, 0xb4, 0x13, 0x01, 0x40, 0xf9, 0x68, 0x12, 0x40, 0xf9, 0x09, 0x01, 0x40, 0xb9, 0xe8,
        0x5f, 0x00, 0x32, 0x3f, 0x01, 0x08, 0x6b, 0x20, 0x38, 0x00, 0x54, 0xd4, 0xf2, 0x01, 0xb0,
        0x88, 0x92, 0x47, 0xf9,
    ],
};

/// `.1363420`'s entry (`sub sp, sp, #0x70`): the battle rule object's KO method (vtable slot +0x60).
/// w1 = the KO'd entry, w2 = the KO kind (kind 8 is not counted by the fault table).
pub const KO_ENTRY_OFFSET: usize = 0x1363420;
pub const KO_ENTRY: Site = Site {
    name: "ko_entry",
    offset: KO_ENTRY_OFFSET,
    guard: [
        0xff, 0xc3, 0x01, 0xd1, 0xfb, 0x0b, 0x00, 0xf9, 0xfa, 0x67, 0x02, 0xa9, 0xf8, 0x5f, 0x03,
        0xa9, 0xf6, 0x57, 0x04, 0xa9, 0xf4, 0x4f, 0x05, 0xa9, 0xfd, 0x7b, 0x06, 0xa9, 0xfd, 0x83,
        0x01, 0x91, 0x08, 0x00, 0x40, 0xf9, 0x08, 0xd5, 0x41, 0xf9, 0xf4, 0x03, 0x02, 0x2a, 0xf3,
        0x03, 0x01, 0xaa, 0xf5, 0x03, 0x00, 0xaa, 0x00, 0x01, 0x3f, 0xd6, 0xe0, 0x29, 0x00, 0x36,
        0xa8, 0xe6, 0x41, 0x39,
    ],
};

/// `.1363420`'s shared epilogue (`ldr x27, [sp, #0x10]` and three `ldp`): every return passes here, after the
/// fault table's update.
pub const KO_EXIT_OFFSET: usize = 0x1363998;
pub const KO_EXIT: Site = Site {
    name: "ko_exit",
    offset: KO_EXIT_OFFSET,
    guard: [
        0xfb, 0x0b, 0x40, 0xf9, 0xf4, 0x4f, 0x45, 0xa9, 0xf6, 0x57, 0x44, 0xa9, 0xf8, 0x5f, 0x43,
        0xa9, 0xfa, 0x67, 0x42, 0xa9, 0xff, 0xc3, 0x01, 0x91, 0xc0, 0x03, 0x5f, 0xd6, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc0, 0x03, 0x5f, 0xd6, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0x03, 0x02, 0xd1,
        0xf9, 0x1b, 0x00, 0xf9,
    ],
};

/// Peer-end path A: the receive handler `.16cc140`'s shared tail into `b .16cced0` at `.16cc4d4`
/// (`ldp x29, x30, [sp, #0x50]`, before x19/x20 are restored). Two jump-table cases reach it: case 7, the other
/// console's end message (w1 = 2, through `b .16cc4bc` at `.16cc49c`), and case 8, a message naming this station
/// (w1 = 1, after `bl .16cbd30`). x0 = x19 = session, x20 = the message, w1 = the reason `.16cced0` gets. The span
/// `.16cc4c0..16cc4cf` is four loads and no branch, call, or jump-table target lands inside it (`.16cc4bc` is the
/// nearest target, `xref.py` and the table at `0x44f37d4`).
pub const PEER_PATH_A_OFFSET: usize = 0x16cc4c0;
pub const PEER_PATH_A: Site = Site {
    name: "peer_path_a",
    offset: PEER_PATH_A_OFFSET,
    guard: [
        0xfd, 0x7b, 0x45, 0xa9, 0xf4, 0x4f, 0x44, 0xa9, 0xf7, 0x13, 0x40, 0xf9, 0xf6, 0x57, 0x43,
        0xa9, 0xff, 0x83, 0x01, 0x91, 0x7f, 0x02, 0x00, 0x14, 0x68, 0x8a, 0x41, 0x39, 0x1f, 0x0d,
        0x00, 0x71, 0xc8, 0x21, 0x00, 0x54, 0x89, 0x06, 0x40, 0x39, 0x6a, 0x12, 0x41, 0x39, 0x3f,
        0x01, 0x0a, 0x6b, 0x88, 0x0a, 0x40, 0x39, 0xc1, 0x00, 0x00, 0x54, 0x6a, 0xc2, 0x40, 0x79,
        0xe9, 0x03, 0x00, 0x32,
    ],
};

/// Peer-end path B: the abort function `.16ccad0` past its "already ending" check (`ldrh w1, [x0, #0x5a]`);
/// it ends in `b .16cced0` at `.16ccc94`. x0 = session, x30 = its caller.
pub const PEER_PATH_B_OFFSET: usize = 0x16ccae8;
pub const PEER_PATH_B: Site = Site {
    name: "peer_path_b",
    offset: PEER_PATH_B_OFFSET,
    guard: [
        0x01, 0xb4, 0x40, 0x79, 0xe8, 0x03, 0x00, 0x32, 0xf3, 0x03, 0x00, 0xaa, 0x08, 0xf4, 0x00,
        0x39, 0x1f, 0xa0, 0x04, 0xb8, 0x1f, 0x9c, 0x00, 0x79, 0x1f, 0x40, 0x01, 0x79, 0x67, 0x00,
        0x00, 0x94, 0x68, 0xec, 0x92, 0x52, 0x69, 0xe7, 0x9a, 0x52, 0xe1, 0x03, 0x00, 0x32, 0x7f,
        0x6a, 0x28, 0x38, 0x68, 0xe2, 0x82, 0x52, 0x28, 0x00, 0xa0, 0x72, 0x7f, 0x6a, 0x29, 0x38,
        0x7f, 0x6a, 0x28, 0x38,
    ],
};

/// `.16cced0`'s entry (`sub sp, sp, #0x80`): the session goes to phase 6. x0 = session, w1 = the reason,
/// x30 = the caller (a tail call leaves the tail caller's caller).
pub const SESSION_END_OFFSET: usize = 0x16cced0;
pub const SESSION_END: Site = Site {
    name: "session_end",
    offset: SESSION_END_OFFSET,
    guard: [
        0xff, 0x03, 0x02, 0xd1, 0xf9, 0x1b, 0x00, 0xf9, 0xf8, 0x5f, 0x04, 0xa9, 0xf6, 0x57, 0x05,
        0xa9, 0xf4, 0x4f, 0x06, 0xa9, 0xfd, 0x7b, 0x07, 0xa9, 0xfd, 0xc3, 0x01, 0x91, 0x08, 0xc5,
        0x92, 0x52, 0x15, 0x00, 0x08, 0x8b, 0xa8, 0x02, 0x40, 0xf9, 0xf4, 0x03, 0x01, 0x2a, 0xf3,
        0x03, 0x00, 0xaa, 0x68, 0x03, 0x00, 0xb4, 0x08, 0xc4, 0x92, 0x52, 0x76, 0x02, 0x08, 0x8b,
        0xd7, 0xdf, 0x01, 0xb0,
    ],
};

/// `.16d5cd0`'s entry: the session stops (the network battle's teardown). x0 = session.
pub const SESSION_STOP_OFFSET: usize = 0x16d5cd0;
pub const SESSION_STOP: Site = Site {
    name: "session_stop",
    offset: SESSION_STOP_OFFSET,
    guard: [
        0xff, 0x83, 0x01, 0xd1, 0xf5, 0x1b, 0x00, 0xf9, 0xf4, 0x4f, 0x04, 0xa9, 0xfd, 0x7b, 0x05,
        0xa9, 0xfd, 0x43, 0x01, 0x91, 0x68, 0xdf, 0x01, 0xf0, 0x08, 0x2d, 0x46, 0xf9, 0x15, 0x05,
        0x40, 0xf9, 0xf3, 0x03, 0x00, 0xaa, 0x01, 0x05, 0x80, 0x52, 0xe0, 0x03, 0x1c, 0x32, 0x65,
        0x61, 0x89, 0x94, 0xf4, 0x03, 0x00, 0xaa, 0x60, 0x02, 0x00, 0xb5, 0xe8, 0xe2, 0x01, 0xb0,
        0x00, 0x81, 0x47, 0xf9,
    ],
};

/// Every 0.5.0 site; all of them must match for the telemetry hooks to go in.
pub const TELEMETRY_SITES: [&Site; 7] =
    [&MATCH_LIMITS, &KO_ENTRY, &KO_EXIT, &PEER_PATH_A, &PEER_PATH_B, &SESSION_END, &SESSION_STOP];

/// The match setup block (`main+0x52c51b0`, 0x410 bytes): the melee byte at +0. Its first 64 bytes are logged raw at
/// match start (the stock, time and team fields are not named yet).
pub const SETUP_BLOCK: usize = 0x52c51b0;
pub const RULE_BYTES: usize = 64;
/// The game mode word.
pub const MODE_WORD: usize = 0x53050f0;
/// `*(main+0x52c4d08) + 8` is the lockstep session; its current frame is the u16 at +2.
pub const LOCKSTEP_HOLDER: usize = 0x52c4d08;
pub const SESSION_FRAME_AT: u64 = 0x2;
/// Fault-object fields (`[[main+0x52c3940]]`).
pub const FAULT_WATCHED_AT: u64 = 0xc;
pub const FAULT_WATCHED_COUNT_AT: u64 = 0x10;
pub const FAULT_WORD2_AT: u64 = 0x20;
pub const FAULT_STAMPS_BEGIN_AT: u64 = 0x200;
pub const FAULT_STAMPS_END_AT: u64 = 0x208;
pub const FAULT_PARAM_AT: u64 = 0x228;
/// Session bytes: the fault code `.16acd30` returned, the phase, the "already ending" flag, the station.
pub const SESSION_FAULT_CODE_AT: u64 = 0x5e;
pub const SESSION_PHASE_AT: u64 = 0x62;
pub const SESSION_ENDING_AT: u64 = 0x63;
pub const SESSION_STATION_AT: u64 = 0x1;
pub const SESSION_PEER_MASK_AT: u64 = 0x5a;
/// `.16cced0`'s callers that are not tail calls: the countdown's end (`.16d7730`) and a failed session start.
pub const CALLER_COUNTDOWN: u64 = 0x16d8d3c;
pub const CALLER_START: u64 = 0x16d9328;
/// The network-layer result `.16cced0` hands the transport (`.167ebf0(.., 0xfffffc1f, 2)`) when the session's fault
/// code is nonzero.
pub const FAULT_NET_RESULT: i32 = -993;
/// A peer-end path seen this recently before `.16cced0` is the path that ended the session.
pub const PEER_PATH_WINDOW_TICKS: u64 = 19_200_000;

pub const EV_MATCH_START: u8 = 12;
pub const EV_KO: u8 = 13;
pub const EV_PEER_PATH: u8 = 14;
pub const EV_SESSION_END: u8 = 15;
pub const EV_SUMMARY: u8 = 16;
/// 0.5.1: an entry's own dead count rose (`stock_lost`).
pub const EV_STOCK_LOST: u8 = 17;

/// 0.5.1: the game's own per-fighter counters, as its exported accessors read them
/// (`app::lua_bind::FighterManager__get_fighter_information_impl` `.2140fc0`, `FighterInformation__dead_count_impl`
/// `.20c9e50`, `__suicide_count_impl` `.20c9e90`, `__stock_count_impl` `.20c9e80`).
/// `*[main+FIGHTER_MANAGER]` is the `FighterManager`; `[[it] + ENTRY_AT + 8e]` is entry `e`'s `FighterEntry`; its
/// `FighterInformation` is at `+0xf0`, whose data pointer is at `+0xf8` (`[info + 8]`).
pub const FIGHTER_MANAGER: usize = 0x52b94f8;
pub const ENTRY_AT: u64 = 0x20;
pub const INFO_DATA_AT: u64 = 0xf8;
pub const ENTRIES: usize = 8;
/// `FighterManager__entry_count_impl` (`.2140d60`): `u32 [[manager] + 0xa0]`, the live entry count.
pub const ENTRY_COUNT_AT: u64 = 0xa0;
/// `FighterManager__is_result_mode_impl` (`.2141620`): `u8 [[manager] + 0xe9]`, nonzero on the results.
pub const RESULT_MODE_AT: u64 = 0xe9;
/// Three `u32` dead counts (`dead_count(info, -2)` sums them); what each slot means is not known.
pub const DEAD_AT: u64 = 0xcc;
/// Two arrays of three `u32` self-destruct counts (`suicide_count(info, -2)` sums all six).
pub const SUICIDE_A_AT: u64 = 0x14c;
pub const SUICIDE_B_AT: u64 = 0x158;
pub const STOCK_AT: u64 = 0xd8;

/// One entry's counters at one poll.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counters {
    pub dead: [u32; 3],
    pub suicides: u32,
    pub stock: u32,
}

impl Counters {
    pub fn dead_total(&self) -> u32 {
        self.dead.iter().fold(0u32, |a, b| a.saturating_add(*b))
    }
}

/// An entry's baseline: its dead and self-destruct totals at the last poll.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Baseline {
    pub dead: u32,
    pub suicides: u32,
}

impl Baseline {
    /// For an atomic slot: bit 63 set, the dead total in bits 32..63, the self-destructs in the low 32.
    pub const fn pack(self) -> u64 {
        1 << 63 | ((self.dead & 0x7fff_ffff) as u64) << 32 | self.suicides as u64
    }

    pub const fn unpack(word: u64) -> Option<Self> {
        if word >> 63 == 0 {
            None
        } else {
            Some(Self { dead: (word >> 32) as u32 & 0x7fff_ffff, suicides: word as u32 })
        }
    }
}

/// What one poll of one entry found: how many stocks it lost since the last poll, and how many of them were
/// self-destructs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Lost {
    pub stocks: u32,
    pub self_destructs: u32,
}

/// One entry's step: the first poll of a match (no baseline) and any count going down (a new battle) only set the
/// baseline; a rise in the dead total is that many lost stocks, the self-destruct rise (at most as many) of them
/// self-destructs.
pub fn stock_step(base: Option<Baseline>, now: &Counters) -> (Baseline, Option<Lost>) {
    let next = Baseline { dead: now.dead_total(), suicides: now.suicides };
    match base {
        Some(b) if next.dead >= b.dead && next.suicides >= b.suicides => {
            let stocks = next.dead - b.dead;
            let lost = (stocks > 0).then(|| Lost { stocks, self_destructs: (next.suicides - b.suicides).min(stocks) });
            (next, lost)
        }
        _ => (next, None),
    }
}

/// Game memory as the poll reads it (the runtime reads the address; the host tests use a fake).
pub trait Memory {
    fn u64_at(&self, at: u64) -> u64;
    fn u32_at(&self, at: u64) -> u32;
    fn u8_at(&self, at: u64) -> u8;
}

/// A pointer read from game memory that is plausible as an object address.
pub fn plausible(pointer: u64) -> Option<u64> {
    (pointer >= 0x10000 && pointer.is_multiple_of(8)).then_some(pointer)
}

/// The live entries' counters, read as the game's own accessors read them, one straight chain per entry:
/// `[main+FIGHTER_MANAGER]` → `[manager]` → (not on the results: `RESULT_MODE_AT`) → entry `e` below the live
/// count (`ENTRY_COUNT_AT`, at most 8) → its information data → the counters. A null or implausible pointer ends that
/// chain (the game's own `total_fighter_num` skips null slots the same way). Returns how many entries were looked at.
pub fn read_entries(mem: &impl Memory, text: u64, out: &mut [Option<Counters>; ENTRIES]) -> usize {
    *out = [None; ENTRIES];
    let Some(manager) = plausible(mem.u64_at(text + FIGHTER_MANAGER as u64)) else {
        return 0;
    };
    let Some(entries) = plausible(mem.u64_at(manager)) else {
        return 0;
    };
    if mem.u8_at(entries + RESULT_MODE_AT) != 0 {
        return 0;
    }
    let count = (mem.u32_at(entries + ENTRY_COUNT_AT) as usize).min(ENTRIES);
    for (e, slot) in out.iter_mut().enumerate().take(count) {
        let Some(fighter) = plausible(mem.u64_at(entries + ENTRY_AT + 8 * e as u64)) else {
            continue;
        };
        let Some(data) = plausible(mem.u64_at(fighter + INFO_DATA_AT)) else {
            continue;
        };
        let suicides = (0..3u64).fold(0u32, |sum, i| {
            sum.saturating_add(mem.u32_at(data + SUICIDE_A_AT + 4 * i))
                .saturating_add(mem.u32_at(data + SUICIDE_B_AT + 4 * i))
        });
        *slot = Some(Counters {
            dead: [mem.u32_at(data + DEAD_AT), mem.u32_at(data + DEAD_AT + 4), mem.u32_at(data + DEAD_AT + 8)],
            suicides,
            stock: mem.u32_at(data + STOCK_AT),
        });
    }
    count
}

/// One `stock_lost` line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StockLost {
    pub entry: u8,
    pub self_destruct: bool,
    pub frame: Option<u16>,
    pub counters: Counters,
}

pub fn pack_stock_lost(s: &StockLost) -> (u64, u64) {
    let c = &s.counters;
    let a = u64::from(s.entry)
        | u64::from(s.self_destruct) << 8
        | u64::from(s.frame.unwrap_or(0)) << 16
        | u64::from(s.frame.is_some()) << 32
        | u64::from(c.stock.min(0xffff)) << 40;
    let b = u64::from(c.dead[0].min(0xffff))
        | u64::from(c.dead[1].min(0xffff)) << 16
        | u64::from(c.dead[2].min(0xffff)) << 32
        | u64::from(c.suicides.min(0xffff)) << 48;
    (a, b)
}

pub fn unpack_stock_lost(a: u64, b: u64) -> StockLost {
    StockLost {
        entry: a as u8,
        self_destruct: a >> 8 & 1 != 0,
        frame: (a >> 32 & 1 != 0).then_some((a >> 16) as u16),
        counters: Counters {
            dead: [b as u16 as u32, (b >> 16) as u16 as u32, (b >> 32) as u16 as u32],
            suicides: (b >> 48) as u16 as u32,
            stock: (a >> 40) as u16 as u32,
        },
    }
}

pub fn describe_stock_lost(tick: u64, s: &StockLost, out: &mut impl fmt::Write) -> fmt::Result {
    let c = &s.counters;
    write!(
        out,
        "{tick} stock_lost frame={} entry={} self_destruct={} dead={},{},{} suicides={} stock={}",
        Frame(s.frame),
        s.entry,
        if s.self_destruct { "yes" } else { "no" },
        c.dead[0],
        c.dead[1],
        c.dead[2],
        c.suicides,
        c.stock
    )
}

/// Side-record words per event (the event's `a` carries the record's sequence number).
pub const SIDE_WORDS: usize = 12;

/// An online match's start: the fault table's arming and limits, and the setup block's first bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatchStart {
    pub melee: u8,
    pub mode: u32,
    pub mask: u16,
    pub watched: u8,
    pub watched_count: u8,
    /// The limit block's offset in the param struct (0x38 or 0x40), 0 if unknown.
    pub block: u8,
    pub seconds: u32,
    pub n: u8,
    pub l: u8,
    pub rules: [u8; RULE_BYTES],
}

impl MatchStart {
    pub fn words(&self) -> [u64; SIDE_WORDS] {
        let mut w = [0u64; SIDE_WORDS];
        w[0] = u64::from(self.melee)
            | u64::from(self.mode) << 8
            | u64::from(self.mask & 0x3ff) << 40
            | u64::from(self.block) << 52;
        w[1] = u64::from(self.watched)
            | u64::from(self.watched_count) << 8
            | u64::from(self.seconds) << 16
            | u64::from(self.n) << 48
            | u64::from(self.l) << 56;
        for (i, chunk) in self.rules.chunks(8).enumerate() {
            let mut b = [0u8; 8];
            b.copy_from_slice(chunk);
            w[2 + i] = u64::from_le_bytes(b);
        }
        w
    }

    pub fn from_words(w: &[u64; SIDE_WORDS]) -> Self {
        let mut rules = [0u8; RULE_BYTES];
        for (i, chunk) in rules.chunks_mut(8).enumerate() {
            chunk.copy_from_slice(&w[2 + i].to_le_bytes());
        }
        Self {
            melee: w[0] as u8,
            mode: (w[0] >> 8) as u32,
            mask: (w[0] >> 40) as u16 & 0x3ff,
            block: (w[0] >> 52) as u8,
            watched: w[1] as u8,
            watched_count: (w[1] >> 8) as u8,
            seconds: (w[1] >> 16) as u32,
            n: (w[1] >> 48) as u8,
            l: (w[1] >> 56) as u8,
            rules,
        }
    }
}

pub fn describe_match_start(tick: u64, m: &MatchStart, out: &mut impl fmt::Write) -> fmt::Result {
    write!(
        out,
        "{tick} match_start melee=0x{:02x} mode=0x{:08x} fault_mask=0x{:03x} watched=0x{:02x} watched_n={} \
         limits=+0x{:x}:window_s={},n={},l={} rules=",
        m.melee, m.mode, m.mask, m.watched, m.watched_count, m.block, m.seconds, m.n, m.l
    )?;
    for byte in m.rules {
        write!(out, "{byte:02x}")?;
    }
    Ok(())
}

/// One KO the fault table counted: before and after `.1363420` (stamp-list length, fault word 2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ko {
    pub entry: u8,
    pub kind: u8,
    pub frame: Option<u16>,
    pub stamps_before: u16,
    pub stamps_after: u16,
    pub word2_before: u32,
    pub word2_after: u32,
}

pub fn pack_ko(k: &Ko) -> (u64, u64) {
    let a = u64::from(k.entry)
        | u64::from(k.kind) << 8
        | u64::from(k.frame.unwrap_or(0)) << 16
        | u64::from(k.frame.is_some()) << 32
        | u64::from(k.stamps_before & 0x3ff) << 40
        | u64::from(k.stamps_after & 0x3ff) << 50;
    (a, u64::from(k.word2_before) | u64::from(k.word2_after) << 32)
}

pub fn unpack_ko(a: u64, b: u64) -> Ko {
    Ko {
        entry: a as u8,
        kind: (a >> 8) as u8,
        frame: (a >> 32 & 1 != 0).then_some((a >> 16) as u16),
        stamps_before: (a >> 40) as u16 & 0x3ff,
        stamps_after: (a >> 50) as u16 & 0x3ff,
        word2_before: b as u32,
        word2_after: (b >> 32) as u32,
    }
}

struct Frame(Option<u16>);

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(frame) => write!(f, "{frame}"),
            None => f.write_str("none"),
        }
    }
}

/// Which peer-end path ran: A, the receive handler's tail into `.16cced0` from `.16cc4d4` (case 7, the other
/// console's end message, or case 8, a message naming this station); B, the abort function `.16ccad0` (ends in
/// `.16cced0` from `.16ccc94`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PeerPath {
    pub path: u8,
    pub station: u8,
    pub frame: Option<u16>,
    pub peer_mask: u16,
    /// A: the reason handed to `.16cced0` (w1: 2 from case 7, 1 from case 8).
    pub end_reason: u8,
    /// A: the message's first byte (type in the low nibble, a station in the high one) and its big-endian half-word
    /// at +1, when the message pointer was valid.
    pub message: Option<(u8, u16)>,
    /// B: the abort function's caller (offset in `.text`).
    pub caller: u64,
}

pub const PATH_A: u8 = 1;
pub const PATH_B: u8 = 2;

pub fn pack_peer_path(p: &PeerPath) -> (u64, u64) {
    let a = u64::from(p.path)
        | u64::from(p.station) << 8
        | u64::from(p.frame.unwrap_or(0)) << 16
        | u64::from(p.frame.is_some()) << 32
        | u64::from(p.peer_mask) << 40
        | u64::from(p.end_reason) << 56;
    let b = if p.path == PATH_A {
        p.message
            .map(|(byte0, word)| u64::from(byte0) | u64::from(word) << 8 | 1 << 24)
            .unwrap_or(0)
    } else {
        p.caller
    };
    (a, b)
}

pub fn unpack_peer_path(a: u64, b: u64) -> PeerPath {
    let path = a as u8;
    PeerPath {
        path,
        station: (a >> 8) as u8,
        frame: (a >> 32 & 1 != 0).then_some((a >> 16) as u16),
        peer_mask: (a >> 40) as u16,
        end_reason: (a >> 56) as u8,
        message: (path == PATH_A && b >> 24 & 1 != 0).then_some((b as u8, (b >> 8) as u16)),
        caller: if path == PATH_A { 0 } else { b },
    }
}

/// `.16cced0`'s entry: the session moves to phase 6.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SessionEnd {
    pub reason: u8,
    pub fault_code: u8,
    pub phase_before: u8,
    pub already_ending: u8,
    pub frame: Option<u16>,
    pub caller: u64,
}

pub fn pack_session_end(e: &SessionEnd) -> (u64, u64) {
    let a = u64::from(e.reason)
        | u64::from(e.fault_code) << 8
        | u64::from(e.phase_before) << 16
        | u64::from(e.already_ending) << 24
        | u64::from(e.frame.unwrap_or(0)) << 32
        | u64::from(e.frame.is_some()) << 48;
    (a, e.caller)
}

pub fn unpack_session_end(a: u64, b: u64) -> SessionEnd {
    SessionEnd {
        reason: a as u8,
        fault_code: (a >> 8) as u8,
        phase_before: (a >> 16) as u8,
        already_ending: (a >> 24) as u8,
        frame: (a >> 48 & 1 != 0).then_some((a >> 32) as u16),
        caller: b,
    }
}

/// How the session ended, for the summary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum EndPath {
    /// Neither a lockstep end nor phase 6 was seen before the session stopped: the connection was lost or the
    /// battle was torn down without an end.
    #[default]
    NoneSeen = 0,
    SyncEndNormal = 1,
    SyncEndAbnormal = 2,
    /// The fault countdown (`.16acd30` nonzero, 30 passes).
    LocalFault = 3,
    PeerEndA = 4,
    PeerEndB = 5,
    StartFailure = 6,
    Other = 7,
}

impl EndPath {
    pub const fn name(self) -> &'static str {
        match self {
            Self::NoneSeen => "none_seen",
            Self::SyncEndNormal => "sync_end_normal",
            Self::SyncEndAbnormal => "sync_end_abnormal",
            Self::LocalFault => "local_fault",
            Self::PeerEndA => "peer_end_a",
            Self::PeerEndB => "peer_end_b",
            Self::StartFailure => "start_failure",
            Self::Other => "phase6_other",
        }
    }

    pub const fn from_code(code: u8) -> Self {
        match code {
            1 => Self::SyncEndNormal,
            2 => Self::SyncEndAbnormal,
            3 => Self::LocalFault,
            4 => Self::PeerEndA,
            5 => Self::PeerEndB,
            6 => Self::StartFailure,
            7 => Self::Other,
            _ => Self::NoneSeen,
        }
    }
}

/// Classifies a phase-6 end from `.16cced0`'s caller and the latest peer-end path (0 none, `PATH_A`, `PATH_B`).
pub fn phase6_path(caller: u64, recent_peer_path: u8) -> EndPath {
    if caller == CALLER_COUNTDOWN {
        EndPath::LocalFault
    } else if caller == CALLER_START {
        EndPath::StartFailure
    } else {
        match recent_peer_path {
            PATH_A => EndPath::PeerEndA,
            PATH_B => EndPath::PeerEndB,
            _ => EndPath::Other,
        }
    }
}

/// `.16cced0`'s caller, as the session_end line names it.
pub fn via(caller: u64) -> &'static str {
    match caller {
        CALLER_COUNTDOWN => "countdown",
        CALLER_START => "start",
        _ => "tail_call",
    }
}

pub fn describe_session_end(tick: u64, e: &SessionEnd, out: &mut impl fmt::Write) -> fmt::Result {
    write!(
        out,
        "{tick} session_end reason={} fault_code={} net_result={} phase_before={} already_ending={} frame={} \
         caller=0x{:x} via={}",
        e.reason,
        e.fault_code,
        if e.fault_code != 0 { "-993" } else { "none" },
        e.phase_before,
        e.already_ending,
        Frame(e.frame),
        e.caller,
        via(e.caller)
    )
}

pub fn describe_ko(tick: u64, k: &Ko, out: &mut impl fmt::Write) -> fmt::Result {
    write!(
        out,
        "{tick} ko frame={} entry={} kind={} stamps={}->{} fault_word2={}->{}",
        Frame(k.frame),
        k.entry,
        k.kind,
        k.stamps_before,
        k.stamps_after,
        k.word2_before,
        k.word2_after
    )
}

pub fn describe_peer_path(tick: u64, p: &PeerPath, out: &mut impl fmt::Write) -> fmt::Result {
    write!(
        out,
        "{tick} peer_end_path path={} frame={} station={} peer_mask=0x{:x}",
        if p.path == PATH_A { "A" } else { "B" },
        Frame(p.frame),
        p.station,
        p.peer_mask
    )?;
    if p.path == PATH_A {
        write!(out, " end_reason={}", p.end_reason)?;
        match p.message {
            Some((byte0, word)) => write!(out, " msg_type={} msg_station={} msg_word={word}", byte0 & 0xf, byte0 >> 4),
            None => out.write_str(" message=unread"),
        }
    } else {
        write!(out, " caller=0x{:x}", p.caller)
    }
}

/// The session's last state when it stopped (`.16d5cd0`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub frame: Option<u16>,
    /// 0.5.1: every stock each entry lost (`stock_lost`), self-destructs included.
    pub kos: [u8; 8],
    /// 0.5.1: of those, the self-destructs.
    pub self_destructs: [u8; 8],
    pub fault_words: Option<[u32; 10]>,
    pub ended: EndPath,
    pub end_reason: u8,
    pub end_fault_code: u8,
    pub end_caller: u64,
    /// The game's own disconnect-penalty state for this console, read-only, when a field is known (none is yet:
    /// `penalty_flag=unknown`).
    pub penalty: Option<u32>,
}

impl Summary {
    pub fn words(&self) -> [u64; SIDE_WORDS] {
        let mut w = [0u64; SIDE_WORDS];
        w[0] = u64::from(self.frame.unwrap_or(0))
            | u64::from(self.frame.is_some()) << 16
            | u64::from(self.fault_words.is_some()) << 17
            | u64::from(self.ended as u8) << 24
            | u64::from(self.end_reason) << 32
            | u64::from(self.end_fault_code) << 40
            | u64::from(self.penalty.is_some()) << 48;
        w[1] = self.end_caller;
        w[2] = u64::from_le_bytes(self.kos);
        if let Some(words) = self.fault_words {
            for (i, pair) in words.chunks(2).enumerate() {
                w[3 + i] = u64::from(pair[0]) | u64::from(pair[1]) << 32;
            }
        }
        w[8] = u64::from(self.penalty.unwrap_or(0));
        w[9] = u64::from_le_bytes(self.self_destructs);
        w
    }

    pub fn from_words(w: &[u64; SIDE_WORDS]) -> Self {
        let fault_words = (w[0] >> 17 & 1 != 0).then(|| {
            let mut words = [0u32; 10];
            for (i, pair) in words.chunks_mut(2).enumerate() {
                pair[0] = w[3 + i] as u32;
                pair[1] = (w[3 + i] >> 32) as u32;
            }
            words
        });
        Self {
            frame: (w[0] >> 16 & 1 != 0).then_some(w[0] as u16),
            kos: w[2].to_le_bytes(),
            self_destructs: w[9].to_le_bytes(),
            fault_words,
            ended: EndPath::from_code((w[0] >> 24) as u8),
            end_reason: (w[0] >> 32) as u8,
            end_fault_code: (w[0] >> 40) as u8,
            end_caller: w[1],
            penalty: (w[0] >> 48 & 1 != 0).then_some(w[8] as u32),
        }
    }
}

/// `ssbu-stall-press-carry`'s export (0.2.0): `extern "C" fn(out: *mut StallStatsV1) -> bool`.
pub const STALL_SYMBOL: &[u8] = b"ssbu_stall_press_carry_stats_v1\0";

/// The export's out value; `size` is 72 for version 1.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StallStatsV1 {
    pub size: u32,
    pub version: u32,
    pub sessions: u32,
    pub decisions: u32,
    pub commits: u32,
    pub dropped: u32,
    pub committed_press_edges: u32,
    pub rescued: u32,
    pub rescued_bits: u32,
    pub jump: u32,
    pub attack: u32,
    pub special: u32,
    pub shield: u32,
    pub grab: u32,
    pub other: u32,
    pub flick_carried: u32,
    pub flick_lost_reversal: u32,
    pub flick_lost_superseded: u32,
}

pub const STALL_V1_SIZE: u32 = core::mem::size_of::<StallStatsV1>() as u32;

/// A filled answer of at least version 1's size, else nothing.
pub fn stall_stats(answer: Option<StallStatsV1>) -> Option<StallStatsV1> {
    answer.filter(|s| s.size >= STALL_V1_SIZE && s.version >= 1)
}

/// `last_session` is the verdict token written to `last-session.txt` for this session (`lastsession::LastSession`).
pub fn describe_summary(
    tick: u64,
    s: &Summary,
    last_session: &str,
    stall: Option<&StallStatsV1>,
    out: &mut impl fmt::Write,
) -> fmt::Result {
    write!(out, "{tick} session_summary frame={} kos=", Frame(s.frame))?;
    for (i, ko) in s.kos.iter().enumerate() {
        write!(out, "{}{ko}", if i == 0 { "" } else { "," })?;
    }
    out.write_str(" self_destructs=")?;
    for (i, sd) in s.self_destructs.iter().enumerate() {
        write!(out, "{}{sd}", if i == 0 { "" } else { "," })?;
    }
    out.write_str(" fault_words=")?;
    match s.fault_words {
        Some(words) => {
            for (i, word) in words.iter().enumerate() {
                write!(out, "{}{word}", if i == 0 { "" } else { "," })?;
            }
        }
        None => out.write_str("unread")?,
    }
    write!(out, " ended={}", s.ended.name())?;
    if !matches!(s.ended, EndPath::NoneSeen | EndPath::SyncEndNormal | EndPath::SyncEndAbnormal) {
        write!(
            out,
            " end_reason={} end_fault_code={} end_caller=0x{:x}",
            s.end_reason, s.end_fault_code, s.end_caller
        )?;
    }
    write!(out, " last_session={last_session} penalty_flag=")?;
    match s.penalty {
        Some(value) => write!(out, "{value}")?,
        None => out.write_str("unknown")?,
    }
    if let Some(t) = stall {
        write!(
            out,
            " stall_press rescued={} (jump={} attack={} special={} shield={} grab={} other={}) dropped={} committed={} \
             stalled={} flick_carried={} flick_lost={} (reversal={} superseded={})",
            t.rescued,
            t.jump,
            t.attack,
            t.special,
            t.shield,
            t.grab,
            t.other,
            t.dropped,
            t.committed_press_edges,
            t.dropped,
            t.flick_carried,
            t.flick_lost_reversal + t.flick_lost_superseded,
            t.flick_lost_reversal,
            t.flick_lost_superseded
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct S(String);

    impl fmt::Write for S {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            self.0.push_str(s);
            Ok(())
        }
    }

    fn text(f: impl FnOnce(&mut S) -> fmt::Result) -> String {
        let mut s = S(String::new());
        f(&mut s).unwrap();
        s.0
    }

    #[test]
    fn sites_are_aligned_and_distinct_from_040() {
        let old = [0x16d6cdc, 0x16cf304, 0x16ce7c0, 0x16cc2c4, 0x135fd60, 0x16d5ee8];
        for site in TELEMETRY_SITES {
            assert_eq!(site.offset % 8, 0, "{}", site.name);
            assert!(!old.contains(&site.offset), "{}", site.name);
            assert_ne!(site.guard, [0u8; 64]);
        }
        // The guards start with the hooked instructions.
        assert_eq!(&KO_ENTRY.guard[..4], &[0xff, 0xc3, 0x01, 0xd1]); // sub sp, sp, #0x70
        assert_eq!(&SESSION_END.guard[..4], &[0xff, 0x03, 0x02, 0xd1]); // sub sp, sp, #0x80
        assert_eq!(&MATCH_LIMITS.guard[..4], &[0x69, 0x1a, 0x02, 0xb9]); // str w9, [x19, #0x218]
    }

    #[test]
    fn match_start_round_trips_and_prints_the_quickplay_limits() {
        let mut rules = [0u8; RULE_BYTES];
        rules[0] = 0x0e;
        rules[63] = 0xab;
        let m = MatchStart {
            melee: 0x0e,
            mode: 0x0701_0101,
            mask: 0x3ee,
            watched: 0x3,
            watched_count: 2,
            block: 0x40,
            seconds: 30,
            n: 6,
            l: 7,
            rules,
        };
        assert_eq!(MatchStart::from_words(&m.words()), m);
        let line = text(|s| describe_match_start(5, &m, s));
        assert!(line.starts_with(
            "5 match_start melee=0x0e mode=0x07010101 fault_mask=0x3ee watched=0x03 watched_n=2 \
             limits=+0x40:window_s=30,n=6,l=7 rules=0e00"
        ));
        assert!(line.ends_with("00ab"));
        assert_eq!(line.len(), line.find("rules=").unwrap() + 6 + 128);
    }

    #[test]
    fn ko_round_trips_with_and_without_a_frame() {
        let k = Ko { entry: 1, kind: 3, frame: Some(4842), stamps_before: 2, stamps_after: 3, word2_before: 0, word2_after: 1 };
        let (a, b) = pack_ko(&k);
        assert_eq!(unpack_ko(a, b), k);
        assert_eq!(text(|s| describe_ko(9, &k, s)), "9 ko frame=4842 entry=1 kind=3 stamps=2->3 fault_word2=0->1");
        let none = Ko { frame: None, ..k };
        let (a, b) = pack_ko(&none);
        assert!(text(|s| describe_ko(9, &unpack_ko(a, b), s)).contains("frame=none"));
    }

    #[test]
    fn peer_paths_carry_the_message_or_the_caller() {
        // Case 7: the other console's end message (type 7 from station 0, half-word 7), reason 2.
        let a = PeerPath {
            path: PATH_A,
            station: 1,
            frame: Some(8444),
            peer_mask: 3,
            end_reason: 2,
            message: Some((0x07, 7)),
            caller: 0,
        };
        let (x, y) = pack_peer_path(&a);
        assert_eq!(unpack_peer_path(x, y), a);
        assert_eq!(
            text(|s| describe_peer_path(1, &a, s)),
            "1 peer_end_path path=A frame=8444 station=1 peer_mask=0x3 end_reason=2 msg_type=7 msg_station=0 msg_word=7"
        );
        // Case 8: a message naming this station (station 1 in the high nibble), reason 1.
        let named = PeerPath { end_reason: 1, message: Some((0x18, 0xffff)), ..a };
        let (x, y) = pack_peer_path(&named);
        assert_eq!(unpack_peer_path(x, y), named);
        assert!(text(|s| describe_peer_path(1, &named, s)).ends_with("end_reason=1 msg_type=8 msg_station=1 msg_word=65535"));
        let unread = PeerPath { message: None, ..a };
        let (x, y) = pack_peer_path(&unread);
        assert!(text(|s| describe_peer_path(1, &unpack_peer_path(x, y), s)).ends_with("end_reason=2 message=unread"));
        let b = PeerPath {
            path: PATH_B,
            station: 0,
            frame: None,
            peer_mask: 1,
            end_reason: 0,
            message: None,
            caller: 0x16cfa38,
        };
        let (x, y) = pack_peer_path(&b);
        assert_eq!(unpack_peer_path(x, y), b);
        assert!(text(|s| describe_peer_path(1, &b, s)).ends_with("frame=none station=0 peer_mask=0x1 caller=0x16cfa38"));
    }

    #[test]
    fn session_end_names_the_network_result_and_the_caller() {
        let e = SessionEnd { reason: 1, fault_code: 2, phase_before: 4, already_ending: 1, frame: Some(100), caller: CALLER_COUNTDOWN };
        let (a, b) = pack_session_end(&e);
        assert_eq!(unpack_session_end(a, b), e);
        assert_eq!(
            text(|s| describe_session_end(3, &e, s)),
            "3 session_end reason=1 fault_code=2 net_result=-993 phase_before=4 already_ending=1 frame=100 \
             caller=0x16d8d3c via=countdown"
        );
        let peer = SessionEnd { fault_code: 0, caller: 0x1234, ..e };
        assert!(text(|s| describe_session_end(3, &peer, s)).contains("net_result=none"));
        assert!(text(|s| describe_session_end(3, &peer, s)).ends_with("via=tail_call"));
    }

    #[test]
    fn phase6_is_classified_by_caller_then_by_the_peer_path() {
        assert_eq!(phase6_path(CALLER_COUNTDOWN, PATH_A), EndPath::LocalFault);
        assert_eq!(phase6_path(CALLER_START, 0), EndPath::StartFailure);
        assert_eq!(phase6_path(0x1569718, PATH_A), EndPath::PeerEndA);
        assert_eq!(phase6_path(0x1569718, PATH_B), EndPath::PeerEndB);
        assert_eq!(phase6_path(0x1569718, 0), EndPath::Other);
        for code in 0..8 {
            assert_eq!(EndPath::from_code(code) as u8, code);
        }
    }

    #[test]
    fn summary_round_trips_and_appends_the_stall_counters_only_when_loaded() {
        let s = Summary {
            frame: Some(5707),
            kos: [1, 0, 0, 0, 0, 0, 0, 0],
            self_destructs: [0; 8],
            fault_words: Some([0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            ended: EndPath::SyncEndNormal,
            end_reason: 0,
            end_fault_code: 0,
            end_caller: 0,
            penalty: None,
        };
        assert_eq!(Summary::from_words(&s.words()), s);
        let bare = text(|o| describe_summary(7, &s, "normal", None, o));
        assert_eq!(
            bare,
            "7 session_summary frame=5707 kos=1,0,0,0,0,0,0,0 self_destructs=0,0,0,0,0,0,0,0 \
             fault_words=0,0,0,0,0,0,0,0,0,0 ended=sync_end_normal last_session=normal penalty_flag=unknown"
        );
        let known = Summary { penalty: Some(3), ..s };
        assert_eq!(Summary::from_words(&known.words()), known);
        assert!(text(|o| describe_summary(7, &known, "peer_ended", None, o))
            .ends_with(" last_session=peer_ended penalty_flag=3"));
        let stall = StallStatsV1 {
            size: 72,
            version: 1,
            rescued: 10,
            jump: 9,
            attack: 1,
            dropped: 123,
            committed_press_edges: 45,
            flick_carried: 3,
            flick_lost_reversal: 1,
            ..StallStatsV1::default()
        };
        let line = text(|o| describe_summary(7, &s, "normal", Some(&stall), o));
        assert!(line.ends_with(
            " penalty_flag=unknown stall_press rescued=10 (jump=9 attack=1 special=0 shield=0 grab=0 other=0) dropped=123 committed=45 \
             stalled=123 flick_carried=3 flick_lost=1 (reversal=1 superseded=0)"
        ));
        assert!(line.len() < 512);
        let fault = Summary { ended: EndPath::LocalFault, end_reason: 1, end_fault_code: 2, end_caller: CALLER_COUNTDOWN, fault_words: None, ..s };
        let f = text(|o| describe_summary(7, &Summary::from_words(&fault.words()), "unknown", None, o));
        assert!(f.contains("fault_words=unread ended=local_fault end_reason=1 end_fault_code=2 end_caller=0x16d8d3c"));
    }

    #[test]
    fn the_stall_export_contract() {
        assert_eq!(STALL_V1_SIZE, 72);
        assert_eq!(STALL_SYMBOL.last(), Some(&0));
        assert!(stall_stats(None).is_none());
        assert!(stall_stats(Some(StallStatsV1 { size: 8, version: 1, ..StallStatsV1::default() })).is_none());
        assert!(stall_stats(Some(StallStatsV1 { size: 72, version: 1, ..StallStatsV1::default() })).is_some());
    }

    fn counters(dead: [u32; 3], suicides: u32, stock: u32) -> Counters {
        Counters { dead, suicides, stock }
    }

    #[test]
    fn the_first_poll_only_sets_the_baseline() {
        // Mid-match counts at the first poll (the previous battle's entry, or a late start): no event.
        let (base, lost) = stock_step(None, &counters([1, 0, 0], 1, 2));
        assert_eq!(lost, None);
        assert_eq!(base, Baseline { dead: 1, suicides: 1 });
        assert_eq!(Baseline::unpack(base.pack()), Some(base));
        assert_eq!(Baseline::unpack(0), None);
        // Nothing changed: no event.
        assert_eq!(stock_step(Some(base), &counters([1, 0, 0], 1, 2)).1, None);
    }

    #[test]
    fn a_stock_lost_to_a_hit_is_not_a_self_destruct() {
        let base = Some(Baseline { dead: 0, suicides: 0 });
        let (next, lost) = stock_step(base, &counters([1, 0, 0], 0, 2));
        assert_eq!(lost, Some(Lost { stocks: 1, self_destructs: 0 }));
        assert_eq!(next, Baseline { dead: 1, suicides: 0 });
    }

    #[test]
    fn a_self_destruct_is_named() {
        let base = Some(Baseline { dead: 1, suicides: 0 });
        // The dead total counts all three slots.
        let (_, lost) = stock_step(base, &counters([1, 0, 1], 1, 1));
        assert_eq!(lost, Some(Lost { stocks: 1, self_destructs: 1 }));
    }

    #[test]
    fn two_stocks_in_one_poll_count_twice() {
        let base = Some(Baseline { dead: 0, suicides: 0 });
        let (_, lost) = stock_step(base, &counters([2, 0, 0], 1, 1));
        assert_eq!(lost, Some(Lost { stocks: 2, self_destructs: 1 }));
        // A self-destruct count can't exceed the stocks lost in the same poll.
        let (_, lost) = stock_step(base, &counters([1, 0, 0], 3, 2));
        assert_eq!(lost, Some(Lost { stocks: 1, self_destructs: 1 }));
    }

    #[test]
    fn a_count_going_down_starts_a_new_baseline() {
        // The next battle's entry starts from zero: a new baseline, no event.
        let base = Some(Baseline { dead: 3, suicides: 1 });
        let (next, lost) = stock_step(base, &counters([0, 0, 0], 0, 3));
        assert_eq!((next, lost), (Baseline { dead: 0, suicides: 0 }, None));
        let (_, lost) = stock_step(Some(Baseline { dead: 3, suicides: 1 }), &counters([4, 0, 0], 0, 0));
        assert_eq!(lost, None, "the self-destruct count went down: a new battle");
    }

    #[test]
    fn stock_lost_round_trips_and_prints() {
        let s = StockLost { entry: 1, self_destruct: true, frame: Some(446), counters: counters([1, 0, 0], 1, 0) };
        let (a, b) = pack_stock_lost(&s);
        assert_eq!(unpack_stock_lost(a, b), s);
        assert_eq!(
            text(|o| describe_stock_lost(9, &s, o)),
            "9 stock_lost frame=446 entry=1 self_destruct=yes dead=1,0,0 suicides=1 stock=0"
        );
        let hit = StockLost { self_destruct: false, frame: None, ..s };
        let (a, b) = pack_stock_lost(&hit);
        assert!(text(|o| describe_stock_lost(9, &unpack_stock_lost(a, b), o))
            .starts_with("9 stock_lost frame=none entry=1 self_destruct=no "));
        assert_eq!(EV_STOCK_LOST, 17);
    }

    /// Game memory for `read_entries`: unset addresses read 0; every read is recorded.
    struct Fake {
        cells: std::collections::HashMap<u64, u64>,
        reads: std::cell::RefCell<Vec<u64>>,
    }

    impl Fake {
        const TEXT: u64 = 0x80_0000_0000;
        const MANAGER: u64 = 0x10_0000;
        const IMPL: u64 = 0x20_0000;

        fn new(count: u32) -> Self {
            let mut cells = std::collections::HashMap::new();
            cells.insert(Self::TEXT + FIGHTER_MANAGER as u64, Self::MANAGER);
            cells.insert(Self::MANAGER, Self::IMPL);
            cells.insert(Self::IMPL + ENTRY_COUNT_AT, u64::from(count));
            let mut f = Self { cells, reads: std::cell::RefCell::new(Vec::new()) };
            // Eight slots filled, so a read past the count would find something.
            for e in 0..ENTRIES as u64 {
                let fighter = 0x30_0000 + e * 0x1000;
                let data = 0x40_0000 + e * 0x1000;
                f.cells.insert(Self::IMPL + ENTRY_AT + 8 * e, fighter);
                f.cells.insert(fighter + INFO_DATA_AT, data);
                f.cells.insert(data + DEAD_AT, e);
                f.cells.insert(data + STOCK_AT, 3);
            }
            f
        }

        fn read(&self, at: u64) -> u64 {
            self.reads.borrow_mut().push(at);
            self.cells.get(&at).copied().unwrap_or(0)
        }

        fn slot(e: u64) -> u64 {
            Self::IMPL + ENTRY_AT + 8 * e
        }
    }

    impl Memory for Fake {
        fn u64_at(&self, at: u64) -> u64 {
            self.read(at)
        }
        fn u32_at(&self, at: u64) -> u32 {
            self.read(at) as u32
        }
        fn u8_at(&self, at: u64) -> u8 {
            self.read(at) as u8
        }
    }

    #[test]
    fn entries_beyond_the_live_count_are_never_read() {
        let mem = Fake::new(2);
        let mut out = [None; ENTRIES];
        assert_eq!(read_entries(&mem, Fake::TEXT, &mut out), 2);
        assert_eq!(out[1], Some(Counters { dead: [1, 0, 0], suicides: 0, stock: 3 }));
        assert!(out[2..].iter().all(Option::is_none));
        let reads = mem.reads.borrow();
        for e in 2..ENTRIES as u64 {
            assert!(!reads.contains(&Fake::slot(e)), "slot {e} was read");
            assert!(!reads.contains(&(0x30_0000 + e * 0x1000 + INFO_DATA_AT)), "entry {e} was followed");
        }
        // A count above 8 is capped at the eight slots the game has.
        let mem = Fake::new(200);
        assert_eq!(read_entries(&mem, Fake::TEXT, &mut out), ENTRIES);
        assert!(!mem.reads.borrow().contains(&Fake::slot(8)));
    }

    #[test]
    fn a_null_slot_or_data_pointer_is_not_followed() {
        let mut mem = Fake::new(2);
        mem.cells.insert(Fake::slot(0), 0);
        mem.cells.insert(0x30_1000 + INFO_DATA_AT, 0x40_1003); // entry 1's data pointer misaligned
        let mut out = [None; ENTRIES];
        assert_eq!(read_entries(&mem, Fake::TEXT, &mut out), 2);
        assert_eq!(out, [None; ENTRIES]);
        let reads = mem.reads.borrow();
        assert!(!reads.contains(&INFO_DATA_AT), "a null slot was followed");
        assert!(!reads.iter().any(|at| (0x40_1003..0x40_1200).contains(at)), "a misaligned pointer was followed");
    }

    #[test]
    fn the_results_screen_and_a_missing_manager_read_no_entry() {
        let mut mem = Fake::new(2);
        mem.cells.insert(Fake::IMPL + RESULT_MODE_AT, 1);
        let mut out = [None; ENTRIES];
        assert_eq!(read_entries(&mem, Fake::TEXT, &mut out), 0);
        assert!(!mem.reads.borrow().contains(&Fake::slot(0)));
        let mut mem = Fake::new(2);
        mem.cells.insert(Fake::TEXT + FIGHTER_MANAGER as u64, 0);
        assert_eq!(read_entries(&mem, Fake::TEXT, &mut out), 0);
        assert_eq!(*mem.reads.borrow(), vec![Fake::TEXT + FIGHTER_MANAGER as u64]);
    }

    #[test]
    fn the_summary_counts_every_lost_stock_and_the_self_destructs() {
        // Entry 0 lost a stock to a hit, entry 1 two: one to a hit, one a self-destruct.
        let s = Summary {
            frame: Some(3004),
            kos: [1, 2, 0, 0, 0, 0, 0, 0],
            self_destructs: [0, 1, 0, 0, 0, 0, 0, 0],
            ended: EndPath::SyncEndNormal,
            ..Summary::default()
        };
        assert_eq!(Summary::from_words(&s.words()), s);
        let line = text(|o| describe_summary(7, &s, "normal", None, o));
        assert!(
            line.starts_with("7 session_summary frame=3004 kos=1,2,0,0,0,0,0,0 self_destructs=0,1,0,0,0,0,0,0 "),
            "{line}"
        );
        assert!(line.contains(" fault_words=unread ended=sync_end_normal last_session=normal penalty_flag=unknown"));
    }
}
