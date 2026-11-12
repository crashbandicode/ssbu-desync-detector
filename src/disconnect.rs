//! Why the game's disconnect dialog is showing, as far as this plugin's own
//! observations can tell.
//!
//! A native UI callback asks [`Latch::verdict`] only while the game is showing
//! a real disconnection. The hooks feed the latch synchronously, as they see
//! things, through the typed methods (`desync`, `sync_end`, `peer_end`,
//! `link`) or through `observe`, which reads a queued `RawEvent` of those four
//! kinds. Pure `core` code: no allocation, no formatting, no locks, no clock
//! of its own, no UI or OS call. The caller passes the system tick
//! (`nn::os::GetSystemTick`, 19.2 MHz) as `now` and stamps each observation
//! with the tick at which it was made (`at`; the same value when a hook feeds
//! it directly).
//!
//! ## Reasons, most specific first
//!
//! 1. [`Reason::Desync`]: a check mismatch was recorded in this session (a
//!    `desync` row, the session's own desync flag, or an end whose trigger is
//!    `desync`). Neither console is blamed: only a vanilla replay of the
//!    recording can say which one diverged.
//! 2. [`Reason::LocalLink`] / [`Reason::LocalStatus`]: the latest link
//!    observation of the session is a fault and is at most
//!    [`LINK_WINDOW_TICKS`] older than the end (or than `now`, while no end
//!    was seen). `LocalLink`: the status call worked and the console said it
//!    is not connected. `LocalStatus`: the status call itself failed, so only
//!    the status, not the link, is known to be bad. A connected observation (a
//!    handover to another interface included) is never a fault, and no text
//!    names hardware.
//! 3. [`Reason::PeerEnded`]: the other console sent an end message whose
//!    reason is not the normal one (`PEER_END_NORMAL_REASON`). That message
//!    (type 7) is sent from the sender's own lockstep end (`.16ce908`, inside
//!    `.16ce7c0`). The text says the opponent disconnected. The abort function
//!    `.16ccad0` (0.5.0's path B) is this console's own decision and never
//!    counts here.
//! 4. [`Reason::Timeout`]: the session ended on peer silence or a session
//!    timeout. Silence is not proof that anyone left on purpose.
//! 5. [`Reason::Unknown`]: an online session is in scope but nothing above
//!    holds, or the clock cannot be trusted.
//!
//! The order is fixed by [`classify`], so the same evidence always gives the
//! same reason.
//!
//! ## When nothing is said
//!
//! [`Verdict::Withheld`] means "leave the game's dialog alone": no online
//! session is in scope, the last session is a replay playback, the session
//! ended normally (the rematch path included), or it ended too long ago for
//! this dialog to be about it. A normal end also discards link evidence.
//!
//! ## Scope of an observation
//!
//! A session exists only after the caller says so: [`Latch::begin_online`] on
//! the first online frame (and again when the frame counter restarts for a new
//! match), [`Latch::begin_playback`] for a replay playback. Nothing reads
//! session starts from queued events, because a writer thread notices them up
//! to a drain interval late; a begin stamped later than `now` plus
//! [`SKEW_TICKS`] is refused and leaves no session in scope. Every begin or
//! reset starts a new [`Latch::generation`] and drops all evidence.
//!
//! An observation is recorded only when it belongs to the current online
//! session, and the refusal says why otherwise ([`Rejection`]): not online,
//! stamped in the future, before the session began, behind a newer
//! observation (a clock regression), or after the end. After the first
//! lockstep end only the other console's end message is still taken, for up
//! to [`END_GRACE_TICKS`] (it follows the local end by a few frames, even on a
//! normal end). Everything else, link observations above all, must be stamped
//! no later than the end: what happens after a disconnection (an undock, a
//! dropped Wi-Fi, a mismatch row of the unwinding session) is never its cause.
//!
//! ## Wrapping it in static state
//!
//! [`Latch`] is `Copy`, about a hundred bytes and built by `const fn new`.
//! Keep it in a `static` behind a bounded, non-blocking try-lock that is held
//! only for one call. A hook that cannot take the lock drops its fact and
//! goes on: a hook never waits. The UI side try-locks, copies the latch out,
//! releases, and asks the copy; when it cannot copy, it shows the game's own
//! dialog. A dropped fact can leave the evidence short of a mismatch or an
//! end, so count drops per session beside the latch and withhold the reason
//! when the count is not zero. The windows below are unmeasured defaults: the
//! game's real silence and timeout durations are not recorded anywhere yet.

