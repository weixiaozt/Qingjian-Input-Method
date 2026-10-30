//! 单字和码表词组显示编码，自动拼出的多段整句始终不显示。
use super::super::engine;
use crate::{Candidate, CodeScheme, Engine};
use qingjian_dictionary::{AuxCodeTable, CodeTable};
use std::sync::Arc;

fn input(continuous: bool) -> Engine {
    let mut e = engine();
    e.set_code_scheme(Some(CodeScheme::new(
        "wubi",
        CodeTable::parse("你\twq\t100\n是\tj\t100\n谁\tywyg\t100\n今天\twygd\t100\n下午\tghtf\t100\n三点\tdghk\t100\n开会\tgawf\t100\n开发\tgant\t100\n").unwrap(),
    )));
    e.set_phonetic(false);
    e.set_code_sentence(continuous);
    e.set_aux_show(true);
    e.set_aux_codes(vec![Arc::new(
        AuxCodeTable::from_pairs([
            ("你".into(), "wqiy".into()),
            ("今天".into(), "wygd".into()),
            // 即使反查表可为整句提供编码，多段解码的整句也不应显示。
            ("你是谁".into(), "wjyw".into()),
        ])
        .unwrap(),
    )]);
    e
}

fn first(e: &mut Engine, keys: &str) -> Candidate {
    e.set_input(keys);
    e.query().unwrap().candidates.items.remove(0)
}

#[test]
fn single_character_prefers_reverse_lookup_over_entered_short_code() {
    for continuous in [false, true] {
        let mut e = input(continuous);
        let candidate = first(&mut e, "wq");
        assert_eq!(candidate.text, "你");
        assert_eq!(candidate.aux_code.as_deref(), Some("wqiy"));
        assert_eq!(e.commit(&candidate), "你");
    }
}

#[test]
fn dictionary_phrase_keeps_its_code_in_both_wubi_flows() {
    for continuous in [false, true] {
        assert_eq!(
            first(&mut input(continuous), "wygd").aux_code.as_deref(),
            Some("wygd")
        );
    }
}

#[test]
fn known_phrase_typed_as_individual_characters_still_has_a_word_hint() {
    let mut e = input(true);
    e.set_code_scheme(Some(CodeScheme::new(
        "wubi",
        CodeTable::parse("今\twynb\t100\n天\tgdi\t100\n今天\twygd\t100\n").unwrap(),
    )));
    let candidate = first(&mut e, "wynbgdi");
    assert_eq!(candidate.text, "今天");
    assert_eq!(candidate.aux_code.as_deref(), Some("wygd"));
}

#[test]
fn composed_sentences_have_no_code_or_separator_annotation() {
    let mut e = input(true);
    for (keys, text) in [
        ("wqjywyg", "你是谁"),
        ("wygdghtfdghkgawf", "今天下午三点开会"),
    ] {
        let candidate = first(&mut e, keys);
        assert_eq!(candidate.text, text);
        assert!(candidate.aux_code.is_none());
        assert_eq!(e.commit(&candidate), text);
    }
}

#[test]
fn missing_reverse_table_falls_back_to_matched_dictionary_code() {
    for continuous in [false, true] {
        let mut e = input(continuous);
        e.set_aux_codes(Vec::new());
        assert_eq!(first(&mut e, "wygd").aux_code.as_deref(), Some("wygd"));
    }
    assert_eq!(
        first(&mut input(false), "gan").aux_code.as_deref(),
        Some("gant")
    );
}

#[test]
fn display_switch_hides_hints_without_changing_candidates() {
    for continuous in [false, true] {
        let mut e = input(continuous);
        e.set_input("wq");
        let before = e.query().unwrap().candidates.items;
        e.set_aux_show(false);
        let after = e.query().unwrap().candidates.items;
        assert!(after.iter().all(|c| c.aux_code.is_none()));
        assert_eq!(
            before.iter().map(|c| &c.text).collect::<Vec<_>>(),
            after.iter().map(|c| &c.text).collect::<Vec<_>>()
        );
    }
}

#[test]
fn code_candidates_in_original_mixed_mode_also_show_word_hints() {
    let mut e = input(false);
    e.set_phonetic(true);
    e.set_input("wq");
    let candidates = e.query().unwrap().candidates.items;
    assert_eq!(
        candidates
            .iter()
            .find(|c| c.text == "你")
            .unwrap()
            .aux_code
            .as_deref(),
        Some("wqiy")
    );
}
