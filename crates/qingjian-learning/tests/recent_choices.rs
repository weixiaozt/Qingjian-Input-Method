//! 真实Engine与磁盘学习器的最近选择闭环，不依赖模型下载或用户数据。
use qingjian_core::{Candidate, CodeScheme, Engine};
use qingjian_dictionary::{CodeTable, Dictionary};
use qingjian_learning::FrequencyLearner;
use std::path::{Path, PathBuf};

#[test]
fn pinyin_sentence_selection_survives_restart() {
    let dir = directory("sentence");
    let path = dir.join("frequency.tsv");
    let mut e = engine(false, Some(&path));
    let keys = "gongshigongshi";
    let list = candidates(&mut e, keys);
    let chosen = list
        .iter()
        .find(|c| c.kind == qingjian_core::CandidateKind::Sentence)
        .unwrap()
        .clone();
    e.commit(&chosen);
    e.learner_mut().flush();
    let mut restored = engine(false, Some(&path));
    assert_eq!(candidates(&mut restored, keys)[0].text, chosen.text);
    std::fs::remove_dir_all(dir).unwrap();
}

struct TranslationProvider(std::sync::Arc<std::sync::atomic::AtomicU64>);
impl qingjian_core::Predictor for TranslationProvider {
    fn policy(&self) -> qingjian_core::PredictionPolicy {
        Default::default()
    }
    fn submit(&mut self, request: qingjian_core::PredictionRequest) {
        self.0
            .store(request.sequence, std::sync::atomic::Ordering::Relaxed);
    }
    fn poll(&mut self) -> Option<qingjian_core::Prediction> {
        None
    }
}

#[test]
fn bilingual_confirmation_remembers_chinese_candidate() {
    use std::time::{Duration, Instant};
    let mut e = engine(true, None);
    let sequence = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    e.set_translation_predictor(Some(Box::new(TranslationProvider(sequence.clone()))));
    let chosen = candidates(&mut e, "wqjywyg")[2].clone();
    let now = Instant::now();
    e.sync_auto_translation(Some(chosen.clone()), Duration::ZERO, now);
    assert!(e.accept_auto_translation(&qingjian_core::Prediction {
        sequence: sequence.load(std::sync::atomic::Ordering::Relaxed),
        words: Vec::new(),
        sentence: Some("Who is he?".into())
    }));
    e.press_translation_space(now);
    let commit = e
        .release_translation_space(now + Duration::from_millis(500), Duration::from_millis(350))
        .unwrap();
    assert_eq!(commit.text, format!("{}\nWho is he?", chosen.text));
    assert_eq!(candidates(&mut e, "wqjywyg")[0].text, chosen.text);
}

fn engine(wubi: bool, path: Option<&Path>) -> Engine {
    let mut e = Engine::new(
        Dictionary::parse("工事\tgong shi\t1000\n公事\tgong shi\t100\n公式\tgong shi\t10\n")
            .unwrap(),
    );
    e = e.with_learner(Box::new(
        path.map(|p| FrequencyLearner::from_path(p).unwrap())
            .unwrap_or_default(),
    ));
    if wubi {
        e.set_code_scheme(Some(CodeScheme::new("wubi86", CodeTable::parse("你\twq\t1000\n我\twq\t100\n他\twq\t10\n是\tj\t100\n谁\tywyg\t100\n工事\tgs\t1000\n公事\tgs\t100\n公式\tgs\t10\n").unwrap())));
        e.set_phonetic(false);
        e.set_code_sentence(true);
    }
    e
}

fn candidates(e: &mut Engine, keys: &str) -> Vec<Candidate> {
    e.set_input(keys);
    e.query().unwrap().candidates.items
}

fn directory(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "qingjian-recent-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn third_choice_becomes_first_and_latest_beats_frequent_choice() {
    for (wubi, keys) in [(false, "gongshi"), (true, "gs"), (true, "wqjywyg")] {
        let mut e = engine(wubi, None);
        let original = candidates(&mut e, keys);
        assert!(original.len() >= 3);
        let old = original[0].clone();
        let third = original[2].clone();
        for _ in 0..12 {
            e.set_input(keys);
            e.commit(&old);
        }
        e.set_input(keys);
        e.commit(&third);
        assert_eq!(candidates(&mut e, keys)[0].text, third.text);
        e.commit(&old);
        assert_eq!(candidates(&mut e, keys)[0].text, old.text);
    }
}

#[test]
fn latest_choice_survives_restart_for_words_and_sentences() {
    for (wubi, keys) in [(false, "gongshi"), (true, "wqjywyg")] {
        let dir = directory("restart");
        let path = dir.join("frequency.tsv");
        let mut e = engine(wubi, Some(&path));
        let chosen = candidates(&mut e, keys)[2].clone();
        e.commit(&chosen);
        e.learner_mut().flush();
        drop(e);
        let mut restored = engine(wubi, Some(&path));
        assert_eq!(candidates(&mut restored, keys)[0].text, chosen.text);
        assert!(FrequencyLearner::files(&path).contains(&dir.join("user-recent-choices.jsonl")));
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[test]
fn private_input_disabled_learning_and_raw_commit_do_not_replace_preference() {
    let mut e = engine(false, None);
    let original = candidates(&mut e, "gongshi");
    e.commit(&original[2]);
    e.set_private(true);
    e.set_input("gongshi");
    e.commit(&original[0]);
    e.set_private(false);
    assert_eq!(candidates(&mut e, "gongshi")[0].text, original[2].text);
    e.set_learning(false);
    e.commit(&original[1]);
    e.set_learning(true);
    assert_eq!(candidates(&mut e, "gongshi")[0].text, original[2].text);
    e.take_raw_explicit();
    assert_eq!(candidates(&mut e, "gongshi")[0].text, original[2].text);
}

#[test]
fn same_letters_in_pinyin_and_wubi_have_separate_preferences() {
    let mut e = engine(true, None);
    let choices = candidates(&mut e, "gs");
    e.commit(&choices[2]);
    e.set_code_scheme(None);
    e.set_code_sentence(false);
    e.set_phonetic(true);
    let plain = candidates(&mut e, "gs");
    let pinyin_choice = plain
        .iter()
        .find(|c| c.text != choices[2].text)
        .unwrap()
        .clone();
    e.commit(&pinyin_choice);
    let wubi = engine(true, None);
    e.set_code_scheme(wubi.code_scheme().cloned());
    e.set_phonetic(false);
    e.set_code_sentence(true);
    assert_eq!(candidates(&mut e, "gs")[0].text, choices[2].text);
    e.set_code_scheme(None);
    e.set_code_sentence(false);
    e.set_phonetic(true);
    assert_eq!(candidates(&mut e, "gs")[0].text, pinyin_choice.text);
}

#[test]
fn forgotten_candidate_is_removed_from_persistent_recent_choices() {
    let dir = directory("forget");
    let path = dir.join("frequency.tsv");
    let mut e = engine(false, Some(&path));
    let choices = candidates(&mut e, "gongshi");
    e.commit(&choices[2]);
    e.learner_mut().forget(&choices[2].text);
    e.learner_mut().flush();
    let mut restored = engine(false, Some(&path));
    assert_ne!(
        candidates(&mut restored, "gongshi")[0].text,
        choices[2].text
    );
    std::fs::remove_dir_all(dir).unwrap();
}
