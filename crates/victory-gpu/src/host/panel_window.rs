//! Diagnostics: the panel's numbers in a window of their own
//! (`VICTORY_PANEL_WINDOW=1`; off by default), beside the title's, so that
//! nothing is drawn over the picture. A thread of ours owns the window and
//! paints the newest text twice a second; the panel only hands it lines.

use std::sync::{Mutex, Once};

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontW, CreateSolidBrush, DT_LEFT, DT_NOPREFIX, DT_TOP, DeleteObject, DrawTextW,
    EndPaint, FillRect, InvalidateRect, PAINTSTRUCT, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect,
    GetMessageW, MSG, RegisterClassW, SetTimer, ShowWindow, SW_SHOWNOACTIVATE, TranslateMessage, WM_DESTROY,
    WM_ERASEBKGND, WM_PAINT, WM_TIMER, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOPMOST, WS_OVERLAPPEDWINDOW,
};

/// The newest text, lines joined by CR LF.
static TEXT: Mutex<String> = Mutex::new(String::new());

pub(super) fn asked() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| crate::switch::on("VICTORY_PANEL_WINDOW"))
}

/// Hands the window new lines (and opens it the first time).
pub(super) fn set(lines: &[String]) {
    *TEXT.lock().unwrap_or_else(|e| e.into_inner()) = lines.join("\r\n");
    static START: Once = Once::new();
    START.call_once(|| {
        let _ = std::thread::Builder::new().name("victory-panel-window".into()).spawn(run);
    });
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn run() {
    let class = wide("VictoryPanelWindow");
    let title = wide("VIctory stats");
    // SAFETY: plain Win32 calls on this thread's own window; the strings live
    // to the end of the function, which the message loop never leaves while
    // the window exists.
    unsafe {
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(proc_),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: std::ptr::null_mut(),
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class.as_ptr(),
        };
        RegisterClassW(&wc);
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_NOACTIVATE,
            class.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            520,
            360,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null(),
        );
        if hwnd.is_null() {
            tracing::warn!("no panel window");
            return;
        }
        ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        SetTimer(hwnd, 1, 500, None);
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

unsafe extern "system" fn proc_(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: called by the system for our window, on its thread.
    unsafe {
        match msg {
            WM_TIMER => {
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            WM_ERASEBKGND => 1,
            WM_PAINT => {
                let mut ps: PAINTSTRUCT = std::mem::zeroed();
                let dc = BeginPaint(hwnd, &mut ps);
                let mut rect: RECT = std::mem::zeroed();
                GetClientRect(hwnd, &mut rect);
                let black = CreateSolidBrush(0x0010_1010);
                FillRect(dc, &rect, black);
                DeleteObject(black);
                let face = wide("Consolas");
                let font = CreateFontW(18, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, face.as_ptr());
                let old = SelectObject(dc, font);
                SetBkMode(dc, TRANSPARENT as i32);
                SetTextColor(dc, 0x00E0_E0E0);
                let mut text: Vec<u16> = TEXT.lock().unwrap_or_else(|e| e.into_inner()).encode_utf16().collect();
                rect.left += 8;
                rect.top += 6;
                DrawTextW(dc, text.as_mut_ptr(), text.len() as i32, &mut rect, DT_LEFT | DT_TOP | DT_NOPREFIX);
                SelectObject(dc, old);
                DeleteObject(font);
                EndPaint(hwnd, &ps);
                0
            }
            WM_DESTROY => 0,
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}
