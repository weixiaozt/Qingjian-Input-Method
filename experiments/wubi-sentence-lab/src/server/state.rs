//! 实验服务的共享状态。
use crate::{decoder::Decoder, translator::Translator};
use std::path::PathBuf;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicUsize},
};

pub(super) struct State {
    pub root: PathBuf,

    pub origin: String,

    pub token: String,

    pub decoder: Mutex<Decoder>,

    pub translator: Mutex<Translator>,

    pub running: AtomicBool,

    pub active: AtomicUsize,
}
