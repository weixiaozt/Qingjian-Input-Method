//! 自动候选翻译的协议闭环：短按、长按、过期结果、取消与隐私边界。
use crate::support::{
    ClientMessage, Frame, KeyEvent, KeyOutcome, Router, RouterConfig, SESSION, ServerMessage,
    SessionId, function_key, letter, press, router_with,
};
use qingjian_core::Prediction;
use qingjian_platform::QuickTranslationConfig;
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod caret;
mod provider;
mod state;
mod wubi;
use provider::{Provider, State};

fn setup() -> (Router, Arc<Mutex<State>>) {
    let mut router = router_with(RouterConfig {
        quick_translation: QuickTranslationConfig {
            candidate_enabled: true,
            hold_ms: 200,
            candidate_debounce_ms: 100,
            ..Default::default()
        },
        ..Default::default()
    });
    let state = Arc::new(Mutex::new(State::default()));
    router
        .engine_mut()
        .set_translation_predictor(Some(Box::new(Provider(state.clone()))));
    (router, state)
}
fn type_text(router: &mut Router, text: &str) -> Frame {
    let mut frame = Frame::default();
    for c in text.chars() {
        frame = press(router, letter(c)).2;
    }
    frame
}
fn poll(router: &mut Router) -> (Frame, Option<String>) {
    match router
        .handle(ClientMessage::Poll { session: SESSION })
        .unwrap()
    {
        ServerMessage::Update { frame, commit, .. } => (frame, commit),
        ServerMessage::CommitAt { text, frame, .. } => (frame, Some(text)),
        other => panic!("unexpected {other:?}"),
    }
}
fn wait_request(router: &mut Router, state: &Arc<Mutex<State>>) -> u64 {
    std::thread::sleep(Duration::from_millis(120));
    poll(router);
    state
        .lock()
        .unwrap()
        .requests
        .last()
        .expect("automatic request")
        .sequence
}
fn response(state: &Arc<Mutex<State>>, sequence: u64, text: Option<&str>) {
    state.lock().unwrap().replies.push(Prediction {
        sequence,
        words: Vec::new(),
        sentence: text.map(str::to_owned),
    });
}
fn space(router: &mut Router, released: bool) -> (KeyOutcome, Option<String>, Frame) {
    let mut event = KeyEvent::new(0x20, Some(' '), Default::default());
    event.released = released;
    press(router, event)
}

#[test]
fn short_press_commits_chinese_without_waiting_for_model() {
    let (mut r, state) = setup();
    let f = type_text(&mut r, "nihao");
    let chinese = f.candidates.items[0].text.clone();
    assert!(space(&mut r, false).1.is_none());
    assert_eq!(space(&mut r, true).1.as_deref(), Some(chinese.as_str()));
    assert!(state.lock().unwrap().requests.is_empty());
}

#[test]
fn automatic_result_is_visible_and_long_hold_commits_once() {
    let (mut r, state) = setup();
    let f = type_text(&mut r, "nihao");
    let chinese = f.candidates.items[0].text.clone();
    let seq = wait_request(&mut r, &state);
    assert_eq!(state.lock().unwrap().requests.last().unwrap().text, chinese);
    response(&state, seq, Some("Hello."));
    assert_eq!(
        poll(&mut r).0.candidates.items[0]
            .translation
            .as_ref()
            .unwrap()
            .senses()[0]
            .text,
        "Hello."
    );
    space(&mut r, false);
    std::thread::sleep(Duration::from_millis(220));
    assert_eq!(poll(&mut r).1, Some(format!("{chinese}\nHello.")));
    assert!(space(&mut r, false).1.is_none());
    assert!(space(&mut r, true).1.is_none());
    assert!(poll(&mut r).1.is_none());
}

#[test]
fn long_release_waits_for_result_then_commits() {
    let (mut r, state) = setup();
    type_text(&mut r, "nihao");
    let seq = wait_request(&mut r, &state);
    space(&mut r, false);
    std::thread::sleep(Duration::from_millis(220));
    assert!(space(&mut r, true).1.is_none());
    response(&state, seq, Some("Hello."));
    assert!(poll(&mut r).1.unwrap().ends_with("\nHello."));
}

#[test]
fn escape_cancels_pending_commit_but_keeps_pinyin() {
    let (mut r, state) = setup();
    type_text(&mut r, "nihao");
    let seq = wait_request(&mut r, &state);
    space(&mut r, false);
    std::thread::sleep(Duration::from_millis(220));
    space(&mut r, true);
    assert!(!press(&mut r, function_key(0x1b)).2.is_empty());
    response(&state, seq, Some("Hello."));
    assert!(poll(&mut r).1.is_none());
    space(&mut r, false);
    assert_eq!(space(&mut r, true).1.as_deref(), Some("你好"));
}

#[test]
fn changed_input_rejects_previous_response() {
    let (mut r, state) = setup();
    type_text(&mut r, "ni");
    let old = wait_request(&mut r, &state);
    type_text(&mut r, "hao");
    response(&state, old, Some("STALE"));
    let (f, commit) = poll(&mut r);
    assert!(commit.is_none());
    assert!(!f.candidates.items.iter().any(|c| {
        c.translation
            .as_ref()
            .is_some_and(|t| t.senses().iter().any(|s| s.text == "STALE"))
    }));
    assert_ne!(wait_request(&mut r, &state), old);
}

