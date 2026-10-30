//! 手动验证本机 TranslateGemma 接口；不读用户输入或改输入法设置。
use std::time::{Duration, Instant};

use qingjian_core::{PredictionKind, PredictionRequest, Predictor};
use qingjian_predict::{CloudPredictor, PredictConfig};

fn main() {
    let config = PredictConfig {
        enabled: true,
        automatic: false,
        base_url: "http://127.0.0.1:11434/v1".into(),
        model: "translategemma:4b".into(),
        api_key: Some("ollama".into()),
        timeout_ms: 60000,
        debounce_ms: 0,
        min_interval_ms: 0,
        reasoning_effort: String::new(),
        ..PredictConfig::default()
    };
    let mut provider = CloudPredictor::new(&config).expect("create local provider");
    let started = Instant::now();
    provider.submit(PredictionRequest {
        sequence: 1,
        manual: true,
        kind: PredictionKind::Translate,
        text: "不好意思，我刚才在开会，没看到你的消息。".into(),
        target_language: "en".into(),
        before: String::new(),
        after: String::new(),
        pinyin: String::new(),
        letters: String::new(),
        syllables: 0,
        candidates: Vec::new(),
        guess: String::new(),
        max_items: 1,
        want_sentence: true,
    });
    loop {
        if let Some(reply) = provider.poll() {
            assert_eq!(reply.sequence, 1);
            let text = reply.sentence.expect("translation must succeed");
            println!("{} ms: {}", started.elapsed().as_millis(), text);
            break;
        }
        assert!(
            started.elapsed() < Duration::from_secs(65),
            "local translation timed out"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
