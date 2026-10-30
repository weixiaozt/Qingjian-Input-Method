//! 在全拼整句和五笔整句之间切换；忙时仅提示，不提交或丢弃原文。
use crate::dispatch::Router;
use qingjian_platform::protocol::{SCHEME_HOLD_SINCE, ServerMessage, SessionId};
use qingjian_platform::{Config, Scheme, code_schemes};
use std::time::{Duration, Instant};

#[cfg(test)]
mod tests;

impl Router {
    pub(super) fn toggle_input_scheme(
        &mut self,
        session: SessionId,
        client_composing: bool,
    ) -> ServerMessage {
        let allowed = self.config.right_control_scheme_hold_ms.is_some()
            && self
                .sessions
                .get(&session)
                .is_some_and(|info| info.protocol >= SCHEME_HOLD_SINCE && !info.private);
        if !allowed {
            return self.scheme_switch_reply(session, None);
        }
        if client_composing
            || !self.engine.composition().is_empty()
            || !self.engine.held_text().is_empty()
            || self.translation.is_some()
            || self.panel_open()
            || self.translation_space_down
            || self.clicked.is_some()
        {
            if self.focused == Some(session) {
                self.scheme_switch_notice("请先确认当前输入，或按 Esc 取消后再切换");
            }
            return self.scheme_switch_reply(session, None);
        }
        self.ensure_focus(session);
        let wubi = self.config.scheme != Scheme::Off || !self.engine.is_code_mode();
        if wubi {
            let user = qingjian_platform::dirs::user_dir();
            let loaded = code_schemes::find(
                "wubi86-sentence",
                user.as_deref(),
                self.bundled_schemes.as_deref(),
            )
            .map(|entry| entry.load());
            if !matches!(loaded, Some(Ok(_))) {
                self.scheme_switch_notice("五笔整句码表不可用，已保持原方案");
                return self.scheme_switch_reply(session, None);
            }
        }
        let Some(path) = self.config_path().map(std::path::Path::to_path_buf) else {
            self.scheme_switch_notice("当前配置不可用，未切换输入方案");
            return self.scheme_switch_reply(session, None);
        };
        let mut switched = false;
        match Config::set_sentence_input_scheme(&path, wubi) {
            Ok(config) => {
                self.apply_config(&config);
                switched = true;
                self.accept_config_stamp();
                self.activate_mode(session);
                self.english = false;
                self.engine.set_english_mode(false);
                self.remember_mode();
                self.reconcile_status();
                self.scheme_switch_notice(if wubi {
                    "已切换：五笔整句"
                } else {
                    "已切换：拼音整句"
                });
            }
            Err(error) => {
                tracing::warn!(%error,"Could not persist input scheme switch");
                self.scheme_switch_notice("输入方案切换失败，已保持原方案");
            }
        }
        self.scheme_switch_reply(session, switched.then_some(false))
    }

    fn scheme_switch_reply(&self, session: SessionId, english: Option<bool>) -> ServerMessage {
        ServerMessage::ModeSync {
            session,
            english,
            input: self.input_settings(session),
            indicator: self.indicator_state(),
        }
    }

    fn scheme_switch_notice(&mut self, message: &str) {
        if self.engine.composition().is_empty() {
            self.flash_notice = Some((message.to_owned(), Instant::now() + Duration::from_secs(2)));
        } else {
            self.notice = Some(message.to_owned());
        }
        let frame = self.self_drawn_frame();
        self.reconcile_candidates(&frame);
    }
}
