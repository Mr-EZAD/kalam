// layout.rs
use windows::Win32::Foundation::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::Threading::{GetCurrentThreadId, AttachThreadInput};

pub unsafe fn foreground_layout() -> HKL {
    let hwnd = GetForegroundWindow();
    let thread_id = GetWindowThreadProcessId(hwnd, None);
    GetKeyboardLayout(thread_id)
}

pub unsafe fn installed_layouts() -> Vec<HKL> {
    let count = GetKeyboardLayoutList(None) as usize;
    if count == 0 {
        return Vec::new();
    }
    let mut buf = vec![HKL(std::ptr::null_mut()); count];
    let actual = GetKeyboardLayoutList(Some(&mut buf));
    buf.truncate(actual as usize);
    buf
}

pub fn next_layout(current: HKL, layouts: &[HKL]) -> Option<HKL> {
    if layouts.len() < 2 {
        return None;
    }
    let idx = layouts.iter().position(|&l| l == current).unwrap_or(0);
    let next_idx = (idx + 1) % layouts.len();
    Some(layouts[next_idx])
}

/// امتیازدهی به یک layout به عنوان source برای متن داده شده.
/// یعنی: چه تعداد کاراکتر در این layout قابل تایپ هستند (VkKeyScanEx != -1)
unsafe fn score_as_source(text: &str, hkl: HKL) -> usize {
    #[link(name = "user32")]
    extern "system" {
        fn VkKeyScanExW(ch: u16, hkl: *mut std::ffi::c_void) -> i16;
    }

    let mut score = 0usize;
    for ch in text.chars() {
        if ch as u32 > 0xFFFF {
            continue;
        }
        let wchar = ch as u16;
        let result = VkKeyScanExW(wchar, hkl.0 as *mut std::ffi::c_void);
        if result == -1 {
            continue;
        }
        let vk = result as u16 & 0x00FF;
        let modifiers = (result as u16 >> 8) & 0x07;
        // VK معتبر و بدون Ctrl/Alt (shift قبوله)
        if vk != 0xFF && vk != 0 && modifiers & 0x06 == 0 {
            score += 1;
        }
    }
    score
}

/// تشخیص layout واقعی متن با در نظر گرفتن layout فعلی.
///
/// منطق:
/// 1. اگر layout فعلی بتونه بیشتر از threshold کاراکترها رو explain کنه → همون layout
/// 2. وگرنه بهترین layout از بین همه رو انتخاب کن
/// 3. اگر هیچ layout نتونست کافی explain کنه → None (متن native است، تبدیل لازم نیست)
pub unsafe fn detect_text_layout(
    text: &str,
    layouts: &[HKL],
    foreground: HKL,
) -> Option<HKL> {
    if layouts.is_empty() || text.is_empty() {
        return None;
    }

    let char_count = text
        .chars()
        .filter(|c| *c as u32 <= 0xFFFF)
        .count();

    if char_count == 0 {
        // متن فقط emoji یا SMP characters داره - قابل تبدیل نیست
        return None;
    }

    // امتیازدهی به همه layout‌ها
    let mut scores: Vec<(HKL, usize)> = layouts
        .iter()
        .map(|&hkl| (hkl, score_as_source(text, hkl)))
        .collect();

    scores.sort_by_key(|&(_, s)| std::cmp::Reverse(s));

    let best_score = scores[0].1;

    // اگر بهترین score خیلی پایین بود → متن native است
    // threshold: حداقل 80% کاراکترها باید قابل تایپ باشن
    let threshold = (char_count * 4).div_ceil(5); // 80%
    if best_score < threshold {
        crate::klog!(
            "detect_text_layout: best score {}/{} < threshold {} → native text",
            best_score, char_count, threshold
        );
        return None;
    }

    // اگر چند layout امتیاز یکسان دارن، foreground layout رو ترجیح بده
    let top_score = scores[0].1;
    let tied: Vec<HKL> = scores
        .iter()
        .filter(|&&(_, s)| s == top_score)
        .map(|&(hkl, _)| hkl)
        .collect();

    if tied.contains(&foreground) {
        crate::klog!(
            "detect_text_layout: tie resolved by foreground layout 0x{:X}",
            foreground.0 as usize & 0xFFFF
        );
        return Some(foreground);
    }

    let detected = scores[0].0;
    crate::klog!(
        "detect_text_layout: best={} score={}/{}",
        detected.0 as usize & 0xFFFF,
        best_score,
        char_count
    );
    Some(detected)
}

pub unsafe fn switch_to_layout(hkl: HKL) {
    let hwnd = GetForegroundWindow();
    if hwnd.0.is_null() {
        eprintln!("[kalam] No foreground window to switch layout for.");
        return;
    }

    let target_tid = GetWindowThreadProcessId(hwnd, None);
    let our_tid = GetCurrentThreadId();

    let attached = AttachThreadInput(our_tid, target_tid, true);
    if !attached.as_bool() {
        eprintln!("[kalam] Failed to attach thread input for layout switch.");
    }

    let result = ActivateKeyboardLayout(hkl, ACTIVATE_KEYBOARD_LAYOUT_FLAGS(0));
    if result.is_err() {
        eprintln!("[kalam] ActivateKeyboardLayout failed: {:?}", result);
    }

    let _ = PostMessageW(
        hwnd,
        WM_INPUTLANGCHANGEREQUEST,
        WPARAM(0),
        LPARAM(hkl.0 as isize),
    );

    std::thread::sleep(std::time::Duration::from_millis(50));

    if attached.as_bool() {
        let _ = AttachThreadInput(our_tid, target_tid, false);
    }
}