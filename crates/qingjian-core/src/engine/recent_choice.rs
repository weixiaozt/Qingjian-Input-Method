//! 同输入方案与完整编码的最近选择：在通用排序后优先展示，不借用词频计数。
use crate::{Candidate, CandidateKind, Engine, Learner};

impl Engine {
    fn exact_choice_key(&self) -> Option<String> {
        let scope = self.composition.scope();
        if self.english_mode
            || self.zhuyin
            || self.nine_key
            || scope.is_empty()
            || self.aux_filter().is_some()
            || self.expression_mode()
            || self.question_mode()
        {
            return None;
        }
        Some(format!(
            "{:?}",
            (
                self.phonetic,
                self.shuangpin.as_ref().map(|s| s.key()),
                self.code.as_ref().map(|s| s.key.as_str()),
                self.code_sentence_enabled(),
                scope
            )
        ))
    }

    pub(super) fn remember_exact_choice(&mut self, candidate: &Candidate) {
        if matches!(
            candidate.kind,
            CandidateKind::Chinese
                | CandidateKind::Sentence
                | CandidateKind::Code
                | CandidateKind::Cloud
                | CandidateKind::Generated
        ) && let Some(key) = self.exact_choice_key()
        {
            self.learner
                .undemote(self.composition.scope(), &candidate.text);
            self.learner.remember_exact(&key, candidate);
        }
    }

    pub(super) fn prioritize_exact_choice(&self, items: &mut Vec<Candidate>) {
        let Some(key) = self.exact_choice_key() else {
            return;
        };
        let Some(mut remembered) = self.learner.recent_exact(&key) else {
            return;
        };
        if self
            .learner
            .demotion(self.composition.scope(), &remembered.text)
            > 0
        {
            return;
        }
        if let Some(index) = items
            .iter()
            .position(|c| c.text == remembered.text && c.kind == remembered.kind)
        {
            remembered = items.remove(index);
        }
        items.retain(|c| c.text != remembered.text);
        items.insert(0, remembered);
    }
}
