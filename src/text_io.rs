use std::ffi::c_void;
use std::time::{Duration, Instant};
use windows::core::*;
use windows::Win32::System::Com::*;
use windows::Win32::UI::Accessibility::*;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct TextSelection {
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextStrategy {
    TextPattern2,
    TextPattern,
    ValuePattern,
    LegacyIAccessible,
    ClipboardFallback,
}

// ── Win32 Structs ─────────────────────────────────────────────────────────────

#[repr(C)]
struct KEYBDINPUT {
    w_vk:          u16,
    w_scan:        u16,
    dw_flags:      u32,
    time:          u32,
    dw_extra_info: usize,
}

#[repr(C)]
union INPUT_UNION {
    ki:       std::mem::ManuallyDrop<KEYBDINPUT>,
    _padding: [u8; 32],
}

#[repr(C)]
struct INPUT {
    type_: u32,
    data:  INPUT_UNION,
}

// ── Win32 API ─────────────────────────────────────────────────────────────────

#[link(name = "user32")]
extern "system" {
    fn SendInput(c_inputs: u32, p_inputs: *const INPUT, cb_size: i32) -> u32;
    fn SetForegroundWindow(hwnd: *mut c_void) -> i32;
    fn GetForegroundWindow() -> *mut c_void;
    fn GetAsyncKeyState(vk: i32) -> i16;
}

// ── Constants ─────────────────────────────────────────────────────────────────

const INPUT_KEYBOARD:     u32 = 1;
const KEYEVENTF_KEYUP:    u32 = 0x0002;
const KEYEVENTF_SCANCODE: u32 = 0x0008;

const VK_CONTROL: u16 = 0x11;
const VK_C:       u16 = 0x43;
const VK_V:       u16 = 0x56;

const SC_CTRL: u16 = 0x1D;
const SC_C:    u16 = 0x2E;
const SC_V:    u16 = 0x2F;

const DELAY_BETWEEN_KEYS_MS: u64 = 20;
const DELAY_FOCUS_MS:        u64 = 250;
const DELAY_AFTER_COPY_MS:   u64 = 300;
const DELAY_AFTER_PASTE_MS:  u64 = 150;
const DELAY_CLIP_SET_MS:     u64 = 40;
const POLL_INTERVAL_MS:      u64 = 20;
const POLL_TIMEOUT_MS:       u64 = 1500;

const DELAY_JAVA_FOCUS_MS:    u64 = 400;  // Java به زمان بیشتری نیاز داره
const DELAY_JAVA_COPY_MS:     u64 = 800;  // Java clipboard lazy است
const POLL_TIMEOUT_JAVA_MS:   u64 = 3000; // timeout بیشتر برای Java

// ── Keyboard Helpers ──────────────────────────────────────────────────────────

#[inline]
fn sleep(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

#[inline]
unsafe fn make_key_input(vk: u16, scan: u16, flags: u32) -> INPUT {
    INPUT {
        type_: INPUT_KEYBOARD,
        data: INPUT_UNION {
            ki: std::mem::ManuallyDrop::new(KEYBDINPUT {
                w_vk:          vk,
                w_scan:        scan,
                dw_flags:      flags,
                time:          0,
                dw_extra_info: 0,
            }),
        },
    }
}

/// Release modifier keys که از hotkey مونده باشن
unsafe fn release_modifier_keys() {
    // (VK, SC)
    let modifiers: &[(u16, u16)] = &[
        (0x10, 0x2A), // VK_SHIFT
        (0x11, 0x1D), // VK_CONTROL
        (0x12, 0x38), // VK_MENU (Alt)
        (0x5B, 0x5B), // VK_LWIN
        (0x5C, 0x5C), // VK_RWIN
    ];

    let mut released = false;
    for &(vk, sc) in modifiers {
        if (GetAsyncKeyState(vk as i32) as u16) & 0x8000 != 0 {
            let input = make_key_input(vk, sc, KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP);
            SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
            crate::klog!("Released modifier VK=0x{:02X}", vk);
            released = true;
        }
    }
    if released {
        sleep(30);
    }
}

unsafe fn send_ctrl_c() {
    let inputs = [
        make_key_input(VK_CONTROL, SC_CTRL, KEYEVENTF_SCANCODE),
        make_key_input(VK_C,       SC_C,    KEYEVENTF_SCANCODE),
        make_key_input(VK_C,       SC_C,    KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP),
        make_key_input(VK_CONTROL, SC_CTRL, KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP),
    ];
    let sent = SendInput(
        inputs.len() as u32,
        inputs.as_ptr(),
        std::mem::size_of::<INPUT>() as i32,
    );
    crate::klog!("send_ctrl_c: sent {} inputs", sent);
    sleep(DELAY_BETWEEN_KEYS_MS);
}

unsafe fn send_ctrl_v() {
    let inputs = [
        make_key_input(VK_CONTROL, SC_CTRL, KEYEVENTF_SCANCODE),
        make_key_input(VK_V,       SC_V,    KEYEVENTF_SCANCODE),
        make_key_input(VK_V,       SC_V,    KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP),
        make_key_input(VK_CONTROL, SC_CTRL, KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP),
    ];
    let sent = SendInput(
        inputs.len() as u32,
        inputs.as_ptr(),
        std::mem::size_of::<INPUT>() as i32,
    );
    crate::klog!("send_ctrl_v: sent {} inputs", sent);
    sleep(DELAY_BETWEEN_KEYS_MS);
}

// ── Focus Helper ──────────────────────────────────────────────────────────────

unsafe fn bring_to_foreground(target_hwnd: *mut c_void) -> bool {
    for attempt in 0..5 {
        SetForegroundWindow(target_hwnd);
        sleep(50);
        let actual = GetForegroundWindow();
        crate::klog!(
            "SetForegroundWindow attempt {}: target={:?} actual={:?} match={}",
            attempt, target_hwnd, actual, actual == target_hwnd
        );
        if actual == target_hwnd {
            return true;
        }
        sleep(50);
    }
    crate::klog!("Warning: could not bring to foreground, continuing anyway");
    true
}

// ── Clipboard Poll ────────────────────────────────────────────────────────────

unsafe fn poll_clipboard_changed(sentinel: &str, timeout_ms: u64) -> Option<String> {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);

    while Instant::now() < deadline {
        sleep(POLL_INTERVAL_MS);
        let clip = crate::clipboard_util::get_clipboard_text();
        if let Some(ref text) = clip {
            if text != sentinel && !text.is_empty() {
                crate::klog!("Clipboard changed: {} chars", text.len());
                return clip;
            }
        }
    }

    crate::klog!("poll_clipboard_changed: TIMEOUT after {}ms", timeout_ms);
    None
}

// ── UIA Init ─────────────────────────────────────────────────────────────────

pub unsafe fn init_uia() -> Result<IUIAutomation> {
    CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
}

pub unsafe fn get_focused_element(uia: &IUIAutomation) -> Result<IUIAutomationElement> {
    uia.GetFocusedElement()
}

// ── Text Acquisition ──────────────────────────────────────────────────────────

pub unsafe fn acquire_selected_text(
    _uia: &IUIAutomation,
    element: &IUIAutomationElement,
) -> Option<(TextSelection, TextStrategy)> {
    if let Some(r) = try_text_pattern2(element) {
        return Some((r, TextStrategy::TextPattern2));
    }
    if let Some(r) = try_text_pattern(element) {
        return Some((r, TextStrategy::TextPattern));
    }
    if let Some(r) = try_value_pattern(element) {
        return Some((r, TextStrategy::ValuePattern));
    }
    if let Some(r) = try_legacy_accessible(element) {
        return Some((r, TextStrategy::LegacyIAccessible));
    }
    if let Some(r) = try_clipboard_acquisition() {
        return Some((r, TextStrategy::ClipboardFallback));
    }
    None
}

// ── UIA Patterns ──────────────────────────────────────────────────────────────

unsafe fn try_text_pattern2(element: &IUIAutomationElement) -> Option<TextSelection> {
    let pattern: IUIAutomationTextPattern2 =
        element.GetCurrentPatternAs(UIA_TextPattern2Id).ok()?;
    let selection = pattern.GetSelection().ok()?;
    if selection.Length().ok()? == 0 { return None; }
    let text = selection.GetElement(0).ok()?.GetText(-1).ok()?.to_string();
    if text.is_empty() { None } else { Some(TextSelection { text }) }
}

unsafe fn try_text_pattern(element: &IUIAutomationElement) -> Option<TextSelection> {
    let pattern: IUIAutomationTextPattern =
        element.GetCurrentPatternAs(UIA_TextPatternId).ok()?;
    let selection = pattern.GetSelection().ok()?;
    if selection.Length().ok()? == 0 { return None; }
    let text = selection.GetElement(0).ok()?.GetText(-1).ok()?.to_string();
    if text.is_empty() { None } else { Some(TextSelection { text }) }
}

unsafe fn try_value_pattern(element: &IUIAutomationElement) -> Option<TextSelection> {
    let pattern: IUIAutomationValuePattern =
        element.GetCurrentPatternAs(UIA_ValuePatternId).ok()?;
    let text = pattern.CurrentValue().ok()?.to_string();
    if text.is_empty() { None } else { Some(TextSelection { text }) }
}

unsafe fn try_legacy_accessible(element: &IUIAutomationElement) -> Option<TextSelection> {
    let pattern: IUIAutomationLegacyIAccessiblePattern =
        element.GetCurrentPatternAs(UIA_LegacyIAccessiblePatternId).ok()?;
    let text = pattern.CurrentValue().ok()?.to_string();
    if text.is_empty() { None } else { Some(TextSelection { text }) }
}

// ── Clipboard Acquisition ─────────────────────────────────────────────────────

pub unsafe fn try_clipboard_acquisition() -> Option<TextSelection> {
    crate::klog!("try_clipboard_acquisition: start");

    // modifier keys که از hotkey مونده رو release کن
    release_modifier_keys();
    sleep(50);

    let sentinel = make_sentinel();
    crate::clipboard_util::set_clipboard_text(&sentinel);
    sleep(DELAY_CLIP_SET_MS);

    let verify = crate::clipboard_util::get_clipboard_text();
    crate::klog!(
        "Sentinel set: {:?}",
        verify.as_deref().map(|s| &s[..s.len().min(20)])
    );

    send_ctrl_c();
    sleep(DELAY_AFTER_COPY_MS);

    let result = poll_clipboard_changed(&sentinel, POLL_TIMEOUT_MS);
    crate::klog!(
        "try_clipboard_acquisition: {}",
        if result.is_some() { "SUCCESS" } else { "FAILED" }
    );
    result.map(|text| TextSelection { text })
}

pub unsafe fn acquire_text_electron(target_hwnd: *mut c_void) -> Option<TextSelection> {
    crate::klog!("acquire_text_electron: HWND={:?}", target_hwnd);

    // modifier keys رو release کن
    release_modifier_keys();
    sleep(30);

    // Focus
    bring_to_foreground(target_hwnd);
    sleep(DELAY_FOCUS_MS);

    // Sentinel
    let sentinel = make_sentinel();
    crate::clipboard_util::set_clipboard_text(&sentinel);
    sleep(DELAY_CLIP_SET_MS);

    let verify = crate::clipboard_util::get_clipboard_text();
    crate::klog!(
        "Sentinel verify: {}",
        verify.as_deref().map(|s| &s[..s.len().min(20)]).unwrap_or("NONE")
    );

    // Ctrl+C
    crate::klog!("Sending Ctrl+C...");
    send_ctrl_c();

    // Poll
    crate::klog!("Polling clipboard...");
    let result = poll_clipboard_changed(&sentinel, POLL_TIMEOUT_MS);

    match &result {
        Some(text) => crate::klog!(
            "acquire_text_electron OK: {} chars: {:?}",
            text.len(),
            &text[..text.len().min(50)]
        ),
        None => {
            crate::klog!("acquire_text_electron FAILED");
            let fc = crate::clipboard_util::get_clipboard_text();
            crate::klog!(
                "Final clipboard: {}",
                fc.as_deref().map(|s| &s[..s.len().min(30)]).unwrap_or("NONE")
            );
        }
    }

    result.map(|text| TextSelection { text })
}

// ── Text Replacement ──────────────────────────────────────────────────────────

pub unsafe fn replace_text(new_text: &str) -> bool {
    let old_clip = crate::clipboard_util::get_clipboard_text();

    crate::clipboard_util::set_clipboard_text(new_text);
    sleep(DELAY_CLIP_SET_MS);

    let verify = crate::clipboard_util::get_clipboard_text();
    if verify.as_deref() != Some(new_text) {
        crate::klog!("replace_text: clipboard set failed");
        return false;
    }

    send_ctrl_v();
    sleep(DELAY_AFTER_PASTE_MS);

    if let Some(old) = old_clip {
        crate::clipboard_util::set_clipboard_text(&old);
    }
    true
}

pub unsafe fn replace_text_electron(new_text: &str, target_hwnd: *mut c_void) -> bool {
    crate::klog!("replace_text_electron: {} chars", new_text.len());

    let old_clip = crate::clipboard_util::get_clipboard_text();

    crate::clipboard_util::set_clipboard_text(new_text);
    sleep(DELAY_CLIP_SET_MS);

    let verify = crate::clipboard_util::get_clipboard_text();
    if verify.as_deref() != Some(new_text) {
        crate::klog!("replace_text_electron: clipboard verify failed");
        if let Some(old) = old_clip {
            crate::clipboard_util::set_clipboard_text(&old);
        }
        return false;
    }
    crate::klog!("Clipboard verified OK");

    bring_to_foreground(target_hwnd);
    sleep(DELAY_FOCUS_MS);

    crate::klog!("Sending Ctrl+V...");
    send_ctrl_v();
    sleep(DELAY_AFTER_PASTE_MS);

    if let Some(old) = old_clip {
        crate::clipboard_util::set_clipboard_text(&old);
    }

    crate::klog!("replace_text_electron: done");
    true
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_sentinel() -> String {
    format!(
        "__kalam_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos()
    )
}

/// استراتژی مخصوص Java AWT/Swing
/// مشکل: Java clipboard رو lazy آپدیت می‌کنه
/// راه‌حل: focus → کپی → منتظر بمون → اگر نشد، فوکوس رو ببر بیار
pub unsafe fn acquire_text_java(target_hwnd: *mut c_void) -> Option<TextSelection> {
    crate::klog!("acquire_text_java: HWND={:?}", target_hwnd);

    release_modifier_keys();
    sleep(50);

    // مرحله 1: focus رو برگردون
    bring_to_foreground(target_hwnd);
    sleep(DELAY_JAVA_FOCUS_MS);

    // مرحله 2: sentinel بذار
    let sentinel = make_sentinel();
    crate::clipboard_util::set_clipboard_text(&sentinel);
    sleep(DELAY_CLIP_SET_MS);

    crate::klog!("Java: Sending Ctrl+C...");
    send_ctrl_c();

    // مرحله 3: اول با timeout کوتاه امتحان کن
    crate::klog!("Java: Polling clipboard (fast)...");
    if let Some(text) = poll_clipboard_changed(&sentinel, 600) {
        crate::klog!("Java: Got text on first try: {} chars", text.len());
        return Some(TextSelection { text });
    }

    // مرحله 4: Java clipboard رو force flush کن
    // با گرفتن focus از پنجره دیگه و برگشتن، Java مجبور میشه clipboard رو sync کنه
    crate::klog!("Java: Forcing clipboard sync...");
    force_java_clipboard_sync(target_hwnd);

    // مرحله 5: دوباره Ctrl+C بزن
    bring_to_foreground(target_hwnd);
    sleep(200);

    let sentinel2 = make_sentinel();
    crate::clipboard_util::set_clipboard_text(&sentinel2);
    sleep(DELAY_CLIP_SET_MS);

    send_ctrl_c();
    sleep(DELAY_JAVA_COPY_MS);

    // مرحله 6: با timeout بیشتر poll کن
    crate::klog!("Java: Polling clipboard (slow)...");
    let result = poll_clipboard_changed(&sentinel2, POLL_TIMEOUT_JAVA_MS);

    match &result {
        Some(text) => crate::klog!("Java: Success: {} chars", text.len()),
        None => crate::klog!("Java: FAILED after all attempts"),
    }

    result.map(|text| TextSelection { text })
}

/// Java رو مجبور می‌کنه clipboard رو به سیستم flush کنه
/// با گرفتن focus به یک پنجره دیگه (taskbar یا desktop) و برگشتن
unsafe fn force_java_clipboard_sync(target_hwnd: *mut c_void) {
    // پیدا کردن taskbar برای فوکوس موقت
    #[link(name = "user32")]
    extern "system" {
        fn FindWindowW(class: *const u16, title: *const u16) -> *mut c_void;
    }

    let taskbar_class: Vec<u16> = "Shell_TrayWnd\0".encode_utf16().collect();
    let taskbar = FindWindowW(taskbar_class.as_ptr(), std::ptr::null());

    if !taskbar.is_null() {
        crate::klog!("Java: Switching focus to taskbar temporarily");
        SetForegroundWindow(taskbar);
        sleep(150); // کافیه که Java clipboard رو flush کنه
    } else {
        crate::klog!("Java: Taskbar not found, using sleep");
        sleep(300);
    }
}

/// replace برای Java - مثل Electron ولی با تأخیر بیشتر
pub unsafe fn replace_text_java(new_text: &str, target_hwnd: *mut c_void) -> bool {
    crate::klog!("replace_text_java: {} chars", new_text.len());

    let old_clip = crate::clipboard_util::get_clipboard_text();

    crate::clipboard_util::set_clipboard_text(new_text);
    sleep(DELAY_CLIP_SET_MS);

    let verify = crate::clipboard_util::get_clipboard_text();
    if verify.as_deref() != Some(new_text) {
        crate::klog!("replace_text_java: clipboard verify failed");
        if let Some(old) = old_clip {
            crate::clipboard_util::set_clipboard_text(&old);
        }
        return false;
    }

    bring_to_foreground(target_hwnd);
    sleep(DELAY_JAVA_FOCUS_MS);

    crate::klog!("Java: Sending Ctrl+V...");
    send_ctrl_v();
    sleep(DELAY_AFTER_PASTE_MS + 100); // Java paste هم کمی کندتره

    if let Some(old) = old_clip {
        crate::clipboard_util::set_clipboard_text(&old);
    }

    crate::klog!("replace_text_java: done");
    true
}