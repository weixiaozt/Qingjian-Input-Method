//! 右Ctrl长按只切输入方案；普通Ctrl键事件仍交给应用，组合操作取消切换。
use super::TextService_Impl;
use crate::com::key::{SchemeMouseGuard, hold::is_right_control};
use crate::com::log::log;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::LPARAM;
use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
use windows::Win32::UI::TextServices::ITfContext;
use windows::core::Ref;

fn other_modifiers() -> bool {
    [0xa2, 0x10, 0x12, 0x5b, 0x5c, 0x01, 0x02, 0x04, 0x05, 0x06]
        .into_iter()
        .any(|key| unsafe { GetKeyState(key) } < 0)
}
impl TextService_Impl {
    pub(super) fn cancel_scheme_hold(&self) {
        self.scheme_hold.cancel();
        self.scheme_context.borrow_mut().take();
        self.scheme_mouse_guard.borrow_mut().take();
    }

    pub(super) fn reset_scheme_hold(&self) {
        self.cancel_scheme_hold();
        self.scheme_hold.reset();
    }

    pub(super) fn scheme_key_down(&self, pic: &Ref<ITfContext>, vk: u32, lparam: LPARAM) {
        let enabled = self
            .input_settings
            .get()
            .unwrap_or_default()
            .right_control_scheme_hold_ms
            .is_some();
        let right = is_right_control(vk, lparam.0);
        if !right {
            self.cancel_scheme_hold();
            return;
        }
        let started = self.scheme_hold.key_down(
            right,
            ((lparam.0 >> 30) & 1) != 0,
            enabled && !other_modifiers() && self.shared.foreground() && pic.ok().is_ok(),
            Instant::now(),
        );
        if started {
            *self.scheme_context.borrow_mut() = pic.ok().ok().cloned();
            match SchemeMouseGuard::new() {
                Ok(guard) => *self.scheme_mouse_guard.borrow_mut() = Some(guard),
                Err(error) => {
                    log(&format!("Scheme hold mouse guard unavailable: {error}"));
                    self.cancel_scheme_hold();
                }
            }
        }
    }

    pub(super) fn scheme_key_up(&self, pic: &Ref<ITfContext>, vk: u32, lparam: LPARAM) {
        let right = is_right_control(vk, lparam.0);
        let threshold = self
            .input_settings
            .get()
            .unwrap_or_default()
            .right_control_scheme_hold_ms;
        let fired = self.scheme_hold.key_up(
            right,
            Instant::now(),
            Duration::from_millis(threshold.unwrap_or(600)),
        );
        if !right {
            self.cancel_scheme_hold();
            return;
        }
        let original = self.scheme_context.borrow_mut().take();
        self.scheme_mouse_guard.borrow_mut().take();
        if !fired || threshold.is_none() {
            return;
        }
        self.key_tap.cancel();
        let valid = !other_modifiers()
            && self.shared.foreground()
            && original.as_ref().is_some_and(|context| {
                pic.ok().is_ok_and(|current| current == context)
                    && self
                        .thread_mgr
                        .borrow()
                        .as_ref()
                        .and_then(|manager| unsafe { manager.GetFocus().ok() })
                        .and_then(|document| unsafe { document.GetTop().ok() })
                        .is_some_and(|focused| focused == *context)
            });
        if !valid || !self.ensure_connected() {
            return;
        }
        if self
            .input_settings
            .get()
            .unwrap_or_default()
            .right_control_scheme_hold_ms
            .is_none()
        {
            return;
        }
        let reply = {
            self.engine.borrow_mut().as_mut().map(|client| {
                client.toggle_input_scheme(self.shared.composing() || self.shared.has_composition())
            })
        };
        match reply {
            Some(Ok(reply)) => {
                self.apply_input_settings(reply.input);
                self.apply_indicator_state(reply.indicator);
                if let Some(english) = reply.english {
                    self.adopt_mode(english);
                }
                if let Ok(context) = pic.ok() {
                    let _ = crate::com::edit::position::request(
                        context,
                        self.client_id.get(),
                        self.engine.clone(),
                    );
                }
            }
            Some(Err(error)) => log(&format!("Input scheme switch failed: {error}")),
            None => {}
        }
    }
}
