//! 一条完整或正在输入的候选路径。
use super::Segment;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Candidate {
    pub text: String,

    pub segments: Vec<Segment>,

    pub score: f64,

    /// 末尾还没构成完整编码的部分，不擅自补字。
    pub pending: String,
}

impl Default for Candidate {
    fn default() -> Self {
        Self {
            text: String::new(),
            segments: Vec::new(),
            score: 0.0,
            pending: String::new(),
        }
    }
}
