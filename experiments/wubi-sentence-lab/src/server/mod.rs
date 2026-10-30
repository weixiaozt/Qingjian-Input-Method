//! 仅监听回环地址的实验台服务。
mod http;
mod state;
#[cfg(test)]
mod tests;
use crate::decoder::Decoder;
use crate::translator::Translator;
use state::State;
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::time::Duration;

pub fn run(root: PathBuf, decoder: Decoder, open: bool) -> Result<(), String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let origin = format!(
        "http://{}",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let token = uuid::Uuid::new_v4().simple().to_string();
    let url = format!("{origin}/#{token}");
    std::fs::write(
        root.join("打开实验台.url"),
        format!("[InternetShortcut]\r\nURL={url}\r\n"),
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(
        root.join("运行状态.json"),
        serde_json::json!({"url":url,"pid":std::process::id(),"entries":decoder.entries})
            .to_string(),
    )
    .map_err(|e| e.to_string())?;
    let state = Arc::new(State {
        root,
        origin,
        token,
        decoder: Mutex::new(decoder),
        translator: Mutex::new(Translator::new()?),
        running: AtomicBool::new(true),
        active: AtomicUsize::new(0),
    });
    if open {
        std::process::Command::new("explorer.exe")
            .arg(&url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    while state.running.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _)) => {
                if state.active.fetch_add(1, Ordering::Relaxed) >= 16 {
                    state.active.fetch_sub(1, Ordering::Relaxed);
                    continue;
                }
                let state = state.clone();
                std::thread::spawn(move || {
                    let _ = http::serve(stream, &state);
                    state.active.fetch_sub(1, Ordering::Relaxed);
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(40))
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    // 留出发送退出响应的时间；译文请求不能拖住关闭。
    let until = std::time::Instant::now() + Duration::from_secs(1);
    while state.active.load(Ordering::Relaxed) > 0 && std::time::Instant::now() < until {
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}
