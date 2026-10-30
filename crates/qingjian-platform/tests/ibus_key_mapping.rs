//! IBus的纯按键映射直接复用于跨平台测试，不依赖Linux、D-Bus或输入法守护进程。
//! 通用协议字段变更时，Windows本机也能发现Linux适配层的构造遗漏。
#[path = "../../../apps/linux/ibus/src/key/mod.rs"]
mod key;

#[test]
fn ibus_release_remains_in_the_outer_linux_event() {
    const RELEASE: u32 = 1 << 30;
    for state in [0, RELEASE, RELEASE | (1 << 0) | (1 << 2)] {
        let event = key::to_key_event('n' as u32, state);
        assert_eq!(event.character, Some('n'));
        assert_eq!(key::is_release(state), state & RELEASE != 0);
        assert!(!event.released, "Windows空格松开字段不代替Linux外层事件");
        assert_eq!(event.modifiers.shift, state & 1 != 0);
        assert_eq!(event.modifiers.ctrl, state & (1 << 2) != 0);
    }
}
