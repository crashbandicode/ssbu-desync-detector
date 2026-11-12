//! The last online session's verdict, kept across boots in `last-session.txt`, and what the game's reconnect warning
//! ("You disconnected in the middle of your last online battle.") gets because of it.
//!
//! The writer thread owns the file: it reads it once at boot, writes `in_progress` when an online session starts and
//! the verdict when that session stops. Hooks only read the code it publishes. A console that stops mid-match (power,
//! crash, closing the software) leaves `in_progress`, so the game's warning is never changed for a session whose end
//! this plugin did not see.
//!
//! Only a session that ended with [`Reason::PeerEnded`] (the other console's own end message with an abnormal
//! reason, and no mismatch or local fault) changes the warning: its text is replaced, the dialog itself still opens
//! and closes as the game wants. Everything else keeps the game's text (and 0.4.0's appended reason, when this boot
//! still has a verdict).
use crate::disconnect::{Reason, Verdict, Withheld};
use core::fmt;

pub const FILE_NAME: &str = "last-session.txt";
/// The warning's replacement text.
pub const OPPONENT_LAST_BATTLE: &str = "Your opponent disconnected in your last online battle.";

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

const ENDED: u8 = 16;

impl LastSession {
    /// The code the writer publishes for the hooks.
    pub const fn code(self) -> u8 {
        match self {
            Self::Unknown => 0,
            Self::InProgress => 1,
            Self::Normal => 2,
            Self::Ended(reason) => ENDED + reason.code() as u8,
        }
    }

    pub fn from_code(code: u8) -> Self {
        match code {
            1 => Self::InProgress,
            2 => Self::Normal,
            _ => code
                .checked_sub(ENDED)
                .and_then(|i| Reason::ALL.get(usize::from(i)).copied())
                .map_or(Self::Unknown, Self::Ended),
        }
    }

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

    /// Whether the reconnect warning is about a session the other console ended.
    pub const fn opponent_ended(self) -> bool {
        matches!(self, Self::Ended(Reason::PeerEnded))
    }
}

/// What a disconnect text gets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Choice {
    /// The game's text as it is.
    Leave,
    /// The game's text plus this reason (0.3.0/0.4.0).
    Append(Reason),
    /// This text instead of the game's.
    Replace(&'static str),
}

/// The text kinds `dialog` recognizes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Text {
    /// A communication-error sentence: itself a disconnect, so missing evidence says `Unknown`.
    CommunicationError,
    /// The Canceled notice: shares words with ordinary cancels, so only a verdict changes it.
    Canceled,
    /// The reconnect warning, about the last online battle.
    ReconnectWarning,
}

/// The one rule for every disconnect text. `live` is this boot's verdict for a session still in scope (0.3.0/0.4.0);
/// `last` is the stored verdict of the last online session.
pub fn choose(text: Text, live: Option<Reason>, last: LastSession) -> Choice {
    match text {
        Text::CommunicationError => Choice::Append(live.unwrap_or(Reason::Unknown)),
        Text::Canceled => live.map_or(Choice::Leave, Choice::Append),
        Text::ReconnectWarning if last.opponent_ended() => Choice::Replace(OPPONENT_LAST_BATTLE),
        Text::ReconnectWarning => live.map_or(Choice::Leave, Choice::Append),
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

    fn every() -> impl Iterator<Item = LastSession> {
        [LastSession::Unknown, LastSession::InProgress, LastSession::Normal]
            .into_iter()
            .chain(Reason::ALL.into_iter().map(LastSession::Ended))
    }

    #[test]
    fn the_file_line_and_the_code_round_trip() {
        for last in every() {
            let mut s = S(String::new());
            last.write_line(&mut s).unwrap();
            assert_eq!(LastSession::parse(s.0.as_bytes()), last, "{}", s.0);
            assert_eq!(LastSession::from_code(last.code()), last);
        }
        assert_eq!(LastSession::parse(b"last_session=peer_ended\r\nmore"), LastSession::Ended(Reason::PeerEnded));
        for junk in [&b""[..], b"last_session=", b"last_session=peer", b"peer_ended", b"last_session=peer_ended2"] {
            assert_eq!(LastSession::parse(junk), LastSession::Unknown);
        }
        assert_eq!(LastSession::from_code(0xff), LastSession::Unknown);
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
    fn only_a_peer_end_replaces_the_reconnect_warning() {
        for last in every() {
            let choice = choose(Text::ReconnectWarning, None, last);
            if last == LastSession::Ended(Reason::PeerEnded) {
                assert_eq!(choice, Choice::Replace(OPPONENT_LAST_BATTLE));
            } else {
                // A local fault, a timeout, a desync, unknown, a normal end or a session that never stopped:
                // the game's text stays.
                assert_eq!(choice, Choice::Leave, "{last:?}");
            }
        }
    }

    #[test]
    fn a_live_verdict_still_appends_to_the_warning_unless_the_peer_ended() {
        for reason in Reason::ALL {
            assert_eq!(choose(Text::ReconnectWarning, Some(reason), LastSession::InProgress), Choice::Append(reason));
        }
        assert_eq!(
            choose(Text::ReconnectWarning, Some(Reason::Timeout), LastSession::Ended(Reason::PeerEnded)),
            Choice::Replace(OPPONENT_LAST_BATTLE)
        );
    }

    #[test]
    fn the_end_dialog_follows_the_live_verdict_and_names_the_opponent() {
        assert_eq!(
            choose(Text::CommunicationError, Some(Reason::PeerEnded), LastSession::Unknown),
            Choice::Append(Reason::PeerEnded)
        );
        assert_eq!(Reason::PeerEnded.text(), "Your opponent disconnected.");
        assert_eq!(choose(Text::CommunicationError, None, LastSession::Unknown), Choice::Append(Reason::Unknown));
        assert_eq!(choose(Text::Canceled, Some(Reason::PeerEnded), LastSession::Unknown), Choice::Append(Reason::PeerEnded));
        assert_eq!(choose(Text::Canceled, None, LastSession::Ended(Reason::PeerEnded)), Choice::Leave);
        // Local, network and unknown verdicts keep today's text.
        for reason in [Reason::LocalLink, Reason::LocalStatus, Reason::Timeout, Reason::Unknown, Reason::Desync] {
            assert_eq!(choose(Text::CommunicationError, Some(reason), LastSession::Unknown), Choice::Append(reason));
        }
    }
}
