// Wake strips on the other displays.
//
// While the island is hidden its window shrinks to an invisible strip at the top
// of one display, and nothing polls. In "display under the cursor" mode every
// other display gets a strip too, so the island can be woken from any of them.
//
// These are bare Win32 windows, not Tauri ones: a WebView2 per display just to
// notice a hover would cost tens of megabytes. They are created on the main
// thread, whose event loop (tao) already dispatches messages for every window
// it owns, and they cost nothing until the cursor touches one: Windows sends
// WM_MOUSEMOVE, nobody polls.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use tauri::{AppHandle, Emitter, Manager, Monitor};

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, LoadCursorW, RegisterClassW,
    SetLayeredWindowAttributes, ShowWindow, IDC_ARROW, LWA_ALPHA, MA_NOACTIVATE,
    SW_SHOWNOACTIVATE, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_MOUSEACTIVATE, WM_MOUSEMOVE,
    WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use crate::island::{self, WINDOW_LABEL};

const CLASS_NAME: PCWSTR = w!("CoucouWakeStrip");

static APP: OnceLock<AppHandle> = OnceLock::new();
static CLASS: OnceLock<bool> = OnceLock::new();
/// Live strip windows. HWND is not Send, so they are kept as raw values.
static STRIPS: Mutex<Vec<isize>> = Mutex::new(Vec::new());
/// One wake per hide: the cursor sweeping across a strip sends a burst of moves.
static WOKEN: AtomicBool = AtomicBool::new(false);
/// Every strip gets WM_DISPLAYCHANGE: rebuild once, not once per strip.
static REBUILD_PENDING: AtomicBool = AtomicBool::new(false);

/// Puts a strip at the top centre of every display except `host`, the one the
/// island window itself sits on. Replaces whatever strips were there.
pub fn show_except(app: &AppHandle, host: Option<Monitor>) {
    let _ = APP.set(app.clone());
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        destroy_all();
        WOKEN.store(false, Ordering::Relaxed);
        let Ok(monitors) = handle.available_monitors() else { return };
        for m in monitors.iter().filter(|m| !host.as_ref().is_some_and(|h| island::same_monitor(h, m))) {
            if let Err(err) = create_strip(m) {
                crate::log::line(format!("wake strip: {err}"));
            }
        }
    });
}

pub fn hide_all(app: &AppHandle) {
    let _ = app.run_on_main_thread(destroy_all);
}

fn destroy_all() {
    let strips = std::mem::take(&mut *STRIPS.lock().unwrap());
    for raw in strips {
        // Fails only if Windows already destroyed it — nothing left to do then.
        let _ = unsafe { DestroyWindow(HWND(raw as *mut _)) };
    }
}

fn register_class() -> bool {
    *CLASS.get_or_init(|| unsafe {
        let Ok(module) = GetModuleHandleW(None) else { return false };
        let class = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: HINSTANCE(module.0),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            lpszClassName: CLASS_NAME,
            ..Default::default()
        };
        RegisterClassW(&class) != 0
    })
}

fn create_strip(m: &Monitor) -> Result<(), String> {
    if !register_class() {
        return Err("could not register the window class".into());
    }
    let (x, y, w, h) = island::strip_rect(m);
    unsafe {
        let module = GetModuleHandleW(None).map_err(|e| e.to_string())?;
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_LAYERED,
            CLASS_NAME,
            w!(""),
            WS_POPUP,
            x,
            y,
            w as i32,
            h as i32,
            None,
            None,
            Some(HINSTANCE(module.0)),
            None,
        )
        .map_err(|e| e.to_string())?;
        // Alpha 0 lets the mouse fall through; 1/255 is invisible and still hit.
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 1, LWA_ALPHA);
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        STRIPS.lock().unwrap().push(hwnd.0 as isize);
    }
    Ok(())
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_MOUSEMOVE => {
            if !WOKEN.swap(true, Ordering::Relaxed) {
                if let Some(app) = APP.get() {
                    let _ = app.emit_to(WINDOW_LABEL, "wake", ());
                }
            }
            LRESULT(0)
        }
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_DISPLAYCHANGE | WM_DPICHANGED => {
            schedule_rebuild();
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// Displays were plugged, unplugged, moved or rescaled while the island is
/// hidden: put the island strip and these strips back where they belong. Queued
/// rather than done inline — this runs inside one of the windows it destroys.
fn schedule_rebuild() {
    if REBUILD_PENDING.swap(true, Ordering::Relaxed) {
        return;
    }
    let Some(app) = APP.get().cloned() else { return };
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        REBUILD_PENDING.store(false, Ordering::Relaxed);
        crate::log::line("display layout changed while hidden — moving wake strips");
        if let Some(shared) = handle.try_state::<crate::Shared>() {
            let pref = shared.settings.lock().unwrap().screen.clone();
            if shared.gate.collapsed.load(Ordering::Relaxed) {
                crate::place_island(&handle, &pref, true);
            }
        }
        let _ = handle.emit("monitors-changed", ());
    });
}
