mod layout;
mod clipboard_util;
mod convert;
mod text_io;
mod app_detect;
mod tray;
mod logger;
mod settings;
mod hotkey_dialog;

use windows::Win32::Foundation::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::Com::*;
use std::ffi::c_void;
use std::thread;
use std::time::Duration;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

pub static HOTKEY_CHANGED: AtomicBool = AtomicBool::new(false);
static HOTKEY_PRESSED:     AtomicBool = AtomicBool::new(false);
pub static SHUTDOWN:       AtomicBool = AtomicBool::new(false);
static MAIN_THREAD_ID:     AtomicU32  = AtomicU32::new(0);
const HOTKEY_ID: i32 = 1;

#[link(name = "user32")]
extern "system" {
    fn GetForegroundWindow() -> *mut c_void;
    fn PostThreadMessageW(tid: u32, msg: u32, wp: usize, lp: isize) -> i32;
    fn GetCurrentThreadId() -> u32;
    fn AllowSetForegroundWindow(pid: u32) -> i32;
    fn GetWindowThreadProcessId(hwnd: *mut c_void, pid: *mut u32) -> u32;
}

pub fn request_hotkey_reregister() {
    HOTKEY_CHANGED.store(true, Ordering::SeqCst);
    unsafe {
        PostThreadMessageW(MAIN_THREAD_ID.load(Ordering::SeqCst), 0, 0, 0);
    }
}

fn main() {
    // ── Single instance check ──
    unsafe {
        #[link(name = "kernel32")]
        extern "system" {
            fn CreateMutexW(attrs: *mut c_void, owner: i32, name: *const u16) -> *mut c_void;
            fn GetLastError() -> u32;
        }
        const ERROR_ALREADY_EXISTS: u32 = 183;
        let name: Vec<u16> = "Global\\KalamSingleInstance\0".encode_utf16().collect();
        CreateMutexW(std::ptr::null_mut(), 1, name.as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            // یه instance دیگه در حال اجراست - خروج کن
            return;
        }
    }

    logger::init();
    klog!("Starting Kalam...");

    let s = settings::load();
    tray::set_autostart(s.autostart);
    klog!("Autostart: {}", s.autostart);

    unsafe {
        std::panic::set_hook(Box::new(|info| {
            klog!("PANIC: {}", info);
        }));

        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        klog!("COM initialized");

        hide_console();
        print_layouts();

        klog!("Starting tray...");
        tray::start_tray();
        klog!("Tray started");

        let s = settings::load();
        klog!("Hotkey: {}", s.hotkey.display());

        // اول unregister کن بعد register - جلوگیری از "already registered"
        let _ = UnregisterHotKey(HWND(std::ptr::null_mut()), HOTKEY_ID);

        match RegisterHotKey(
            HWND(std::ptr::null_mut()),
            HOTKEY_ID,
            HOT_KEY_MODIFIERS(s.hotkey.modifiers()),
            s.hotkey.vk_code(),
        ) {
            Ok(_)  => klog!("Hotkey registered: {}", s.hotkey.display()),
            Err(e) => klog!("Hotkey registration FAILED: {:?}", e),
        }

        let main_tid = GetCurrentThreadId();
        MAIN_THREAD_ID.store(main_tid, Ordering::SeqCst);
        klog!("Main thread ID: {}", main_tid);

        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(500));
                if SHUTDOWN.load(Ordering::SeqCst) {
                    klog!("Shutdown requested");
                    PostThreadMessageW(main_tid, 0x0012, 0, 0);
                    break;
                }
            }
        });

        klog!("Entering message loop");
        let mut msg = MSG::default();
        loop {
            let ret = GetMessageW(&mut msg, HWND(std::ptr::null_mut()), 0, 0);
            if ret.0 <= 0 {
                klog!("GetMessage returned {}, exiting", ret.0);
                break;
            }

            if HOTKEY_CHANGED.swap(false, Ordering::SeqCst) {
                let s = settings::load();
                klog!("Re-registering hotkey: {}", s.hotkey.display());

                let _ = UnregisterHotKey(HWND(std::ptr::null_mut()), HOTKEY_ID);

                match RegisterHotKey(
                    HWND(std::ptr::null_mut()),
                    HOTKEY_ID,
                    HOT_KEY_MODIFIERS(s.hotkey.modifiers()),
                    s.hotkey.vk_code(),
                ) {
                    Ok(_)  => klog!("Hotkey re-registered: {}", s.hotkey.display()),
                    Err(e) => klog!("Hotkey re-register FAILED: {:?}", e),
                }
                continue;
            }

            if msg.message == WM_HOTKEY && msg.wParam.0 as i32 == HOTKEY_ID {
                if !tray::KALAM_ENABLED.load(Ordering::SeqCst) {
                    klog!("Hotkey pressed but disabled");
                    continue;
                }

                if HOTKEY_PRESSED.swap(true, Ordering::Acquire) {
                    klog!("Hotkey already processing");
                    continue;
                }

                klog!("Hotkey pressed - capturing foreground window");
                let hwnd_raw = GetForegroundWindow();
                klog!("Foreground HWND: {:?}", hwnd_raw);

                // به پروسه target اجازه بده foreground بگیره
                let mut target_pid = 0u32;
                GetWindowThreadProcessId(hwnd_raw, &mut target_pid);
                if target_pid != 0 {
                    AllowSetForegroundWindow(target_pid);
                    klog!("AllowSetForegroundWindow for PID: {}", target_pid);
                }

                let hwnd_usize = hwnd_raw as usize;
                thread::spawn(move || {
                    let result = std::panic::catch_unwind(|| {
                        on_hotkey(hwnd_usize as *mut c_void);
                    });
                    if let Err(e) = result {
                        klog!("Worker thread PANIC: {:?}", e);
                    }
                    HOTKEY_PRESSED.store(false, Ordering::Release);
                });
            }

            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        klog!("Message loop exited");
        let _ = UnregisterHotKey(HWND(std::ptr::null_mut()), HOTKEY_ID);
        CoUninitialize();
        klog!("Kalam exited cleanly");
    }
}

