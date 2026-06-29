//! System tray icon با منوی راست‌کلیک

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::settings;

pub static KALAM_ENABLED: AtomicBool = AtomicBool::new(true);

// ── Constants ─────────────────────────────────────────────────────────────────

const CLASS_NAME: &str = "KalamTrayClass\0";
const WM_TRAYICON: u32 = 0x0400 + 1;

const IDM_TOGGLE: usize = 1001;
const IDM_AUTOSTART: usize = 1002;
const IDM_SWITCH_LAYOUT: usize = 1003;
const IDM_ABOUT: usize = 1004;
const IDM_EXIT: usize = 1005;
const IDM_SET_HOTKEY: usize = 1006;

const TRAY_ID: u32 = 1;

const WM_COMMAND: u32 = 0x0111;
const WM_DESTROY: u32 = 0x0002;
const WM_RBUTTONUP: u32 = 0x0205;
const WM_LBUTTONDBLCLK: u32 = 0x0203;
const NIM_ADD: u32 = 0x00000000;
const NIM_MODIFY: u32 = 0x00000001;
const NIM_DELETE: u32 = 0x00000002;
const NIF_MESSAGE: u32 = 0x00000001;
const NIF_ICON: u32 = 0x00000002;
const NIF_TIP: u32 = 0x00000004;
const NIF_INFO: u32 = 0x00000010;
const NIIF_INFO: u32 = 0x00000001;
const MF_STRING: u32 = 0x00000000;
const MF_SEPARATOR: u32 = 0x00000800;
const MF_CHECKED: u32 = 0x00000008;
const TPM_BOTTOMALIGN: u32 = 0x0020;
const TPM_RIGHTALIGN: u32 = 0x0008;
const IMAGE_ICON: u32 = 1;
const LR_LOADFROMFILE: u32 = 0x00000010;

// ── Structs ───────────────────────────────────────────────────────────────────

#[repr(C)]
struct NOTIFYICONDATAW {
    cb_size: u32,
    hwnd: *mut c_void,
    uid: u32,
    u_flags: u32,
    u_callback_message: u32,
    h_icon: *mut c_void,
    sz_tip: [u16; 128],
    dw_state: u32,
    dw_state_mask: u32,
    sz_info: [u16; 256],
    u_timeout_version: u32,
    sz_info_title: [u16; 64],
    dw_info_flags: u32,
    guid_item: [u8; 16],
    h_balloon_icon: *mut c_void,
}

impl Default for NOTIFYICONDATAW {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C)]
struct POINT {
    x: i32,
    y: i32,
}

#[repr(C)]
struct MSG {
    hwnd: *mut c_void,
    message: u32,
    w_param: usize,
    l_param: isize,
    time: u32,
    pt: POINT,
}

#[repr(C)]
struct WNDCLASSEXW {
    cb_size: u32,
    style: u32,
    lpfn_wnd_proc: *mut c_void,
    cb_cls_extra: i32,
    cb_wnd_extra: i32,
    h_instance: *mut c_void,
    h_icon: *mut c_void,
    h_cursor: *mut c_void,
    hbr_background: *mut c_void,
    lpsz_menu_name: *const u16,
    lpsz_class_name: *const u16,
    h_icon_sm: *mut c_void,
}

// ── Win32 API ─────────────────────────────────────────────────────────────────

