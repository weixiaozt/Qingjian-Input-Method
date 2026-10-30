//! 有界 HTTP 请求处理；状态变更必须携带本次启动令牌。
use super::state::State;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::atomic::Ordering;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(super) fn serve(mut stream: TcpStream, state: &State) -> Result<(), String> {
    // Windows 的 accept 可能继承 listener 的非阻塞模式；工作线程必须完整等待请求体。
    stream.set_nonblocking(false).map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| e.to_string())?;
    if line.len() > 2048 {
        return Err("request line too long".into());
    }
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("").to_owned();
    let path = parts.next().unwrap_or("").to_owned();
    let mut length = 0;
    let mut token = String::new();
    let mut origin = String::new();
    let mut host = String::new();
    let mut expect_continue = false;
    let mut header_bytes = 0;
    loop {
        line.clear();
        reader.read_line(&mut line).map_err(|e| e.to_string())?;
        header_bytes += line.len();
        if header_bytes > 8192 {
            return Err("headers too long".into());
        }
        if line.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            match name.trim().to_ascii_lowercase().as_str() {
                "content-length" => {
                    length = value.trim().parse::<usize>().map_err(|e| e.to_string())?
                }
                "x-lab-token" => token = value.trim().to_owned(),
                "origin" => origin = value.trim().to_owned(),
                "host" => host = value.trim().to_owned(),
                "expect" => expect_continue = value.trim().eq_ignore_ascii_case("100-continue"),
                _ => {}
            }
        }
    }
    if format!("http://{host}") != state.origin {
        return reply(&mut stream, 403, "text/plain", b"Invalid host");
    }
    if method == "GET" {
        return match path.as_str() {
            "/" => reply(
                &mut stream,
                200,
                "text/html; charset=utf-8",
                include_bytes!("../../ui/index.html"),
            ),
            "/app.js" => reply(
                &mut stream,
                200,
                "text/javascript; charset=utf-8",
                include_bytes!("../../ui/app.js"),
            ),
            "/style.css" => reply(
                &mut stream,
                200,
                "text/css; charset=utf-8",
                include_bytes!("../../ui/style.css"),
            ),
            _ => reply(&mut stream, 404, "text/plain", b"Not found"),
        };
    }
    if method != "POST" || token != state.token || (!origin.is_empty() && origin != state.origin) {
        return reply(&mut stream, 403, "text/plain", b"Forbidden");
    }
    if length > 16384 {
        return reply(&mut stream, 413, "text/plain", b"Body too large");
    }
    if expect_continue {
        stream
            .write_all(b"HTTP/1.1 100 Continue\r\n\r\n")
            .map_err(|e| e.to_string())?;
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).map_err(|e| e.to_string())?;
    let input: Value = serde_json::from_slice(&body).map_err(|e| e.to_string())?;
    let result: Result<Value, String> = match path.as_str() {
        "/api/status" => Ok(json!({"application":"wubi-sentence-lab","version":"0.1.0"})),
        "/api/decode" => serde_json::from_value(input)
            .map_err(|e| e.to_string())
            .and_then(|request| {
                state
                    .decoder
                    .lock()
                    .map_err(|e| e.to_string())?
                    .decode(request)
            })
            .and_then(|response| serde_json::to_value(response).map_err(|e| e.to_string())),
        "/api/translate" => match state.translator.try_lock() {
            Ok(mut translator) => translator.translate(input["text"].as_str().unwrap_or("")),
            Err(_) => {
                return reply_json(
                    &mut stream,
                    429,
                    &json!({"error":"正在完成前一条本地翻译","retry":true}),
                );
            }
        },
        "/api/feedback" => {
            let expected = input["expected"].as_str().unwrap_or("").trim();
            let codes = input["codes"].as_str().unwrap_or("");
            if expected.is_empty()
                || expected.chars().count() > 256
                || codes.is_empty()
                || codes.len() > 96
            {
                Err("请填写编码和你期望的中文句子".into())
            } else {
                let entry = json!({"time":SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),"codes":codes,"expected":expected,"actual":input["actual"]});
                let mut file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(state.root.join("实验反馈.jsonl"))
                    .map_err(|e| e.to_string())?;
                writeln!(file, "{entry}").map_err(|e| e.to_string())?;
                Ok(json!({"saved":true}))
            }
        }
        "/api/quit" => {
            state.running.store(false, Ordering::Relaxed);
            Ok(json!({"stopped":true}))
        }
        _ => Err("Unknown endpoint".into()),
    };
    match result {
        Ok(value) => reply_json(&mut stream, 200, &value),
        Err(error) => reply_json(&mut stream, 400, &json!({"error":error})),
    }
}

fn reply_json(stream: &mut TcpStream, status: u16, value: &Value) -> Result<(), String> {
    reply(
        stream,
        status,
        "application/json; charset=utf-8",
        value.to_string().as_bytes(),
    )
}

fn reply(stream: &mut TcpStream, status: u16, mime: &str, body: &[u8]) -> Result<(), String> {
    write!(stream, "HTTP/1.1 {status} Response\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'\r\nConnection: close\r\n\r\n", body.len()).map_err(|e| e.to_string())?;
    stream.write_all(body).map_err(|e| e.to_string())
}
