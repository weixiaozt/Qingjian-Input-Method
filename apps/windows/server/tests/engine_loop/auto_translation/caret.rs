//! 双语提交的中文光标位置：两条确认路径、旧DLL兼容与后续输入。
use super::{poll, response, setup, space, type_text, wait_request};
use crate::support::{ClientMessage, KeyEvent, SESSION, ServerMessage};
use std::time::Duration;

#[test]
fn timer_commit_places_caret_before_english_and_preserves_chinese_context() {
    let (mut r, state) = setup();
    r.engine_mut().set_auto_space(true);
    type_text(&mut r, "nihao");
    let seq = wait_request(&mut r, &state);
    response(&state, seq, Some("Hello there"));
    poll(&mut r);
    space(&mut r, false);
    std::thread::sleep(Duration::from_millis(220));
    let message = r.handle(ClientMessage::Poll { session: SESSION }).unwrap();
    assert!(
        matches!(message,ServerMessage::CommitAt{ref text,caret_utf16:2,..} if text=="你好\nHello there")
    );
    assert!(!r.engine_mut().auto_space_before("明天"));
    assert!(space(&mut r, true).1.is_none());
    assert!(poll(&mut r).1.is_none());
}

#[test]
fn released_long_space_uses_the_same_chinese_caret() {
    let (mut r, state) = setup();
    type_text(&mut r, "nihao");
    let seq = wait_request(&mut r, &state);
    response(&state, seq, Some("Hello\nthere"));
    poll(&mut r);
    space(&mut r, false);
    std::thread::sleep(Duration::from_millis(220));
    let mut up = KeyEvent::new(0x20, Some(' '), Default::default());
    up.released = true;
    let message = r
        .handle(ClientMessage::Key {
            session: SESSION,
            event: up,
        })
        .unwrap();
    assert!(
        matches!(message,ServerMessage::CommitAt{ref text,caret_utf16:2,..} if text=="你好\nHello\nthere")
    );
}

#[test]
fn old_dll_receives_legacy_commit_without_unknown_variant() {
    let (mut r, state) = setup();
    r.handle(ClientMessage::OpenSession {
        session: SESSION,
        app: None,
        protocol: 11,
    });
    r.engine_mut().set_auto_space(true);
    type_text(&mut r, "nihao");
    let seq = wait_request(&mut r, &state);
    response(&state, seq, Some("Hello"));
    poll(&mut r);
    space(&mut r, false);
    std::thread::sleep(Duration::from_millis(220));
    assert!(matches!(
        r.handle(ClientMessage::Poll { session: SESSION }),
        Some(ServerMessage::Update {
            commit: Some(_),
            ..
        })
    ));
    assert!(r.engine_mut().auto_space_before("明天"));
}

#[test]
fn short_press_has_no_caret_override() {
    let (mut r, _) = setup();
    type_text(&mut r, "nihao");
    space(&mut r, false);
    let mut up = KeyEvent::new(0x20, Some(' '), Default::default());
    up.released = true;
    assert!(
        matches!(r.handle(ClientMessage::Key{session:SESSION,event:up}),Some(ServerMessage::KeyResult{commit:Some(ref text),..}) if text=="你好")
    );
}