#[test]
fn failed_translation_preserves_chinese_and_allows_short_press() {
    let (mut r, state) = setup();
    type_text(&mut r, "nihao");
    let seq = wait_request(&mut r, &state);
    space(&mut r, false);
    std::thread::sleep(Duration::from_millis(220));
    space(&mut r, true);
    response(&state, seq, None);
    let (f, c) = poll(&mut r);
    assert!(c.is_none());
    assert!(f.notice.unwrap().contains("不可用"));
    space(&mut r, false);
    assert_eq!(space(&mut r, true).1.as_deref(), Some("你好"));
}

#[test]
fn private_input_never_requests_translation() {
    let (mut r, state) = setup();
    r.handle(ClientMessage::Privacy {
        session: SESSION,
        private: true,
    });
    type_text(&mut r, "nihao");
    std::thread::sleep(Duration::from_millis(120));
    poll(&mut r);
    assert!(state.lock().unwrap().requests.is_empty());
    assert_eq!(space(&mut r, false).1.as_deref(), Some("你好"));
}

#[test]
fn focus_switch_invalidates_pending_translation_and_late_keyup() {
    let (mut r, state) = setup();
    type_text(&mut r, "nihao");
    let seq = wait_request(&mut r, &state);
    space(&mut r, false);
    std::thread::sleep(Duration::from_millis(220));
    space(&mut r, true);
    r.handle(ClientMessage::Commit { session: SESSION });
    let other = SessionId(2);
    crate::support::open_session(&mut r, other, Some("browser.exe".into()));
    r.handle(ClientMessage::Key {
        session: other,
        event: letter('w'),
    });
    response(&state, seq, Some("Hello."));
    space(&mut r, true);
    assert_eq!(r.engine_mut().composition().text(), "w");
    assert!(poll(&mut r).1.is_none());
}

#[test]
fn old_dll_keeps_immediate_space_confirmation() {
    let (mut r, _) = setup();
    r.handle(ClientMessage::OpenSession {
        session: SESSION,
        app: None,
        protocol: 10,
    });
    type_text(&mut r, "nihao");
    assert_eq!(space(&mut r, false).1.as_deref(), Some("你好"));
}

#[test]
fn typing_before_space_release_does_not_lose_chinese() {
    let (mut r, _) = setup();
    type_text(&mut r, "nihao");
    space(&mut r, false);
    assert_eq!(press(&mut r, letter('w')).1.as_deref(), Some("你好"));
    assert!(space(&mut r, true).1.is_none());
    assert_eq!(r.engine_mut().composition().text(), "w");
}

#[test]
fn full_sentence_is_translated_as_one_candidate() {
    let (mut r, state) = setup();
    let f = type_text(&mut r, "mingtiankaihui");
    assert_eq!(f.candidates.items[0].text, "明天开会");
    let seq = wait_request(&mut r, &state);
    assert_eq!(
        state.lock().unwrap().requests.last().unwrap().text,
        "明天开会"
    );
    response(&state, seq, Some("We have a meeting tomorrow."));
    poll(&mut r);
    space(&mut r, false);
    std::thread::sleep(Duration::from_millis(220));
    assert_eq!(
        space(&mut r, true).1.as_deref(),
        Some("明天开会\nWe have a meeting tomorrow.")
    );
}

#[test]
fn changing_highlight_retranslates_the_selected_word() {
    let (mut r, state) = setup();
    let f = type_text(&mut r, "nihao");
    assert!(f.candidates.items.len() > 1);
    let old = wait_request(&mut r, &state);
    let f = press(&mut r, function_key(0x28)).2;
    let selected = f.candidates.items[f.highlight].text.clone();
    response(&state, old, Some("STALE"));
    poll(&mut r);
    let new = wait_request(&mut r, &state);
    assert_ne!(old, new);
    assert_eq!(
        state.lock().unwrap().requests.last().unwrap().text,
        selected
    );
}

#[test]
fn held_chinese_is_included_in_translation_after_partial_choice() {
    let (mut r, state) = setup();
    let f = type_text(&mut r, "nihaozhongguo");
    let index = f
        .candidates
        .items
        .iter()
        .position(|c| c.text == "你好")
        .unwrap();
    let (_, commit, f) = press(&mut r, crate::support::digit((index + 1) as u32));
    assert!(commit.is_none());
    assert_eq!(f.candidates.items[0].text, "中国");
    let seq = wait_request(&mut r, &state);
    assert_eq!(
        state.lock().unwrap().requests.last().unwrap().text,
        "你好中国"
    );
    response(&state, seq, Some("Hello, China."));
    poll(&mut r);
    space(&mut r, false);
    std::thread::sleep(Duration::from_millis(220));
    assert_eq!(
        space(&mut r, true).1.as_deref(),
        Some("你好中国\nHello, China.")
    );
}

#[test]
fn space_without_candidates_remains_native() {
    let (mut r, _) = setup();
    assert!(space(&mut r, false).1.is_none());
    assert_eq!(space(&mut r, false).0, KeyOutcome::Passthrough);
}
