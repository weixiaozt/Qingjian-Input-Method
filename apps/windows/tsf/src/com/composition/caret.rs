//! 提交插入点校验：只接受UTF-16字符边界，避免截断代理对。
pub(super) fn checked_offset(text: &str, offset: usize) -> Option<i32> {
    let mut units = 0;
    for ch in text.chars() {
        if units == offset {
            return i32::try_from(offset).ok();
        }
        units += ch.len_utf16();
        if units > offset {
            return None;
        }
    }
    (units == offset)
        .then(|| i32::try_from(offset).ok())
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::checked_offset;
    #[test]
    fn accepts_chinese_end_but_not_half_a_surrogate_pair() {
        assert_eq!(checked_offset("你𠮷好\nHello", 4), Some(4));
        assert_eq!(checked_offset("你𠮷好\nHello", 2), None);
        assert_eq!(checked_offset("中文\r\nEnglish", 2), Some(2));
        assert_eq!(checked_offset("中文", 0), Some(0));
        assert_eq!(checked_offset("中文", 3), None);
        assert_eq!(checked_offset("中文", usize::MAX), None);
    }
}