use crate::model::{
    sync_end_trigger, EndState, RawEvent, EV_DESYNC, EV_LINK, EV_PEER_ABORT, EV_SYNC_END,
    PEER_END_NORMAL_REASON,
};

/// `nn::os::GetSystemTick` rate, the same 19.2 MHz as `runtime.rs`.
pub const TICKS_PER_SECOND: u64 = 19_200_000;
/// Stamps taken by different threads (a hook on the game thread, the link
/// poll on the writer) can disagree by a drain interval (100 ms in
/// `runtime.rs`) plus scheduling. Differences up to this are not a regression,
/// a future stamp or a previous-session stamp.
pub const SKEW_TICKS: u64 = TICKS_PER_SECOND / 4;
/// How long after the first lockstep end the other console's end message
/// still belongs to it. In the Eden logs it follows the local end by 4 frames.
pub const END_GRACE_TICKS: u64 = TICKS_PER_SECOND;
/// How much older than the end a link fault may be and still count.
pub const LINK_WINDOW_TICKS: u64 = 30 * TICKS_PER_SECOND;
/// How long after the end a verdict is still about that session.
pub const VERDICT_WINDOW_TICKS: u64 = 30 * TICKS_PER_SECOND;
/// `nn::nifm` connection state of a connected link (`state=4` in the Eden
/// logs next to `result=0x0`).
pub const LINK_CONNECTED: u32 = 4;

/// What the plugin can say about a disconnection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reason {
    /// A check mismatch was recorded in this session.
    Desync,
    /// This console reported that its own network link is not connected.
    LocalLink,
    /// This console's network status could not be read.
    LocalStatus,
    /// The other console sent an abnormal end message.
    PeerEnded,
    /// The session ended on peer silence or a session timeout.
    Timeout,
    /// The observations are missing, contradictory or untrustworthy.
    Unknown,
}

impl Reason {
    pub const ALL: [Reason; 6] = [
        Reason::Desync,
        Reason::LocalLink,
        Reason::LocalStatus,
        Reason::PeerEnded,
        Reason::Timeout,
        Reason::Unknown,
    ];

    /// A stable lower-case token for `events.log`.
    pub const fn name(self) -> &'static str {
        crate::model::COMMUNICATION_REASONS[self.code() as usize]
    }

    /// Stable event payload for the writer; no UI formatting in the hook.
    pub const fn code(self) -> u64 {
        match self {
            Reason::Desync => 0,
            Reason::LocalLink => 1,
            Reason::LocalStatus => 2,
            Reason::PeerEnded => 3,
            Reason::Timeout => 4,
            Reason::Unknown => 5,
        }
    }

    /// The text for the dialog: static, complete sentences, no raw codes.
    pub const fn text(self) -> &'static str {
        match self {
            Reason::Desync => "Desync detected. The consoles stopped agreeing on the match.",
            Reason::LocalLink => {
                "Local connection problem. This console did not report a connected network."
            }
            Reason::LocalStatus => {
                "Local network status error. This console could not read its network status."
            }
            Reason::PeerEnded => "Your opponent disconnected.",
            Reason::Timeout => {
                "Connection timed out. Nothing arrived from the other console in time."
            }
            Reason::Unknown => "The cause of the disconnection could not be determined.",
        }
    }
}

/// A local network fault seen in a link observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalFault {
    /// The status call worked and the state is not connected.
    NotConnected,
    /// The status call returned an error.
    StatusError,
}

