//! 本机 Ollama 的拼音自动翻译闭环验收，不操作桌面或发送消息。
use qingjian_platform::QuickTranslationConfig;
use qingjian_platform::protocol::{
    ClientMessage, KeyEvent, PROTOCOL_VERSION, ServerMessage, SessionId,
};
use qingjian_windows_server::{AssemblySpec, Router, RouterConfig, assembly};
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn main() {
    let dict = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../assets/sample/dict.tsv");
    let engine = assembly::assemble(&AssemblySpec::new(dict)).expect("sample dictionary");
    let mut router = Router::new(
        engine,
        RouterConfig {
            quick_translation: QuickTranslationConfig {
                candidate_enabled: true,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let session = SessionId(1);
    router.handle(ClientMessage::OpenSession {
        session,
        app: Some("acceptance.exe".into()),
        protocol: PROTOCOL_VERSION,
    });
    for pinyin in ["nihao", "mingtiankaihui"] {
        for c in pinyin.chars() {
            router.handle(ClientMessage::Key {
                session,
                event: KeyEvent::new(c.to_ascii_uppercase() as u32, Some(c), Default::default()),
            });
        }
        let start = Instant::now();
        let chinese;
        let english;
        loop {
            assert!(
                start.elapsed() < Duration::from_secs(75),
                "translation timed out"
            );
            if let Some(ServerMessage::Update { frame, .. }) =
                router.handle(ClientMessage::Poll { session })
            {
                assert!(
                    !frame
                        .notice
                        .as_deref()
                        .is_some_and(|s| s.contains("不可用")),
                    "model failed"
                );
                if let Some(candidate) = frame.candidates.items.first()
                    && let Some(translation) = &candidate.translation
                {
                    chinese = candidate.text.clone();
                    english = translation.senses()[0].text.clone();
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(40));
        }
        let elapsed = start.elapsed().as_millis();
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
            panic!("long hold did not commit");
        };
        assert_eq!(commit, format!("{chinese}\n{english}"));
        let mut up = KeyEvent::new(0x20, Some(' '), Default::default());
        up.released = true;
        router.handle(ClientMessage::Key { session, event: up });
        println!(
            "{}",
            serde_json::json!({"pinyin":pinyin,"chinese":chinese,"english":english,"translation_ms":elapsed,"bilingual_commit":commit})
        );
    }
}