unsafe fn hide_console() {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetConsoleWindow() -> *mut c_void;
    }
    #[link(name = "user32")]
    extern "system" {
        fn ShowWindow(hwnd: *mut c_void, cmd: i32) -> i32;
    }
    let hwnd = GetConsoleWindow();
    if !hwnd.is_null() {
        ShowWindow(hwnd, 0);
    }
}

unsafe fn print_layouts() {
    let layouts = layout::installed_layouts();
    klog!("Installed layouts ({}):", layouts.len());
    for (i, hkl) in layouts.iter().enumerate() {
        klog!(
            "  [{}] 0x{:016X}  lang=0x{:04X}",
            i, hkl.0 as usize, hkl.0 as usize & 0xFFFF,
        );
    }
}

fn on_hotkey(target_hwnd: *mut c_void) {
    unsafe {
        klog!("on_hotkey start, HWND={:?}", target_hwnd);
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let app_type = app_detect::detect_foreground_app_from_hwnd(target_hwnd);
        klog!("App type: {:?}", app_type);

        let foreground_layout = layout::foreground_layout();
        let layouts = layout::installed_layouts();

        klog!(
            "Layouts available: {}",
            layouts.iter()
                .map(|h| format!("0x{:X}", h.0 as usize & 0xFFFF))
                .collect::<Vec<_>>()
                .join(", ")
        );

        if layouts.len() < 2 {
            klog!("ERROR: Need at least 2 layouts");
            CoUninitialize();
            return;
        }

        // ── متن رو اول بگیر ──
        let selection = match app_type {
            app_detect::AppType::ElectronChromium |
            app_detect::AppType::VisualStudio => {
                klog!("Using Electron/VS strategy");
                match text_io::acquire_text_electron(target_hwnd) {
                    Some(s) => { klog!("Acquired {} chars", s.text.len()); s }
                    None => {
                        klog!("ERROR: Could not acquire text (Electron)");
                        CoUninitialize();
                        return;
                    }
                }
            }
            // ── جدید ──
            app_detect::AppType::JavaAwt => {
                klog!("Using Java AWT strategy");
                match text_io::acquire_text_java(target_hwnd) {
                    Some(s) => { klog!("Acquired {} chars", s.text.len()); s }
                    None => {
                        klog!("ERROR: Could not acquire text (Java)");
                        CoUninitialize();
                        return;
                    }
                }
            }
            app_detect::AppType::Standard => {
                klog!("Using Standard/UIA strategy");
                match acquire_standard(target_hwnd) {
                    Some(s) => { klog!("Acquired {} chars", s.text.len()); s }
                    None => {
                        klog!("ERROR: Could not acquire text (Standard)");
                        CoUninitialize();
                        return;
                    }
                }
            }
        };

        klog!("Input:  {}", logger::preview(&selection.text, 80));

        let actual_src_layout = match layout::detect_text_layout(
            &selection.text,
            &layouts,
            foreground_layout,
        ) {
            Some(l) => l,
            None => {
                klog!("Text appears to be native/untranslatable, skipping");
                CoUninitialize();
                return;
            }
        };

        klog!(
            "Foreground layout: 0x{:X}  Detected text layout: 0x{:X}",
            foreground_layout.0 as usize & 0xFFFF,
            actual_src_layout.0 as usize & 0xFFFF,
        );

        let target_layout = match layout::next_layout(actual_src_layout, &layouts) {
            Some(l) => l,
            None => {
                klog!("ERROR: Could not determine target layout");
                CoUninitialize();
                return;
            }
        };

        klog!(
            "Convert: 0x{:X} → 0x{:X}",
            actual_src_layout.0 as usize & 0xFFFF,
            target_layout.0 as usize & 0xFFFF,
        );

        let converted = convert::convert_text(
            &selection.text,
            actual_src_layout.0 as isize,
            target_layout.0 as isize,
        );

        klog!("Output: {}", logger::preview(&converted, 80));

        if converted == selection.text {
            klog!("No change after conversion");
            CoUninitialize();
            return;
        }

        // ── جایگزینی ──
        let ok = match app_type {
            app_detect::AppType::ElectronChromium |
            app_detect::AppType::VisualStudio => {
                klog!("Replacing via Electron strategy");
                text_io::replace_text_electron(&converted, target_hwnd)
            }
            // ── جدید ──
            app_detect::AppType::JavaAwt => {
                klog!("Replacing via Java strategy");
                text_io::replace_text_java(&converted, target_hwnd)
            }
            app_detect::AppType::Standard => {
                klog!("Replacing via Standard strategy");
                text_io::replace_text(&converted)
            }
        };

        if !ok {
            klog!("ERROR: Replace failed");
        } else {
            klog!("Replace successful");
            let should_switch = settings::load().switch_layout;
            if should_switch {
                thread::sleep(Duration::from_millis(50));
                klog!("Switching layout to: 0x{:X}", target_layout.0 as usize & 0xFFFF);
                layout::switch_to_layout(target_layout);
            } else {
                klog!("Layout switch skipped (disabled in settings)");
            }
        }

        klog!("on_hotkey complete");
        CoUninitialize();
    }
}

unsafe fn acquire_standard(_target_hwnd: *mut c_void) -> Option<text_io::TextSelection> {
    // مستقیم clipboard - UIA دوباره Ctrl+C میزنه و باعث مشکل میشه
    text_io::try_clipboard_acquisition()
}