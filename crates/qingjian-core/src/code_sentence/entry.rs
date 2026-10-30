//! 一个码表词目。
#[derive(Clone)]
pub(super) struct Entry {
    pub text: String,

    pub frequency: u32,
}
