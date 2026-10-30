//! 当前候选的异步翻译与空格确认状态。
use crate::Candidate;
use std::time::Instant;

pub(in crate::engine) struct AutoTranslation {
    pub candidate: Candidate,

    pub source: String,

    pub scope: String,

    pub due: Instant,

    pub sequence: Option<u64>,

    pub result: Option<String>,

    pub failed: bool,

    pub pressed: Option<Instant>,

    pub waiting: bool,
}
