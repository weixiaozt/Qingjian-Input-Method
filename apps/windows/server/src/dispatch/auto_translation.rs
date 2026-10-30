//! 将候选和空格生命周期交给 Core，复用正常组句上屏通道。
use super::Router;
use qingjian_platform::protocol::{KeyEvent, KeyOutcome, ServerMessage, SessionId};
use std::time::{Duration, Instant};

impl Router {
    fn auto_translation_enabled(&self) -> bool {
        self.config.quick_translation.candidate_enabled
            && !self.panel_open()
            && self.translation.is_none()
            && self.focused_dll_protocol()
                >= qingjian_platform::protocol::CANDIDATE_TRANSLATION_SINCE
    }

    pub(super) fn sync_candidate_translation(&mut self) {
        let candidate = self
            .auto_translation_enabled()
            .then(|| self.layout_candidate(self.highlight))
            .flatten();
        let delay = Duration::from_millis(
            self.config
                .quick_translation
                .candidate_debounce_ms
                .clamp(100, 2000),
        );
        self.engine
            .sync_auto_translation(candidate, delay, Instant::now());
    }

    fn translation_hold(&self) -> Duration {
        Duration::from_millis(self.config.quick_translation.hold_ms.clamp(200, 1500))
    }

    pub(super) fn candidate_space_key(
        &mut self,
        session: SessionId,
        event: &KeyEvent,
    ) -> Option<ServerMessage> {
        let space = event.virtual_key == 0x20;
        if event.released {
            if !space || !self.translation_space_down {
                return Some(self.space_reply(session, None));
            }
            self.translation_space_down = false;
            let commit = self
                .engine
                .release_translation_space(Instant::now(), self.translation_hold());
            return Some(self.space_reply(session, commit));
        }
        if space && self.translation_space_down {
            return Some(self.space_reply(session, None));
        }
        if !space
            && event.character.is_some_and(|c| !c.is_control())
            && !event.modifiers.has_command_key()
            && let Some(text) = self
                .engine
                .interrupt_translation_space(Instant::now(), self.translation_hold())
        {
            self.deliver_clicked(session, text);
        }
        if !space || event.modifiers.has_command_key() || event.modifiers.shift {
            let cancelled = self.engine.cancel_translation_space();
            if event.virtual_key == 0x1b && cancelled {
                return Some(self.space_reply(session, None));
            }
            return None;
        }
        if !self.auto_translation_enabled() {
            return None;
        }
        self.sync_candidate_translation();
        if !self.engine.press_translation_space(Instant::now()) {
            return None;
        }
        self.translation_space_down = true;
        self.navigated = true;
        self.stop_rescoring();
        Some(self.space_reply(session, None))
    }

    fn space_reply(&mut self, session: SessionId, commit: Option<String>) -> ServerMessage {
        if commit.is_some() {
            self.recompose();
        }
        self.sync_candidate_translation();
        let shown = self.self_drawn_frame();
        self.reconcile_candidates(&shown);
        ServerMessage::KeyResult {
            session,
            outcome: KeyOutcome::Consumed,
            commit: commit.filter(|text| !text.is_empty()),
            frame: self.current_frame(),
            english: None,
        }
    }

    pub(super) fn advance_candidate_translation(&mut self) {
        self.sync_candidate_translation();
        let Some(session) = self.focused else {
            return;
        };
        if let Some(text) = self
            .engine
            .advance_translation_space(Instant::now(), self.translation_hold())
        {
            self.deliver_clicked(session, text);
        }
    }
}
