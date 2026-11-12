//! Nonblocking handoff between the game's observations, the link poll and UI.
//!
//! Session identity comes from the existing frame hook, before file I/O. A
//! missed mutation invalidates the episode rather than risking an old reason.
use crate::disconnect::{Latch, Reason, Verdict, VERDICT_WINDOW_TICKS};
use crate::model::{RawEvent, EV_LINK, KIND_ONLINE, KIND_PLAYBACK};
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

struct State {
    latch: Latch,
    kind: u32,
    session: u64,
    last_frame: u32,
    last_tick: u64,
    loss_epoch: u64,
    dialog_tick: Option<u64>,
}

/// All accesses to `state` use one successful atomic acquisition. Neither a
/// producer nor the dialog retries when another thread holds it.
pub struct Evidence {
    held: AtomicBool,
    losses: AtomicU64,
    state: UnsafeCell<State>,
}

// SAFETY: the atomic guard exclusively owns every access to UnsafeCell. Failed
// acquisitions touch only atomics. No references escape an acquisition.
unsafe impl Sync for Evidence {}

struct Guard<'a>(&'a AtomicBool);

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

impl Default for Evidence {
    fn default() -> Self {
        Self::new()
    }
}

impl Evidence {
    pub const fn new() -> Self {
        Self {
            held: AtomicBool::new(false),
            losses: AtomicU64::new(0),
            state: UnsafeCell::new(State {
                latch: Latch::new(),
                kind: 0,
                session: 0,
                last_frame: 0,
                last_tick: 0,
                loss_epoch: 0,
                dialog_tick: None,
            }),
        }
    }

