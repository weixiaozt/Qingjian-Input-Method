//! 快照绑定实际文档、选区与原文，确认前同一编辑锁内复核。
use super::super::{anchor::selection_range, surrounding::private_input};
use windows::Win32::UI::TextServices::{
    ITfContext, ITfRange, ITfThreadMgr, TF_ANCHOR_END, TF_ANCHOR_START,
};
use windows::core::Result;

#[derive(Clone)]
pub(crate) struct Snapshot {
    pub request: u64,

    pub foreground: windows::Win32::Foundation::HWND,

    pub context: ITfContext,

    pub manager: ITfThreadMgr,

    pub range: ITfRange,

    pub caret: ITfRange,

    pub probe: ITfRange,

    pub before: String,

    pub text: String,
}

impl Snapshot {
    pub fn valid(&self, ec: u32) -> bool {
        if unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow() }
            != self.foreground
        {
            return false;
        }
        let Ok(active) = (unsafe { self.manager.GetFocus().and_then(|doc| doc.GetTop()) }) else {
            return false;
        };
        if active != self.context {
            return false;
        }
        let Some(current) = selection_range(&self.context, ec) else {
            return false;
        };
        if private_input(&self.context, ec, &current) {
            return false;
        }
        (unsafe {
            current.IsEmpty(ec).is_ok_and(|empty| empty.as_bool())
                && current
                    .CompareStart(ec, &self.caret, TF_ANCHOR_START)
                    .is_ok_and(|offset| offset == 0)
                && current
                    .CompareEnd(ec, &self.caret, TF_ANCHOR_END)
                    .is_ok_and(|offset| offset == 0)
        }) && read_text(&self.range, ec).is_ok_and(|text| text == self.text)
            && read_text(&self.probe, ec).is_ok_and(|text| text == self.before)
    }
}

pub(super) fn read_text(range: &ITfRange, ec: u32) -> Result<String> {
    let mut buffer = [0u16; 1025];
    let mut fetched = 0;
    unsafe {
        range.GetText(ec, 0, &mut buffer, &mut fetched)?;
    }
    if fetched > 1024 {
        return Err(windows::core::Error::from(
            windows::Win32::Foundation::E_FAIL,
        ));
    }
    String::from_utf16(&buffer[..fetched as usize])
        .map_err(|_| windows::core::Error::from(windows::Win32::Foundation::E_FAIL))
}
