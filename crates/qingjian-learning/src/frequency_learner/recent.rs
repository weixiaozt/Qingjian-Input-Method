//! 最近选择单独落盘，保留原词频格式；坏行跳过，原子写入，译文不存入偏好。
use super::FrequencyLearner;
use crate::error::LearningError;
use qingjian_core::storage::{read_text_lossy, write_atomic};
use qingjian_core::{Candidate, CandidateKind};
use std::io::Write;
use std::path::{Path, PathBuf};

impl FrequencyLearner {
    pub(super) fn recent_path(path: &Path) -> PathBuf {
        path.with_file_name("user-recent-choices.jsonl")
    }

    pub(super) fn remember_recent(&mut self, key: &str, candidate: &Candidate) {
        if key.is_empty() || candidate.text.is_empty() {
            return;
        }
        if self.recent.len() >= 50_000 && !self.recent.contains_key(key) {
            self.recent.pop_first();
        }
        let mut saved = candidate.clone();
        saved.translation = None;
        saved.aux_code = None;
        self.recent.insert(key.to_owned(), saved);
        self.recent_dirty = true;
    }

    pub(super) fn load_recent(&mut self, path: &Path) -> Result<(), LearningError> {
        if let Some(source) = read_text_lossy(&Self::recent_path(path))? {
            let mut skipped = 0;
            for line in source.lines() {
                match serde_json::from_str::<(String, Candidate)>(line) {
                    Ok((key, candidate))
                        if !key.is_empty()
                            && !candidate.text.is_empty()
                            && matches!(
                                candidate.kind,
                                CandidateKind::Chinese
                                    | CandidateKind::Sentence
                                    | CandidateKind::Code
                                    | CandidateKind::Cloud
                                    | CandidateKind::Generated
                            ) =>
                    {
                        self.remember_recent(&key, &candidate);
                    }
                    _ => skipped += 1,
                }
            }
            if skipped > 0 {
                tracing::warn!(skipped, "Skipped invalid recent exact choices");
            }
        }
        self.recent_dirty = false;
        Ok(())
    }

    pub(super) fn save_recent(&mut self, path: &Path) -> Result<(), LearningError> {
        write_atomic(&Self::recent_path(path), |file| {
            for row in &self.recent {
                serde_json::to_writer(&mut *file, &row).map_err(std::io::Error::other)?;
                writeln!(file)?;
            }
            Ok(())
        })?;
        self.recent_dirty = false;
        Ok(())
    }
}
