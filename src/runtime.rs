//! Switch half: version check, guarded inline hooks, two static rings and one
//! writer thread. Simulation hooks observe the game's own reads. The two UI
//! text hooks append a reason to a native text buffer; they write no battle
//! state.
//! Events go to the rings. The writer thread
//! runs on a static stack and owns every file, opened through `nn::fs` with
//! paths built on the stack. Nothing allocates: the rings are static, buffers
//! live on the stack, and the counting allocator below reports the plugin's
//! heap allocations (expected 0) in install, session and telemetry lines.
use crate::dialog;
use crate::disconnect::Reason;
use crate::evidence::Evidence;
use crate::matchlog::{
    self, describe_ko, describe_match_start, describe_peer_path, describe_session_end, describe_summary,
    pack_ko, pack_peer_path, pack_session_end, phase6_path, unpack_ko, unpack_peer_path, unpack_session_end,
    EndPath, Ko, MatchStart, PeerPath, SessionEnd, StallStatsV1, Summary, EV_KO, EV_MATCH_START, EV_PEER_PATH,
    EV_SESSION_END, EV_SUMMARY, PATH_A, PATH_B, SIDE_WORDS,
};
use crate::model::{
    self, describe, describe_fault, fault_code_is_logged, minimal_outer, pack_desync, pack_fault,
    pack_recording, pack_sync_end, recorder, unpack_fault, xor_words, EndState, FaultView,
    FrameRecord, Line, RawEvent, Site, Telemetry, CFMK_BYTES, CFMK_PAYLOAD_LEN, CFMK_RECORDS,
    DESYNC, EV_DESYNC, EV_FAULT, EV_LINK, EV_PEER_ABORT, EV_PLAYBACK_MISMATCH, EV_RECORDING,
    EV_SESSION, EV_SYNC_END, FAULT_ARM, FAULT_CALL_OFFSET, FAULT_CALL_WORD, FAULT_CODE_AT,
    FAULT_OBJECT, FAULT_SESSION_OFFSET, FAULT_SESSION_WORD, FAULT_STORE_OFFSET, FAULT_STORE_WORD,
    FAULT_WORD_AT, FAULT_WORD_COUNT, FRAME_CHECK, KIND_ONLINE, KIND_PLAYBACK, OUTER_BYTES,
    PEER_ABORT, PLAYBACK_CHECK, RECORDING_NORMAL_END, RECORD_BYTES, REPLAY_FRAME_INDEX,
    REPLAY_PLAYER, RNG_CHECK_WORD, RNG_MANAGER, RNG_STATE_STRIDE, SYNC_END, TAG_CFMK, TAG_FRAM,
};
use core::cell::UnsafeCell;
use core::fmt::Write as _;
use skyline::hooks::{getRegionAddress, InlineCtx, Region};
use skyline::nn;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

const DATA_DIR: &str = "sd:/ultimate/ssbu-desync-detector";
/// Files `frames-0.bin` .. `frames-7.bin` (and `match-N.bin`), reused in rotation.
const SLOTS: usize = 8;
const FRAME_RING: usize = 2048;
const EVENT_RING: usize = 64;
const TICKS_PER_SECOND: u64 = 19_200_000;
/// A new session starts after this long without frames.
const SESSION_GAP_TICKS: u64 = 5 * TICKS_PER_SECOND;
/// The staged recording is saved this long after the lockstep session ends
/// (the forced end writes one last record first); it is replaced by the
/// committed entry if the game finalises the match within `COMMIT_WAIT_TICKS`.
const STAGED_DELAY_TICKS: u64 = TICKS_PER_SECOND;
const OUTER_WAIT_TICKS: u64 = 5 * TICKS_PER_SECOND;
const COMMIT_WAIT_TICKS: u64 = 60 * TICKS_PER_SECOND;
const WRITER_STACK: usize = 32 * 1024;
const EVENTS_LOG_LIMIT: i64 = 1 << 20;
const TELEMETRY_INTERVAL_TICKS: u64 = 5 * TICKS_PER_SECOND;
/// The frame hook on the same pass runs well inside this. After it, the
/// writer logs the fault with no frame rather than attaching a later one.
const FAULT_FRESH_TICKS: u64 = TICKS_PER_SECOND / 20;
const FAULT_STALE_TICKS: u64 = TICKS_PER_SECOND / 10;
const FAULT_LOG_RING: usize = 4;
const OPEN_READ: i32 = 1;
const OPEN_WRITE_APPEND: i32 = 2 | 4;

