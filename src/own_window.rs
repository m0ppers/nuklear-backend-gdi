use super::nuklear_rust::NkContext;

use super::winapi;

use std::{ptr, mem, str};
use std::os::windows::ffi::OsStrExt;
use std::ffi::OsStr;

use super::Drawer;

pub fn create_env(window_name: &str, width: u16, height: u16) -> (winapi::shared::windef::HWND, winapi::shared::windef::HDC) {
    unsafe {
        let window = create_window(window_name, width as i32, height as i32);
        let hdc = winapi::um::winuser::GetDC(window);
        (window, hdc)
    }
}

pub fn process_events(drawer: &mut Drawer, ctx: &mut NkContext) {
    unsafe {
        if EVENTS.is_none() {
            EVENTS = Some(Vec::new());
        }

        for &mut (wnd, msg, wparam, lparam) in EVENTS.as_mut().unwrap() {
            drawer.handle_event(ctx, wnd, msg, wparam, lparam);
        }

        EVENTS.as_mut().unwrap().clear();
    }
}

fn register_window_class() -> Vec<u16> {
    unsafe {
        let class_name = OsStr::new("NuklearWindowClass")
            .encode_wide()
            .chain(Some(0).into_iter())
            .collect::<Vec<_>>();

        let class = winapi::um::winuser::WNDCLASSEXW {
            cbSize: mem::size_of::<winapi::um::winuser::WNDCLASSEXW>() as winapi::shared::minwindef::UINT,
            style: winapi::um::winuser::CS_DBLCLKS,
            lpfnWndProc: Some(callback),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: winapi::um::libloaderapi::GetModuleHandleW(ptr::null()),
            hIcon: winapi::um::winuser::LoadIconW(ptr::null_mut(), winapi::um::winuser::IDI_APPLICATION),
            hCursor: winapi::um::winuser::LoadCursorW(ptr::null_mut(), winapi::um::winuser::IDC_ARROW),
            hbrBackground: ptr::null_mut(),
            lpszMenuName: ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: ptr::null_mut(),
        };
        winapi::um::winuser::RegisterClassExW(&class);

        class_name
    }
}

unsafe fn create_window(window_name: &str, width: i32, height: i32) -> winapi::shared::windef::HWND {
    let class_name = register_window_class();

    let mut rect = winapi::shared::windef::RECT {
        left: 0,
        top: 0,
        right: width,
        bottom: height,
    };
    let style = winapi::um::winuser::WS_OVERLAPPEDWINDOW;
    let exstyle = winapi::um::winuser::WS_EX_APPWINDOW;

    winapi::um::winuser::AdjustWindowRectEx(&mut rect, style, winapi::shared::minwindef::FALSE, exstyle);
    let window_name = OsStr::new(window_name)
        .encode_wide()
        .chain(Some(0).into_iter())
        .collect::<Vec<_>>();

    winapi::um::winuser::CreateWindowExW(exstyle,
                            class_name.as_ptr(),
                            window_name.as_ptr() as winapi::shared::ntdef::LPCWSTR,
                            style | winapi::um::winuser::WS_VISIBLE,
                            winapi::um::winuser::CW_USEDEFAULT,
                            winapi::um::winuser::CW_USEDEFAULT,
                            rect.right - rect.left,
                            rect.bottom - rect.top,
                            ptr::null_mut(),
                            ptr::null_mut(),
                            winapi::um::libloaderapi::GetModuleHandleW(ptr::null()),
                            ptr::null_mut())
}

unsafe extern "system" fn callback(wnd: winapi::shared::windef::HWND, msg: winapi::shared::minwindef::UINT, wparam: winapi::shared::minwindef::WPARAM, lparam: winapi::shared::minwindef::LPARAM) -> winapi::shared::minwindef::LRESULT {
    match msg {
        winapi::um::winuser::WM_DESTROY => {
            winapi::um::winuser::PostQuitMessage(0);
            return 0;
        }
        _ => {
            if EVENTS.is_none() {
                EVENTS = Some(Vec::new());
            }
            EVENTS
                .as_mut()
                .unwrap()
                .push((wnd, msg, wparam, lparam));
        }
    }

    winapi::um::winuser::DefWindowProcW(wnd, msg, wparam, lparam)
}

static mut EVENTS: Option<Vec<(winapi::shared::windef::HWND, winapi::shared::minwindef::UINT, winapi::shared::minwindef::WPARAM, winapi::shared::minwindef::LPARAM)>> = None;
