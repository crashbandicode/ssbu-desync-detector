//! Switch half: version check, guarded inline hooks, two static rings and one
//! writer thread. The hooks only read registers and memory the game is reading
//! at that instruction, then store integers into the rings. The writer thread
//! runs on a static stack and owns every file, opened through `nn::fs` with
//! paths built on the stack. Nothing allocates: the rings are static, buffers
//! live on the stack, and the counting allocator below reports the plugin's
//! heap allocations (expected 0) in the install and `session` lines.
use crate::model::{
    self, describe, minimal_outer, pack_desync, pack_recording, pack_sync_end, recorder, xor_words,
    FrameRecord, Line, RawEvent, Site, CFMK_BYTES, CFMK_PAYLOAD_LEN, CFMK_RECORDS, DESYNC,
    EV_DESYNC, EV_LINK, EV_PEER_ABORT, EV_PLAYBACK_MISMATCH, EV_RECORDING, EV_SESSION, EV_SYNC_END,
    FRAME_CHECK, KIND_ONLINE, KIND_PLAYBACK, OUTER_BYTES, PEER_ABORT, PLAYBACK_CHECK, RECORD_BYTES,
    REPLAY_FRAME_INDEX, REPLAY_PLAYER, RNG_CHECK_WORD, RNG_MANAGER, RNG_STATE_STRIDE, SYNC_END,
    TAG_CFMK, TAG_FRAM,
};
use core::cell::UnsafeCell;
use core::fmt::Write as _;
use skyline::hooks::{getRegionAddress, InlineCtx, Region};
use skyline::nn;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};

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
    let index = EVENT_HEAD.fetch_add(1, Ordering::AcqRel);
    let slot = &EVENTS[(index % EVENT_RING as u64) as usize];
    slot.tick.store(tick(), Ordering::Relaxed);
    slot.code.store(u64::from(code), Ordering::Relaxed);
    slot.a.store(a, Ordering::Relaxed);
    slot.b.store(b, Ordering::Relaxed);
    slot.seq.store(index + 1, Ordering::Release);
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

#[skyline::hook(offset = model::FRAME_CHECK_OFFSET, inline)]
unsafe fn desync_detector_frame_check(ctx: &InlineCtx) {
    let record = ctx.registers[10].x().wrapping_sub(0xa);
    if record < 0x10000 {
        return;
    }
    push_frame(FrameRecord {
        frame: u32::from(read_u16(record)),
        rng: ctx.registers[11].w(),
        position: 0,
        recorded: 0,
        flags: KIND_ONLINE | ctx.registers[8].w().trailing_zeros().min(0xff) << 8,
    });
}

#[skyline::hook(offset = model::DESYNC_OFFSET, inline)]
unsafe fn desync_detector_desync(ctx: &InlineCtx) {
    let now = tick();
    if now.wrapping_sub(DESYNC_WINDOW.load(Ordering::Acquire)) > 10 * TICKS_PER_SECOND {
        DESYNC_WINDOW.store(now, Ordering::Release);
        DESYNC_ROWS.store(0, Ordering::Release);
    }
    if DESYNC_ROWS.fetch_add(1, Ordering::AcqRel) >= 8 {
        return;
    }
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
    push_event(EV_DESYNC, a, b);
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
    push_event(EV_SYNC_END, a, caller);
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
    push_event(
        EV_PEER_ABORT,
        sender | reason << 8,
        u64::from(read_u16(session + 0x2)),
    );
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

fn append_log(bytes: &[u8]) {
    if let Some(mut log) = NxFile::append(&mut path(format_args!("{DATA_DIR}/events.log"))) {
        log.write(bytes);
    }
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
    pending: Option<Pending>,
}

impl Writer {
    fn open() -> Option<Self> {
        let mut log = NxFile::append(&mut path(format_args!("{DATA_DIR}/events.log")))?;
        if log.offset > EVENTS_LOG_LIMIT {
            log.truncate()?;
        }
        drop(log);
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
            self.in_session = false;
            self.frames = None;
            if let Some(pending) = self.pending.as_mut().filter(|p| p.ended == 0) {
                pending.ended = last;
            }
        }
    }

    fn drain_events(&mut self) {
        let head = EVENT_HEAD.load(Ordering::Acquire);
        if head.saturating_sub(self.event_tail) > EVENT_RING as u64 {
            self.event_tail = head - EVENT_RING as u64;
        }
        let mut line = Line::<192>::new();
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
            if event.code == EV_SYNC_END {
                if let Some(pending) = self.pending.as_mut().filter(|p| p.ended == 0) {
                    pending.ended = event.tick;
                }
            }
            line.clear();
            let _ = describe(&event, &mut line);
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

    fn poll_link(&mut self) {
        let mut raw = [0u8; 12];
        let result = unsafe { GetInternetConnectionStatus(raw.as_mut_ptr()) };
        let a = u64::from(raw[0]) | u64::from(raw[4]) << 32;
        let b = u64::from(result)
            | u64::from(u32::from_le_bytes([raw[8], raw[9], raw[10], raw[11]])) << 32;
        if (a, b) != self.link {
            self.link = (a, b);
            push_event(EV_LINK, a, b);
        }
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
            if iteration.is_multiple_of(5) {
                self.poll_link();
            }
            self.poll_pending();
            self.drain_events();
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
fn report_install(report: &mut Line<256>) {
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
    if let Some(mut log) = NxFile::append(&mut path(format_args!("{DATA_DIR}/events.log"))) {
        log.write(report.as_bytes());
    }
}

#[skyline::main(name = "ssbu-desync-detector")]
pub fn main() {
    let mut report = Line::<256>::new();
    let _ = write!(
        report,
        "{} plugin=ssbu-desync-detector/{}",
        tick(),
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
    // The writer appends to the same log, so it starts only after this line
    // (and the folder) exist.
    report_install(&mut report);
    if !start_writer() {
        report.clear();
        let _ = write!(report, "{} writer=not_started", tick());
        report_install(&mut report);
    }
}
