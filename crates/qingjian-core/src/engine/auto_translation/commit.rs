//! 自动翻译确认的文本与中文末尾位置；位置使用Windows文档的UTF-16单位。
#[derive(Debug, Clone)]
pub struct AutoCommit {
    pub text: String,

    pub caret_utf16: Option<usize>,

    pub(super) english_chars: usize,

    pub(super) chinese_tail: Option<char>,
}

impl AutoCommit {
    pub fn plain(text: String) -> Self {
        Self {
            text,
            caret_utf16: None,
            english_chars: 0,
            chinese_tail: None,
        }
    }

    pub(super) fn bilingual(chinese: &str, english: &str) -> Self {
        Self {
            text: format!("{chinese}\n{english}"),
            caret_utf16: Some(chinese.encode_utf16().count()),
            english_chars: 1 + english.chars().count(),
            chinese_tail: chinese.chars().last(),
        }
    }

    pub fn prepend(mut self, prefix: &str) -> Self {
        self.caret_utf16 = self
            .caret_utf16
            .map(|offset| offset + prefix.encode_utf16().count());
        self.text = prefix.to_owned() + &self.text;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::AutoCommit;
    #[test]
    fn chinese_end_is_before_newline_and_counts_surrogate_pairs() {
        let c = AutoCommit::bilingual("你𠮷好", "Hello\nthere");
        assert_eq!(c.text, "你𠮷好\nHello\nthere");
        assert_eq!(c.caret_utf16, Some(4));
        let c = c.prepend("🙂");
        assert_eq!(c.caret_utf16, Some(6));
        assert_eq!(AutoCommit::plain("中文".into()).caret_utf16, None);
    }
}
