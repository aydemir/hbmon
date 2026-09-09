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
