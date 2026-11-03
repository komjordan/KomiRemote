//! KomiRemote: the person sitting at this PC always wins.
//!
//! While the local user moves the mouse, clicks or types, remote mouse and
//! keyboard input is ignored. Control comes back on its own about one second
//! after the last local input, so nobody can ever be locked out.
//!
//! Local input is told apart from injected input (ours or any other tool) with
//! the low-level hook flags `LLMHF_INJECTED` / `LLKHF_INJECTED`.

#[cfg(target_os = "windows")]
use std::sync::{
    atomic::{AtomicI64, Ordering},
    Once,
};

/// How long the host keeps priority after its last local input.
#[cfg(target_os = "windows")]
const PRIORITY_MS: i64 = 1000;

#[cfg(target_os = "windows")]
static LAST_LOCAL_INPUT: AtomicI64 = AtomicI64::new(0);
#[cfg(target_os = "windows")]
static START: Once = Once::new();

/// True while the host has used its own mouse or keyboard within the last second.
pub fn active() -> bool {
    #[cfg(target_os = "windows")]
    {
        START.call_once(|| {
            std::thread::spawn(win::run_hooks);
        });
        let last = LAST_LOCAL_INPUT.load(Ordering::Relaxed);
        if last <= 0 {
            return false;
        }
        let elapsed = hbb_common::get_time() - last;
        return elapsed >= 0 && elapsed < PRIORITY_MS;
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

#[cfg(target_os = "windows")]
mod win {
    use super::{Ordering, LAST_LOCAL_INPUT};
    use hbb_common::log;
    use std::ptr::null_mut;
    use winapi::{
        shared::minwindef::{LPARAM, LRESULT, WPARAM},
        um::winuser::{
            CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
            UnhookWindowsHookEx, KBDLLHOOKSTRUCT, MSG, MSLLHOOKSTRUCT, WH_KEYBOARD_LL,
            WH_MOUSE_LL,
        },
    };

    const LLMHF_INJECTED_ANY: u32 = 0x1 | 0x2; // LLMHF_INJECTED | LLMHF_LOWER_IL_INJECTED
    const LLKHF_INJECTED_ANY: u32 = 0x10 | 0x2; // LLKHF_INJECTED | LLKHF_LOWER_IL_INJECTED

    #[inline]
    fn is_ours(extra: usize) -> bool {
        extra == enigo::ENIGO_INPUT_EXTRA_VALUE as usize
    }

    unsafe extern "system" fn mouse_proc(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
        if code == 0 && l != 0 {
            let s = &*(l as *const MSLLHOOKSTRUCT);
            if s.flags & LLMHF_INJECTED_ANY == 0 && !is_ours(s.dwExtraInfo as usize) {
                LAST_LOCAL_INPUT.store(hbb_common::get_time(), Ordering::Relaxed);
            }
        }
        CallNextHookEx(null_mut(), code, w, l)
    }

    unsafe extern "system" fn keyboard_proc(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
        if code == 0 && l != 0 {
            let s = &*(l as *const KBDLLHOOKSTRUCT);
            if s.flags & LLKHF_INJECTED_ANY == 0 && !is_ours(s.dwExtraInfo as usize) {
                LAST_LOCAL_INPUT.store(hbb_common::get_time(), Ordering::Relaxed);
            }
        }
        CallNextHookEx(null_mut(), code, w, l)
    }

    pub(super) fn run_hooks() {
        unsafe {
            let mouse = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), null_mut(), 0);
            let keyboard = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), null_mut(), 0);
            if mouse.is_null() && keyboard.is_null() {
                log::warn!("KomiRemote host priority: hooks unavailable, priority disabled");
                return;
            }
            log::info!("KomiRemote host priority: local input hooks installed");
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            if !mouse.is_null() {
                UnhookWindowsHookEx(mouse);
            }
            if !keyboard.is_null() {
                UnhookWindowsHookEx(keyboard);
            }
        }
    }
}
