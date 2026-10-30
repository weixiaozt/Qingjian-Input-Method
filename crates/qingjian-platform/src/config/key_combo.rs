//! 组合键与功能键，配置使用 control+option+t 或 f8。
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::modifiers::Modifiers;

/// 功能键内部沿用 Cocoa 功能键的私用字符区，不会作为文字上屏。
const FUNCTION_BASE: u32 = 0xf703;

/// 字符键需修饰键，F1–F12 可单独使用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct KeyCombo {
    /// 字符键至少一个；功能键允许为空。
    pub modifiers: Modifiers,

    /// 小写字符键，或由 function() 构造的功能键内部编码。
    pub key: char,
}

impl KeyCombo {
    pub const TRADITIONAL_DEFAULT: Self = Self {
        modifiers: Modifiers::SHIFT_CONTROL,
        key: 'f',
    };

    pub const PREDICT_DEFAULT: Self = Self {
        key: 'j',
        ..Self::TRANSLATE_DEFAULT
    };

    pub const TRANSLATE_DEFAULT: Self = Self {
        modifiers: Modifiers {
            option: true,
            shift: false,
            control: true,
            command: false,
        },
        key: 't',
    };

    pub fn function(number: u8) -> Option<Self> {
        (1..=12).contains(&number).then(|| Self {
            modifiers: Modifiers::default(),
            key: char::from_u32(FUNCTION_BASE + u32::from(number)).expect("function key range"),
        })
    }

    pub fn function_number(self) -> Option<u8> {
        Self::function_key_number(self.key)
    }

    pub fn function_key_number(key: char) -> Option<u8> {
        let number = (key as u32).checked_sub(FUNCTION_BASE)?;
        (1..=12).contains(&number).then_some(number as u8)
    }

    pub fn key_label(self) -> String {
        self.function_number()
            .map(|n| format!("F{n}"))
            .unwrap_or_else(|| self.key.to_ascii_uppercase().to_string())
    }

    /// 配置文件里的写法。
    pub fn key_string(&self) -> String {
        let key = self
            .function_number()
            .map(|n| format!("f{n}"))
            .unwrap_or_else(|| self.key.to_string());
        if self.modifiers.is_empty() {
            key
        } else {
            format!("{}+{}", self.modifiers.key(), key)
        }
    }

    /// 给人看的写法：`⌃⌥T`。
    pub fn label(&self) -> String {
        format!("{}{}", self.modifiers.label(), self.key_label())
    }
}

impl Default for KeyCombo {
    fn default() -> Self {
        Self::TRANSLATE_DEFAULT
    }
}

impl FromStr for KeyCombo {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        let (modifiers, key) = text.rsplit_once('+').unwrap_or(("", text));
        let key = key.trim().to_ascii_lowercase();
        if let Some(number) = key.strip_prefix('f').and_then(|n| n.parse::<u8>().ok()) {
            let mut combo = Self::function(number).ok_or("function key must be F1-F12")?;
            if !modifiers.trim().is_empty() {
                combo.modifiers = modifiers.parse()?;
            }
            return Ok(combo);
        }
        if modifiers.trim().is_empty() {
            return Err(format!("character key needs modifiers: {text:?}"));
        }
        let mut chars = key.trim().chars();
        let (Some(key), None) = (chars.next(), chars.next()) else {
            return Err(format!("key must be a single character: {key:?}"));
        };
        if !key.is_ascii_alphanumeric() && key != '.' {
            return Err(format!("key must be a letter, digit or period: {key:?}"));
        }
        Ok(Self {
            modifiers: modifiers.parse()?,
            key: key.to_ascii_lowercase(),
        })
    }
}

impl TryFrom<String> for KeyCombo {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<KeyCombo> for String {
    fn from(combo: KeyCombo) -> Self {
        combo.key_string()
    }
}

impl fmt::Display for KeyCombo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.key_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_keys_round_trip_without_modifiers() {
        for number in 1..=12 {
            let combo: KeyCombo = format!("F{number}").parse().unwrap();
            assert_eq!(combo.function_number(), Some(number));
            assert!(combo.modifiers.is_empty());
            assert_eq!(combo.label(), format!("F{number}"));
            assert_eq!(combo.key_string().parse::<KeyCombo>().unwrap(), combo);
        }
        assert!("f13".parse::<KeyCombo>().is_err());
        assert!("f0".parse::<KeyCombo>().is_err());
        assert_eq!(
            "ctrl+f8".parse::<KeyCombo>().unwrap().key_string(),
            "control+f8"
        );
    }

    #[test]
    fn parses_and_labels() {
        let combo: KeyCombo = "control+option+t".parse().unwrap();
        assert_eq!(combo, KeyCombo::TRANSLATE_DEFAULT);
        assert_eq!(combo.label(), "⌃⌥T");
        assert_eq!(combo.key_string(), "control+option+t");
        assert!("t".parse::<KeyCombo>().is_err());
        assert!("option+tt".parse::<KeyCombo>().is_err());
        assert!("option+-".parse::<KeyCombo>().is_err());
        for option in ["control+shift+t", "control+option+e", "shift+command+9"] {
            assert_eq!(option.parse::<KeyCombo>().unwrap().key_string(), option);
        }
    }
}
