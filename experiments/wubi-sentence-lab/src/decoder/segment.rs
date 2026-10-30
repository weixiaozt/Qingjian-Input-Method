//! 一个确定的编码边界及其中文字词。
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Segment {
    pub code: String,

    pub text: String,
}
