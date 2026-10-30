//! 正式Engine与码表的五笔整句本地翻译验收，不操作用户桌面。
use qingjian_core::CodeScheme;
use qingjian_dictionary::CodeTable;
use qingjian_lm::BigramModel;
use qingjian_platform::protocol::{
    ClientMessage, KeyEvent, PROTOCOL_VERSION, ServerMessage, SessionId,
};
use qingjian_platform::{QuickTranslationConfig, Scheme};
use qingjian_windows_server::{AssemblySpec, Router, RouterConfig, assembly};
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let data = PathBuf::from("C:/Program Files/Qingjian/data/generated");
    let engine = assembly::assemble(&AssemblySpec::new(root.join("assets/sample/dict.tsv")))
        .unwrap()
        .with_language_model(Box::new(
            BigramModel::from_path(&data.join("lm.qj")).unwrap(),
        ));
    let mut router = Router::new(
        engine,
        RouterConfig {
            scheme: Scheme::Off,
            quick_translation: QuickTranslationConfig {
                candidate_enabled: true,
                wubi_sentence: true,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    router.engine_mut().set_code_scheme(Some(CodeScheme::new(
        "wubi86-sentence",
        CodeTable::from_path(root.join("assets/schemes/wubi86-sentence/table.tsv")).unwrap(),
    )));
    router.engine_mut().set_phonetic(false);
    let session = SessionId(1);
    router.handle(ClientMessage::OpenSession {
        session,
        app: Some("wubi-test.exe".into()),
        protocol: PROTOCOL_VERSION,
    });
    for (codes, expected) in [
        ("wqjywyg", "你是谁"),
        ("wygdghtfdghkgawf", "今天下午三点开会"),
    ] {
        let started = Instant::now();
        for c in codes.chars() {
            let reply = router.handle(ClientMessage::Key {
                session,
                event: KeyEvent::new(c.to_ascii_uppercase() as u32, Some(c), Default::default()),
            });
            assert!(
                matches!(reply, Some(ServerMessage::KeyResult { commit: None, .. })),
                "committed early"
            );
        }
        let english;
        loop {
            assert!(
                started.elapsed() < Duration::from_secs(75),
                "translation timeout"
            );
            if let Some(ServerMessage::Update { frame, .. }) =
                router.handle(ClientMessage::Poll { session })
            {
                let candidate = frame
                    .candidates
                    .items
                    .first()
                    .expect("whole sentence candidate");
                assert_eq!(candidate.text, expected);
                if let Some(translation) = &candidate.translation {
                    english = translation.senses()[0].text.clone();
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(40));
        }
        let elapsed = started.elapsed().as_millis();
        router.handle(ClientMessage::Key {
            session,
            event: KeyEvent::new(0x20, Some(' '), Default::default()),
        });
        std::thread::sleep(Duration::from_millis(380));
        let Some(ServerMessage::Update {
            commit: Some(commit),
            ..
        }) = router.handle(ClientMessage::Poll { session })
        else {
            panic!("no bilingual commit")
        };
        assert_eq!(commit, format!("{expected}\n{english}"));
        let mut up = KeyEvent::new(0x20, Some(' '), Default::default());
        up.released = true;
        router.handle(ClientMessage::Key { session, event: up });
        println!(
            "{}",
            serde_json::json!({"codes":codes,"chinese":expected,"english":english,"elapsed_ms":elapsed,"commit":commit})
        );
    }
}