    fn access<R>(&self, mutation: bool, f: impl FnOnce(&mut State) -> R) -> Option<R> {
        if self
            .held
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            if mutation {
                self.losses.fetch_add(1, Ordering::AcqRel);
            }
            return None;
        }
        let _guard = Guard(&self.held);
        // SAFETY: this acquisition exclusively owns state until guard drops.
        Some(f(unsafe { &mut *self.state.get() }))
    }

    /// Coverage telemetry only; reading this cannot freeze or alter a verdict.
    pub fn losses(&self) -> u64 {
        self.losses.load(Ordering::Acquire)
    }

    /// Called directly by the online or playback frame hook. Online `session`
    /// is the lockstep object's address; playback uses its game object address.
    pub fn frame(&self, now: u64, kind: u32, session: u64, frame: u32) {
        let epoch = self.losses.load(Ordering::Acquire);
        self.access(true, |state| {
            if !matches!(kind, KIND_ONLINE | KIND_PLAYBACK) || session == 0 {
                state.latch.reset();
                state.kind = 0;
                return;
            }
            if now < state.last_tick {
                state.latch.reset();
                state.kind = 0;
                return;
            }
            // The online frame is u16. Its natural wrap is not a new match.
            let wrapped = kind == KIND_ONLINE && state.last_frame > 0xff00 && frame < 0x100;
            let restarted = frame < state.last_frame && !wrapped;
            if state.kind != kind || state.session != session || restarted {
                if kind == KIND_ONLINE {
                    state.latch.begin_online(now, now);
                } else {
                    state.latch.begin_playback(now, now);
                }
                state.kind = kind;
                state.session = session;
                state.loss_epoch = epoch;
                state.dialog_tick = None;
            }
            state.last_frame = frame;
            state.last_tick = now;
        });
    }

    /// A hook's own observation, scoped to the same lockstep object. Writer
    /// session/log events are deliberately never fed back through this API.
    pub fn event(&self, session: u64, event: &RawEvent) {
        self.access(true, |state| {
            if state.kind == KIND_ONLINE && state.session == session {
                state.latch.observe(event.tick, event);
            }
        });
    }

    /// Polled even when the status has not changed, independently of logging.
    /// Nothing observed after the first dialog can explain that dialog.
    pub fn link(&self, now: u64, interface_strength: u64, result_state: u64) {
        self.access(true, |state| {
            if state.kind == KIND_ONLINE && state.dialog_tick.is_none_or(|shown| now <= shown) {
                state.latch.observe(
                    now,
                    &RawEvent {
                        tick: now,
                        code: EV_LINK,
                        a: interface_strength,
                        b: result_state,
                    },
                );
            }
        });
    }

    /// The online session's verdict at `now` without marking a dialog as shown (the writer's
    /// `last-session.txt`). `None` when no online session is in scope, the lock is busy or a fact was lost.
    pub fn peek(&self, now: u64) -> Option<Verdict> {
        self.access(false, |state| {
            let epoch = self.losses.load(Ordering::Acquire);
            if state.kind != KIND_ONLINE || epoch != state.loss_epoch || now < state.last_tick {
                return None;
            }
            let verdict = state.latch.verdict(now);
            (epoch == self.losses.load(Ordering::Acquire)).then_some(verdict)
        })
        .flatten()
    }

    /// Only the native communication-error text requests a precise reason.
    /// Failure or uncertain lifetime withholds that classification; the UI
    /// caller can append Unknown. This never opens a UI.
    pub fn reason(&self, now: u64) -> Option<Reason> {
        self.access(false, |state| {
            let epoch = self.losses.load(Ordering::Acquire);
            if epoch != state.loss_epoch
                || now < state.last_tick
                || now - state.last_tick > VERDICT_WINDOW_TICKS
                || state
                    .dialog_tick
                    .is_some_and(|shown| now.saturating_sub(shown) > VERDICT_WINDOW_TICKS)
            {
                return None;
            }
            let reason = state.latch.verdict(now).reason();
            if reason.is_some() {
                state.dialog_tick.get_or_insert(now);
            }
            // A concurrent producer that could not acquire the guard may
            // have lost the very fact on which this answer would depend.
            (epoch == self.losses.load(Ordering::Acquire))
                .then_some(reason)
                .flatten()
        })
        .flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EV_SYNC_END;

    #[test]
    fn a_contended_end_cannot_leave_a_precise_old_verdict() {
        let evidence = Evidence::new();
        evidence.frame(1, KIND_ONLINE, 0x10000, 20);
        // Hold the guard while another producer attempts to record an end.
        // Its call returns immediately, and this episode becomes unusable.
        evidence.access(false, |_| {
            evidence.event(
                0x10000,
                &RawEvent {
                    tick: 2,
                    code: EV_SYNC_END,
                    a: 1,
                    b: 0,
                },
            );
        });
        assert_eq!(evidence.losses(), 1);
        assert_eq!(evidence.reason(3), None);
        evidence.frame(4, KIND_ONLINE, 0x10000, 0);
        assert_eq!(evidence.reason(5), Some(Reason::Unknown));
    }

    #[test]
    fn peek_gives_the_verdict_without_marking_a_dialog() {
        use crate::disconnect::{Verdict, TICKS_PER_SECOND};
        use crate::model::EV_PEER_ABORT;
        let evidence = Evidence::new();
        assert_eq!(evidence.peek(1), None);
        evidence.frame(10, KIND_ONLINE, 0x10000, 20);
        // The other console's end message with an abnormal reason (2).
        evidence.event(0x10000, &RawEvent { tick: 11, code: EV_PEER_ABORT, a: 2 << 8, b: 0 });
        assert_eq!(evidence.peek(12), Some(Verdict::Show(Reason::PeerEnded)));
        // Peeking marked no dialog: a link observation after it still counts (a dialog would have frozen the
        // evidence), so the local fault now takes precedence.
        evidence.link(13, 0, 0);
        assert_eq!(evidence.reason(14), Some(Reason::LocalLink));
        // A playback session is not online.
        evidence.frame(20 * TICKS_PER_SECOND, crate::model::KIND_PLAYBACK, 0x20000, 1);
        assert_eq!(evidence.peek(21 * TICKS_PER_SECOND), None);
    }
}
