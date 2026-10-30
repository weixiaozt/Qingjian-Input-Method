//! 解码结果与耗时，供界面和离线评测使用。
use super::Candidate;
use serde::Serialize;

#[derive(Serialize)]
pub struct Response {
    pub codes: String,

    pub candidates: Vec<Candidate>,

    pub elapsed_ms: f64,

    pub complete: bool,

    pub message: String,
}
