//! TranslateGemma 单条用户消息格式，原文不与 JSON 生成任务混用。
use qingjian_core::PredictionRequest;

pub(crate) fn body(model: &str, request: &PredictionRequest) -> serde_json::Value {
    let (source, source_code, target, target_code) = if request.target_language == "zh" {
        ("English", "en", "Chinese (Simplified)", "zh-Hans")
    } else {
        ("Chinese (Simplified)", "zh-Hans", "English", "en")
    };
    let prompt = format!(
        "You are a professional {source} ({source_code}) to {target} ({target_code}) translator. Your goal is to accurately convey the meaning and nuances of the original {source} text while adhering to {target} grammar, vocabulary, and cultural sensitivities.\nProduce only the {target} translation, without any additional explanations or commentary. Please translate the following {source} text into {target}:\n\n\n{}",
        request.text
    );
    serde_json::json!({ "model": model, "messages": [{"role":"user", "content":prompt}], "temperature":0, "max_tokens":1024, "stream":false })
}
