use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

pub static LOGGING_ENABLED: AtomicBool = AtomicBool::new(false);

static LOG_FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

pub fn init() {
    if !LOGGING_ENABLED.load(Ordering::Relaxed) {
        return;
    }

    let mut exe_path = [0u16; 260];
    unsafe {
        #[link(name = "kernel32")]
        extern "system" {
            fn GetModuleFileNameW(h: *mut std::ffi::c_void, buf: *mut u16, size: u32) -> u32;
        }
        GetModuleFileNameW(std::ptr::null_mut(), exe_path.as_mut_ptr(), 260);
    }
    
    let path = String::from_utf16_lossy(&exe_path);
    let path = path.trim_matches('\0');
    let log_path = if let Some(i) = path.rfind('\\') {
        format!("{}\\kalam.log", &path[..i])
    } else {
        "kalam.log".to_string()
    };

    if let Ok(file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        *LOG_FILE.lock().unwrap() = Some(file);
        log(&format!("=== Kalam started === log: {}", log_path));
    }
}

pub fn log(msg: &str) {
    if !LOGGING_ENABLED.load(Ordering::Relaxed) {
        return;
    }

    let timestamp = {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    };
    
    let line = format!("[{}] {}\n", timestamp, msg);
    
    print!("{}", line);
    
    if let Ok(mut guard) = LOG_FILE.lock() {
        if let Some(ref mut file) = *guard {
            let _ = file.write_all(line.as_bytes());
            let _ = file.flush();
        }
    }
}

#[macro_export]
macro_rules! klog {
    ($($arg:tt)*) => {
        crate::logger::log(&format!($($arg)*))
    };
}

pub fn preview(s: &str, max_chars: usize) -> String {
    let p: String = s.chars().take(max_chars).collect();
    if s.chars().count() > max_chars {
        format!("{}...", p)
    } else {
        p
    }
}