/// The evidence the classifier weighs, already scoped and freshness-checked.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Facts {
    pub mismatch: bool,
    pub local: Option<LocalFault>,
    pub peer_ended: bool,
    pub timed_out: bool,
}

/// The fixed precedence: a mismatch, then a fresh local fault, then an
/// explicit abnormal peer end, then a timeout, then unknown.
pub const fn classify(facts: Facts) -> Reason {
    if facts.mismatch {
        Reason::Desync
    } else if let Some(fault) = facts.local {
        match fault {
            LocalFault::NotConnected => Reason::LocalLink,
            LocalFault::StatusError => Reason::LocalStatus,
        }
    } else if facts.peer_ended {
        Reason::PeerEnded
    } else if facts.timed_out {
        Reason::Timeout
    } else {
        Reason::Unknown
    }
}

/// Why the dialog should be left as the game made it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Withheld {
    /// No online session is in scope (none began, or the latch was reset).
    NoSession,
    /// The last session is a replay playback; playback has no online reasons.
    Playback,
    /// The session ended normally and nothing abnormal was seen.
    NormalEnd,
    /// The session ended too long ago for this dialog to be about it.
    Stale,
}

/// The answer to a dialog request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Verdict {
    /// Add this reason to the dialog.
    Show(Reason),
    /// Leave the dialog alone.
    Withheld(Withheld),
}

impl Verdict {
    pub const fn reason(self) -> Option<Reason> {
        match self {
            Verdict::Show(reason) => Some(reason),
            Verdict::Withheld(_) => None,
        }
    }

    pub const fn text(self) -> Option<&'static str> {
        match self {
            Verdict::Show(reason) => Some(reason.text()),
            Verdict::Withheld(_) => None,
        }
    }
}

/// Why an observation or a session start was not taken.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rejection {
    /// No online session is in scope (idle, reset or a replay playback).
    NotOnline,
    /// Stamped later than `now` plus [`SKEW_TICKS`].
    Future,
    /// Stamped before the session began (beyond [`SKEW_TICKS`]).
    PreviousSession,
    /// Stamped before an observation already recorded (beyond [`SKEW_TICKS`]).
    ClockRegression,
    /// Stamped after the session ended (the other console's end message gets
    /// [`END_GRACE_TICKS`], everything else none).
    AfterEnd,
}

/// What a feeding call did.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Observed {
    /// `observe` does not read this kind of event.
    Irrelevant,
    /// A session began or the observation was added to its evidence.
    Recorded,
    /// The call was refused and dropped.
    Rejected(Rejection),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Scope {
    Idle,
    Online,
    Playback,
}

/// What a lockstep end's caller says about the cause.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EndCause {
    Mismatch,
    Silence,
    Other,
}

/// Only the mismatch and the two silence ends name a cause. The rest (a
/// station lost or missing at the start, the referee, the end judge, an
/// unlisted caller) say nothing about why, so they stay unclassified.
fn end_cause(caller: u64) -> EndCause {
    match sync_end_trigger(caller) {
        "desync" => EndCause::Mismatch,
        "peer_silent" | "session_timeout" => EndCause::Silence,
        _ => EndCause::Other,
    }
}

/// The latest link observation of the session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Link {
    at: u64,
    result: u32,
    state: u32,
    fault: Option<LocalFault>,
}

/// The evidence of one online session and the rules that scope it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Latch {
    scope: Scope,
    generation: u32,
    started: u64,
    newest: u64,
    ended: Option<u64>,
    end: EndState,
    mismatch: bool,
    peer_ended: bool,
    timed_out: bool,
    link: Option<Link>,
}

impl Default for Latch {
    fn default() -> Self {
        Self::new()
    }
}

impl Latch {
    pub const fn new() -> Self {
        Self {
            scope: Scope::Idle,
            generation: 0,
            started: 0,
            newest: 0,
            ended: None,
            end: EndState {
                normal_end_seen: false,
                abnormal: false,
            },
            mismatch: false,
            peer_ended: false,
            timed_out: false,
            link: None,
        }
    }

