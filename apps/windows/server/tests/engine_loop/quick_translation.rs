//! 快捷翻译的协议闭环：不依赖网络，覆盖取消、三种模式、旧 DLL 和等待态。
use crate::support::{
    ClientMessage, KeyEvent, KeyModifiers, KeyOutcome, Router, RouterConfig, SESSION, ScreenRect,
    ServerMessage, router_with,
};
use qingjian_core::quick_translation::TranslationMode;
use qingjian_core::{Prediction, PredictionPolicy, PredictionRequest, Predictor};
use qingjian_platform::{KeyCombo, QuickTranslationConfig};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct State {
    requests: Vec<PredictionRequest>,

    replies: Vec<Prediction>,
}
struct Provider(Arc<Mutex<State>>);
impl Predictor for Provider {
    fn policy(&self) -> PredictionPolicy {
        PredictionPolicy::default()
    }
    fn submit(&mut self, request: PredictionRequest) {
        self.0.lock().unwrap().requests.push(request);
    }
    fn poll(&mut self) -> Option<Prediction> {
        self.0.lock().unwrap().replies.pop()
    }
}
fn setup(mode: TranslationMode) -> (Router, Arc<Mutex<State>>) {
    let mut router = router_with(RouterConfig {
        // 下面喂的是Ctrl+Alt组合，不能依赖宿主系统的缺省翻译键（Linux为Alt+Shift）。
        translate_selection: KeyCombo::TRANSLATE_DEFAULT,
        quick_translation: QuickTranslationConfig {
            enabled: true,
            mode,
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
fn hotkey(router: &mut Router, key: char) -> ServerMessage {
    let combo = KeyCombo {
        key,
        ..KeyCombo::TRANSLATE_DEFAULT
    };
    router
        .handle(ClientMessage::Key {
            session: SESSION,
            event: KeyEvent::new(
                key.to_ascii_uppercase() as u32,
                Some(key),
                combo.modifiers.into(),
            ),
        })
        .unwrap()
}
fn begin(router: &mut Router) -> u64 {
    match hotkey(router, 't') {
        ServerMessage::RequestSentence { request, .. } => request,
        other => panic!("expected read request: {other:?}"),
    }
}
fn source(router: &mut Router, request: u64, text: &str) {
    router.handle(ClientMessage::Sentence {
        session: SESSION,
        request,
        text: text.into(),
        rect: ScreenRect {
            left: 0,
            top: 0,
            right: 1,
            bottom: 20,
        },
        failure: None,
    });
}
fn reply(router: &mut Router, state: &Arc<Mutex<State>>) {
    let mut state = state.lock().unwrap();
    let sequence = state.requests.last().unwrap().sequence;
    state.replies.push(Prediction {
        sequence,
        words: Vec::new(),
        sentence: Some("Meet tomorrow.".into()),
    });
    drop(state);
    router.handle(ClientMessage::Poll { session: SESSION });
}
#[test]
fn three_modes_only_write_on_explicit_confirmation() {
    for mode in [
        TranslationMode::Preview,
        TranslationMode::Bilingual,
        TranslationMode::Replace,
    ] {
        let (mut router, state) = setup(mode);
        let request = begin(&mut router);
        source(&mut router, request, "明天开会。");
        reply(&mut router, &state);
        let result = hotkey(&mut router, 't');
        match mode {
            TranslationMode::Preview => assert!(matches!(
                result,
                ServerMessage::KeyResult { commit: None, .. }
            )),
            _ => assert!(
                matches!(result, ServerMessage::CommitTranslation { request: id, mode: actual, ref text, .. } if id == request && actual == mode && text == "Meet tomorrow.")
            ),
        }
        assert_eq!(state.lock().unwrap().requests.len(), 1);
    }
}
#[test]
fn repeated_trigger_while_waiting_and_cycle_do_not_resubmit() {
    let (mut router, state) = setup(TranslationMode::Bilingual);
    let request = begin(&mut router);
    source(&mut router, request, "明天开会。");
    assert!(
        matches!(hotkey(&mut router, 't'), ServerMessage::KeyResult { commit: None, frame, .. } if !frame.is_empty())
    );
    hotkey(&mut router, 'm');
    reply(&mut router, &state);
    assert!(matches!(
        hotkey(&mut router, 't'),
        ServerMessage::CommitTranslation {
            mode: TranslationMode::Replace,
            ..
        }
    ));
    assert_eq!(state.lock().unwrap().requests.len(), 1);
}
#[test]
fn escape_discards_late_result_and_text_changes_invalidate_pending_read() {
    let (mut router, state) = setup(TranslationMode::Replace);
    let request = begin(&mut router);
    source(&mut router, request, "明天开会。");
    router.handle(ClientMessage::Key {
        session: SESSION,
        event: KeyEvent::new(0x1b, None, KeyModifiers::default()),
    });
    reply(&mut router, &state);
    assert!(
        matches!(router.handle(ClientMessage::Poll { session: SESSION }), Some(ServerMessage::Update { commit: None, frame, .. }) if frame.is_empty())
    );
    let stale = begin(&mut router);
    router.handle(ClientMessage::Key {
        session: SESSION,
        event: KeyEvent::new(0x25, None, KeyModifiers::default()),
    });
    source(&mut router, stale, "不应提交。");
    assert_eq!(state.lock().unwrap().requests.len(), 1);
}
#[test]
fn unreadable_and_oversized_sentences_do_not_send_requests() {
    let (mut router, state) = setup(TranslationMode::Preview);
    for text in [String::new(), "字".repeat(501)] {
        let request = begin(&mut router);
        source(&mut router, request, &text);
    }
    assert!(state.lock().unwrap().requests.is_empty());
}
#[test]
fn ordinary_key_cancels_preview_and_passes_to_application() {
    let (mut router, _) = setup(TranslationMode::Replace);
    let request = begin(&mut router);
    source(&mut router, request, "明天开会。");
    let result = router.handle(ClientMessage::Key {
        session: SESSION,
        event: KeyEvent::new(0x25, None, KeyModifiers::default()),
    });
    assert!(matches!(
        result,
        Some(ServerMessage::KeyResult {
            outcome: KeyOutcome::Passthrough,
            commit: None,
            ..
        })
    ));
}
