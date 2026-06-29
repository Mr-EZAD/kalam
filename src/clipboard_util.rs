use windows::Win32::Foundation::{HANDLE, HWND, HGLOBAL};
use windows::Win32::System::DataExchange::{
    OpenClipboard, CloseClipboard, GetClipboardData,
    SetClipboardData, EmptyClipboard,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

const CF_UNICODETEXT: u32 = 13;

pub fn get_clipboard_text() -> Option<String> {
    unsafe {
        if OpenClipboard(HWND(std::ptr::null_mut())).is_err() {
            return None;
        }

        let handle = GetClipboardData(CF_UNICODETEXT).ok();

        let result = handle.and_then(|h| {
            let hglobal = HGLOBAL(h.0);
            let ptr = GlobalLock(hglobal) as *const u16;
            if ptr.is_null() {
                return None;
            }

            let mut len = 0;
            while *ptr.add(len) != 0 {
                len += 1;
            }

            let slice = std::slice::from_raw_parts(ptr, len);
            let text = String::from_utf16_lossy(slice).to_string();

            let _ = GlobalUnlock(hglobal);

            if text.is_empty() { None } else { Some(text) }
        });

        let _ = CloseClipboard();
        result
    }
}

pub fn set_clipboard_text(text: &str) {
    unsafe {
        if OpenClipboard(HWND(std::ptr::null_mut())).is_err() {
            return;
        }

        let _ = EmptyClipboard();

        let utf16: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let byte_size = utf16.len() * 2;

        if let Ok(hmem) = GlobalAlloc(GMEM_MOVEABLE, byte_size) {
            let ptr = GlobalLock(hmem) as *mut u16;
            if !ptr.is_null() {
                std::ptr::copy_nonoverlapping(utf16.as_ptr(), ptr, utf16.len());
                let _ = GlobalUnlock(hmem);
                let _ = SetClipboardData(CF_UNICODETEXT, HANDLE(hmem.0));
            }
        }

        let _ = CloseClipboard();
    }
}