pub mod dep_missing;
pub mod oom;
pub mod stall;
pub mod timeout;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Running,
    Stalled,
    OomKilled,
    Done,
    Failed,
    DepMissing,
    Timeout,
}

impl State {
    pub fn as_str(&self) -> &'static str {
        match self {
            State::Running => "running",
            State::Stalled => "stalled",
            State::OomKilled => "oom_killed",
            State::Done => "done",
            State::Failed => "failed",
            State::DepMissing => "dep_missing",
            State::Timeout => "timeout",
        }
    }

    pub fn exit_code(&self, build_code: i32) -> i32 {
        match self {
            State::Done => 0,
            State::Failed => 1,
            State::DepMissing => 2,
            // 3: crash/signal — caller refines with 130/137/143 where known
            State::Running | State::Stalled => build_code,
            State::OomKilled => 137,
            State::Timeout => 124,
        }
    }
}

/// RFC 5.2.3 exit-code mapping for raw build waits.
pub fn map_build_exit(code: i32) -> (State, i32) {
    if code == 0 {
        (State::Done, 0)
    } else {
        (State::Failed, 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_follow_rfc() {
        assert_eq!(State::Done.exit_code(0), 0);
        assert_eq!(State::Failed.exit_code(1), 1);
        assert_eq!(State::DepMissing.exit_code(0), 2);
        assert_eq!(State::OomKilled.exit_code(0), 137);
        assert_eq!(State::Timeout.exit_code(0), 124);
    }

    #[test]
    fn build_exit_mapping() {
        assert_eq!(map_build_exit(0), (State::Done, 0));
        assert_eq!(map_build_exit(1), (State::Failed, 1));
    }

    #[test]
    fn state_names() {
        assert_eq!(State::Stalled.as_str(), "stalled");
        assert_eq!(State::OomKilled.as_str(), "oom_killed");
    }
}
