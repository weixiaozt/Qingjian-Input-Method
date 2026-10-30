//! 轮询返回的帧、提交文本与可选插入点。
use qingjian_platform::protocol::Frame;
pub struct PollResponse {
    pub frame: Frame,

    pub commit: Option<String>,

    pub caret_utf16: Option<usize>,
}
