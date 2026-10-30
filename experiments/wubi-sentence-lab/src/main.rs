//! 隔离的本地实验台；不注册输入法、不读写现用青简配置。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod decoder;
mod server;
mod translator;

use decoder::{Decoder, Request};
use std::io::{BufRead, Write};
use std::path::PathBuf;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        if let Ok(exe) = std::env::current_exe() {
            let _ = std::fs::write(exe.with_file_name("启动错误.txt"), &error);
        }
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    let root = args
        .iter()
        .position(|a| a == "--root")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .to_path_buf()
        });
    if !args.iter().any(|a| a == "--jsonl" || a == "--no-open") && reuse(&root) {
        return Ok(());
    }
    let decoder = Decoder::load(&root.join("data"))?;
    if args.iter().any(|a| a == "--jsonl") {
        let mut output = std::io::stdout().lock();
        for line in std::io::stdin().lock().lines() {
            let request: Result<Request, _> =
                serde_json::from_str(&line.map_err(|e| e.to_string())?);
            let result = request
                .map_err(|e| e.to_string())
                .and_then(|r| decoder.decode(r));
            let value = match result {
                Ok(response) => serde_json::to_value(response).unwrap(),
                Err(error) => serde_json::json!({"error":error}),
            };
            writeln!(output, "{value}").map_err(|e| e.to_string())?;
            output.flush().map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    server::run(root, decoder, !args.iter().any(|a| a == "--no-open"))
}

/// 双击时复用同一实验服务；不凭记录的 PID 终止任何进程。
fn reuse(root: &std::path::Path) -> bool {
    let Some(state) = std::fs::read(root.join("运行状态.json"))
        .ok()
        .and_then(|data| serde_json::from_slice::<serde_json::Value>(&data).ok())
    else {
        return false;
    };
    let Some(url) = state["url"]
        .as_str()
        .and_then(|url| reqwest::Url::parse(url).ok())
    else {
        return false;
    };
    if url.scheme() != "http" || url.host_str() != Some("127.0.0.1") || url.path() != "/" {
        return false;
    }
    let Some(token) = url
        .fragment()
        .filter(|value| value.len() == 32 && value.bytes().all(|c| c.is_ascii_hexdigit()))
    else {
        return false;
    };
    let Ok(client) = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(2))
        .build()
    else {
        return false;
    };
    let endpoint = format!("{}/api/status", url.origin().ascii_serialization());
    let valid = client
        .post(endpoint)
        .header("X-Lab-Token", token)
        .json(&serde_json::json!({}))
        .send()
        .ok()
        .and_then(|r| r.json::<serde_json::Value>().ok())
        .is_some_and(|value| value["application"] == "wubi-sentence-lab");
    valid
        && std::process::Command::new("explorer.exe")
            .arg(url.as_str())
            .spawn()
            .is_ok()
}
