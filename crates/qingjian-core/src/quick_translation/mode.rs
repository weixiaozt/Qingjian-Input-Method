//! 快捷翻译确认动作，默认保留原文并追加译文。
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationMode {
    Preview,
    #[default]
    Bilingual,
    Replace,
}

impl TranslationMode {
    pub fn next(self) -> Self {
        match self {
            Self::Preview => Self::Bilingual,
            Self::Bilingual => Self::Replace,
            Self::Replace => Self::Preview,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Preview => "仅查看",
            Self::Bilingual => "双语",
            Self::Replace => "英文",
        }
    }

    pub fn confirmation(self) -> &'static str {
        match self {
            Self::Preview => "再按翻译键收起",
            Self::Bilingual => "再按翻译键追加英文",
            Self::Replace => "再按翻译键替换中文",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Preview => "preview",
            Self::Bilingual => "bilingual",
            Self::Replace => "replace",
        }
    }
}
