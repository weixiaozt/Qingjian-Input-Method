//! 无网络翻译器，允许测试按请求序号回放响应。
pub(super) use super::state::State;
use qingjian_core::{Prediction, PredictionPolicy, PredictionRequest, Predictor};
use std::sync::{Arc, Mutex};
pub(super) struct Provider(pub Arc<Mutex<State>>);
impl Predictor for Provider {
    fn policy(&self) -> PredictionPolicy {
        PredictionPolicy::default()
    }
    fn submit(&mut self, request: PredictionRequest) {
        self.0.lock().unwrap().requests.push(request);
    }
    fn poll(&mut self) -> Option<Prediction> {
        self.0.lock().unwrap().replies.pop()
    }
}
