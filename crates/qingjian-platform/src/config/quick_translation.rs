//! 独立的本机快捷翻译配置，不启用云联想。
use qingjian_core::quick_translation::TranslationMode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct QuickTranslationConfig {
    pub enabled: bool,

    pub candidate_enabled: bool,

    pub wubi_sentence: bool,

    pub hold_ms: u64,

    pub candidate_debounce_ms: u64,

    pub mode: TranslationMode,

    pub newline: bool,

    pub base_url: String,

    pub model: String,
}

impl Default for QuickTranslationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            candidate_enabled: false,
            wubi_sentence: false,
            hold_ms: 350,
            candidate_debounce_ms: 250,
            mode: TranslationMode::Bilingual,
            newline: false,
            base_url: "http://127.0.0.1:11434/v1".into(),
            model: "translategemma:4b".into(),
        }
    }
}
