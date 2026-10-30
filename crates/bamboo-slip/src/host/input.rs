//! winit 的输入 → 交互层的事件。

use winit::event::MouseScrollDelta;
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::ui::{KeyCode, Modifiers};

/// 滚轮一格滚多远（点）。
const LINE: f32 = 40.0;

/// 字符键按不带修饰键时的字算（⌥1 是 1 不是 ¡，⇧/ 是 / 不是 ?），快捷键才认得出来；打出来的字另走文字输入。
pub(super) fn key_code(key: &Key) -> Option<KeyCode> {
    Some(match key {
        Key::Named(NamedKey::F1) => KeyCode::Function(1),
        Key::Named(NamedKey::F2) => KeyCode::Function(2),
        Key::Named(NamedKey::F3) => KeyCode::Function(3),
        Key::Named(NamedKey::F4) => KeyCode::Function(4),
        Key::Named(NamedKey::F5) => KeyCode::Function(5),
        Key::Named(NamedKey::F6) => KeyCode::Function(6),
        Key::Named(NamedKey::F7) => KeyCode::Function(7),
        Key::Named(NamedKey::F8) => KeyCode::Function(8),
        Key::Named(NamedKey::F9) => KeyCode::Function(9),
        Key::Named(NamedKey::F10) => KeyCode::Function(10),
        Key::Named(NamedKey::F11) => KeyCode::Function(11),
        Key::Named(NamedKey::F12) => KeyCode::Function(12),
        Key::Named(NamedKey::Tab) => KeyCode::Tab,
        Key::Named(NamedKey::Enter) => KeyCode::Enter,
        Key::Named(NamedKey::Space) => KeyCode::Space,
        Key::Named(NamedKey::Escape) => KeyCode::Escape,
        Key::Named(NamedKey::ArrowLeft) => KeyCode::ArrowLeft,
        Key::Named(NamedKey::ArrowRight) => KeyCode::ArrowRight,
        Key::Named(NamedKey::ArrowUp) => KeyCode::ArrowUp,
        Key::Named(NamedKey::ArrowDown) => KeyCode::ArrowDown,
        Key::Named(NamedKey::Home) => KeyCode::Home,
        Key::Named(NamedKey::End) => KeyCode::End,
        Key::Named(NamedKey::Backspace) => KeyCode::Backspace,
        Key::Named(NamedKey::Delete) => KeyCode::Delete,
        Key::Character(text) => KeyCode::Character(text.to_string()),
        _ => return None,
    })
}

/// `command` 是 macOS 的 ⌘、其他平台的 Ctrl。
pub(super) fn modifiers(state: ModifiersState) -> Modifiers {
    Modifiers {
        shift: state.shift_key(),
        alt: state.alt_key(),
        command: if cfg!(target_os = "macos") {
            state.super_key()
        } else {
            state.control_key()
        },
        control: state.control_key(),
        logo: state.super_key(),
    }
}

/// 滚动位置的增量（像素）：winit 给的是内容该挪的方向，反过来；行增量按行高换成像素。
pub(super) fn scroll(delta: MouseScrollDelta, scale: f32) -> (f32, f32) {
    let (dx, dy) = match delta {
        MouseScrollDelta::PixelDelta(position) => (position.x as f32, position.y as f32),
        MouseScrollDelta::LineDelta(x, y) => (x * LINE * scale, y * LINE * scale),
    };
    (-dx, -dy)
}
