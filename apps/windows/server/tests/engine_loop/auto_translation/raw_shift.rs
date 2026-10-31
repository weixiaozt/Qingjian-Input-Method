//! Shift单击原样提交的协议回归：拼音、五笔、空码、过期翻译与焦点隔离。
use super::{poll, response, setup, space, type_text, wait_request, wubi};
use crate::support::{
    ClientMessage, KeyEvent, KeyOutcome, SESSION, ServerMessage, SessionId, function_key, press,
};
use std::time::Duration;

fn shift() -> KeyEvent {
    let mut event = function_key(0x10);
    event.released = true;
    event
}

#[test]
fn pinyin_and_wubi_commit_letters_once_and_keep_chinese() {
    for wubi_mode in [false, true] {
        for word in ["hello", "github", "wqjywyg", "zzzzzz"] {
            let (mut router, _) = if wubi_mode { wubi::setup() } else { setup() };
            type_text(&mut router, word);
            let (outcome, commit, frame) = press(&mut router, shift());
            assert_eq!(outcome, KeyOutcome::Consumed);
            assert_eq!(commit.as_deref(), Some(word));
            assert!(frame.preedit.is_empty());
            assert!(router.engine_mut().composition().is_empty());
            assert!(!router.engine_mut().english_mode());
            assert!(press(&mut router, shift()).1.is_none());
            let next = type_text(&mut router, if wubi_mode { "wqjywyg" } else { "nihao" });
            assert!(next.candidates.items.iter().any(|c| !c.text.is_ascii()));
        }
    }
}

#[test]
fn shift_cancels_waiting_bilingual_commit_and_ignores_late_translation() {
    for wubi_mode in [false, true] {
        let (mut router, state) = if wubi_mode { wubi::setup() } else { setup() };
        let text = if wubi_mode { "wqjywyg" } else { "nihao" };
        type_text(&mut router, text);
        let sequence = wait_request(&mut router, &state);
        space(&mut router, false);
        std::thread::sleep(Duration::from_millis(220));
        assert!(space(&mut router, true).1.is_none());
        assert_eq!(press(&mut router, shift()).1.as_deref(), Some(text));
        response(&state, sequence, Some("Late translation."));
        assert!(poll(&mut router).1.is_none());
        assert!(space(&mut router, true).1.is_none());
    }
}

#[test]
fn shifted_chord_keydown_or_english_release_does_not_commit() {
    let (mut router, _) = wubi::setup();
    type_text(&mut router, "hello");
    let mut chord = shift();
    chord.modifiers.ctrl = true;
    assert!(press(&mut router, chord).1.is_none());
    let mut english = shift();
    english.modifiers.english_mode = true;
    assert!(press(&mut router, english).1.is_none());
    assert!(press(&mut router, function_key(0x10)).1.is_none());
    assert_eq!(router.engine_mut().composition().text(), "hello");
}

#[test]
fn stale_shift_release_never_commits_another_sessions_text() {
    let (mut router, _) = wubi::setup();
    type_text(&mut router, "hello");
    let reply = router.handle(ClientMessage::Key {
        session: SessionId(999),
        event: shift(),
    });
    assert!(matches!(
        reply,
        Some(ServerMessage::KeyResult { commit: None, .. })
    ));
    assert_eq!(router.engine_mut().composition().text(), "hello");
    assert_eq!(press(&mut router, shift()).1.as_deref(), Some("hello"));
    assert!(matches!(
        router.handle(ClientMessage::Commit { session: SESSION }),
        Some(ServerMessage::Committed { text: None, .. })
    ));
}
