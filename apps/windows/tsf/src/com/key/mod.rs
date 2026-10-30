//! 按键相关：TSF 虚拟键码到协议 [`KeyEvent`](qingjian_platform::protocol::KeyEvent) 的翻译（[`event`]）、
//! 单击中英切换键的判定（[`tap`]，键来自 `[shortcut] switch_mode`）、「翻译选中文字」快捷键的保留键登记（[`preserved`]）。

pub(crate) mod event;
pub(crate) mod hold;
mod layout;
mod mouse_guard;
pub(crate) mod preserved;
mod tap;
pub(crate) use hold::SchemeHold;
pub(crate) use mouse_guard::SchemeMouseGuard;

#[cfg(test)]
mod test_support;

pub(crate) use self::tap::KeyTap;
