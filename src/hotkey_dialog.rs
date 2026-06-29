//! Dialog برای انتخاب hotkey جدید

use std::ffi::c_void;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct CapturedHotkey {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub win: bool,
    #[allow(dead_code)]
    pub vk: u32,
    pub key_name: String,
}

// ── Win32 ─────────────────────────────────────────────────────────────────────

#[repr(C)]
struct MSG {
    hwnd:    *mut c_void,
    message: u32,
    w_param: usize,
    l_param: isize,
    time:    u32,
    pt_x:    i32,
    pt_y:    i32,
}

#[repr(C)]
struct WNDCLASSEXW {
    cb_size:         u32,
    style:           u32,
    lpfn_wnd_proc:   *mut c_void,
    cb_cls_extra:    i32,
    cb_wnd_extra:    i32,
    h_instance:      *mut c_void,
    h_icon:          *mut c_void,
    h_cursor:        *mut c_void,
    hbr_background:  *mut c_void,
    lpsz_menu_name:  *const u16,
    lpsz_class_name: *const u16,
    h_icon_sm:       *mut c_void,
}

#[link(name = "user32")]
extern "system" {
    fn RegisterClassExW(wc: *const WNDCLASSEXW) -> u16;
    fn CreateWindowExW(
        ex_style: u32, class: *const u16, title: *const u16,
        style: u32, x: i32, y: i32, w: i32, h: i32,
        parent: *mut c_void, menu: *mut c_void,
        instance: *mut c_void, param: *mut c_void,
    ) -> *mut c_void;
    fn ShowWindow(hwnd: *mut c_void, cmd: i32) -> i32;
    fn DefWindowProcW(hwnd: *mut c_void, msg: u32, wp: usize, lp: isize) -> isize;
    fn GetMessageW(msg: *mut MSG, hwnd: *mut c_void, min: u32, max: u32) -> i32;
    fn TranslateMessage(msg: *const MSG) -> i32;
    fn DispatchMessageW(msg: *const MSG) -> isize;
    fn PostQuitMessage(code: i32);
    fn DestroyWindow(hwnd: *mut c_void) -> i32;
    fn GetKeyState(vk: i32) -> i16;
    fn SetWindowTextW(hwnd: *mut c_void, text: *const u16) -> i32;
    fn GetSystemMetrics(index: i32) -> i32;
    fn SetForegroundWindow(hwnd: *mut c_void) -> i32;
    fn MessageBoxW(hwnd: *mut c_void, text: *const u16, caption: *const u16, ty: u32) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(name: *const u16) -> *mut c_void;
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ── VK به نام ─────────────────────────────────────────────────────────────────

fn vk_to_name(vk: u32) -> String {
    match vk {
        0x41..=0x5A => ((b'A' + (vk - 0x41) as u8) as char).to_string(),
        0x30..=0x39 => ((b'0' + (vk - 0x30) as u8) as char).to_string(),
        0x70 => "F1".to_string(),  0x71 => "F2".to_string(),
        0x72 => "F3".to_string(),  0x73 => "F4".to_string(),
        0x74 => "F5".to_string(),  0x75 => "F6".to_string(),
        0x76 => "F7".to_string(),  0x77 => "F8".to_string(),
        0x78 => "F9".to_string(),  0x79 => "F10".to_string(),
        0x7A => "F11".to_string(), 0x7B => "F12".to_string(),
        0x20 => "Space".to_string(),
        0x09 => "Tab".to_string(),
        0x0D => "Enter".to_string(),
        0x1B => "Esc".to_string(),
        0x2E => "Delete".to_string(),
        0x2D => "Insert".to_string(),
        0x24 => "Home".to_string(),
        0x23 => "End".to_string(),
        0x21 => "PageUp".to_string(),
        0x22 => "PageDown".to_string(),
        0x25 => "Left".to_string(),
        0x26 => "Up".to_string(),
        0x27 => "Right".to_string(),
        0x28 => "Down".to_string(),
        _ => format!("VK({:#X})", vk),
    }
}

// ── Global State ──────────────────────────────────────────────────────────────

static CAPTURED: std::sync::OnceLock<Arc<Mutex<Option<CapturedHotkey>>>> =
    std::sync::OnceLock::new();

static LABEL_HWND: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

static DIALOG_HWND: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

// ── Window Messages ───────────────────────────────────────────────────────────

const WM_KEYDOWN:          u32 = 0x0100;
const WM_SYSKEYDOWN:       u32 = 0x0104;
const WM_DESTROY:          u32 = 0x0002;
const WM_CLOSE:            u32 = 0x0010;
const WM_COMMAND:          u32 = 0x0111;
const WS_OVERLAPPEDWINDOW: u32 = 0x00CF0000;
const WS_VISIBLE:          u32 = 0x10000000;
const WS_CHILD:            u32 = 0x40000000;
const SS_CENTER:           u32 = 0x00000001;
const BS_PUSHBUTTON:       u32 = 0x00000000;
const SW_SHOW:             i32 = 5;

const IDC_LABEL:  usize = 101;
const IDC_OK:     usize = 102;
const IDC_CANCEL: usize = 103;

unsafe extern "system" fn dialog_proc(
    hwnd: *mut c_void,
    msg: u32,
    wp: usize,
    lp: isize,
) -> isize {
    match msg {
        m if m == WM_KEYDOWN || m == WM_SYSKEYDOWN => {
            let vk = wp as u32;

            if matches!(vk, 0x10 | 0x11 | 0x12 | 0x5B | 0x5C) {
                return 0;
            }

            let ctrl  = (GetKeyState(0x11) as u16) & 0x8000 != 0;
            let shift = (GetKeyState(0x10) as u16) & 0x8000 != 0;
            let alt   = (GetKeyState(0x12) as u16) & 0x8000 != 0;
            let win   = (GetKeyState(0x5B) as u16 | GetKeyState(0x5C) as u16) & 0x8000 != 0;

            if !ctrl && !shift && !alt && !win {
                let warn = to_wide("Please use at least one modifier key\n(Alt, Shift, Alt, or Win)");
                let cap  = to_wide("Kalam - Hotkey");
                MessageBoxW(hwnd, warn.as_ptr(), cap.as_ptr(), 0x30);
                return 0;
            }

            let key_name = vk_to_name(vk);

            let mut parts = Vec::new();
            if ctrl  { parts.push("Ctrl"); }
            if shift { parts.push("Shift"); }
            if alt   { parts.push("Alt"); }
            if win   { parts.push("Win"); }
            parts.push(key_name.as_str());
            let display = parts.join(" + ");

            let label_text = format!("Selected: {}", display);
            let label_wide = to_wide(&label_text);
            let label_hwnd = LABEL_HWND.load(std::sync::atomic::Ordering::SeqCst) as *mut c_void;
            SetWindowTextW(label_hwnd, label_wide.as_ptr());

            if let Some(captured) = CAPTURED.get() {
                *captured.lock().unwrap() = Some(CapturedHotkey {
                    ctrl, shift, alt, win,
                    vk,
                    key_name,
                });
            }

            0
        }

        m if m == WM_COMMAND => {
            let id = wp & 0xFFFF;
            match id {
                IDC_OK => {
                    DestroyWindow(hwnd);
                    PostQuitMessage(0);
                }
                IDC_CANCEL => {
                    if let Some(captured) = CAPTURED.get() {
                        *captured.lock().unwrap() = None;
                    }
                    DestroyWindow(hwnd);
                    PostQuitMessage(1);
                }
                _ => {}
            }
            0
        }

        m if m == WM_CLOSE => {
            if let Some(captured) = CAPTURED.get() {
                *captured.lock().unwrap() = None;
            }
            DestroyWindow(hwnd);
            PostQuitMessage(1);
            0
        }

        m if m == WM_DESTROY => {
            PostQuitMessage(0);
            0
        }

        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

pub fn show_hotkey_dialog(current: &crate::settings::HotkeySettings) -> Option<CapturedHotkey> {
    let new_arc = Arc::new(Mutex::new(None::<CapturedHotkey>));
    let arc = CAPTURED.get_or_init(|| new_arc.clone());
    *arc.lock().unwrap() = None;

    unsafe {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let class_name = to_wide("KalamHotkeyDialog");

        let wc = WNDCLASSEXW {
            cb_size:         std::mem::size_of::<WNDCLASSEXW>() as u32,
            style:           0x0003,
            lpfn_wnd_proc:   dialog_proc as *mut c_void,
            cb_cls_extra:    0,
            cb_wnd_extra:    0,
            h_instance:      hinstance,
            h_icon:          std::ptr::null_mut(),
            h_cursor:        std::ptr::null_mut(),
            hbr_background:  6 as *mut c_void,
            lpsz_menu_name:  std::ptr::null(),
            lpsz_class_name: class_name.as_ptr(),
            h_icon_sm:       std::ptr::null_mut(),
        };
        RegisterClassExW(&wc);

        let sw = GetSystemMetrics(0);
        let sh = GetSystemMetrics(1);
        let w = 380;
        let h = 180;
        let x = (sw - w) / 2;
        let y = (sh - h) / 2;

        let title = to_wide("Kalam - Set Hotkey");
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            x, y, w, h,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null_mut(),
        );

        if hwnd.is_null() { return None; }

        DIALOG_HWND.store(hwnd as usize, std::sync::atomic::Ordering::SeqCst);

        let inst_class = to_wide("STATIC");
        let current_display = format!(
            "Current: {}  →  Press new hotkey combination",
            current.display()
        );
        let inst_text = to_wide(&current_display);
        CreateWindowExW(
            0, inst_class.as_ptr(), inst_text.as_ptr(),
            WS_CHILD | WS_VISIBLE | SS_CENTER,
            10, 15, 360, 40,
            hwnd, std::ptr::null_mut(), hinstance, std::ptr::null_mut(),
        );

        let label_class = to_wide("STATIC");
        let label_text  = to_wide("Selected: (press a key combination)");
        let label = CreateWindowExW(
            0, label_class.as_ptr(), label_text.as_ptr(),
            WS_CHILD | WS_VISIBLE | SS_CENTER,
            10, 65, 360, 30,
            hwnd, IDC_LABEL as *mut c_void, hinstance, std::ptr::null_mut(),
        );
        LABEL_HWND.store(label as usize, std::sync::atomic::Ordering::SeqCst);

        let btn_class = to_wide("BUTTON");
        let ok_text   = to_wide("Apply");
        CreateWindowExW(
            0, btn_class.as_ptr(), ok_text.as_ptr(),
            WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON,
            80, 110, 90, 30,
            hwnd, IDC_OK as *mut c_void, hinstance, std::ptr::null_mut(),
        );

        let cancel_text = to_wide("Cancel");
        CreateWindowExW(
            0, btn_class.as_ptr(), cancel_text.as_ptr(),
            WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON,
            210, 110, 90, 30,
            hwnd, IDC_CANCEL as *mut c_void, hinstance, std::ptr::null_mut(),
        );

        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);

        let mut msg: MSG = std::mem::zeroed();
        loop {
            let ret = GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0);
            if ret <= 0 { break; }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    CAPTURED.get()?.lock().unwrap().clone()
}