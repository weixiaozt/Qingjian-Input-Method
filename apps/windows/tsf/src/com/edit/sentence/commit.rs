//! 翻译确认使用同一个读写编辑会话校验并写入，永远不模拟退格或回车。
use crate::com::composition::{Shared, move_selection_to_end};
use qingjian_core::quick_translation::TranslationMode;
use std::rc::Rc;
use windows::Win32::UI::TextServices::{
    ITfContext, ITfEditSession, ITfEditSession_Impl, TF_ES_READWRITE,
};
use windows::core::{Result, implement};

#[implement(ITfEditSession)]
struct CommitTranslation {
    context: ITfContext,

    shared: Rc<Shared>,

    request: u64,

    text: String,

    mode: TranslationMode,

    newline: bool,
}

impl ITfEditSession_Impl for CommitTranslation_Impl {
    fn DoEditSession(&self, ec: u32) -> Result<()> {
        let Some(snapshot) = self.shared.sentence().filter(|s| s.request == self.request) else {
            return Ok(());
        };
        if snapshot.context != self.context || !snapshot.valid(ec) {
            self.shared.cancel_sentence();
            crate::com::log::log("Translation commit cancelled: document or caret changed");
            return Ok(());
        }
        let result = (|| {
            let (range, text) = match self.mode {
                TranslationMode::Preview => return Ok(()),
                TranslationMode::Replace => (snapshot.range.clone(), self.text.clone()),
                TranslationMode::Bilingual => (
                    snapshot.caret.clone(),
                    format!("{}{}", if self.newline { "\r\n" } else { " " }, self.text),
                ),
            };
            let text: Vec<u16> = text.encode_utf16().collect();
            unsafe {
                range.SetText(ec, 0, &text)?;
            }
            move_selection_to_end(&self.context, ec, &range)
        })();
        self.shared.cancel_sentence();
        result
    }
}

pub(crate) fn request(
    context: &ITfContext,
    client_id: u32,
    shared: Rc<Shared>,
    request: u64,
    text: String,
    mode: TranslationMode,
    newline: bool,
) -> Result<()> {
    let session: ITfEditSession = CommitTranslation {
        context: context.clone(),
        shared,
        request,
        text,
        mode,
        newline,
    }
    .into();
    super::super::update::request(context, client_id, session, TF_ES_READWRITE)
}
