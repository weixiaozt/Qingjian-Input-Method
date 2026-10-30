//! 一次性切换整句方案，保留其余设置和注释，避免半写入的混输状态。
use super::{Config, ConfigError, TEMPLATE, section_table, write_file};
use std::path::Path;

impl Config {
    pub fn set_sentence_input_scheme(path: &Path, wubi: bool) -> Result<Self, ConfigError> {
        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => TEMPLATE.to_owned(),
            Err(source) => {
                return Err(ConfigError::Read {
                    path: path.to_owned(),
                    source,
                });
            }
        };
        let mut document: toml_edit::DocumentMut =
            source.parse().map_err(|source| ConfigError::Edit {
                path: path.to_owned(),
                source: Box::new(source),
            })?;
        section_table(&mut document, "general")["scheme"] =
            toml_edit::value(if wubi { "none" } else { "pinyin" });
        section_table(&mut document, "general")["code"] =
            toml_edit::value(if wubi { "wubi86-sentence" } else { "" });
        section_table(&mut document, "quick_translation")["wubi_sentence"] = toml_edit::value(true);
        let text = document.to_string();
        let config: Self = toml::from_str(&text).map_err(|source| ConfigError::Parse {
            path: path.to_owned(),
            source: Box::new(source),
        })?;
        if let Some(key) = config.predict.resolve_api_key() {
            crate::logs::secrets::register(&key);
        }
        write_file(path, &text)?;
        Ok(config)
    }
}
