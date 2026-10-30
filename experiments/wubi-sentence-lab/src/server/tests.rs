//! 回归：Windows 接收连接后，分两次抵达的 HTTP 头和请求体必须完整读取。
use super::{http, state::State};
use crate::{decoder::Decoder, translator::Translator};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicUsize},
};
use std::time::Duration;

#[test]
fn split_request_body_survives_nonblocking_listener() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let state = State {
        root: root.clone(),
        origin: format!("http://{address}"),
        token: "test".into(),
        decoder: Mutex::new(Decoder::test_fixture()),
        translator: Mutex::new(Translator::new().unwrap()),
        running: AtomicBool::new(true),
        active: AtomicUsize::new(0),
    };
    let mut client = TcpStream::connect(address).unwrap();
    let (stream, _) = listener.accept().unwrap();
    let worker = std::thread::spawn(move || http::serve(stream, &state));
    write!(client,"POST /api/status HTTP/1.1\r\nHost: {address}\r\nX-Lab-Token: test\r\nContent-Length: 2\r\n\r\n").unwrap();
    std::thread::sleep(Duration::from_millis(50));
    client.write_all(b"{}").unwrap();
    let mut response = String::new();
    client.read_to_string(&mut response).unwrap();
    worker.join().unwrap().unwrap();
    assert!(response.contains("200 Response"));
    assert!(response.contains("wubi-sentence-lab"));
}
