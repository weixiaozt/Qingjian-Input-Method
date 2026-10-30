//! 连续五笔编码的有界词图搜索，与实验界面、翻译服务分离。
mod candidate;
mod entry;
mod request;
mod response;
mod search;
mod segment;

pub use candidate::Candidate;
pub use request::Request;
pub use response::Response;
pub use search::Decoder;
pub use segment::Segment;

#[cfg(test)]
mod tests;
