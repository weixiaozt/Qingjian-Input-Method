//! 解码请求；锁定的前缀仍必须属于真实码表。
use super::Segment;
use serde::Deserialize;

#[derive(Default, Deserialize)]
pub struct Request {
    pub codes: String,

    #[serde(default)]
    pub locked: Vec<Segment>,
}
