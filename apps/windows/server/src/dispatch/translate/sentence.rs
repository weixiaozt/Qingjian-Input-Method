//! 快捷翻译的请求与确认分派；文档范围验证交给 DLL，翻译交给 Core。
use super::Translation;
use crate::dispatch::Router;
use crate::dispatch::key::ESCAPE;
use qingjian_core::quick_translation::TranslationMode;
use qingjian_platform::{
    Config,
    protocol::{Frame, KeyEvent, KeyOutcome, ScreenRect, ServerMessage, SessionId},
};

impl Router {
    pub(in crate::dispatch) fn handle_sentence(
        &mut self,
        session: SessionId,
        request: u64,
        text: String,
        rect: ScreenRect,
        failure: Option<&str>,
    ) -> ServerMessage {
        if self.focused != Some(session)
            || self.pending_selection != Some(request)
            || !self.config.quick_translation.enabled
        {
            return empty(session, KeyOutcome::Consumed);
        }
        self.pending_selection = None;
        if text.trim().is_empty()
            || text.encode_utf16().count() > 500
            || self.engine.request_english_translation(&text).is_none()
        {
            self.last_rect = Some(rect);
            self.flash_notice = Some((
                capture_failure_message(failure).into(),
                std::time::Instant::now() + std::time::Duration::from_secs(3),
            ));
            let frame = self.self_drawn_frame();
            self.reconcile_candidates(&frame);
            return empty(session, KeyOutcome::Consumed);
        }
        self.last_rect = Some(rect);
        self.translation = Some(Translation {
            result: None,
            request: Some(request),
            failed: false,
        });
        let frame = self.self_drawn_frame();
        self.reconcile_candidates(&frame);
        ServerMessage::KeyResult {
            session,
            outcome: KeyOutcome::Consumed,
            commit: None,
            frame,
            english: None,
        }
    }

    pub(super) fn handle_sentence_review(
        &mut self,
        session: SessionId,
        event: &KeyEvent,
    ) -> ServerMessage {
        if self.matches_translate_combo(event) {
            let translation = self.translation.as_ref().expect("review exists");
            if translation.result.is_none() && !translation.failed {
                return ServerMessage::KeyResult {
                    session,
                    outcome: KeyOutcome::Consumed,
                    commit: None,
                    frame: self.translation_frame(translation),
                    english: None,
                };
            }
            let request = translation.request.expect("sentence review");
            let result = translation.result.clone();
            let mode = self.config.quick_translation.mode;
            self.end_translation();
            if mode != TranslationMode::Preview
                && let Some(text) = result
            {
                return ServerMessage::CommitTranslation {
                    session,
                    request,
                    text,
                    mode,
                    newline: self.config.quick_translation.newline,
                };
            }
            return empty(session, KeyOutcome::Consumed);
        }
        self.end_translation();
        empty(
            session,
            if event.virtual_key == ESCAPE {
                KeyOutcome::Consumed
            } else {
                KeyOutcome::Passthrough
            },
        )
    }

    pub(in crate::dispatch) fn cycle_translation_mode(
        &mut self,
        session: SessionId,
    ) -> ServerMessage {
        let mode = self.config.quick_translation.mode.next();
        if let Some(path) = self.config_path()
            && let Err(error) = Config::set_value(path, "quick_translation", "mode", mode.key())
        {
            tracing::warn!(%error, "Could not persist translation mode");
            return empty(session, KeyOutcome::Consumed);
        }
        self.config.quick_translation.mode = mode;
        self.flash_notice = Some((
            format!("已切换：{}模式", mode.label()),
            std::time::Instant::now() + std::time::Duration::from_secs(2),
        ));
        let frame = self.self_drawn_frame();
        self.reconcile_candidates(&frame);
        ServerMessage::KeyResult {
            session,
            outcome: KeyOutcome::Consumed,
            commit: None,
            frame,
            english: None,
        }
    }
}

fn capture_failure_message(failure: Option<&str>) -> &'static str {
    match failure {
        Some("no_surrounding_text" | "lookback_unavailable" | "text_unreadable") => {
            "此输入框未提供已输入文字，暂不能自动取句"
        }
        Some("different_context" | "focus_unavailable") => {
            "无法确认输入焦点，请重新点击输入框后重试"
        }
        Some("selection_not_empty") => "请取消文字选中，将光标放在句末",
        Some("private_input") => "私密输入框不读取内容",
        Some(
            "range_start_unavailable"
            | "range_start_mismatch"
            | "range_end_unavailable"
            | "range_end_mismatch",
        ) => "此输入框无法定位原文范围，已取消翻译",
        Some("selection_unavailable" | "selection_unreadable") => "此输入框未提供可读的光标位置",
        _ => "未能取得可翻译的完整句子，请重试",
    }
}

fn empty(session: SessionId, outcome: KeyOutcome) -> ServerMessage {
    ServerMessage::KeyResult {
        session,
        outcome,
        commit: None,
        frame: Frame::default(),
        english: None,
    }
}
