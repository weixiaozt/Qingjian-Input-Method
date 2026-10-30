//! 模式切换必须保留原输入、翻译设置和注释；使用临时配置。
use crate::dispatch::{DataDirs, Router, RouterConfig};
use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use qingjian_platform::protocol::{ClientMessage, PROTOCOL_VERSION, ServerMessage, SessionId};
use qingjian_platform::{Config, Scheme};
use std::path::PathBuf;

fn setup(name: &str) -> (Router, PathBuf) {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "qingjian-scheme-hold-{}-{stamp}-{name}",
        std::process::id()
    ));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("config.toml");
    std::fs::write(&path,"# keep my settings\n[general]\nscheme=\"pinyin\"\ncode=\"\"\npreedit=\"window\"\n[shortcut]\nright_control_scheme_switch=true\nright_control_scheme_hold_ms=600\n[quick_translation]\ncandidate_enabled=false\nhold_ms=450\n").unwrap();
    let config = Config::load(&path).unwrap();
    let mut r = Router::new(
        Engine::new(Dictionary::parse("你好\tni hao\t100\n").unwrap()),
        RouterConfig::from(&config),
    );
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    r.configure_schemes(&config, Some(root.join("assets/schemes")));
    r.watch_config(&config, path, root, DataDirs::default());
    r.handle(ClientMessage::OpenSession {
        session: SessionId(1),
        app: Some("test.exe".into()),
        protocol: PROTOCOL_VERSION,
    });
    (r, dir)
}
fn toggle(r: &mut Router, composing: bool) -> ServerMessage {
    r.handle(ClientMessage::ToggleInputScheme {
        session: SessionId(1),
        composing,
    })
    .unwrap()
}
fn cleanup(r: Router, dir: PathBuf) {
    drop(r);
    assert!(
        dir.canonicalize()
            .unwrap()
            .starts_with(std::env::temp_dir().canonicalize().unwrap())
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn translation_preferences_survive_and_private_context_cannot_switch() {
    let (mut r, dir) = setup("preferences");
    Config::set_bool(
        &dir.join("config.toml"),
        "quick_translation",
        "candidate_enabled",
        true,
    )
    .unwrap();
    let config = Config::load(&dir.join("config.toml")).unwrap();
    r.apply_config(&config);
    assert!(r.engine.translation_enabled());
    toggle(&mut r, false);
    assert!(r.engine.translation_enabled());
    assert_eq!(r.config.quick_translation.hold_ms, 450);
    let before = std::fs::read(dir.join("config.toml")).unwrap();
    r.handle(ClientMessage::Privacy {
        session: SessionId(1),
        private: true,
    });
    toggle(&mut r, false);
    assert_eq!(std::fs::read(dir.join("config.toml")).unwrap(), before);
    cleanup(r, dir);
}

#[test]
fn toggles_both_schemes_and_preserves_unrelated_config() {
    let (mut r, dir) = setup("roundtrip");
    assert!(matches!(
        toggle(&mut r, false),
        ServerMessage::ModeSync {
            english: Some(false),
            ..
        }
    ));
    assert!(r.engine.code_sentence_enabled());
    r.engine.set_input("wqjywyg");
    assert!(
        r.engine
            .query()
            .unwrap()
            .candidates
            .items
            .iter()
            .any(|c| c.text == "你是谁")
    );
    r.engine.clear();
    let config = Config::load(&dir.join("config.toml")).unwrap();
    assert_eq!(config.general.scheme(), Scheme::Off);
    assert_eq!(config.general.code().as_deref(), Some("wubi86-sentence"));
    assert!(!config.quick_translation.candidate_enabled);
    assert_eq!(config.quick_translation.hold_ms, 450);
    toggle(&mut r, false);
    assert!(r.engine.is_phonetic());
    assert!(!r.engine.is_code_mode());
    r.engine.set_input("nihao");
    assert!(
        r.engine
            .query()
            .unwrap()
            .candidates
            .items
            .iter()
            .any(|c| c.text == "你好")
    );
    assert!(
        std::fs::read_to_string(dir.join("config.toml"))
            .unwrap()
            .contains("# keep my settings")
    );
    cleanup(r, dir);
}
#[test]
fn unconfirmed_input_and_client_composition_refuse_switch() {
    let (mut r, dir) = setup("busy");
    let bytes = std::fs::read(dir.join("config.toml")).unwrap();
    r.ensure_focus(SessionId(1));
    r.engine.set_input("nihao");
    r.recompose();
    assert!(matches!(
        toggle(&mut r, false),
        ServerMessage::ModeSync { english: None, .. }
    ));
    assert_eq!(r.engine.composition().text(), "nihao");
    assert_eq!(std::fs::read(dir.join("config.toml")).unwrap(), bytes);
    assert!(r.self_drawn_frame().notice.unwrap().contains("确认"));
    r.engine.clear();
    r.recompose();
    assert!(matches!(
        toggle(&mut r, true),
        ServerMessage::ModeSync { english: None, .. }
    ));
    assert_eq!(std::fs::read(dir.join("config.toml")).unwrap(), bytes);
    cleanup(r, dir);
}
#[test]
fn old_clients_and_disabled_switch_cannot_toggle() {
    let (mut r, dir) = setup("old");
    r.handle(ClientMessage::OpenSession {
        session: SessionId(1),
        app: None,
        protocol: 12,
    });
    assert!(
        r.input_settings(SessionId(1))
            .right_control_scheme_hold_ms
            .is_none()
    );
    toggle(&mut r, false);
    assert!(r.engine.is_phonetic());
    r.handle(ClientMessage::OpenSession {
        session: SessionId(1),
        app: None,
        protocol: 13,
    });
    r.config.right_control_scheme_hold_ms = None;
    toggle(&mut r, false);
    assert!(r.engine.is_phonetic());
    cleanup(r, dir);
}
#[test]
fn broken_config_and_missing_table_leave_the_profile_unchanged() {
    let (mut r, dir) = setup("broken");
    std::fs::write(dir.join("config.toml"), "[bad").unwrap();
    toggle(&mut r, false);
    assert!(r.engine.is_phonetic());
    assert_eq!(
        std::fs::read_to_string(dir.join("config.toml")).unwrap(),
        "[bad"
    );
    cleanup(r, dir);
    let (mut r, dir) = setup("missing");
    r.bundled_schemes = None;
    toggle(&mut r, false);
    assert!(r.engine.is_phonetic());
    cleanup(r, dir);
}