#[link(name = "user32")]
extern "system" {
    fn RegisterClassExW(lpwcx: *const WNDCLASSEXW) -> u16;
    fn CreateWindowExW(
        dw_ex_style: u32,
        lp_class_name: *const u16,
        lp_window_name: *const u16,
        dw_style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        hwnd_parent: *mut c_void,
        h_menu: *mut c_void,
        h_instance: *mut c_void,
        lp_param: *mut c_void,
    ) -> *mut c_void;
    fn DefWindowProcW(hwnd: *mut c_void, msg: u32, wp: usize, lp: isize) -> isize;
    fn GetMessageW(msg: *mut MSG, hwnd: *mut c_void, min: u32, max: u32) -> i32;
    fn TranslateMessage(msg: *const MSG) -> i32;
    fn DispatchMessageW(msg: *const MSG) -> isize;
    fn PostQuitMessage(exit_code: i32);
    fn CreatePopupMenu() -> *mut c_void;
    fn AppendMenuW(hmenu: *mut c_void, flags: u32, id: usize, text: *const u16) -> i32;
    fn TrackPopupMenu(
        hmenu: *mut c_void,
        flags: u32,
        x: i32,
        y: i32,
        reserved: i32,
        hwnd: *mut c_void,
        rect: *mut c_void,
    ) -> i32;
    fn DestroyMenu(hmenu: *mut c_void) -> i32;
    fn GetCursorPos(pt: *mut POINT) -> i32;
    fn SetForegroundWindow(hwnd: *mut c_void) -> i32;
    fn LoadImageW(
        hinst: *mut c_void,
        name: *const u16,
        type_: u32,
        cx: i32,
        cy: i32,
        flags: u32,
    ) -> *mut c_void;
    fn MessageBoxW(hwnd: *mut c_void, text: *const u16, caption: *const u16, ty: u32) -> i32;
}

