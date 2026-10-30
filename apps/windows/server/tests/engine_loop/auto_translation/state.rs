//! 假翻译器的请求与响应队列。
use qingjian_core::{Prediction, PredictionRequest};
#[derive(Default)]
pub(super) struct State {
    pub requests: Vec<PredictionRequest>,

    pub replies: Vec<Prediction>,
}
