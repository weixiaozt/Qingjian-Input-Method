//! 在异步只读会话中读取光标前最后一句，不修改应用选区。
use super::super::{
    anchor::{anchor_rect, mouse_screen_rect, selection_range},
    surrounding::private_input,
};
use super::{Snapshot, snapshot::read_text};
use crate::com::{composition::Shared, service::SharedClient};
use qingjian_core::quick_translation::last_sentence;
use std::rc::Rc;
use windows::Win32::UI::TextServices::{
    ITfContext, ITfEditSession, ITfEditSession_Impl, ITfThreadMgr, TF_ES_READ,
};
use windows::core::{Result, implement};

#[implement(ITfEditSession)]
struct ReadSentence {
    context: ITfContext,

    manager: ITfThreadMgr,

    engine: SharedClient,

    shared: Rc<Shared>,

    request: u64,
}

impl ITfEditSession_Impl for ReadSentence_Impl {
    fn DoEditSession(&self, ec: u32) -> Result<()> {
        let captured = self.capture(ec);
        let failure = captured.as_ref().err().map(|code| (*code).to_owned());
        if let Some(code) = &failure {
            crate::com::log::log(&format!("Sentence capture rejected: {code}"));
        }
        let snapshot = captured.ok();
        let text = snapshot
            .as_ref()
            .map(|s| s.text.clone())
            .unwrap_or_default();
        let rect = snapshot
            .as_ref()
            .map(|s| anchor_rect(&self.context, ec, &s.caret))
            .unwrap_or_else(mouse_screen_rect);
        if let Ok(mut guard) = self.engine.try_borrow_mut()
            && let Some(client) = guard.as_mut()
        {
            match client.sentence(self.request, text, rect, failure) {
                Ok(frame) if !frame.is_empty() => {
                    if let Some(snapshot) = snapshot {
                        self.shared.set_sentence(snapshot);
                        self.shared.set_translating(true);
                    }
                }
                _ => self.shared.set_translating(false),
            }
        }
        Ok(())
    }
}

impl ReadSentence_Impl {
    fn capture(&self, ec: u32) -> std::result::Result<Snapshot, &'static str> {
        let active = unsafe { self.manager.GetFocus().and_then(|doc| doc.GetTop()) }
            .map_err(|_| "focus_unavailable")?;
        if active != self.context {
            return Err("different_context");
        }
        let caret = selection_range(&self.context, ec).ok_or("selection_unavailable")?;
        if !unsafe { caret.IsEmpty(ec) }
            .map_err(|_| "selection_unreadable")?
            .as_bool()
        {
            return Err("selection_not_empty");
        }
        if private_input(&self.context, ec, &caret) {
            return Err("private_input");
        }
        let probe = unsafe { caret.Clone() }.map_err(|_| "clone_failed")?;
        let mut shifted = 0;
        unsafe { probe.ShiftStart(ec, -1024, &mut shifted, std::ptr::null()) }
            .map_err(|_| "lookback_unavailable")?;
        let before = read_text(&probe, ec).map_err(|_| "text_unreadable")?;
        crate::com::log::log(&format!(
            "Sentence capture: shifted={shifted}, utf16={}",
            before.encode_utf16().count()
        ));
        if before.is_empty() {
            return Err("no_surrounding_text");
        }
        let span = last_sentence(&before, shifted == -1024).ok_or("no_complete_sentence")?;
        let text = before[span.clone()].to_owned();
        let range = unsafe { caret.Clone() }.map_err(|_| "clone_failed")?;
        let back = before[span.start..].encode_utf16().count() as i32;
        let trailing = before[span.end..].encode_utf16().count() as i32;
        unsafe { range.ShiftStart(ec, -back, &mut shifted, std::ptr::null()) }
            .map_err(|_| "range_start_unavailable")?;
        if shifted != -back {
            return Err("range_start_mismatch");
        }
        unsafe { range.ShiftEnd(ec, -trailing, &mut shifted, std::ptr::null()) }
            .map_err(|_| "range_end_unavailable")?;
        if shifted != -trailing {
            return Err("range_end_mismatch");
        }
        Ok(Snapshot {
            request: self.request,
            foreground: unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow() },
            context: self.context.clone(),
            manager: self.manager.clone(),
            range,
            caret,
            probe,
            before,
            text,
        })
    }
}

pub(crate) fn request(
    context: &ITfContext,
    manager: ITfThreadMgr,
    client_id: u32,
    engine: SharedClient,
    shared: Rc<Shared>,
    request: u64,
) -> Result<()> {
    shared.clear_sentence();
    let session: ITfEditSession = ReadSentence {
        context: context.clone(),
        manager,
        engine,
        shared,
        request,
    }
    .into();
    super::super::update::request(context, client_id, session, TF_ES_READ)
}
