//! 内存双工流验证中文光标元数据在按键与轮询路径都不会丢失。
use qingjian_platform::protocol::{
    Frame, InputSettings, KeyEvent, ServerMessage, SessionId, write_message,
};
use qingjian_tsf::client::{EngineClient, KeyReply};
use std::io::{self, Cursor, Read, Write};

struct Scripted {
    input: Cursor<Vec<u8>>,
}
impl Read for Scripted {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.input.read(buffer)
    }
}
impl Write for Scripted {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        Ok(buffer.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn both_native_response_paths_keep_the_commit_caret() {
    let session = SessionId(1);
    let mut bytes = Vec::new();
    for message in [
        ServerMessage::SessionOpened {
            session,
            input: InputSettings::default(),
        },
        ServerMessage::CommitAt {
            session,
            text: "中文\nEnglish".into(),
            caret_utf16: 2,
            frame: Frame::default(),
        },
        ServerMessage::CommitAt {
            session,
            text: "你𠮷好\nHello".into(),
            caret_utf16: 4,
            frame: Frame::default(),
        },
        ServerMessage::Update {
            session,
            commit: Some("中文".into()),
            frame: Frame::default(),
        },
    ] {
        write_message(&mut bytes, &message).unwrap();
    }
    let (mut client, _) = EngineClient::open(
        Scripted {
            input: Cursor::new(bytes),
        },
        session,
        None,
    )
    .unwrap();
    let KeyReply::Result(reply) = client
        .key(KeyEvent::new(0x20, Some(' '), Default::default()))
        .unwrap()
    else {
        panic!("expected document")
    };
    assert_eq!(reply.caret_utf16, Some(2));
    assert_eq!(reply.commit.as_deref(), Some("中文\nEnglish"));
    let reply = client.poll().unwrap();
    assert_eq!(reply.caret_utf16, Some(4));
    assert_eq!(reply.commit.as_deref(), Some("你𠮷好\nHello"));
    let reply = client.poll().unwrap();
    assert_eq!(reply.caret_utf16, None);
}
