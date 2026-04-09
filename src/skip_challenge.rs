//! 休息跳过挑战逻辑。
//!
//! 使用随机句子替代固定短语，并要求用户在限定时间内连续完成输入。

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAX_CHAR_GAP: Duration = Duration::from_secs(2);

const PHRASES: &[&str] = &[
    "rest first and work better later",
    "a calm pause helps me focus again",
    "slow down and breathe for a moment",
    "i choose a short break for my eyes",
    "one minute of rest is worth it",
    "my body deserves a gentle pause",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feedback {
    Ready,
    Progress,
    Mismatch,
    Timeout,
    Completed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub phrase: &'static str,
    pub matched_len: usize,
    pub total_len: usize,
    pub feedback: Feedback,
    pub failure_seq: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UpdateResult {
    pub completed: bool,
    pub snapshot: Snapshot,
}

#[derive(Clone, Debug)]
pub struct SkipChallenge {
    phrase: &'static str,
    matched_len: usize,
    last_input_at: Option<Instant>,
    feedback: Feedback,
    failure_seq: u64,
}

impl SkipChallenge {
    pub fn random() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let idx = (seed ^ (seed >> 19) ^ (seed >> 41)) as usize % PHRASES.len();
        Self::new(PHRASES[idx])
    }

    pub const fn new(phrase: &'static str) -> Self {
        Self {
            phrase,
            matched_len: 0,
            last_input_at: None,
            feedback: Feedback::Ready,
            failure_seq: 0,
        }
    }

    pub const fn snapshot(&self) -> Snapshot {
        Snapshot {
            phrase: self.phrase,
            matched_len: self.matched_len,
            total_len: self.phrase.len(),
            feedback: self.feedback,
            failure_seq: self.failure_seq,
        }
    }

    pub fn register_char(&mut self, ch: char, now: Instant) -> UpdateResult {
        let ch = normalize_char(ch);
        if ch.is_control() {
            return UpdateResult {
                completed: false,
                snapshot: self.snapshot(),
            };
        }

        if self.is_timeout(now) {
            self.reset(Feedback::Timeout);
        }

        if self.try_advance(ch, now) {
            return UpdateResult {
                completed: self.matched_len == self.phrase.len(),
                snapshot: self.snapshot(),
            };
        }

        self.reset(Feedback::Mismatch);
        let _ = self.try_advance(ch, now);

        UpdateResult {
            completed: self.matched_len == self.phrase.len(),
            snapshot: self.snapshot(),
        }
    }

    fn is_timeout(&self, now: Instant) -> bool {
        self.matched_len > 0
            && self
                .last_input_at
                .is_some_and(|last_input_at| now.duration_since(last_input_at) > MAX_CHAR_GAP)
    }

    fn try_advance(&mut self, ch: char, now: Instant) -> bool {
        let expected = self.phrase.as_bytes()[self.matched_len] as char;
        if ch != expected {
            return false;
        }

        self.matched_len += 1;
        self.last_input_at = Some(now);
        self.feedback = if self.matched_len == self.phrase.len() {
            Feedback::Completed
        } else {
            Feedback::Progress
        };
        true
    }

    fn reset(&mut self, feedback: Feedback) {
        self.matched_len = 0;
        self.last_input_at = None;
        self.feedback = feedback;
        self.failure_seq += u64::from(matches!(feedback, Feedback::Mismatch | Feedback::Timeout));
    }
}

const fn normalize_char(ch: char) -> char {
    if ch.is_ascii() {
        ch.to_ascii_lowercase()
    } else {
        ch
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completes_phrase_when_typed_in_order() {
        let start = Instant::now();
        let mut challenge = SkipChallenge::new("ab");

        let first = challenge.register_char('a', start);
        assert!(!first.completed);
        assert_eq!(first.snapshot.matched_len, 1);
        assert_eq!(first.snapshot.feedback, Feedback::Progress);

        let second = challenge.register_char('b', start + Duration::from_millis(300));
        assert!(second.completed);
        assert_eq!(second.snapshot.feedback, Feedback::Completed);
    }

    #[test]
    fn timeout_resets_and_restarts_from_current_char() {
        let start = Instant::now();
        let mut challenge = SkipChallenge::new("ab");

        let _ = challenge.register_char('a', start);
        let result = challenge.register_char('a', start + Duration::from_secs(3));

        assert!(!result.completed);
        assert_eq!(result.snapshot.matched_len, 1);
        assert_eq!(result.snapshot.feedback, Feedback::Progress);
        assert_eq!(result.snapshot.failure_seq, 1);
    }

    #[test]
    fn mismatch_resets_and_can_restart_immediately() {
        let start = Instant::now();
        let mut challenge = SkipChallenge::new("aba");

        let _ = challenge.register_char('a', start);
        let result = challenge.register_char('a', start + Duration::from_millis(100));

        assert!(!result.completed);
        assert_eq!(result.snapshot.matched_len, 1);
        assert_eq!(result.snapshot.feedback, Feedback::Progress);
        assert_eq!(result.snapshot.failure_seq, 1);
    }
}
