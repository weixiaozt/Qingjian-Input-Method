//! 连续五笔的Engine闭环：满码不顶出、完整确认、普通输入与拼音隔离。
use super::super::engine;
use crate::{CodeScheme, Engine};
use qingjian_dictionary::CodeTable;
const TABLE: &str = "你\twq\t100\n是\tj\t100\n谁\tywyg\t100\n今天\twygd\t100\n下午\tghtf\t100\n三点\tdghk\t100\n开会\tgawf\t100\n";
fn input() -> Engine {
    let mut e = engine();
    let mut scheme = CodeScheme::new("wubi", CodeTable::parse(TABLE).unwrap());
    scheme.auto_commit = true;
    scheme.top = true;
    e.set_code_scheme(Some(scheme));
    e.set_phonetic(false);
    e.set_code_sentence(true);
    e
}
#[test]
fn continuous_short_codes_decode_and_commit_whole_sentence() {
    let mut e = input();
    for c in "wqjywyg".chars() {
        assert!(!e.code_tops(c));
        e.push(c);
        assert!(e.commit_unique_code().is_none());
    }
    let candidate = e.query().unwrap().candidates.items.remove(0);
    assert_eq!(candidate.text, "你是谁");
    assert_eq!(e.commit(&candidate), "你是谁");
    assert!(e.composition().is_empty());
}
#[test]
fn four_phrase_codes_do_not_commit_at_four_letters() {
    let mut e = input();
    for c in "wygdghtfdghkgawf".chars() {
        assert!(!e.code_tops(c));
        e.push(c);
        assert!(e.commit_unique_code().is_none());
    }
    assert_eq!(
        e.query().unwrap().candidates.items[0].text,
        "今天下午三点开会"
    );
}
#[test]
fn switch_off_restores_original_wubi_rules() {
    let mut e = input();
    e.set_code_sentence(false);
    for c in "wygd".chars() {
        e.push(c);
    }
    assert!(e.code_tops('g'));
    assert_eq!(e.commit_unique_code().as_deref(), Some("今天"));
}
#[test]
fn pinyin_and_english_do_not_enter_continuous_wubi() {
    let mut e = input();
    e.set_phonetic(true);
    assert!(!e.code_sentence_enabled());
    e.set_input("kaifa");
    assert!(
        e.query()
            .unwrap()
            .candidates
            .items
            .iter()
            .any(|c| c.text == "开发")
    );
    e.clear();
    e.set_phonetic(false);
    e.set_english_mode(true);
    assert!(!e.code_sentence_enabled());
}
#[test]
fn unfinished_code_does_not_offer_or_commit_partial_sentence() {
    let mut e = input();
    e.set_input("wqjywy");
    assert!(e.query().unwrap().candidates.items.is_empty());
    assert!(e.take_raw().is_empty());
    assert!(e.composition().is_empty());
}
#[test]
fn changing_table_rebuilds_the_sentence_index() {
    let mut e = input();
    e.set_code_scheme(Some(CodeScheme::new(
        "test",
        CodeTable::parse("测试\tabcd\t100\n").unwrap(),
    )));
    e.set_input("abcdabcd");
    assert_eq!(e.query().unwrap().candidates.items[0].text, "测试测试");
}
#[test]
fn input_length_is_bounded_and_backspace_still_works() {
    let mut e = input();
    for _ in 0..100 {
        e.push('w');
    }
    assert_eq!(e.composition().text().len(), 96);
    e.backspace();
    assert_eq!(e.composition().text().len(), 95);
}
