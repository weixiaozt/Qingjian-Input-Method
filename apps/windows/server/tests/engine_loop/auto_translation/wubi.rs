//! Windows整句五笔与自动翻译、空格键及焦点生命周期的协议闭环。
use super::{Provider, State, poll, response, space, wait_request};
use crate::support::{
    ClientMessage, Router, RouterConfig, SESSION, ServerMessage, function_key, letter, press,
    router_with,
};
use qingjian_core::CodeScheme;
use qingjian_dictionary::CodeTable;
use qingjian_platform::{QuickTranslationConfig, Scheme};
use std::sync::{Arc, Mutex};
use std::time::Duration;
fn setup() -> (Router, Arc<Mutex<State>>) {
    let mut r = router_with(RouterConfig {
        scheme: Scheme::Off,
        quick_translation: QuickTranslationConfig {
            candidate_enabled: true,
            wubi_sentence: true,
            hold_ms: 200,
            candidate_debounce_ms: 100,
            ..Default::default()
        },
        ..Default::default()
    });
    let table=CodeTable::parse("你\twq\t100\n是\tj\t100\n谁\tywyg\t100\n今天\twygd\t100\n下午\tghtf\t100\n三点\tdghk\t100\n开会\tgawf\t100\n").unwrap();
    let mut scheme = CodeScheme::new("wubi", table);
    scheme.auto_commit = true;
    scheme.top = true;
    r.engine_mut().set_code_scheme(Some(scheme));
    r.engine_mut().set_phonetic(false);
    let state = Arc::new(Mutex::new(State::default()));
    r.engine_mut()
        .set_translation_predictor(Some(Box::new(Provider(state.clone()))));
    (r, state)
}
fn type_codes(r: &mut Router, codes: &str) {
    for c in codes.chars() {
        assert!(
            press(r, letter(c)).1.is_none(),
            "must not commit while typing"
        );
    }
}
#[test]
fn short_space_confirms_whole_chinese_without_codes() {
    let (mut r, _) = setup();
    type_codes(&mut r, "wqjywyg");
    assert!(space(&mut r, false).1.is_none());
    assert_eq!(space(&mut r, true).1.as_deref(), Some("你是谁"));
}
#[test]
fn long_space_confirms_bilingual_and_consumes_all_codes_once() {
    let (mut r, state) = setup();
    type_codes(&mut r, "wygdghtfdghkgawf");
    let seq = wait_request(&mut r, &state);
    assert_eq!(
        state.lock().unwrap().requests.last().unwrap().text,
        "今天下午三点开会"
    );
    response(&state, seq, Some("Meet at three today."));
    poll(&mut r);
    space(&mut r, false);
    std::thread::sleep(Duration::from_millis(220));
    assert_eq!(
        poll(&mut r).1.as_deref(),
        Some("今天下午三点开会\nMeet at three today.")
    );
    assert!(space(&mut r, false).1.is_none());
    assert!(space(&mut r, true).1.is_none());
    assert!(r.engine_mut().composition().is_empty());
}
#[test]
fn escape_cancels_translation_wait_and_retains_codes() {
    let (mut r, state) = setup();
    type_codes(&mut r, "wqjywyg");
    let seq = wait_request(&mut r, &state);
    space(&mut r, false);
    std::thread::sleep(Duration::from_millis(220));
    space(&mut r, true);
    press(&mut r, function_key(0x1b));
    response(&state, seq, Some("Who are you?"));
    assert!(poll(&mut r).1.is_none());
    assert_eq!(r.engine_mut().composition().text(), "wqjywyg");
}
#[test]
fn focus_loss_discards_unconfirmed_codes() {
    let (mut r, _) = setup();
    type_codes(&mut r, "wqjywyg");
    assert!(matches!(
        r.handle(ClientMessage::Commit { session: SESSION }),
        Some(ServerMessage::Committed { text: None, .. })
    ));
    assert!(r.engine_mut().composition().is_empty());
}
#[test]
fn enter_confirms_chinese_and_never_returns_raw_wubi() {
    let (mut r, _) = setup();
    type_codes(&mut r, "wqjywyg");
    assert_eq!(
        press(&mut r, function_key(0x0d)).1.as_deref(),
        Some("你是谁")
    );
}
#[test]
fn incomplete_tail_stays_for_editing_on_space() {
    let (mut r, _) = setup();
    type_codes(&mut r, "wqjywy");
    assert!(space(&mut r, false).1.is_none());
    assert_eq!(r.engine_mut().composition().text(), "wqjywy");
}
#[test]
fn native_client_receives_space_release_support_for_wubi() {
    let (mut r, _) = setup();
    let message = r.handle(ClientMessage::SyncMode { session: SESSION });
    assert!(
        matches!(message,Some(ServerMessage::ModeSync{input,..}) if input.candidate_translation)
    );
}

#[test]
fn word_code_hint_never_enters_translation_or_bilingual_commit() {
    let (mut r, state) = setup();
    r.engine_mut().set_aux_show(true);
    r.engine_mut().set_aux_codes(vec![Arc::new(
        qingjian_dictionary::AuxCodeTable::from_pairs([("你".into(), "wqiy".into())]).unwrap(),
    )]);
    type_codes(&mut r, "wq");
    let seq = wait_request(&mut r, &state);
    let frame = poll(&mut r).0;
    assert_eq!(frame.candidates.items[0].aux_code.as_deref(), Some("wqiy"));
    assert_eq!(state.lock().unwrap().requests.last().unwrap().text, "你");
    response(&state, seq, Some("You."));
    poll(&mut r);
    space(&mut r, false);
    std::thread::sleep(Duration::from_millis(220));
    assert_eq!(poll(&mut r).1.as_deref(), Some("你\nYou."));
}
