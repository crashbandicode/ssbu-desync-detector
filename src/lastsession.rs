//! The last online session's verdict, kept across boots in `last-session.txt`. A record only: no dialog text depends
//! on it.
//!
//! The writer thread owns the file: it reads it once at boot (the `last_session_loaded` line), writes `in_progress`
//! when an online session starts and the verdict when that session stops (the `session_summary` line's
//! `last_session`). A console that stops mid-match (power, crash, closing the software) leaves `in_progress`.
use crate::disconnect::{Reason, Verdict, Withheld};
use core::fmt;

pub const FILE_NAME: &str = "last-session.txt";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LastSession {
    /// No file, an unreadable one, or a session whose end had no verdict (none in scope, stale).
    Unknown,
    /// An online session started and its stop was not seen.
    InProgress,
    /// It ended normally (the rematch path included).
    Normal,
    /// It ended with this verdict.
    Ended(Reason),
}

impl LastSession {
    /// `none` for no verdict at all; `unknown` is the verdict [`Reason::Unknown`].
    pub const fn token(self) -> &'static str {
        match self {
            Self::Unknown => "none",
            Self::InProgress => "in_progress",
            Self::Normal => "normal",
            Self::Ended(reason) => reason.name(),
        }
    }

    /// The verdict a session had when it stopped.
    pub const fn from_verdict(verdict: Option<Verdict>) -> Self {
        match verdict {
            Some(Verdict::Show(reason)) => Self::Ended(reason),
            Some(Verdict::Withheld(Withheld::NormalEnd)) => Self::Normal,
            _ => Self::Unknown,
        }
    }

    /// The file's one line, `last_session=<token>`.
    pub fn write_line(self, out: &mut impl fmt::Write) -> fmt::Result {
        writeln!(out, "last_session={}", self.token())
    }

    /// Reads the file's first line; anything else is `Unknown`.
    pub fn parse(bytes: &[u8]) -> Self {
        let line = bytes.split(|b| *b == b'\n').next().unwrap_or(&[]);
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        let Some(token) = line.strip_prefix(b"last_session=") else {
            return Self::Unknown;
        };
        [Self::Unknown, Self::InProgress, Self::Normal]
            .into_iter()
            .chain(Reason::ALL.into_iter().map(Self::Ended))
            .find(|last| last.token().as_bytes() == token)
            .unwrap_or(Self::Unknown)
    }
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

    #[test]
    fn the_file_line_round_trips() {
        let every = [LastSession::Unknown, LastSession::InProgress, LastSession::Normal]
            .into_iter()
            .chain(Reason::ALL.into_iter().map(LastSession::Ended));
        for last in every {
            let mut s = S(String::new());
            last.write_line(&mut s).unwrap();
            assert_eq!(LastSession::parse(s.0.as_bytes()), last, "{}", s.0);
        }
        assert_eq!(LastSession::parse(b"last_session=peer_ended\r\nmore"), LastSession::Ended(Reason::PeerEnded));
        for junk in [&b""[..], b"last_session=", b"last_session=peer", b"peer_ended", b"last_session=peer_ended2"] {
            assert_eq!(LastSession::parse(junk), LastSession::Unknown);
        }
    }

    #[test]
    fn a_session_stops_with_its_verdict() {
        assert_eq!(
            LastSession::from_verdict(Some(Verdict::Show(Reason::PeerEnded))),
            LastSession::Ended(Reason::PeerEnded)
        );
        assert_eq!(LastSession::from_verdict(Some(Verdict::Withheld(Withheld::NormalEnd))), LastSession::Normal);
        for withheld in [Withheld::NoSession, Withheld::Playback, Withheld::Stale] {
            assert_eq!(LastSession::from_verdict(Some(Verdict::Withheld(withheld))), LastSession::Unknown);
        }
        assert_eq!(LastSession::from_verdict(None), LastSession::Unknown);
    }

    #[test]
    fn no_record_names_a_side() {
        // Option 1 (2026-11-12): nothing says who disconnected; the tokens are the 0.3.0 reason names.
        for reason in Reason::ALL {
            assert!(!LastSession::Ended(reason).token().contains("opponent"));
        }
        assert_eq!(Reason::PeerEnded.text(), "The other console ended the session.");
    }
}