/// Counts calls into the heap. Nothing in the plugin is meant to make one.
struct CountingAllocator;
static HEAP_ALLOCATIONS: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        HEAP_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        System.alloc(layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        HEAP_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        System.alloc_zeroed(layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        HEAP_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        System.realloc(ptr, layout, size)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

static FRAMES: [[AtomicU32; 5]; FRAME_RING] =
    [const { [const { AtomicU32::new(0) }; 5] }; FRAME_RING];
static FRAME_HEAD: AtomicUsize = AtomicUsize::new(0);
static LAST_FRAME_TICK: AtomicU64 = AtomicU64::new(0);

struct EventSlot {
    seq: AtomicU64,
    tick: AtomicU64,
    code: AtomicU64,
    a: AtomicU64,
    b: AtomicU64,
}
static EVENTS: [EventSlot; EVENT_RING] = [const {
    EventSlot {
        seq: AtomicU64::new(0),
        tick: AtomicU64::new(0),
        code: AtomicU64::new(0),
        a: AtomicU64::new(0),
        b: AtomicU64::new(0),
    }
}; EVENT_RING];
static EVENT_HEAD: AtomicU64 = AtomicU64::new(0);

/// Mismatch rows in the current 10 s window (the compare repeats per frame).
static DESYNC_ROWS: AtomicU64 = AtomicU64::new(0);
static DESYNC_WINDOW: AtomicU64 = AtomicU64::new(0);
static PLAYBACK_MISMATCHES: AtomicU64 = AtomicU64::new(0);
static TEXT_BASE: AtomicU64 = AtomicU64::new(0);
static DISCONNECT_EVIDENCE: Evidence = Evidence::new();
static BOOT_TICK: AtomicU64 = AtomicU64::new(0);
static TELEMETRY_REQUEST: AtomicBool = AtomicBool::new(false);
/// Staged by the arming hook and taken by the frame hook on the same pass.
static FAULT_CODE: AtomicU32 = AtomicU32::new(0);
static FAULT_TICK: AtomicU64 = AtomicU64::new(0);
static FAULT_MASK: AtomicU32 = AtomicU32::new(0);
static FAULT_MASK_OK: AtomicU32 = AtomicU32::new(0);
static FAULT_WORDS_OK: AtomicU32 = AtomicU32::new(0);
static FAULT_WORDS: [AtomicU32; FAULT_WORD_COUNT] = [const { AtomicU32::new(0) }; FAULT_WORD_COUNT];
static FAULT_LOG_HEAD: AtomicU64 = AtomicU64::new(0);

struct FaultLogSlot {
    seq: AtomicU64,
    words_ok: AtomicU32,
    words: [AtomicU32; FAULT_WORD_COUNT],
}
static FAULT_LOG: [FaultLogSlot; FAULT_LOG_RING] = [const {
    FaultLogSlot {
        seq: AtomicU64::new(0),
        words_ok: AtomicU32::new(0),
        words: [const { AtomicU32::new(0) }; FAULT_WORD_COUNT],
    }
}; FAULT_LOG_RING];

/// 0.5.0: records too wide for an event (`match_start`, `session_summary`); the event's `a` is the sequence.
const SIDE_RING: usize = 8;
struct SideSlot {
    seq: AtomicU64,
    words: [AtomicU64; SIDE_WORDS],
}
static SIDE: [SideSlot; SIDE_RING] = [const {
    SideSlot {
        seq: AtomicU64::new(0),
        words: [const { AtomicU64::new(0) }; SIDE_WORDS],
    }
}; SIDE_RING];
static SIDE_HEAD: AtomicU64 = AtomicU64::new(0);
/// A KO's state before `.1363420`'s update, taken by its epilogue (game thread; bit 63 = set).
static KO_PENDING_A: AtomicU64 = AtomicU64::new(0);
static KO_PENDING_B: AtomicU64 = AtomicU64::new(0);
/// KOs per entry since the match start, for the summary.
static KO_COUNTS: [AtomicU32; 8] = [const { AtomicU32::new(0) }; 8];
/// The first end seen this match (`EndPath` code, 0 = none), its reason, fault code and caller.
static END_PATH: AtomicU32 = AtomicU32::new(0);
static END_REASON: AtomicU32 = AtomicU32::new(0);
static END_FAULT: AtomicU32 = AtomicU32::new(0);
static END_CALLER: AtomicU64 = AtomicU64::new(0);
static PEER_PATH_LAST: AtomicU32 = AtomicU32::new(0);
static PEER_PATH_TICK: AtomicU64 = AtomicU64::new(0);
/// `ssbu_stall_press_carry_stats_v1`, 0 until found (looked up by the writer thread only).
static STALL_EXPORT: AtomicUsize = AtomicUsize::new(0);

/// The writer thread's object and stack, static so no heap is involved.
#[repr(C, align(4096))]
struct ThreadStack(UnsafeCell<[u8; WRITER_STACK]>);
#[repr(C, align(16))]
struct ThreadObject(UnsafeCell<[u8; 0x1c0]>);
// SAFETY: handed to nn::os once, at startup, and never touched again.
unsafe impl Sync for ThreadStack {}
unsafe impl Sync for ThreadObject {}
static WRITER_STACK_MEMORY: ThreadStack = ThreadStack(UnsafeCell::new([0; WRITER_STACK]));
static WRITER_THREAD: ThreadObject = ThreadObject(UnsafeCell::new([0; 0x1c0]));

extern "C" {
    #[link_name = "\u{1}_ZN2nn4nifm27GetInternetConnectionStatusEPNS0_24InternetConnectionStatusE"]
    fn GetInternetConnectionStatus(out: *mut u8) -> u32;
}

fn tick() -> u64 {
    unsafe { nn::os::GetSystemTick() as u64 }
}

fn push_event(code: u8, a: u64, b: u64) {
    publish_event(&RawEvent {
        tick: tick(),
        code,
        a,
        b,
    });
}

fn push_session_event(session: u64, code: u8, a: u64, b: u64) {
    let event = RawEvent {
        tick: tick(),
        code,
        a,
        b,
    };
    DISCONNECT_EVIDENCE.event(session, &event);
    publish_event(&event);
}

fn publish_event(event: &RawEvent) {
    if matches!(
        event.code,
        EV_SYNC_END | EV_DESYNC | EV_PEER_ABORT | model::EV_COMMUNICATION_TEXT
    ) {
        TELEMETRY_REQUEST.store(true, Ordering::Release);
    }
    let index = EVENT_HEAD.fetch_add(1, Ordering::AcqRel);
    let slot = &EVENTS[(index % EVENT_RING as u64) as usize];
    slot.tick.store(event.tick, Ordering::Relaxed);
    slot.code.store(u64::from(event.code), Ordering::Relaxed);
    slot.a.store(event.a, Ordering::Relaxed);
    slot.b.store(event.b, Ordering::Relaxed);
    slot.seq.store(index + 1, Ordering::Release);
}

fn disconnect_reason(now: u64, notice: bool) -> Option<Reason> {
    // A communication-error sentence is itself the disconnect, so missing
    // evidence still says Unknown. The Canceled notice and the reconnect
    // warning share words with ordinary cancels, so they stay as the game
    // wrote them unless this session has a verdict.
    let precise = DISCONNECT_EVIDENCE.reason(now);
    if notice {
        precise
    } else {
        Some(precise.unwrap_or(Reason::Unknown))
    }
}

/// The body of every text hook: `route` says where the count and the buffer
/// are. Only the count register is changed, and only after an append.
unsafe fn communication_text(route: &dialog::Route, ctx: &mut InlineCtx) {
    let now = tick();
    // SAFETY: installation matched every guard of `route`, which fixes the
    // frame layout, the buffer size and zeroing, and the instructions around
    // the hook. The buffer belongs to this call.
    let sp = ctx.sp.x();
    let fp = ctx.registers[29].x();
    let len = ctx.registers[route.count].w() as usize;
    let text_base = TEXT_BASE.load(Ordering::Relaxed);
    // The plain function's caller is who asked for this label. Read it only
    // after the frame check inside `notice_kind_in_frame` has accepted FP.
    let kind = dialog::notice_kind_in_frame(route, sp, fp, len);
    let appended = dialog::append_in_frame(route, text_base, sp, fp, len, |notice| {
        disconnect_reason(now, notice)
    });
    if let Some(kind) = kind {
        let caller = if route.caller.is_none() {
            let saved = fp.checked_add(dialog::PLAIN_CALLER_LR).unwrap_or(0) as *const u64;
            let lr = saved.read_unaligned();
            lr.checked_sub(text_base)
                .filter(|offset| *offset < 0x1000_0000)
                .unwrap_or(0)
        } else {
            0
        };
        let verdict = appended
            .map(|(reason, _)| u64::from(reason.code()))
            .unwrap_or(model::NOTICE_VERDICT_WITHHELD);
        publish_event(&RawEvent {
            tick: now,
            code: model::EV_NOTICE_TEXT,
            a: u64::from(kind) | route.code << 8 | verdict << 16 | (len as u64) << 32,
            b: caller,
        });
    }
    if let Some((reason, new_len)) = appended {
        // The relocated MOV hands this count on (args: the formatter's return
        // value, whose epilogue then restores the caller's x20; plain: the
        // setter's count argument).
        ctx.registers[route.count].set_w(new_len as u32);
        publish_event(&RawEvent {
            tick: now,
            code: model::EV_COMMUNICATION_TEXT,
            a: reason.code() | route.code << 8,
            b: new_len as u64,
        });
    }
}

/// Formatter epilogue, before MOV w0,w20. Its guarded frame contains the
/// caller's return address; only the zeroed-buffer caller is accepted.
/// The aligned far patch relocates MOV/ADD/LDP/LDP, no branch or call.
#[skyline::hook(offset = dialog::ARGS_HOOK, inline)]
unsafe fn desync_detector_communication_text(ctx: &mut InlineCtx) {
    communication_text(&dialog::ARGS, ctx);
}

/// `.377a120` between its formatter and the pane setter, at MOV w2,w0. The
/// aligned far patch relocates MOV/ADD/MOV/MOV; the setter call stays put.
#[skyline::hook(offset = dialog::PLAIN_HOOK, inline)]
unsafe fn desync_detector_plain_text(ctx: &mut InlineCtx) {
    communication_text(&dialog::PLAIN, ctx);
}

/// Single producer: the game thread that runs the lockstep or the playback.
fn push_frame(record: FrameRecord) {
    let head = FRAME_HEAD.load(Ordering::Relaxed);
    let slot = &FRAMES[head % FRAME_RING];
    for (cell, word) in slot.iter().zip([
        record.frame,
        record.rng,
        record.position,
        record.recorded,
        record.flags,
    ]) {
        cell.store(word, Ordering::Relaxed);
    }
    FRAME_HEAD.store(head + 1, Ordering::Release);
    LAST_FRAME_TICK.store(tick(), Ordering::Relaxed);
}

unsafe fn read_u8(address: u64) -> u8 {
    (address as *const u8).read_volatile()
}

unsafe fn read_u16(address: u64) -> u16 {
    (address as *const u16).read_unaligned()
}

unsafe fn read_u32(address: u64) -> u32 {
    (address as *const u32).read_unaligned()
}

unsafe fn read_u64(address: u64) -> u64 {
    (address as *const u64).read_unaligned()
}

/// The online check value recomputed from the RNG manager, as the lockstep
/// does at `.text+0x16d6c3c`; 0 when the manager is not set up.
unsafe fn rng_check_now() -> u32 {
    let slot = TEXT_BASE.load(Ordering::Relaxed) + RNG_MANAGER as u64;
    let manager = read_u64(read_u64(slot));
    if manager < 0x10000 {
        return 0;
    }
    let (begin, end) = (read_u64(manager), read_u64(manager + 8));
    if begin < 0x10000 || end < begin || end - begin > 64 * RNG_STATE_STRIDE as u64 {
        return 0;
    }
    let count = (end - begin) / RNG_STATE_STRIDE as u64;
    model::rng_check(
        (0..count).map(|i| read_u32(begin + i * RNG_STATE_STRIDE as u64 + RNG_CHECK_WORD as u64)),
    )
}

/// Diag build only: the online frame whose stored check value is flipped
/// (`force_desync_frame` in `diag.txt`; `u64::MAX` = off).
#[cfg(feature = "diag")]
static FORCE_DESYNC_FRAME: AtomicU64 = AtomicU64::new(u64::MAX);

/// Diag build only: reads `diag.txt` (at most 256 bytes) into the switches.
#[cfg(feature = "diag")]
fn read_diag() -> model::Diag {
    let mut text = [0u8; 256];
    let mut diag_path = path(format_args!("{DATA_DIR}/diag.txt"));
    let Some(path) = diag_path.with_nul() else {
        return model::Diag::default();
    };
    let read = unsafe {
        let mut handle = nn::fs::FileHandle { handle: 0 };
        if nn::fs::OpenFile(&mut handle, path.as_ptr(), OPEN_READ) != 0 {
            return model::Diag::default();
        }
        let mut read = 0u64;
        let result = nn::fs::ReadFile1(&mut read, handle, 0, text.as_mut_ptr(), text.len() as u64);
        nn::fs::CloseFile(handle);
        if result != 0 {
            return model::Diag::default();
        }
        read as usize
    };
    model::parse_diag(&text[..read.min(text.len())])
}

#[skyline::hook(offset = model::FRAME_CHECK_OFFSET, inline)]
unsafe fn desync_detector_frame_check(ctx: &mut InlineCtx) {
    let record = ctx.registers[10].x().wrapping_sub(0xa);
    if record < 0x10000 {
        return;
    }
    let frame = u32::from(read_u16(record));
    // Diag build only: `stp w11, w8, [x9]` at this site stores w11 as this
    // console's check value for the frame; a forced frame stores it flipped.
    #[cfg(feature = "diag")]
    {
        let forced = FORCE_DESYNC_FRAME.load(Ordering::Relaxed);
        if forced <= u64::from(u32::MAX) {
            let diag = model::Diag {
                force_desync_frame: Some(forced as u32),
            };
            let value = ctx.registers[11].w();
            let stored = model::diag_check_value(value, frame, &diag);
            ctx.registers[11].set_w(stored);
            if stored != value {
                // Once per boot: the next match runs unforced and can end normally.
                FORCE_DESYNC_FRAME.store(u64::MAX, Ordering::Relaxed);
            }
        }
    }
    DISCONNECT_EVIDENCE.frame(tick(), KIND_ONLINE, ctx.registers[19].x(), frame);
    attach_fault_frame(frame);
    push_frame(FrameRecord {
        frame,
        rng: ctx.registers[11].w(),
        position: 0,
        recorded: 0,
        flags: KIND_ONLINE | ctx.registers[8].w().trailing_zeros().min(0xff) << 8,
    });
}

/// The countdown-arming stores. Runs once, when `.16acd30` has just returned
/// nonzero. Reads that stored byte and the ten words; writes nothing.
#[skyline::hook(offset = model::FAULT_ARM_OFFSET, inline)]
unsafe fn desync_detector_fault_arm(ctx: &InlineCtx) {
    let session = ctx.registers[19].x();
    if session < 0x10000 {
        return;
    }
    let code = read_u8(session + FAULT_CODE_AT as u64);
    if !fault_code_is_logged(code) {
        return;
    }
    let (mask, words) = read_fault_table();
    stage_fault(code, mask, words);
}

fn stage_fault(code: u8, mask: Option<u16>, words: Option<[u32; FAULT_WORD_COUNT]>) {
    if let Some(words) = words {
        for (cell, word) in FAULT_WORDS.iter().zip(words) {
            cell.store(word, Ordering::Relaxed);
        }
        FAULT_WORDS_OK.store(1, Ordering::Relaxed);
    } else {
        FAULT_WORDS_OK.store(0, Ordering::Relaxed);
    }
    match mask {
        Some(mask) => {
            FAULT_MASK.store(u32::from(mask), Ordering::Relaxed);
            FAULT_MASK_OK.store(1, Ordering::Relaxed);
        }
        None => FAULT_MASK_OK.store(0, Ordering::Relaxed),
    }
    FAULT_TICK.store(tick(), Ordering::Relaxed);
    FAULT_CODE.store(u32::from(code), Ordering::Release);
}

fn claim_fault(now: u64, fresh: bool) -> Option<FaultView> {
    if FAULT_CODE.load(Ordering::Relaxed) == 0 {
        return None;
    }
    let age = now.saturating_sub(FAULT_TICK.load(Ordering::Relaxed));
    let take = if fresh {
        age <= FAULT_FRESH_TICKS
    } else {
        age > FAULT_STALE_TICKS
    };
    if !take {
        return None;
    }
    let code = FAULT_CODE.swap(0, Ordering::AcqRel);
    if !fault_code_is_logged(code as u8) {
        return None;
    }
    let words = (FAULT_WORDS_OK.load(Ordering::Relaxed) == 1).then(|| {
        let mut words = [0; FAULT_WORD_COUNT];
        for (word, cell) in words.iter_mut().zip(FAULT_WORDS.iter()) {
            *word = cell.load(Ordering::Relaxed);
        }
        words
    });
    let mask = (FAULT_MASK_OK.load(Ordering::Relaxed) == 1)
        .then(|| FAULT_MASK.load(Ordering::Relaxed) as u16);
    Some(FaultView {
        code: code as u8,
        frame: None,
        mask,
        words,
    })
}

fn publish_fault(mut view: FaultView, frame: Option<u32>) {
    view.frame = frame;
    let seq = FAULT_LOG_HEAD.fetch_add(1, Ordering::AcqRel) + 1;
    let slot = &FAULT_LOG[((seq - 1) % FAULT_LOG_RING as u64) as usize];
    if let Some(words) = view.words {
        for (cell, word) in slot.words.iter().zip(words) {
            cell.store(word, Ordering::Relaxed);
        }
        slot.words_ok.store(1, Ordering::Relaxed);
    } else {
        slot.words_ok.store(0, Ordering::Relaxed);
    }
    slot.seq.store(seq, Ordering::Release);
    push_event(EV_FAULT, pack_fault(&view), seq);
}

fn attach_fault_frame(frame: u32) {
    if let Some(view) = claim_fault(tick(), true) {
        publish_fault(view, Some(frame));
    }
}

fn flush_stale_fault() {
    if let Some(view) = claim_fault(tick(), false) {
        publish_fault(view, None);
    }
}

/// `[[main+0x52c3940]]`: the mask in the low 10 bits, then the ten words.
unsafe fn read_fault_table() -> (Option<u16>, Option<[u32; FAULT_WORD_COUNT]>) {
    let slot = TEXT_BASE.load(Ordering::Relaxed) + FAULT_OBJECT as u64;
    let cell = read_u64(slot);
    if cell < 0x10000 {
        return (None, None);
    }
    let object = read_u64(cell);
    if object < 0x10000 || !object.is_multiple_of(4) {
        return (None, None);
    }
    let mask = (read_u64(object) & 0x3ff) as u16;
    let mut words = [0; FAULT_WORD_COUNT];
    for (index, word) in words.iter_mut().enumerate() {
        *word = read_u32(object + FAULT_WORD_AT as u64 + index as u64 * 4);
    }
    (Some(mask), Some(words))
}

#[skyline::hook(offset = model::DESYNC_OFFSET, inline)]
unsafe fn desync_detector_desync(ctx: &InlineCtx) {
    let now = tick();
    let record = ctx.registers[2].x();
    let frame = if record < 0x10000 {
        0
    } else {
        read_u16(record)
    };
    let (a, b) = pack_desync(
        ctx.registers[8].w(),
        ctx.registers[9].w(),
        (ctx.registers[1].w() & 0xff) as u8,
        frame,
    );
    let event = RawEvent {
        tick: now,
        code: EV_DESYNC,
        a,
        b,
    };
    // Rate limiting affects only the log; every actual mismatch is evidence.
    DISCONNECT_EVIDENCE.event(ctx.registers[0].x(), &event);
    if now.wrapping_sub(DESYNC_WINDOW.load(Ordering::Acquire)) > 10 * TICKS_PER_SECOND {
        DESYNC_WINDOW.store(now, Ordering::Release);
        DESYNC_ROWS.store(0, Ordering::Release);
    }
    if DESYNC_ROWS.fetch_add(1, Ordering::AcqRel) < 8 {
        publish_event(&event);
    }
}

#[skyline::hook(offset = model::SYNC_END_OFFSET, inline)]
unsafe fn desync_detector_sync_end(ctx: &InlineCtx) {
    let session = ctx.registers[0].x();
    if session < 0x10000 {
        return;
    }
    let a = pack_sync_end(
        ctx.registers[1].w(),
        read_u8(session + 0x5f) & 1 != 0,
        read_u8(session + 0x63) != 0,
        read_u8(session + 0x1),
        read_u16(session + 0x2),
    );
    let caller = ctx.registers[30]
        .x()
        .wrapping_sub(TEXT_BASE.load(Ordering::Relaxed));
    let normal = ctx.registers[1].w() == model::END_KIND_NORMAL && read_u8(session + 0x5f) & 1 == 0;
    note_end(
        if normal { EndPath::SyncEndNormal } else { EndPath::SyncEndAbnormal },
        0,
        0,
        caller,
    );
    push_session_event(session, EV_SYNC_END, a, caller);
}

#[skyline::hook(offset = model::PEER_ABORT_OFFSET, inline)]
unsafe fn desync_detector_peer_end(ctx: &InlineCtx) {
    let (session, message) = (ctx.registers[19].x(), ctx.registers[20].x());
    if session < 0x10000 || message < 0x10000 {
        return;
    }
    let sender = u64::from(read_u8(message) >> 4);
    let reason = u64::from(u16::from_be_bytes([
        read_u8(message + 1),
        read_u8(message + 2),
    ]));
    push_session_event(
        session,
        EV_PEER_ABORT,
        sender | reason << 8,
        u64::from(read_u16(session + 0x2)),
    );
}

// --- 0.5.0 match telemetry (matchlog.rs): read-only hooks; each writes only this plugin's statics. ---

fn publish_side(code: u8, words: &[u64; SIDE_WORDS]) {
    let seq = SIDE_HEAD.fetch_add(1, Ordering::AcqRel) + 1;
    let slot = &SIDE[((seq - 1) % SIDE_RING as u64) as usize];
    slot.seq.store(0, Ordering::Release);
    for (cell, word) in slot.words.iter().zip(words) {
        cell.store(*word, Ordering::Relaxed);
    }
    slot.seq.store(seq, Ordering::Release);
    push_event(code, seq, 0);
}

fn read_side(seq: u64) -> Option<[u64; SIDE_WORDS]> {
    let slot = &SIDE[(seq.wrapping_sub(1) % SIDE_RING as u64) as usize];
    if seq == 0 || slot.seq.load(Ordering::Acquire) != seq {
        return None;
    }
    let mut words = [0u64; SIDE_WORDS];
    for (word, cell) in words.iter_mut().zip(slot.words.iter()) {
        *word = cell.load(Ordering::Relaxed);
    }
    (slot.seq.load(Ordering::Acquire) == seq).then_some(words)
}

/// The first end of the match wins; `match_limits` clears it.
fn note_end(path: EndPath, reason: u8, fault_code: u8, caller: u64) {
    if END_PATH
        .compare_exchange(0, path as u32, Ordering::AcqRel, Ordering::Relaxed)
        .is_ok()
    {
        END_REASON.store(u32::from(reason), Ordering::Relaxed);
        END_FAULT.store(u32::from(fault_code), Ordering::Relaxed);
        END_CALLER.store(caller, Ordering::Relaxed);
    }
}

/// The fault object `[[main+0x52c3940]]`, if one exists.
unsafe fn fault_object() -> Option<u64> {
    let cell = read_u64(TEXT_BASE.load(Ordering::Relaxed) + FAULT_OBJECT as u64);
    if cell < 0x10000 {
        return None;
    }
    let object = read_u64(cell);
    (object >= 0x10000 && object.is_multiple_of(4)).then_some(object)
}

/// The stamp list's length (`[+0x200, +0x208)`, 8 bytes each), 0x3ff if implausible.
unsafe fn stamps(object: u64) -> u16 {
    let begin = read_u64(object + matchlog::FAULT_STAMPS_BEGIN_AT);
    let end = read_u64(object + matchlog::FAULT_STAMPS_END_AT);
    if end >= begin && (end - begin) / 8 < 0x3ff {
        ((end - begin) / 8) as u16
    } else {
        0x3ff
    }
}

/// The lockstep session's current frame, if a session exists.
unsafe fn session_frame() -> Option<u16> {
    let holder = read_u64(TEXT_BASE.load(Ordering::Relaxed) + matchlog::LOCKSTEP_HOLDER as u64);
    if holder < 0x10000 {
        return None;
    }
    let session = read_u64(holder + 8);
    (session >= 0x10000).then(|| read_u16(session + matchlog::SESSION_FRAME_AT))
}

/// Match setup, while the fault table's limits are copied (x19 = the object, x8 = the chosen block). Starts the
/// match's counts; logs `match_start` only when the table is armed (an online melee mode).
#[skyline::hook(offset = matchlog::MATCH_LIMITS_OFFSET, inline)]
unsafe fn desync_detector_match_limits(ctx: &InlineCtx) {
    for count in &KO_COUNTS {
        count.store(0, Ordering::Relaxed);
    }
    KO_PENDING_A.store(0, Ordering::Relaxed);
    END_PATH.store(0, Ordering::Release);
    PEER_PATH_LAST.store(0, Ordering::Relaxed);
    let (object, block) = (ctx.registers[19].x(), ctx.registers[8].x());
    if object < 0x10000 || block < 0x10000 {
        return;
    }
    let mask = (read_u64(object) & 0x3ff) as u16;
    if mask == 0 {
        return;
    }
    let text = TEXT_BASE.load(Ordering::Relaxed);
    let mut rules = [0u8; matchlog::RULE_BYTES];
    for (i, byte) in rules.iter_mut().enumerate() {
        *byte = read_u8(text + matchlog::SETUP_BLOCK as u64 + i as u64);
    }
    let offset = block.wrapping_sub(read_u64(object + matchlog::FAULT_PARAM_AT));
    let start = MatchStart {
        melee: rules[0],
        mode: read_u32(text + matchlog::MODE_WORD as u64),
        mask,
        watched: read_u32(object + matchlog::FAULT_WATCHED_AT) as u8,
        watched_count: read_u32(object + matchlog::FAULT_WATCHED_COUNT_AT) as u8,
        block: if offset < 0x100 { offset as u8 } else { 0 },
        seconds: f32::from_bits(read_u32(block)) as u32,
        n: read_u8(block + 4),
        l: read_u8(block + 5),
        rules,
    };
    publish_side(EV_MATCH_START, &start.words());
}

/// `.1363420`'s entry: w1 = the KO'd entry, w2 = the kind. Stages the before-state while the table is armed.
#[skyline::hook(offset = matchlog::KO_ENTRY_OFFSET, inline)]
unsafe fn desync_detector_ko_entry(ctx: &InlineCtx) {
    KO_PENDING_A.store(0, Ordering::Relaxed);
    let Some(object) = fault_object() else {
        return;
    };
    if read_u64(object) & 0x3ff == 0 {
        return;
    }
    let ko = Ko {
        entry: ctx.registers[1].w() as u8,
        kind: ctx.registers[2].w() as u8,
        frame: session_frame(),
        stamps_before: stamps(object),
        stamps_after: 0,
        word2_before: read_u32(object + matchlog::FAULT_WORD2_AT),
        word2_after: 0,
    };
    let (a, b) = pack_ko(&ko);
    KO_PENDING_B.store(b, Ordering::Relaxed);
    KO_PENDING_A.store(a | 1 << 63, Ordering::Relaxed);
}

/// `.1363420`'s epilogue (every return): the after-state of a staged KO.
#[skyline::hook(offset = matchlog::KO_EXIT_OFFSET, inline)]
unsafe fn desync_detector_ko_exit(_ctx: &InlineCtx) {
    let a = KO_PENDING_A.swap(0, Ordering::Relaxed);
    if a & 1 << 63 == 0 {
        return;
    }
    let mut ko = unpack_ko(a & !(1 << 63), KO_PENDING_B.load(Ordering::Relaxed));
    if let Some(object) = fault_object() {
        ko.stamps_after = stamps(object);
        ko.word2_after = read_u32(object + matchlog::FAULT_WORD2_AT);
    }
    if let Some(count) = KO_COUNTS.get(usize::from(ko.entry)) {
        count.fetch_add(1, Ordering::Relaxed);
    }
    let (a, b) = pack_ko(&ko);
    push_event(EV_KO, a, b);
}

fn note_peer_path(path: u8) {
    PEER_PATH_LAST.store(u32::from(path), Ordering::Relaxed);
    PEER_PATH_TICK.store(tick(), Ordering::Release);
}

/// Peer-end path A, the receive handler's tail into `.16cced0` (x19 = session, x20 = the message: `.16cc140` keeps
/// x2 there, as at `PEER_ABORT`; w1 = the reason).
#[skyline::hook(offset = matchlog::PEER_PATH_A_OFFSET, inline)]
unsafe fn desync_detector_peer_path_a(ctx: &InlineCtx) {
    let (session, message) = (ctx.registers[19].x(), ctx.registers[20].x());
    if session < 0x10000 {
        return;
    }
    note_peer_path(PATH_A);
    let path = PeerPath {
        path: PATH_A,
        station: read_u8(session + matchlog::SESSION_STATION_AT),
        frame: Some(read_u16(session + matchlog::SESSION_FRAME_AT)),
        peer_mask: read_u16(session + matchlog::SESSION_PEER_MASK_AT),
        end_reason: ctx.registers[1].w() as u8,
        message: (message >= 0x10000)
            .then(|| (read_u8(message), u16::from_be_bytes([read_u8(message + 1), read_u8(message + 2)]))),
        caller: 0,
    };
    let (a, b) = pack_peer_path(&path);
    push_event(EV_PEER_PATH, a, b);
}

/// Peer-end path B, the abort function (x0 = session, x30 = its caller).
#[skyline::hook(offset = matchlog::PEER_PATH_B_OFFSET, inline)]
unsafe fn desync_detector_peer_path_b(ctx: &InlineCtx) {
    let session = ctx.registers[0].x();
    if session < 0x10000 {
        return;
    }
    note_peer_path(PATH_B);
    let path = PeerPath {
        path: PATH_B,
        station: read_u8(session + matchlog::SESSION_STATION_AT),
        frame: Some(read_u16(session + matchlog::SESSION_FRAME_AT)),
        peer_mask: read_u16(session + matchlog::SESSION_PEER_MASK_AT),
        end_reason: 0,
        message: None,
        caller: ctx.registers[30].x().wrapping_sub(TEXT_BASE.load(Ordering::Relaxed)),
    };
    let (a, b) = pack_peer_path(&path);
    push_event(EV_PEER_PATH, a, b);
}

/// `.16cced0`'s entry: phase 6 (x0 = session, w1 = reason, x30 = caller).
#[skyline::hook(offset = matchlog::SESSION_END_OFFSET, inline)]
unsafe fn desync_detector_session_end(ctx: &InlineCtx) {
    let session = ctx.registers[0].x();
    if session < 0x10000 {
        return;
    }
    let caller = ctx.registers[30].x().wrapping_sub(TEXT_BASE.load(Ordering::Relaxed));
    let end = SessionEnd {
        reason: ctx.registers[1].w() as u8,
        fault_code: read_u8(session + matchlog::SESSION_FAULT_CODE_AT),
        phase_before: read_u8(session + matchlog::SESSION_PHASE_AT),
        already_ending: read_u8(session + matchlog::SESSION_ENDING_AT),
        frame: Some(read_u16(session + matchlog::SESSION_FRAME_AT)),
        caller,
    };
    let recent = if tick().saturating_sub(PEER_PATH_TICK.load(Ordering::Acquire)) <= matchlog::PEER_PATH_WINDOW_TICKS {
        PEER_PATH_LAST.load(Ordering::Relaxed) as u8
    } else {
        0
    };
    note_end(phase6_path(caller, recent), end.reason, end.fault_code, caller);
    let (a, b) = pack_session_end(&end);
    push_event(EV_SESSION_END, a, b);
}

/// `.16d5cd0`'s entry: the session stops. One summary line per session.
#[skyline::hook(offset = matchlog::SESSION_STOP_OFFSET, inline)]
unsafe fn desync_detector_session_stop(ctx: &InlineCtx) {
    let session = ctx.registers[0].x();
    if session < 0x10000 {
        return;
    }
    let (_, words) = read_fault_table();
    let mut kos = [0u8; 8];
    for (ko, count) in kos.iter_mut().zip(KO_COUNTS.iter()) {
        *ko = count.load(Ordering::Relaxed).min(255) as u8;
    }
    let summary = Summary {
        frame: Some(read_u16(session + matchlog::SESSION_FRAME_AT)),
        kos,
        fault_words: words,
        ended: EndPath::from_code(END_PATH.load(Ordering::Acquire) as u8),
        end_reason: END_REASON.load(Ordering::Relaxed) as u8,
        end_fault_code: END_FAULT.load(Ordering::Relaxed) as u8,
        end_caller: END_CALLER.load(Ordering::Relaxed),
    };
    publish_side(EV_SUMMARY, &summary.words());
    PEER_PATH_LAST.store(0, Ordering::Relaxed);
}

/// `ssbu-stall-press-carry`'s counters, through its export (writer thread only).
fn stall_stats_now() -> Option<StallStatsV1> {
    let mut address = STALL_EXPORT.load(Ordering::Acquire);
    if address == 0 {
        let mut found = 0usize;
        unsafe { nn::ro::LookupSymbol(&mut found, matchlog::STALL_SYMBOL.as_ptr()) };
        address = found;
        STALL_EXPORT.store(address, Ordering::Release);
    }
    if address == 0 {
        return None;
    }
    // SAFETY: a found address is that export, whose ABI matchlog::STALL_SYMBOL declares; `stats` is writable.
    let answer = unsafe {
        let f: unsafe extern "C" fn(*mut StallStatsV1) -> bool = core::mem::transmute(address);
        let mut stats = StallStatsV1::default();
        f(&mut stats).then_some(stats)
    };
    matchlog::stall_stats(answer)
}

#[skyline::hook(offset = model::PLAYBACK_CHECK_OFFSET, inline)]
unsafe fn desync_detector_playback_check(ctx: &InlineCtx) {
    let (expected, game) = (ctx.registers[22].x(), ctx.registers[27].x());
    if expected < 0x10000 || game < 0x10000 {
        return;
    }
    let player = read_u64(game + REPLAY_PLAYER as u64);
    let index = if player < 0x10000 {
        0
    } else {
        read_u32(player + REPLAY_FRAME_INDEX as u64).wrapping_sub(1)
    };
    let (computed, recorded) = (ctx.registers[8].w(), read_u32(expected + 4));
    DISCONNECT_EVIDENCE.frame(tick(), KIND_PLAYBACK, game, index);
    push_frame(FrameRecord {
        frame: index,
        rng: rng_check_now(),
        position: computed,
        recorded,
        flags: KIND_PLAYBACK,
    });
    if computed != recorded && PLAYBACK_MISMATCHES.fetch_add(1, Ordering::AcqRel) < 8 {
        push_event(
            EV_PLAYBACK_MISMATCH,
            u64::from(computed) | u64::from(recorded) << 32,
            u64::from(index),
        );
    }
}

type Path = Line<96>;

fn path(args: core::fmt::Arguments) -> Path {
    let mut path = Path::new();
    let _ = path.write_fmt(args);
    path
}

/// An `nn::fs` file opened for writing; closed on drop.
struct NxFile {
    handle: nn::fs::FileHandle,
    offset: i64,
}

impl NxFile {
    /// Opens `path` for writing at its end, creating it when missing.
    fn append(path: &mut Path) -> Option<Self> {
        let path = path.with_nul()?.as_ptr();
        unsafe {
            // Fails harmlessly when the file exists.
            nn::fs::CreateFile(path, 0);
            let mut handle = nn::fs::FileHandle { handle: 0 };
            if nn::fs::OpenFile(&mut handle, path, OPEN_WRITE_APPEND) != 0 {
                return None;
            }
            let mut offset = 0;
            if nn::fs::GetFileSize(&mut offset, handle) != 0 {
                nn::fs::CloseFile(handle);
                return None;
            }
            Some(Self { handle, offset })
        }
    }

    /// Opens `path` empty.
    fn create(path: &mut Path) -> Option<Self> {
        let mut file = Self::append(path)?;
        if file.offset != 0 {
            file.truncate()?;
        }
        Some(file)
    }

    fn truncate(&mut self) -> Option<()> {
        (unsafe { nn::fs::SetFileSize(self.handle, 0) } == 0).then(|| self.offset = 0)
    }

    fn write(&mut self, bytes: &[u8]) -> bool {
        let option = nn::fs::WriteOption { flags: 0 };
        let written = unsafe {
            nn::fs::WriteFile(
                self.handle,
                self.offset,
                bytes.as_ptr(),
                bytes.len() as u64,
                &option,
            )
        } == 0;
        if written {
            self.offset += bytes.len() as i64;
        }
        written
    }

    fn flush(&self) {
        unsafe { nn::fs::FlushFile(self.handle) };
    }
}

impl Drop for NxFile {
    fn drop(&mut self) {
        unsafe {
            nn::fs::FlushFile(self.handle);
            nn::fs::CloseFile(self.handle);
        }
    }
}

/// Appends to `events.log`. A log that would pass `EVENTS_LOG_LIMIT` becomes
/// `events.previous.log` (replacing the older one), so the two files never
/// hold more than twice the limit.
fn append_log(bytes: &[u8]) {
    let mut log_path = path(format_args!("{DATA_DIR}/events.log"));
    let Some(mut log) = NxFile::append(&mut log_path) else {
        return;
    };
    if log.offset + bytes.len() as i64 > EVENTS_LOG_LIMIT {
        drop(log);
        let mut previous = path(format_args!("{DATA_DIR}/events.previous.log"));
        delete(&mut previous);
        if let (Some(from), Some(to)) = (log_path.with_nul(), previous.with_nul()) {
            unsafe { nn::fs::RenameFile(from.as_ptr(), to.as_ptr()) };
        }
        let Some(fresh) = NxFile::create(&mut log_path) else {
            return;
        };
        log = fresh;
    }
    log.write(bytes);
}

fn delete(path: &mut Path) {
    if let Some(path) = path.with_nul() {
        unsafe { nn::fs::DeleteFile(path.as_ptr()) };
    }
}

fn read_first_byte(path: &mut Path) -> Option<u8> {
    let path = path.with_nul()?.as_ptr();
    unsafe {
        let mut handle = nn::fs::FileHandle { handle: 0 };
        if nn::fs::OpenFile(&mut handle, path, OPEN_READ) != 0 {
            return None;
        }
        let (mut byte, mut read) = (0u8, 0u64);
        let result = nn::fs::ReadFile1(&mut read, handle, 0, &mut byte, 1);
        nn::fs::CloseFile(handle);
        (result == 0 && read == 1).then_some(byte)
    }
}

/// The local time packed as replay files store it; 0 when the clock is unset.
fn packed_now() -> u64 {
    use nn::time::{
        CalendarAdditionalInfo, CalendarTime, PosixTime, StandardUserSystemClock, TimeZone,
    };
    unsafe {
        if !nn::time::IsInitialized() {
            return 0;
        }
        let mut posix = PosixTime { time: 0 };
        if StandardUserSystemClock::GetCurrentTime(&mut posix) != 0 {
            return 0;
        }
        let mut calendar = CalendarTime {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0,
        };
        let mut info = CalendarAdditionalInfo {
            dayOfTheWeek: 0,
            dayofYear: 0,
            timeZone: TimeZone::default(),
        };
        if nn::time::ToCalendarTime(&mut calendar, &mut info, &posix) != 0 {
            return 0;
        }
        model::pack_time(
            calendar.year as u16,
            calendar.month as u8,
            calendar.day as u8,
            calendar.hour as u8,
            calendar.minute as u8,
            calendar.second as u8,
        )
    }
}

const NO_RECORDER: u8 = 1;
const NO_ENTRY: u8 = 2;
const OVERFLOW: u8 = 3;
const INVALID_ENTRY: u8 = 4;
const NOT_CFMK: u8 = 5;
const LENGTH_MISMATCH: u8 = 6;
const WRITE_FAILED: u8 = 7;

/// Read-only view of the game's match recorder.
struct Recorder {
    player: u64,
    arena: u64,
}

impl Recorder {
    unsafe fn find() -> Option<Self> {
        let game = read_u64(TEXT_BASE.load(Ordering::Relaxed) + recorder::GAME_OBJECT as u64);
        if game < 0x10000 {
            return None;
        }
        let player = read_u64(game + REPLAY_PLAYER as u64);
        if player < 0x10000 {
            return None;
        }
        let arena = read_u64(player + recorder::ARENA as u64);
        (arena >= 0x10000).then_some(Self { player, arena })
    }

    unsafe fn recording(&self) -> bool {
        read_u32(self.player + recorder::MODE as u64) == recorder::MODE_RECORD
    }

    unsafe fn cursor(&self) -> u64 {
        read_u64(self.player + recorder::CURSOR as u64)
    }

    fn staging(&self) -> u64 {
        self.player + recorder::STAGING as u64
    }

    unsafe fn entries(&self) -> u64 {
        read_u64(self.arena)
    }

    /// Entry `index`: its address and size.
    unsafe fn entry(&self, index: u64) -> Result<(u64, usize), u8> {
        if read_u8(self.arena + recorder::OVERFLOW as u64) != 0 {
            return Err(OVERFLOW);
        }
        let count = self.entries();
        if index >= count || count > 0x10000 {
            return Err(NO_ENTRY);
        }
        let descriptor =
            self.arena + recorder::DESCRIPTORS as u64 - recorder::DESCRIPTOR_STRIDE as u64 * index;
        let (offset, size) = (read_u64(descriptor), read_u64(descriptor + 8));
        if read_u8(descriptor + 16) & 1 == 0
            || size < CFMK_BYTES as u64
            || offset.saturating_add(size) > recorder::ARENA_BYTES as u64
        {
            return Err(INVALID_ENTRY);
        }
        let data = self.arena + recorder::ARENA_DATA as u64 + offset;
        if read_u32(data + 4) != TAG_CFMK || usize::from(read_u16(data + 0x10)) != CFMK_BYTES {
            return Err(NOT_CFMK);
        }
        Ok((data, size as usize))
    }

    /// Whether the end-of-match commit has appended the `FRAM` chunk.
    unsafe fn committed(data: u64, size: usize) -> bool {
        let mut at = CFMK_BYTES;
        while at + 8 <= size {
            if read_u32(data + at as u64) == TAG_FRAM {
                return true;
            }
            at += 8 + read_u32(data + at as u64 + 4) as usize;
        }
        false
    }

    /// The outer header the game built for entry `index` when it finalised
    /// the match (a `std::map` keyed by entry index).
    unsafe fn outer(&self, index: u64) -> Option<u64> {
        let mut node = read_u64(self.arena + recorder::OUTER_MAP_ROOT as u64);
        for _ in 0..64 {
            if node < 0x10000 {
                return None;
            }
            let key = read_u64(node + recorder::NODE_KEY as u64);
            if key == index {
                let object = read_u64(node + recorder::NODE_OBJECT as u64);
                return (object >= 0x10000).then_some(object + recorder::OBJECT_OUTER as u64);
            }
            node = read_u64(node + if index < key { 0 } else { 8 });
        }
        None
    }
}

unsafe fn game_bytes<'a>(address: u64, len: usize) -> &'a [u8] {
    core::slice::from_raw_parts(address as *const u8, len)
}

/// An online session's match recording, waiting to be saved as `match-N.bin`.
struct Pending {
    slot: usize,
    /// The recorder cursor when the session started; the entry is the one
    /// being written once the cursor moves.
    start_cursor: u64,
    entry: Option<u64>,
    /// When the lockstep session ended (0 until then).
    ended: u64,
    committed_seen: u64,
    staged_saved: bool,
    /// How the session ended; a normal end does not keep its recording.
    end: EndState,
}

/// The writer thread's files and rotation state. `events.log` is opened only
/// to append each batch of lines: a closed file is on the card at once, while
/// Eden keeps an open file's writes until the host file is closed (its
/// `FlushFile` does not reach the host).
struct Writer {
    frames: Option<NxFile>,
    slot: usize,
    in_session: bool,
    session_kind: u32,
    last_frame: u32,
    frame_tail: usize,
    event_tail: u64,
    link: (u64, u64),
    last_poll_tick: u64,
    poll_gap_ticks: u64,
    poll_count: u64,
    last_telemetry: u64,
    event_drops: u64,
    frame_drops: u64,
    pending: Option<Pending>,
}

impl Writer {
    fn open() -> Option<Self> {
        // The data folder must be writable (the install line created it).
        NxFile::append(&mut path(format_args!("{DATA_DIR}/events.log")))?;
        let previous = match read_first_byte(&mut path(format_args!("{DATA_DIR}/next-slot.txt"))) {
            Some(digit) if digit.is_ascii_digit() => usize::from(digit - b'0') % SLOTS,
            _ => SLOTS - 1,
        };
        Some(Self {
            frames: None,
            slot: previous,
            in_session: false,
            session_kind: 0,
            last_frame: 0,
            frame_tail: 0,
            event_tail: 0,
            link: (u64::MAX, u64::MAX),
            last_poll_tick: 0,
            poll_gap_ticks: 0,
            poll_count: 0,
            last_telemetry: 0,
            event_drops: 0,
            frame_drops: 0,
            pending: None,
        })
    }

    fn start_session(&mut self, kind: u32) {
        if self.pending.is_some() {
            // A new match reuses the staging buffer; keep what was saved.
            self.finish_pending(true);
        }
        self.slot = (self.slot + 1) % SLOTS;
        let slot = self.slot;
        self.frames = NxFile::create(&mut path(format_args!("{DATA_DIR}/frames-{slot}.bin")));
        delete(&mut path(format_args!("{DATA_DIR}/match-{slot}.bin")));
        if let Some(mut next) = NxFile::create(&mut path(format_args!("{DATA_DIR}/next-slot.txt")))
        {
            next.write(&[b'0' + slot as u8]);
        }
        self.in_session = true;
        self.session_kind = kind;
        PLAYBACK_MISMATCHES.store(0, Ordering::Release);
        if kind == KIND_ONLINE {
            self.pending = Some(Pending {
                slot,
                start_cursor: unsafe { Recorder::find().map_or(0, |r| r.cursor()) },
                entry: None,
                ended: 0,
                committed_seen: 0,
                staged_saved: false,
                end: EndState::default(),
            });
        }
        push_event(
            EV_SESSION,
            slot as u64,
            u64::from(kind) | HEAP_ALLOCATIONS.load(Ordering::Relaxed) << 8,
        );
    }

    fn write_frames(&mut self, bytes: &[u8]) {
        if let Some(file) = self.frames.as_mut().filter(|_| !bytes.is_empty()) {
            file.write(bytes);
        }
    }

    fn drain_frames(&mut self) {
        let head = FRAME_HEAD.load(Ordering::Acquire);
        if head.saturating_sub(self.frame_tail) > FRAME_RING {
            self.frame_drops = self
                .frame_drops
                .saturating_add((head - self.frame_tail - FRAME_RING) as u64);
            TELEMETRY_REQUEST.store(true, Ordering::Release);
            self.frame_tail = head - FRAME_RING;
        }
        let mut chunk = [0u8; RECORD_BYTES * 64];
        let mut used = 0;
        while self.frame_tail < head {
            let cells = &FRAMES[self.frame_tail % FRAME_RING];
            let word = |i: usize| cells[i].load(Ordering::Relaxed);
            let record = FrameRecord {
                frame: word(0),
                rng: word(1),
                position: word(2),
                recorded: word(3),
                flags: word(4),
            };
            self.frame_tail += 1;
            let kind = record.kind();
            let restarted = kind == KIND_ONLINE && record.frame + 60 < self.last_frame;
            if !self.in_session || kind != self.session_kind || restarted {
                self.write_frames(&chunk[..used]);
                used = 0;
                self.start_session(kind);
            }
            self.last_frame = record.frame;
            chunk[used..used + RECORD_BYTES].copy_from_slice(&record.to_bytes());
            used += RECORD_BYTES;
            if used == chunk.len() {
                self.write_frames(&chunk);
                used = 0;
            }
        }
        self.write_frames(&chunk[..used]);
        let last = LAST_FRAME_TICK.load(Ordering::Relaxed);
        if self.in_session && tick().saturating_sub(last) > SESSION_GAP_TICKS {
            push_event(
                model::EV_FRAME_GAP,
                u64::from(self.last_frame)
                    | u64::from(self.session_kind) << 32
                    | (self.slot as u64) << 40,
                last,
            );
            TELEMETRY_REQUEST.store(true, Ordering::Release);
            self.in_session = false;
            self.frames = None;
            if let Some(pending) = self.pending.as_mut().filter(|p| p.ended == 0) {
                pending.ended = last;
            }
        }
    }

    fn drain_events(&mut self) {
        flush_stale_fault();
        let head = EVENT_HEAD.load(Ordering::Acquire);
        if head.saturating_sub(self.event_tail) > EVENT_RING as u64 {
            self.event_drops = self
                .event_drops
                .saturating_add(head - self.event_tail - EVENT_RING as u64);
            TELEMETRY_REQUEST.store(true, Ordering::Release);
            self.event_tail = head - EVENT_RING as u64;
        }
        let mut line = Line::<512>::new();
        let mut batch = [0u8; 2048];
        let mut used = 0;
        while self.event_tail < head {
            let slot = &EVENTS[(self.event_tail % EVENT_RING as u64) as usize];
            if slot.seq.load(Ordering::Acquire) != self.event_tail + 1 {
                break;
            }
            let event = RawEvent {
                tick: slot.tick.load(Ordering::Relaxed),
                code: slot.code.load(Ordering::Relaxed) as u8,
                a: slot.a.load(Ordering::Relaxed),
                b: slot.b.load(Ordering::Relaxed),
            };
            self.event_tail += 1;
            if let Some(pending) = self.pending.as_mut() {
                match event.code {
                    EV_SYNC_END => {
                        pending
                            .end
                            .sync_end((event.a & 0xff) as u32, event.a >> 8 & 1 != 0);
                        if pending.ended == 0 {
                            pending.ended = event.tick;
                        }
                    }
                    EV_DESYNC => pending.end.desync(),
                    EV_PEER_ABORT => pending.end.peer_end(event.a >> 8 & 0xffff),
                    _ => {}
                }
            }
            line.clear();
            if event.code == EV_FAULT {
                let mut view = unpack_fault(event.a);
                if event.b != 0 {
                    let slot = &FAULT_LOG[((event.b - 1) % FAULT_LOG_RING as u64) as usize];
                    if slot.seq.load(Ordering::Acquire) == event.b
                        && slot.words_ok.load(Ordering::Relaxed) == 1
                    {
                        let mut words = [0; FAULT_WORD_COUNT];
                        for (word, cell) in words.iter_mut().zip(slot.words.iter()) {
                            *word = cell.load(Ordering::Relaxed);
                        }
                        view.words = Some(words);
                    }
                }
                let _ = describe_fault(event.tick, &view, &mut line);
            } else if event.code == EV_MATCH_START || event.code == EV_SUMMARY {
                match read_side(event.a) {
                    Some(words) if event.code == EV_MATCH_START => {
                        let _ = describe_match_start(event.tick, &MatchStart::from_words(&words), &mut line);
                    }
                    Some(words) => {
                        let stall = stall_stats_now();
                        let _ = describe_summary(event.tick, &Summary::from_words(&words), stall.as_ref(), &mut line);
                    }
                    None => {
                        let _ = write!(line, "{} side_record_lost code={}", event.tick, event.code);
                    }
                }
            } else if event.code == EV_KO {
                let _ = describe_ko(event.tick, &unpack_ko(event.a, event.b), &mut line);
            } else if event.code == EV_PEER_PATH {
                let _ = describe_peer_path(event.tick, &unpack_peer_path(event.a, event.b), &mut line);
            } else if event.code == EV_SESSION_END {
                let _ = describe_session_end(event.tick, &unpack_session_end(event.a, event.b), &mut line);
            } else {
                let _ = describe(&event, &mut line);
            }
            let _ = line.write_char('\n');
            let text = line.as_bytes();
            if used + text.len() > batch.len() {
                append_log(&batch[..used]);
                used = 0;
            }
            batch[used..used + text.len()].copy_from_slice(text);
            used += text.len();
        }
        if used > 0 {
            append_log(&batch[..used]);
        }
    }

    fn poll_link(&mut self) -> bool {
        let mut raw = [0u8; 12];
        let result = unsafe { GetInternetConnectionStatus(raw.as_mut_ptr()) };
        let a = u64::from(raw[0]) | u64::from(raw[4]) << 32;
        let b = u64::from(result)
            | u64::from(u32::from_le_bytes([raw[8], raw[9], raw[10], raw[11]])) << 32;
        let now = tick();
        self.poll_gap_ticks = if self.last_poll_tick == 0 {
            0
        } else {
            now.saturating_sub(self.last_poll_tick)
        };
        self.last_poll_tick = now;
        self.poll_count = self.poll_count.saturating_add(1);
        DISCONNECT_EVIDENCE.link(now, a, b);
        if (a, b) != self.link {
            self.link = (a, b);
            publish_event(&RawEvent {
                tick: now,
                code: EV_LINK,
                a,
                b,
            });
            return true;
        }
        false
    }

    /// Runs only on the existing writer. Five-second heartbeats plus event
    /// snapshots preserve coverage and a console-clock anchor across gaps.
    fn telemetry(&mut self) {
        let start = tick();
        let initialized = unsafe { nn::time::IsInitialized() };
        let mut posix = nn::time::PosixTime { time: 0 };
        let result = if initialized {
            nn::time::StandardUserSystemClock::GetCurrentTime(&mut posix)
        } else {
            0
        };
        let span = tick().saturating_sub(start);
        let sample = Telemetry {
            tick: start,
            boot_tick: BOOT_TICK.load(Ordering::Relaxed),
            unix_seconds: posix.time,
            clock_initialized: initialized,
            clock_result: result,
            clock_query_ticks: span,
            poll: self.poll_count,
            poll_gap_ticks: self.poll_gap_ticks,
            link: self.link,
            active: self.in_session,
            kind: self.session_kind,
            slot: self.slot,
            frame: self.last_frame,
            last_frame_tick: LAST_FRAME_TICK.load(Ordering::Relaxed),
            event_drops: self.event_drops,
            frame_drops: self.frame_drops,
            evidence_losses: DISCONNECT_EVIDENCE.losses(),
            heap_allocations: HEAP_ALLOCATIONS.load(Ordering::Relaxed),
        };
        let mut line = Line::<768>::new();
        let _ = model::describe_telemetry(&sample, &mut line);
        let _ = line.write_char('\n');
        append_log(line.as_bytes());
        self.last_telemetry = start;
    }

    /// Advances the pending recording; drops it once it is saved or lost.
    fn poll_pending(&mut self) {
        if let Some(pending) = self.pending.as_mut() {
            if pending.entry.is_none() {
                if let Some(recorder) = unsafe { Recorder::find() } {
                    unsafe {
                        if recorder.recording()
                            && recorder.cursor() != pending.start_cursor
                            && recorder.entries() > 0
                        {
                            pending.entry = Some(recorder.entries() - 1);
                        }
                    }
                }
            }
            if pending.ended == 0 || tick().saturating_sub(pending.ended) < STAGED_DELAY_TICKS {
                return;
            }
        }
        if self.pending.is_some() {
            self.finish_pending(false);
        }
    }

    /// Saves the recording if it can; with `forced` (a new match started),
    /// this is the last chance and the staged buffer is no longer this match.
    fn finish_pending(&mut self, forced: bool) {
        let Some(mut pending) = self.pending.take() else {
            return;
        };
        if !pending.end.keep_recording() {
            let (a, b) = pack_recording(
                pending.slot as u8,
                pending.entry.unwrap_or(0) as u32,
                0,
                0,
                false,
                false,
                RECORDING_NORMAL_END,
            );
            push_event(EV_RECORDING, a, b);
            return;
        }
        let now = tick();
        let result = unsafe { self.try_save(&mut pending, now, forced) };
        let keep = match result {
            Ok(Some((bytes, records, committed, game_outer))) => {
                let (a, b) = pack_recording(
                    pending.slot as u8,
                    pending.entry.unwrap_or(0) as u32,
                    bytes,
                    records,
                    committed,
                    game_outer,
                    0,
                );
                push_event(EV_RECORDING, a, b);
                if !committed {
                    pending.staged_saved = true;
                }
                !committed && !forced
            }
            Ok(None) => !forced,
            Err(reason) => {
                if !pending.staged_saved {
                    let (a, b) = pack_recording(
                        pending.slot as u8,
                        pending.entry.unwrap_or(0) as u32,
                        0,
                        0,
                        false,
                        false,
                        reason,
                    );
                    push_event(EV_RECORDING, a, b);
                }
                false
            }
        };
        if keep && now.saturating_sub(pending.ended) < COMMIT_WAIT_TICKS {
            self.pending = Some(pending);
        }
    }

    /// `Ok(Some(..))` = a file was written, `Ok(None)` = wait (the staged copy
    /// is saved; the commit or its outer header is still to come).
    unsafe fn try_save(
        &mut self,
        pending: &mut Pending,
        now: u64,
        forced: bool,
    ) -> Result<Option<(u32, u16, bool, bool)>, u8> {
        let recorder = Recorder::find().ok_or(NO_RECORDER)?;
        let index = pending.entry.ok_or(NO_ENTRY)?;
        let (data, size) = recorder.entry(index)?;
        if Recorder::committed(data, size) {
            let outer = recorder.outer(index);
            if pending.committed_seen == 0 {
                pending.committed_seen = now;
            }
            if outer.is_none() && !forced && now - pending.committed_seen < OUTER_WAIT_TICKS {
                return Ok(None);
            }
            let mut header = [0u8; CFMK_BYTES];
            header.copy_from_slice(game_bytes(data, CFMK_BYTES));
            let chunks = game_bytes(data + CFMK_BYTES as u64, size - CFMK_BYTES);
            let records = u16::from_le_bytes([header[CFMK_RECORDS], header[CFMK_RECORDS + 1]]);
            let bytes = self.save(pending.slot, outer, &mut header, &[chunks])?;
            return Ok(Some((bytes, records, true, outer.is_some())));
        }
        if pending.staged_saved {
            return Ok(None);
        }
        // The game may still append records: take the header and the cursor
        // together, so the header's payload length matches what is copied.
        // Once a new match starts recording, the cursor no longer matches the
        // old header and nothing is saved.
        let mut header = [0u8; CFMK_BYTES];
        for _ in 0..8 {
            let cursor = recorder.cursor();
            header.copy_from_slice(game_bytes(data, CFMK_BYTES));
            let len = cursor.wrapping_sub(recorder.staging());
            let stated = u64::from(u32::from_le_bytes([
                header[CFMK_PAYLOAD_LEN],
                header[CFMK_PAYLOAD_LEN + 1],
                header[CFMK_PAYLOAD_LEN + 2],
                header[CFMK_PAYLOAD_LEN + 3],
            ]));
            if recorder.cursor() == cursor
                && stated == len
                && len > 0
                && len <= recorder::STAGING_BYTES as u64
            {
                let mut chunk = [0u8; 8];
                chunk[..4].copy_from_slice(&TAG_FRAM.to_le_bytes());
                chunk[4..].copy_from_slice(&(len as u32).to_le_bytes());
                let payload = game_bytes(recorder.staging(), len as usize);
                let records = u16::from_le_bytes([header[CFMK_RECORDS], header[CFMK_RECORDS + 1]]);
                let bytes = self.save(pending.slot, None, &mut header, &[&chunk, payload])?;
                return Ok(Some((bytes, records, false, false)));
            }
            nn::os::SleepThread(nn::TimeSpan {
                nanoseconds: 5_000_000,
            });
        }
        Err(LENGTH_MISMATCH)
    }

    /// Writes `match-N.bin`: the outer header (the game's, or a minimal one),
    /// then the entry with its XOR word resealed over `pieces`.
    unsafe fn save(
        &mut self,
        slot: usize,
        outer: Option<u64>,
        header: &mut [u8; CFMK_BYTES],
        pieces: &[&[u8]],
    ) -> Result<u32, u8> {
        let outer = match outer {
            Some(address) => {
                let mut copy = [0u8; OUTER_BYTES];
                copy.copy_from_slice(game_bytes(address, OUTER_BYTES));
                copy
            }
            None => minimal_outer(packed_now()),
        };
        let word = pieces
            .iter()
            .fold(xor_words(&header[4..]), |acc, piece| acc ^ xor_words(piece));
        header[..4].copy_from_slice(&word.to_le_bytes());
        let mut file_path = path(format_args!("{DATA_DIR}/match-{slot}.bin"));
        let mut file = NxFile::create(&mut file_path).ok_or(WRITE_FAILED)?;
        let mut ok = file.write(&outer) && file.write(&header[..]);
        for piece in pieces {
            for part in piece.chunks(64 * 1024) {
                ok = ok && file.write(part);
            }
        }
        let bytes = file.offset as u32;
        drop(file);
        if !ok {
            delete(&mut file_path);
            return Err(WRITE_FAILED);
        }
        Ok(bytes)
    }

    fn run(mut self) -> ! {
        let mut iteration = 0u64;
        loop {
            unsafe {
                nn::os::SleepThread(nn::TimeSpan {
                    nanoseconds: 100_000_000,
                })
            };
            self.drain_frames();
            let urgent = TELEMETRY_REQUEST.swap(false, Ordering::AcqRel);
            let changed = (iteration.is_multiple_of(5) || urgent) && self.poll_link();
            self.poll_pending();
            self.drain_events();
            if urgent
                || changed
                || tick().saturating_sub(self.last_telemetry) >= TELEMETRY_INTERVAL_TICKS
            {
                self.telemetry();
            }
            if iteration.is_multiple_of(10) {
                if let Some(frames) = self.frames.as_ref() {
                    frames.flush();
                }
            }
            iteration += 1;
        }
    }
}

extern "C" fn writer_main(_: *mut skyline::libc::c_void) {
    if let Some(writer) = Writer::open() {
        writer.run();
    }
}

fn start_writer() -> bool {
    unsafe {
        let thread = WRITER_THREAD.0.get().cast::<nn::os::ThreadType>();
        let priority = nn::os::GetThreadPriority(nn::os::GetCurrentThread());
        let created = nn::os::CreateThread(
            thread,
            writer_main,
            core::ptr::null_mut(),
            WRITER_STACK_MEMORY.0.get().cast::<u8>(),
            WRITER_STACK,
            priority,
        );
        if created != 0 {
            return false;
        }
        nn::os::SetThreadNamePointer(thread, c"desync-detector".as_ptr().cast());
        nn::os::StartThread(thread);
        true
    }
}

unsafe fn word_matches(offset: usize, expected: u32) -> bool {
    let base = getRegionAddress(Region::Text) as usize;
    let end = getRegionAddress(Region::Rodata) as usize;
    let address = base + offset;
    address.is_multiple_of(4)
        && address + 4 <= end
        && core::ptr::read_unaligned(address as *const u32) == expected
}

unsafe fn guard_matches(site: &Site) -> bool {
    let base = getRegionAddress(Region::Text) as usize;
    let end = getRegionAddress(Region::Rodata) as usize;
    let address = base + site.offset;
    address.is_multiple_of(4)
        && address + site.guard.len() <= end
        && core::slice::from_raw_parts(address as *const u8, site.guard.len()) == site.guard
}

fn version_is_supported() -> bool {
    let mut version = nn::oe::DisplayVersion { name: [0; 16] };
    unsafe { nn::oe::GetDisplayVersion(&mut version) };
    let end = version.name.iter().position(|v| *v == 0).unwrap_or(16);
    version.name[..end] == *model::SUPPORTED_VERSION.as_bytes()
}

/// Appends the install line to `events.log` and the Skyline log.
fn report_install(report: &mut Line<512>) {
    let _ = write!(
        report,
        " heap_allocations={}",
        HEAP_ALLOCATIONS.load(Ordering::Relaxed)
    );
    skyline::logging::log(core::str::from_utf8(report.as_bytes()).unwrap_or(""));
    let _ = report.write_char('\n');
    for directory in ["sd:/ultimate", DATA_DIR] {
        if let Some(directory) = path(format_args!("{directory}")).with_nul() {
            unsafe { nn::fs::CreateDirectory(directory.as_ptr()) };
        }
    }
    append_log(report.as_bytes());
}

#[skyline::main(name = "ssbu-desync-detector")]
pub fn main() {
    let started = tick();
    BOOT_TICK.store(started, Ordering::Relaxed);
    let mut report = Line::<512>::new();
    let _ = write!(
        report,
        "{} plugin=ssbu-desync-detector/{} telemetry=1",
        started,
        env!("CARGO_PKG_VERSION")
    );
    if !version_is_supported() {
        let _ = write!(report, " result=unsupported_version");
        report_install(&mut report);
        return;
    }
    TEXT_BASE.store(
        unsafe { getRegionAddress(Region::Text) as u64 },
        Ordering::Relaxed,
    );
    #[cfg(feature = "diag")]
    if let Some(frame) = read_diag().force_desync_frame {
        FORCE_DESYNC_FRAME.store(u64::from(frame), Ordering::Relaxed);
    }
    let _ = write!(report, " hooks=");
    let lockstep = [&FRAME_CHECK, &DESYNC, &SYNC_END, &PEER_ABORT]
        .iter()
        .all(|site| unsafe { guard_matches(site) });
    if lockstep {
        skyline::install_hooks!(
            desync_detector_frame_check,
            desync_detector_desync,
            desync_detector_sync_end,
            desync_detector_peer_end
        );
        let _ = write!(report, "lockstep");
    }
    if unsafe { guard_matches(&PLAYBACK_CHECK) } {
        skyline::install_hook!(desync_detector_playback_check);
        let _ = write!(report, "{}playback", if lockstep { "," } else { "" });
    }
    let fault_ready = unsafe {
        guard_matches(&FAULT_ARM)
            && word_matches(FAULT_CALL_OFFSET, FAULT_CALL_WORD)
            && word_matches(FAULT_STORE_OFFSET, FAULT_STORE_WORD)
            && word_matches(FAULT_SESSION_OFFSET, FAULT_SESSION_WORD)
    };
    if fault_ready {
        skyline::install_hook!(desync_detector_fault_arm);
    }
    let _ = write!(
        report,
        " fault={}",
        if fault_ready { "installed" } else { "skipped" }
    );
    // Each text route goes in only when all of its own guards match.
    let mut installed = [false; dialog::ROUTES.len()];
    for (ready, route) in installed.iter_mut().zip(dialog::ROUTES) {
        *ready = true;
        for site in route.guards {
            if !unsafe { guard_matches(site) } {
                *ready = false;
                let _ = write!(report, " dialog_guard_failed={}", site.name);
            }
        }
    }
    if installed[dialog::ARGS.code as usize] {
        skyline::install_hook!(desync_detector_communication_text);
    }
    if installed[dialog::PLAIN.code as usize] {
        skyline::install_hook!(desync_detector_plain_text);
    }
    let count = installed.iter().filter(|ready| **ready).count();
    let _ = write!(
        report,
        " dialog={}",
        match count {
            0 => "not_installed",
            n if n == installed.len() => "installed",
            _ => "partial",
        }
    );
    let mut separator = " text_routes=";
    for (_, route) in installed.iter().zip(dialog::ROUTES).filter(|(r, _)| **r) {
        let _ = write!(report, "{separator}{}", route.name);
        separator = ",";
    }
    // 0.5.0 match telemetry: all seven sites or none.
    let failed = matchlog::TELEMETRY_SITES
        .iter()
        .find(|site| !unsafe { guard_matches(site) });
    match failed {
        None => {
            skyline::install_hooks!(
                desync_detector_match_limits,
                desync_detector_ko_entry,
                desync_detector_ko_exit,
                desync_detector_peer_path_a,
                desync_detector_peer_path_b,
                desync_detector_session_end,
                desync_detector_session_stop
            );
            let _ = write!(report, " match_telemetry=installed");
        }
        Some(site) => {
            let _ = write!(report, " match_telemetry=skipped guard_failed={}", site.name);
        }
    }
    #[cfg(feature = "diag")]
    {
        let forced = FORCE_DESYNC_FRAME.load(Ordering::Relaxed);
        if forced <= u64::from(u32::MAX) {
            let _ = write!(report, " diag_force_desync_frame={forced}");
        }
    }
    // The writer appends to the same log, so it starts only after this line
    // (and the folder) exist.
    report_install(&mut report);
    if !start_writer() {
        report.clear();
        let _ = write!(report, "{} writer=not_started", tick());
        report_install(&mut report);
    }
}