    /// The number of begins and resets so far, wrapping. It changes whenever
    /// the evidence is discarded, never otherwise.
    pub const fn generation(&self) -> u32 {
        self.generation
    }

    /// Forgets everything: no online session is in scope afterwards.
    pub fn reset(&mut self) {
        self.start(Scope::Idle, 0);
    }

    /// An online session begins at tick `at`: call it on the first online
    /// frame and again when the frame counter restarts for a new match, from
    /// the hook that sees the frame, not from a drain that notices it later.
    /// All earlier evidence is dropped. A stamp later than `now` plus
    /// [`SKEW_TICKS`] is refused and leaves no session in scope.
    pub fn begin_online(&mut self, now: u64, at: u64) -> Observed {
        self.begin(Scope::Online, now, at)
    }

    /// A replay playback begins at tick `at`: no online reason is possible
    /// until the next online begin. Refused like `begin_online` on a future
    /// stamp.
    pub fn begin_playback(&mut self, now: u64, at: u64) -> Observed {
        self.begin(Scope::Playback, now, at)
    }

    fn begin(&mut self, scope: Scope, now: u64, at: u64) -> Observed {
        if at > now.saturating_add(SKEW_TICKS) {
            self.start(Scope::Idle, 0);
            return Observed::Rejected(Rejection::Future);
        }
        self.start(scope, at);
        Observed::Recorded
    }

    fn start(&mut self, scope: Scope, at: u64) {
        *self = Self {
            scope,
            generation: self.generation.wrapping_add(1),
            started: at,
            newest: at,
            ..Self::new()
        };
    }

    /// Feeds one queued event of the four kinds this module reads
    /// (`EV_SYNC_END`, `EV_DESYNC`, `EV_PEER_ABORT`, `EV_LINK`), unpacked as
    /// `model::describe` prints them. Session events are not read: sessions
    /// begin only through `begin_online` and `begin_playback`.
    pub fn observe(&mut self, now: u64, event: &RawEvent) -> Observed {
        match event.code {
            EV_SYNC_END => self.sync_end(
                now,
                event.tick,
                (event.a & 0xff) as u32,
                event.a >> 8 & 1 != 0,
                event.b,
            ),
            EV_DESYNC => self.desync(now, event.tick),
            EV_PEER_ABORT => self.peer_end(now, event.tick, event.a >> 8 & 0xffff),
            EV_LINK => self.link(
                now,
                event.tick,
                (event.b & 0xffff_ffff) as u32,
                (event.b >> 32) as u32,
            ),
            _ => Observed::Irrelevant,
        }
    }

    /// Admission for an observation stamped `at`. `peer_message` observations
    /// (the other console's end message) may follow the end by
    /// [`END_GRACE_TICKS`]; nothing else may follow it at all.
    fn admit(&mut self, now: u64, at: u64, peer_message: bool) -> Result<(), Rejection> {
        if self.scope != Scope::Online {
            return Err(Rejection::NotOnline);
        }
        if at > now.saturating_add(SKEW_TICKS) {
            return Err(Rejection::Future);
        }
        let early = at.saturating_add(SKEW_TICKS);
        if early < self.started {
            return Err(Rejection::PreviousSession);
        }
        if early < self.newest {
            return Err(Rejection::ClockRegression);
        }
        if let Some(ended) = self.ended {
            let limit = if peer_message {
                ended.saturating_add(END_GRACE_TICKS)
            } else {
                ended
            };
            if at > limit {
                return Err(Rejection::AfterEnd);
            }
        }
        self.newest = self.newest.max(at);
        Ok(())
    }

    /// A check mismatch row.
    pub fn desync(&mut self, now: u64, at: u64) -> Observed {
        if let Err(why) = self.admit(now, at, false) {
            return Observed::Rejected(why);
        }
        self.mismatch = true;
        self.end.desync();
        Observed::Recorded
    }

