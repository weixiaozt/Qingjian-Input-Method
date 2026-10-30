//! 拼音与五笔整句候选自动英译：停键防抖、过期结果隔离及短按 / 长按空格上屏。
mod commit;
mod state;
pub use commit::AutoCommit;

use super::Engine;
use crate::{Candidate, CandidateKind, Language, Prediction, Sense, Translation};
pub(super) use state::AutoTranslation;
use std::time::{Duration, Instant};

impl Engine {
    /// 平台只提供当前高亮候选；资格、原文拼接与请求生命周期由 Core 决定。
    pub fn sync_auto_translation(
        &mut self,
        candidate: Option<Candidate>,
        delay: Duration,
        now: Instant,
    ) {
        let candidate = candidate.filter(|candidate| {
            !self.private
                && (!self.composing_code() || self.code_sentence_enabled())
                && !self.english_mode()
                && !self.expression_mode()
                && !self.question_mode()
                && !self.is_zhuyin_mode()
                && !self.composition.is_empty()
                && (matches!(
                    candidate.kind,
                    CandidateKind::Chinese
                        | CandidateKind::Sentence
                        | CandidateKind::Generated
                        | CandidateKind::Cloud
                ) || (self.code_sentence_enabled() && candidate.kind == CandidateKind::Code))
        });
        let Some(candidate) = candidate else {
            self.clear_auto_translation();
            return;
        };
        let source = self.held_text() + &candidate.text;
        let scope = self.composition.text().to_owned();
        if !self.auto_translation.as_ref().is_some_and(|state| {
            state.source == source
                && state.scope == scope
                && state.candidate.syllables == candidate.syllables
        }) {
            self.clear_auto_translation();
            self.auto_translation = Some(AutoTranslation {
                candidate,
                source,
                scope,
                due: now + delay,
                sequence: None,
                result: None,
                failed: false,
                pressed: None,
                waiting: false,
            });
        }
        let source = self
            .auto_translation
            .as_ref()
            .filter(|state| state.sequence.is_none() && !state.failed && now >= state.due)
            .map(|state| state.source.clone());
        if let Some(source) = source {
            let sequence = self.request_english_translation(&source);
            if let Some(state) = self.auto_translation.as_mut() {
                state.sequence = sequence;
                state.failed = sequence.is_none();
            }
        }
    }

    pub fn clear_auto_translation(&mut self) {
        if let Some(state) = self.auto_translation.take()
            && let Some(sequence) = state.sequence
        {
            self.cancel_candidate_translation(sequence);
        }
    }

    pub fn accept_auto_translation(&mut self, prediction: &Prediction) -> bool {
        let Some(state) = self
            .auto_translation
            .as_mut()
            .filter(|state| state.sequence == Some(prediction.sequence))
        else {
            return false;
        };
        state.result = prediction
            .sentence
            .as_ref()
            .filter(|text| !text.trim().is_empty())
            .cloned();
        state.failed = state.result.is_none();
        true
    }

    pub fn auto_translation_annotation(&self, candidate: &Candidate) -> Option<Translation> {
        let state = self
            .auto_translation
            .as_ref()
            .filter(|state| state.candidate.text == candidate.text)?;
        Some(Translation::new(
            Language::English,
            vec![Sense {
                text: state.result.clone()?,
                part_of_speech: None,
                reading: None,
                fresh: false,
            }],
        ))
    }

    pub fn auto_translation_notice(&self) -> Option<&'static str> {
        let state = self.auto_translation.as_ref()?;
        if state.failed {
            Some("翻译暂不可用，短按空格上屏中文")
        } else if state.waiting {
            Some("翻译中，完成后双语上屏；Esc 取消等待")
        } else if state.result.is_none() {
            Some("翻译中… 短按空格中文，长按空格双语")
        } else {
            Some("短按空格中文，长按空格双语")
        }
    }

    pub fn press_translation_space(&mut self, now: Instant) -> bool {
        let Some(state) = self.auto_translation.as_mut() else {
            return false;
        };
        if state.failed {
            state.waiting = false;
        }
        state.pressed.get_or_insert(now);
        true
    }

    /// 其他编辑键或 Esc 取消等待，不抹掉原来的拼音组句。
    pub fn cancel_translation_space(&mut self) -> bool {
        let Some(state) = self.auto_translation.as_mut() else {
            return false;
        };
        let active = state.pressed.take().is_some() || state.waiting;
        state.waiting = false;
        active
    }

    /// 快速打字时下一字母可能先于空格松开到达；仍按一次短按确认，避免漏字。
    pub fn interrupt_translation_space(&mut self, now: Instant, hold: Duration) -> Option<String> {
        let short = self.auto_translation.as_ref().is_some_and(|state| {
            !state.waiting
                && state
                    .pressed
                    .is_some_and(|pressed| now.duration_since(pressed) < hold)
        });
        if short {
            self.release_translation_space(now, hold)
                .map(|commit| commit.text)
        } else {
            self.cancel_translation_space();
            None
        }
    }

    pub fn release_translation_space(
        &mut self,
        now: Instant,
        hold: Duration,
    ) -> Option<AutoCommit> {
        let state = self.auto_translation.as_mut()?;
        let pressed = state.pressed.take()?;
        if !state.waiting && now.duration_since(pressed) < hold {
            let candidate = state.candidate.clone();
            self.clear_auto_translation();
            return Some(AutoCommit::plain(self.commit(&candidate)));
        }
        state.waiting = true;
        self.advance_translation_space(now, hold)
    }

    /// 长按到点就确认；异步译文未到时等待，失败时保留组句供中文确认。
    pub fn advance_translation_space(
        &mut self,
        now: Instant,
        hold: Duration,
    ) -> Option<AutoCommit> {
        if self.private {
            self.clear_auto_translation();
            return None;
        }
        let state = self.auto_translation.as_mut()?;
        if state
            .pressed
            .is_some_and(|pressed| now.duration_since(pressed) >= hold)
        {
            state.waiting = true;
        }
        if !state.waiting {
            return None;
        }
        let english = state.result.clone()?;
        let candidate = state.candidate.clone();
        self.clear_auto_translation();
        let mut chinese = self.commit(&candidate);
        if chinese.is_empty() {
            chinese = self.take_held();
        }
        let commit = AutoCommit::bilingual(&chinese, &english);
        if let Some(last) = self.recent_commits.last_mut() {
            last.chars += 1 + english.chars().count();
            last.tail = english.chars().last();
        }
        Some(commit)
    }

    /// 文档光标位于中文后时，退格和自动空格的上文也恢复为中文。
    pub fn finish_auto_commit_at_chinese(&mut self, commit: &AutoCommit) {
        if commit.caret_utf16.is_some()
            && let Some(last) = self.recent_commits.last_mut()
        {
            last.chars = last.chars.saturating_sub(commit.english_chars);
            last.tail = commit.chinese_tail;
        }
    }
}
