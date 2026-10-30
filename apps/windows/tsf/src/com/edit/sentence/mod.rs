//! 最后一句的只读快照、持续校验与原子确认写入。
pub(crate) mod commit;
pub(crate) mod read;
mod snapshot;
mod validate;
pub(crate) use snapshot::Snapshot;
pub(crate) use validate::request_validation;