    /// The lockstep session ended: `kind` and the session's desync flag as the
    /// hook read them, `caller` the return address offset in `.text`.
    pub fn sync_end(
        &mut self,
        now: u64,
        at: u64,
        kind: u32,
        desync_seen: bool,
        caller: u64,
    ) -> Observed {
        if let Err(why) = self.admit(now, at, false) {
            return Observed::Rejected(why);
        }
        // The model's own rule for "was this end normal", applied to this end.
        let mut this_end = EndState::default();
        this_end.sync_end(kind, desync_seen);
        self.end.sync_end(kind, desync_seen);
        self.ended.get_or_insert(at);
        if desync_seen {
            self.mismatch = true;
        }
        match end_cause(caller) {
            EndCause::Mismatch => {
                self.mismatch = true;
                self.end.desync();
            }
            EndCause::Silence if this_end.abnormal => self.timed_out = true,
            EndCause::Silence | EndCause::Other => {}
        }
        if self.end.normal_end_seen {
            self.link = None;
        }
        Observed::Recorded
    }

    /// The other console's end message with its reason code.
    pub fn peer_end(&mut self, now: u64, at: u64, reason: u64) -> Observed {
        if let Err(why) = self.admit(now, at, true) {
            return Observed::Rejected(why);
        }
        self.end.peer_end(reason);
        if reason != PEER_END_NORMAL_REASON {
            self.peer_ended = true;
        }
        Observed::Recorded
    }

    /// A link observation: the `nn::nifm` status call's `result` and the
    /// connection `state` it returned. Only `result == 0` with
    /// [`LINK_CONNECTED`] is connected. A failed call is a status error; a
    /// working call that reports another state is not connected.
    pub fn link(&mut self, now: u64, at: u64, result: u32, state: u32) -> Observed {
        if let Err(why) = self.admit(now, at, false) {
            return Observed::Rejected(why);
        }
        // A normal end discards link evidence, and none is kept after it; a
        // newer observation is not replaced by one that is slightly older.
        if self.end.normal_end_seen || self.link.is_some_and(|seen| seen.at > at) {
            return Observed::Recorded;
        }
        let fault = if result != 0 {
            Some(LocalFault::StatusError)
        } else if state != LINK_CONNECTED {
            Some(LocalFault::NotConnected)
        } else {
            None
        };
        self.link = Some(Link {
            at,
            result,
            state,
            fault,
        });
        Observed::Recorded
    }

    /// The raw `nn::nifm` result and state of the latest link observation when
    /// it is a fault. For log lines only: no dialog text depends on them.
    pub fn failed_link(&self) -> Option<(u32, u32)> {
        self.link
            .filter(|seen| seen.fault.is_some())
            .map(|seen| (seen.result, seen.state))
    }

    /// What to show for a dialog the game is displaying at tick `now`. Pure:
    /// repeating the call without new observations repeats the answer.
    pub fn verdict(&self, now: u64) -> Verdict {
        match self.scope {
            Scope::Idle => return Verdict::Withheld(Withheld::NoSession),
            Scope::Playback => return Verdict::Withheld(Withheld::Playback),
            Scope::Online => {}
        }
        if self.end.normal_end_seen && !self.end.abnormal {
            return Verdict::Withheld(Withheld::NormalEnd);
        }
        if self
            .ended
            .is_some_and(|ended| now > ended.saturating_add(VERDICT_WINDOW_TICKS))
        {
            return Verdict::Withheld(Withheld::Stale);
        }
        // The clock is behind what was already recorded: no freshness can be
        // judged, so nothing precise is claimed.
        if now.saturating_add(SKEW_TICKS) < self.newest {
            return Verdict::Show(Reason::Unknown);
        }
        // A link fault counts only when it was seen no later than the end (or
        // than now, while no end was seen) and not too long before it.
        let reference = self.ended.unwrap_or(now);
        let local = self.link.and_then(|seen| {
            (seen.at <= reference && reference - seen.at <= LINK_WINDOW_TICKS)
                .then_some(seen.fault)
                .flatten()
        });
        Verdict::Show(classify(Facts {
            mismatch: self.mismatch,
            local,
            peer_ended: self.peer_ended,
            timed_out: self.timed_out,
        }))
    }
}
