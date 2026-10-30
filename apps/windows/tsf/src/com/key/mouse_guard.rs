//! 仅在本输入线程长按期间观察鼠标组合；不拦截或转发到其他进程。
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, HHOOK, SetWindowsHookExW, UnhookWindowsHookEx, WH_MOUSE,
};
use windows::core::Result;

pub(crate) struct SchemeMouseGuard(HHOOK);
impl SchemeMouseGuard {
    pub(crate) fn new() -> Result<Self> {
        unsafe {
            SetWindowsHookExW(
                WH_MOUSE,
                Some(mouse_event),
                Some(crate::com::dll_instance()),
                GetCurrentThreadId(),
            )
            .map(Self)
        }
    }
}
impl Drop for SchemeMouseGuard {
    fn drop(&mut self) {
        let _ = unsafe { UnhookWindowsHookEx(self.0) };
    }
}
fn interrupts(message: usize) -> bool {
    matches!(
        message,
        0x0201 | 0x0204 | 0x0207 | 0x020b | 0x020a | 0x020e | 0x00a1 | 0x00a4 | 0x00a7 | 0x00ab
    )
}
unsafe extern "system" fn mouse_event(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && interrupts(wparam.0) {
        let _ = std::panic::catch_unwind(crate::com::service::cancel_scheme_hold_from_mouse);
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}
#[cfg(test)]
mod tests {
    #[test]
    fn observation_hook_can_be_installed_on_its_own_message_thread() {
        use windows::Win32::UI::WindowsAndMessaging::{MSG, PM_NOREMOVE, PeekMessageW};
        let mut message = MSG::default();
        let _ = unsafe { PeekMessageW(&mut message, None, 0, 0, PM_NOREMOVE) };
        let guard = super::SchemeMouseGuard::new().expect("thread-local mouse guard");
        drop(guard);
    }
    #[test]
    fn click_or_wheel_cancels_but_pointer_motion_does_not() {
        assert!(super::interrupts(0x0201));
        assert!(super::interrupts(0x020a));
        assert!(!super::interrupts(0x0200));
    }
}
