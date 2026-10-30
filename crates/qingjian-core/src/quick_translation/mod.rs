//! 快捷翻译的句子边界与输出模式；不读取平台文档、不访问网络。
mod mode;
pub use mode::TranslationMode;

/// 光标前的最后一句（UTF-8 字节范围）。缓冲左侧被截断且找不到边界时拒绝猜测。
pub fn last_sentence(text: &str, truncated: bool) -> Option<std::ops::Range<usize>> {
    let end = text.trim_end_matches([' ', '\t']).len();
    let source = &text[..end];
    if source.is_empty() || source.ends_with(['\n', '\r']) {
        return None;
    }
    let body = source.trim_end_matches(['。', '！', '？', '.', '!', '?', '”', '’', '"', '\'']);
    if body.trim().is_empty() {
        return None;
    }
    let boundary = body.char_indices().rev().find(|&(i, c)| {
        matches!(c, '。' | '！' | '？' | '!' | '?' | '\n' | '\r')
            || (c == '.' && body[i + 1..].starts_with(char::is_whitespace))
    });
    let start = match boundary {
        Some((i, c)) => i + c.len_utf8(),
        None if truncated => return None,
        None => 0,
    };
    let start = start + source[start..].len() - source[start..].trim_start().len();
    let sentence = &source[start..];
    if sentence.is_empty() || sentence.encode_utf16().count() > 500 {
        return None;
    }
    Some(start..end)
}

#[cfg(test)]
mod tests {
    use super::last_sentence;
    #[test]
    fn chinese_boundaries_keep_last_sentence_and_terminal_punctuation() {
        let s = "上一句。明天三点开会！  ";
        assert_eq!(&s[last_sentence(s, false).unwrap()], "明天三点开会！");
        let s = "上一句。未打标点，仍是一句";
        assert_eq!(&s[last_sentence(s, false).unwrap()], "未打标点，仍是一句");
    }
    #[test]
    fn preserves_decimal_and_unicode_offsets() {
        let s = "😀上一句。价格是3.14元。";
        assert_eq!(&s[last_sentence(s, false).unwrap()], "价格是3.14元。");
        let s = "First sentence. Second sentence.";
        assert_eq!(&s[last_sentence(s, false).unwrap()], "Second sentence.");
    }
    #[test]
    fn refuses_empty_overlong_or_partial_sentence() {
        assert!(last_sentence("\n", false).is_none());
        assert!(last_sentence("未完整读取的长句", true).is_none());
        assert!(last_sentence(&"字".repeat(501), false).is_none());
        assert!(last_sentence("上一句。末句。", true).is_some());
    }
}
