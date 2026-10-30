//! 仅调用本机 Ollama 的英语翻译，缓存限于本实验进程。
use reqwest::blocking::Client;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct Translator {
    client: Client,

    cache: HashMap<String, String>,
}

impl Translator {
    pub fn new() -> Result<Self, String> {
        let client = Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(65))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            client,
            cache: HashMap::new(),
        })
    }

    pub fn translate(&mut self, text: &str) -> Result<Value, String> {
        let text = text.trim();
        if text.is_empty() || text.chars().count() > 256 {
            return Err("待翻译文字长度须为 1–256 字".into());
        }
        let now = Instant::now();
        if let Some(english) = self.cache.get(text) {
            return Ok(json!({"text":text,"english":english,"elapsed_ms":0,"cached":true}));
        }
        let prompt = format!(
            "You are a professional Chinese (Simplified) (zh-Hans) to English (en) translator. Your goal is to accurately convey the meaning and nuances of the original Chinese (Simplified) text while adhering to English grammar, vocabulary, and cultural sensitivities.\nProduce only the English translation, without any additional explanations or commentary. Please translate the following Chinese (Simplified) text into English:\n\n\n{text}"
        );
        let response: Value = self.client.post("http://127.0.0.1:11434/v1/chat/completions")
            .json(&json!({"model":"translategemma:4b","messages":[{"role":"user","content":prompt}],"temperature":0,"max_tokens":1024,"stream":false}))
            .send().map_err(|e| format!("本地模型连接失败：{e}"))?
            .error_for_status().map_err(|e| format!("本地模型返回错误：{e}"))?
            .json().map_err(|e| e.to_string())?;
        if response["choices"][0]["finish_reason"] == "length" {
            return Err("译文超过长度限制，请缩短句子重试".into());
        }
        let english = response["choices"][0]["message"]["content"]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .ok_or("模型没有返回译文")?
            .trim()
            .to_owned();
        if self.cache.len() >= 128 {
            self.cache.clear();
        }
        self.cache.insert(text.into(), english.clone());
        Ok(
            json!({"text":text,"english":english,"elapsed_ms":now.elapsed().as_millis(),"cached":false}),
        )
    }
}