#[link(name = "shell32")]
extern "system" {
    fn Shell_NotifyIconW(dw_message: u32, lp_data: *mut NOTIFYICONDATAW) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(name: *const u16) -> *mut c_void;
    fn GetModuleFileNameW(hmodule: *mut c_void, buf: *mut u16, size: u32) -> u32;
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn copy_to_wide_buf<const N: usize>(s: &str, buf: &mut [u16; N]) {
    for (i, c) in s.encode_utf16().enumerate() {
        if i >= N - 1 {
            break;
        }
        buf[i] = c;
    }
}

// ── Autostart ──────────────────────────────────────────────────────────────────

const AUTOSTART_KEY: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run";
const AUTOSTART_NAME: &str = "Kalam";

#[allow(dead_code)]
pub fn is_autostart_enabled() -> bool {
    use std::ptr;

    #[link(name = "advapi32")]
    extern "system" {
        fn RegOpenKeyExW(
            hkey: isize,
            sub_key: *const u16,
            options: u32,
            sam: u32,
            result: *mut isize,
        ) -> i32;
        fn RegQueryValueExW(
            hkey: isize,
            value: *const u16,
            reserved: *mut u32,
            type_: *mut u32,
            data: *mut u8,
            cb_data: *mut u32,
        ) -> i32;
        fn RegCloseKey(hkey: isize) -> i32;
    }

    unsafe {
        let hkcu: isize = -2147483647isize;
        let key = to_wide(AUTOSTART_KEY);
        let name = to_wide(AUTOSTART_NAME);

        let mut hkey: isize = 0;
        if RegOpenKeyExW(hkcu, key.as_ptr(), 0, 0x20019, &mut hkey) != 0 {
            return false;
        }

        let mut cb: u32 = 0;
        let result = RegQueryValueExW(
            hkey,
            name.as_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            &mut cb,
        );
        RegCloseKey(hkey);
        result == 0
    }
}

pub fn set_autostart(enable: bool) {
    #[link(name = "advapi32")]
    extern "system" {
        fn RegCreateKeyExW(
            hkey: isize,
            sub_key: *const u16,
            reserved: u32,
            class: *const u16,
            options: u32,
            sam: u32,
            security: *mut std::ffi::c_void,
            result: *mut isize,
            disposition: *mut u32,
        ) -> i32;
        fn GetLastError() -> u32;
        fn RegSetValueExW(
            hkey: isize,
            value: *const u16,
            reserved: u32,
            type_: u32,
            data: *const u8,
            cb_data: u32,
        ) -> i32;
        fn RegDeleteValueW(hkey: isize, value: *const u16) -> i32;
        fn RegCloseKey(hkey: isize) -> i32;
    }

    unsafe {
        let hkcu: isize = -2147483647isize;
        let key = to_wide(AUTOSTART_KEY);
        let name = to_wide(AUTOSTART_NAME);

        let mut hkey: isize = 0;
        let mut disposition: u32 = 0;
        let ret = RegCreateKeyExW(
            hkcu,
            key.as_ptr(),
            0,
            std::ptr::null(),
            0,
            0x20002, // KEY_SET_VALUE
            std::ptr::null_mut(),
            &mut hkey,
            &mut disposition,
        );
        crate::klog!("[autostart] ret={} LastError={}", ret, GetLastError());
        if ret != 0 {
            return;
        }

        if enable {
            let mut path = [0u16; 260];
            let len = GetModuleFileNameW(std::ptr::null_mut(), path.as_mut_ptr(), 260);
            crate::klog!("[autostart] exe path len={}", len);
            if len == 0 {
                RegCloseKey(hkey);
                return;
            }

            let path_bytes = std::slice::from_raw_parts(
                path.as_ptr() as *const u8,
                (len as usize + 1) * 2,
            );

            let path_str = String::from_utf16_lossy(&path[..len as usize]);
            crate::klog!("[autostart] Writing path: {}", path_str);

            let ret2 = RegSetValueExW(
                hkey,
                name.as_ptr(),
                0,
                1, // REG_SZ
                path_bytes.as_ptr(),
                path_bytes.len() as u32,
            );
            crate::klog!("[autostart] RegSetValueExW ret={}", ret2);
        } else {
            let ret2 = RegDeleteValueW(hkey, name.as_ptr());
            crate::klog!("[autostart] RegDeleteValueW ret={}", ret2);
        }

        RegCloseKey(hkey);
    }
}

// ── Tray Window Procedure ─────────────────────────────────────────────────────

static mut TRAY_HWND: *mut c_void = std::ptr::null_mut();
unsafe extern "system" fn tray_wnd_proc(
    hwnd: *mut c_void,
    msg: u32,
    wp: usize,
    lp: isize,
) -> isize {
    match msg {
        m if m == WM_TRAYICON => {
            let event = (lp & 0xFFFF) as u32;
            if event == WM_RBUTTONUP || event == WM_LBUTTONDBLCLK {
                show_context_menu(hwnd);
            }
        }
        m if m == WM_COMMAND => {
            let id = wp & 0xFFFF;
            match id {
                IDM_TOGGLE => {
                    let enabled = KALAM_ENABLED.fetch_xor(true, Ordering::SeqCst);
                    let new_state = !enabled;
                    update_tray_icon(hwnd, new_state);
                    crate::klog!("[tray] {}", if new_state { "Enabled" } else { "Disabled" });
                }
                IDM_AUTOSTART => {
                    let new = settings::toggle_autostart(); // ← settings.json
                    set_autostart(new); // ← registry
                    crate::klog!("[tray] Autostart: {}", if new { "ON" } else { "OFF" });
                }
                IDM_SWITCH_LAYOUT => {
                    let new = settings::toggle_switch_layout();
                    update_tray_icon(hwnd, KALAM_ENABLED.load(Ordering::SeqCst));
                    crate::klog!(
                        "[tray] Switch layout after convert: {}",
                        if new { "ON" } else { "OFF" }
                    );
                }
                IDM_ABOUT => {
                    show_about(hwnd);
                }
                IDM_EXIT => {
                    remove_tray_icon(hwnd);
                    PostQuitMessage(0);
                    crate::SHUTDOWN.store(true, Ordering::SeqCst);
                }
                IDM_SET_HOTKEY => {
                    let current = settings::load().hotkey;
                    std::thread::spawn(move || {
                        if let Some(captured) = crate::hotkey_dialog::show_hotkey_dialog(&current) {
                            let mut s = settings::load();
                            s.hotkey = settings::HotkeySettings {
                                ctrl: captured.ctrl,
                                shift: captured.shift,
                                alt: captured.alt,
                                win: captured.win,
                                key: captured.key_name.to_uppercase(),
                            };
                            settings::save(&s).ok();
                            crate::klog!("[tray] Hotkey changed to: {}", s.hotkey.display());

                            // ← اینجا تغییر کن: بجای فقط set کردن flag، thread رو بیدار کن
                            crate::request_hotkey_reregister();
                        }
                    });
                }
                _ => {}
            }
        }
        m if m == WM_DESTROY => {
            remove_tray_icon(hwnd);
            PostQuitMessage(0);
        }
        _ => return DefWindowProcW(hwnd, msg, wp, lp),
    }
    0
}

unsafe fn show_context_menu(hwnd: *mut c_void) {
    let hmenu = CreatePopupMenu();
    if hmenu.is_null() {
        return;
    }

    let enabled = KALAM_ENABLED.load(Ordering::SeqCst);
    let switch = settings::load().switch_layout;

    // Toggle Enable/Disable
    let toggle_text = if enabled {
        to_wide("✓  Enabled  (click to disable)")
    } else {
        to_wide("✗  Disabled  (click to enable)")
    };
    AppendMenuW(hmenu, MF_STRING, IDM_TOGGLE, toggle_text.as_ptr());

    AppendMenuW(hmenu, MF_SEPARATOR, 0, std::ptr::null());

    // Autostart
    let auto_flags = if settings::load().autostart {
        MF_STRING | MF_CHECKED
    } else {
        MF_STRING
    };
    AppendMenuW(
        hmenu,
        auto_flags,
        IDM_AUTOSTART,
        to_wide("Start with Windows").as_ptr(),
    );

    // Switch Layout After Convert
    let switch_flags = if switch {
        MF_STRING | MF_CHECKED
    } else {
        MF_STRING
    };
    AppendMenuW(
        hmenu,
        switch_flags,
        IDM_SWITCH_LAYOUT,
        to_wide("Switch layout after convert").as_ptr(),
    );
    AppendMenuW(
        hmenu,
        MF_STRING,
        IDM_SET_HOTKEY,
        to_wide(&format!(
            "Set Hotkey  ({})",
            settings::load().hotkey.display()
        ))
        .as_ptr(),
    );

    AppendMenuW(hmenu, MF_SEPARATOR, 0, std::ptr::null());
    AppendMenuW(hmenu, MF_STRING, IDM_ABOUT, to_wide("About Kalam").as_ptr());
    AppendMenuW(hmenu, MF_STRING, IDM_EXIT, to_wide("Exit").as_ptr());

    let mut pt = POINT { x: 0, y: 0 };
    GetCursorPos(&mut pt);

    SetForegroundWindow(hwnd);
    TrackPopupMenu(
        hmenu,
        TPM_BOTTOMALIGN | TPM_RIGHTALIGN,
        pt.x,
        pt.y,
        0,
        hwnd,
        std::ptr::null_mut(),
    );
    DestroyMenu(hmenu);
}

unsafe fn show_about(hwnd: *mut c_void) {
    let text = to_wide(
        "Kalam v1.0\n\
         کَلَم - Dynamic keyboard layout converter\n\n\
         Hotkey: Alt+Shift+Z\n\
         Select text → press hotkey → text converts!\n\n\
         github.com/Mr-EZAD/kalam"
    );
    let caption = to_wide("About Kalam");
    MessageBoxW(hwnd, text.as_ptr(), caption.as_ptr(), 0x40);
}

unsafe fn load_tray_icon() -> *mut c_void {
    // اول از فایل امتحان کن (برای development)
    let mut exe_path = [0u16; 260];
    let len = GetModuleFileNameW(std::ptr::null_mut(), exe_path.as_mut_ptr(), 260);

    if len > 0 {
        let path = String::from_utf16_lossy(&exe_path[..len as usize]);
        let dir = path.rfind('\\').map(|i| &path[..i]).unwrap_or(".");

        let icon_16_path = format!("{}\\kalam-16.ico", dir);
        let icon_16 = to_wide(&icon_16_path);
        let icon = LoadImageW(
            std::ptr::null_mut(),
            icon_16.as_ptr(),
            IMAGE_ICON,
            16,
            16,
            LR_LOADFROMFILE,
        );
        if !icon.is_null() {
            return icon;
        }

        let icon_path = format!("{}\\kalam.ico", dir);
        let icon_wide = to_wide(&icon_path);
        let icon = LoadImageW(
            std::ptr::null_mut(),
            icon_wide.as_ptr(),
            IMAGE_ICON,
            16,
            16,
            LR_LOADFROMFILE,
        );
        if !icon.is_null() {
            return icon;
        }
    }

    // ← fallback: از embedded resource بخون
    let hinstance = GetModuleHandleW(std::ptr::null());
    let icon = LoadImageW(
        hinstance,
        1 as *const u16, // ID=1 که در kalam.rc تعریف کردیم
        IMAGE_ICON,
        16,
        16,
        0, // بدون LR_LOADFROMFILE = از resource
    );
    if !icon.is_null() {
        return icon;
    }

    // آخرین fallback: icon پیش‌فرض ویندوز
    LoadImageW(
        std::ptr::null_mut(),
        32512 as *const u16, // IDI_APPLICATION
        IMAGE_ICON,
        16,
        16,
        0x0040, // LR_SHARED
    )
}

unsafe fn add_tray_icon(hwnd: *mut c_void) {
    let icon = load_tray_icon();

    let mut nid = NOTIFYICONDATAW::default();
    nid.cb_size = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hwnd = hwnd;
    nid.uid = TRAY_ID;
    nid.u_flags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    nid.u_callback_message = WM_TRAYICON;
    nid.h_icon = icon;
    copy_to_wide_buf("Kalam - Alt+Shift+Z", &mut nid.sz_tip);

    Shell_NotifyIconW(NIM_ADD, &mut nid);
}

unsafe fn remove_tray_icon(hwnd: *mut c_void) {
    let mut nid = NOTIFYICONDATAW::default();
    nid.cb_size = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hwnd = hwnd;
    nid.uid = TRAY_ID;
    Shell_NotifyIconW(NIM_DELETE, &mut nid);
}

pub unsafe fn update_tray_icon(hwnd: *mut c_void, enabled: bool) {
    let switch = settings::load().switch_layout;

    let mut nid = NOTIFYICONDATAW::default();
    nid.cb_size = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hwnd = hwnd;
    nid.uid = TRAY_ID;
    nid.u_flags = NIF_TIP | NIF_INFO;

    let tip = if enabled {
        if switch {
            "Kalam - Active | Layout switch: ON"
        } else {
            "Kalam - Active | Layout switch: OFF"
        }
    } else {
        "Kalam - Disabled"
    };
    copy_to_wide_buf(tip, &mut nid.sz_tip);

    nid.u_flags |= NIF_INFO;
    copy_to_wide_buf(
        if enabled {
            "Kalam enabled"
        } else {
            "Kalam disabled"
        },
        &mut nid.sz_info,
    );
    copy_to_wide_buf("Kalam", &mut nid.sz_info_title);
    nid.dw_info_flags = NIIF_INFO;
    nid.u_timeout_version = 2000;

    Shell_NotifyIconW(NIM_MODIFY, &mut nid);
}

// ── Public Entry Point ────────────────────────────────────────────────────────

pub fn start_tray() {
    std::thread::spawn(|| unsafe { tray_thread() });
}

unsafe fn tray_thread() {
    let hinstance = GetModuleHandleW(std::ptr::null());
    let class_name = to_wide(CLASS_NAME);

    let wc = WNDCLASSEXW {
        cb_size: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: 0,
        lpfn_wnd_proc: tray_wnd_proc as *mut c_void,
        cb_cls_extra: 0,
        cb_wnd_extra: 0,
        h_instance: hinstance,
        h_icon: std::ptr::null_mut(),
        h_cursor: std::ptr::null_mut(),
        hbr_background: std::ptr::null_mut(),
        lpsz_menu_name: std::ptr::null(),
        lpsz_class_name: class_name.as_ptr(),
        h_icon_sm: std::ptr::null_mut(),
    };
    RegisterClassExW(&wc);

    let hwnd = CreateWindowExW(
        0,
        class_name.as_ptr(),
        to_wide("Kalam").as_ptr(),
        0,
        0,
        0,
        0,
        0,
        (-3isize) as *mut c_void,
        std::ptr::null_mut(),
        hinstance,
        std::ptr::null_mut(),
    );

    TRAY_HWND = hwnd;
    add_tray_icon(hwnd);

    let mut msg: MSG = std::mem::zeroed();
    loop {
        let ret = GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0);
        if ret <= 0 {
            break;
        }
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
}