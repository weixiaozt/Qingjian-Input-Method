//! 原样上屏只接受前端确认的Shift单击；失焦清理仍走独立的Commit消息。
use crate::dispatch::Router;
use qingjian_platform::protocol::{KeyEvent, KeyOutcome, ServerMessage, SessionId};

impl Router {
    pub(crate) fn raw_shift_key(
        &mut self,
        session: SessionId,
        event: &KeyEvent,
    ) -> Option<ServerMessage> {
        if !event.released || !matches!(event.virtual_key, 0x10 | 0xA0 | 0xA1) {
            return None;
        }
        let allowed = self.config.english_mode
            && self.config.switch_mode.shift
            && !event.modifiers.english_mode
            && !event.modifiers.has_command_key()
            && !event.modifiers.caps
            && !self.panel_open()
            && self.translation.is_none();
        let commit = if allowed && self.composing() {
            let raw = self.engine.take_raw_explicit();
            self.reset_composition();
            Some(raw).filter(|text| !text.is_empty())
        } else {
            None
        };
        Some(ServerMessage::KeyResult {
            session,
            outcome: KeyOutcome::Consumed,
            commit,
            frame: self.current_frame(),
            english: None,
        })
    }
}
