//! 记录测试前布局，仅卸载本次测试新增的布局；断言失败也执行恢复。
use std::sync::{Mutex, MutexGuard};

use windows::Win32::UI::Input::KeyboardAndMouse::{
    ActivateKeyboardLayout, GetKeyboardLayout, GetKeyboardLayoutList, GetKeyboardState, HKL,
    SetKeyboardState, UnloadKeyboardLayout,
};

static KEYBOARD_TEST_LOCK: Mutex<()> = Mutex::new(());

pub(in crate::com::key) struct KeyboardGuard {
    _lock: MutexGuard<'static, ()>,

    layouts: Vec<HKL>,

    original: HKL,

    state: [u8; 256],
}

impl KeyboardGuard {
    pub(in crate::com::key) fn enter() -> Self {
        let lock = KEYBOARD_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut state = [0; 256];
        unsafe { GetKeyboardState(&mut state) }.expect("read keyboard state before test");
        Self {
            _lock: lock,
            layouts: layouts(),
            original: unsafe { GetKeyboardLayout(0) },
            state,
        }
    }
}

impl Drop for KeyboardGuard {
    fn drop(&mut self) {
        let mut errors = Vec::new();
        if let Err(error) = unsafe { ActivateKeyboardLayout(self.original, Default::default()) } {
            errors.push(format!("restore layout: {error}"));
        }
        if let Err(error) = unsafe { SetKeyboardState(&self.state) } {
            errors.push(format!("restore keyboard state: {error}"));
        }
        for layout in layouts()
            .into_iter()
            .filter(|layout| !self.layouts.contains(layout))
        {
            if let Err(error) = unsafe { UnloadKeyboardLayout(layout) } {
                errors.push(format!("unload test layout {layout:?}: {error}"));
            }
        }
        if !errors.is_empty() {
            if std::thread::panicking() {
                eprintln!("Keyboard test cleanup failed: {errors:?}");
            } else {
                panic!("Keyboard test cleanup failed: {errors:?}");
            }
        }
    }
}

fn layouts() -> Vec<HKL> {
    let count = unsafe { GetKeyboardLayoutList(None) };
    let mut list = vec![HKL::default(); count.max(0) as usize];
    let read = unsafe { GetKeyboardLayoutList(Some(&mut list)) };
    list.truncate(read.max(0) as usize);
    list
}
