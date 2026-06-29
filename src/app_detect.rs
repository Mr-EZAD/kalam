use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::Threading::*;
#[allow(unused_imports)]
use crate::klog;

#[derive(Debug, Clone, PartialEq)]
pub enum AppType {
    ElectronChromium,
    VisualStudio,
    JavaAwt,        // ← جدید
    Standard,
}

pub unsafe fn detect_foreground_app_from_hwnd(hwnd: *mut std::ffi::c_void) -> AppType {
    let hwnd = HWND(hwnd);
    if hwnd.0.is_null() { return AppType::Standard; }

    // اول class name چک کن - سریع‌تر از exe name
    let mut class_buf = [0u16; 128];
    let len = GetClassNameW(hwnd, &mut class_buf);
    if len > 0 {
        let class = String::from_utf16_lossy(&class_buf[..len as usize]);
        klog!("Class: {}", class);

        // Java AWT/Swing
        if class.starts_with("SunAwt")
            || class == "JAVAWS-DESKTOP"
            || class.starts_with("awt_")
        {
            return AppType::JavaAwt;
        }

        if class.starts_with("Chrome_WidgetWin") {
            return AppType::ElectronChromium;
        }
    }

    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));

    if let Some(exe) = get_exe_name(pid) {
        let exe_lower: String = String::from_utf16_lossy(&exe).to_lowercase();
        klog!("EXE: {}", exe_lower);

        if exe_lower == "devenv.exe" {
            return AppType::VisualStudio;
        }
        if exe_lower == "code.exe"
            || exe_lower == "code - insiders.exe"
            || exe_lower == "cursor.exe"
        {
            return AppType::ElectronChromium;
        }
        if exe_lower.contains("chrome")
            || exe_lower.contains("msedge")
            || exe_lower.contains("brave")
            || exe_lower.contains("firefox")
        {
            return AppType::ElectronChromium;
        }

        // Java apps معمولاً javaw.exe یا java.exe هستن
        // ولی DBVisualizer به صورت wrapped exe هست
        // پس class name کافیه، ولی اینجا هم چک می‌کنیم
        if exe_lower == "javaw.exe"
            || exe_lower == "java.exe"
            || exe_lower == "dbvis.exe"
            || exe_lower.contains("jdk")
            || exe_lower.contains("jre")
        {
            return AppType::JavaAwt;
        }
    }

    AppType::Standard
}

unsafe fn get_exe_name(pid: u32) -> Option<Vec<u16>> {
    let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;

    let mut buf = [0u16; 260];
    let mut size = buf.len() as u32;

    let ok = QueryFullProcessImageNameW(
        handle,
        PROCESS_NAME_FORMAT(0),
        windows::core::PWSTR(buf.as_mut_ptr()),
        &mut size,
    );

    let _ = CloseHandle(handle);

    if ok.is_ok() {
        let path = &buf[..size as usize];
        let start = path
            .iter()
            .rposition(|&c| c == b'\\' as u16 || c == b'/' as u16)
            .map(|i| i + 1)
            .unwrap_or(0);
        Some(path[start..].to_vec())
    } else {
        None
    }
}