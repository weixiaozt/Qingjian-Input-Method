//! 已确认的Shift单击：中文组句时请求原样上屏，其余仍走模式切换。
use qingjian_platform::protocol::{KeyEvent, KeyModifiers};

pub(crate) fn raw_shift_event(vk: u32, composing: bool, english: bool) -> Option<KeyEvent> {
    if !composing || english || !matches!(vk, 0x10 | 0xA0 | 0xA1) {
        return None;
    }
    let mut event = KeyEvent::new(vk, None, KeyModifiers::default());
    event.released = true;
    Some(event)
}

#[cfg(test)]
mod tests {
    use super::raw_shift_event;

    #[test]
    fn only_chinese_composition_uses_raw_shift() {
        for vk in [0x10, 0xA0, 0xA1] {
            let event = raw_shift_event(vk, true, false).unwrap();
            assert!(event.released);
            assert!(event.character.is_none());
            assert!(!event.modifiers.english_mode);
            assert!(raw_shift_event(vk, false, false).is_none());
            assert!(raw_shift_event(vk, true, true).is_none());
        }
        for vk in [0x11, 0xA2, 0xA3, 0x20, 0x0D] {
            assert!(raw_shift_event(vk, true, false).is_none());
        }
    }
}
