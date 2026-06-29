use std::ffi::c_void;

#[link(name = "user32")]
extern "system" {
    fn VkKeyScanExW(ch: u16, dwhkl: *mut c_void) -> i16;
    fn MapVirtualKeyExW(u_code: u32, u_map_type: u32, dwhkl: *mut c_void) -> u32;
    fn ToUnicodeEx(
        w_virt_key: u32,
        w_scan_code: u32,
        lp_key_state: *const u8,
        pwsz_buff: *mut u16,
        cch_buff: i32,
        w_flags: u32,
        dwhkl: *mut c_void,
    ) -> i32;
    fn GetKeyboardState(lp_key_state: *mut u8) -> i32;
}

const MAPVK_VK_TO_VSC: u32 = 0;
const TOUNICODE_DO_NOT_CHANGE_DEAD_KEY_STATE: u32 = 0x4;

const VK_SHIFT: usize = 0x10;
const VK_CONTROL: usize = 0x11;
const VK_MENU: usize = 0x12;
const VK_CAPITAL: usize = 0x14;
const VK_KANA: usize = 0x15;
const VK_SPACE: u32 = 0x20;

const CJK_LANG_IDS: &[u16] = &[0x0404, 0x0804, 0x0C04, 0x1004, 0x1404, 0x0411, 0x0412];

pub unsafe fn convert_text(text: &str, src_hkl: isize, dst_hkl: isize) -> String {
    if src_hkl == dst_hkl {
        return text.to_owned();
    }

    let src_ptr = src_hkl as *mut c_void;
    let dst_ptr = dst_hkl as *mut c_void;

    if is_ime_layout(src_hkl) || is_ime_layout(dst_hkl) {
        static WARNED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !WARNED.swap(true, std::sync::atomic::Ordering::Relaxed) {
            eprintln!("[kalam] IME layout detected");
        }
        return text.to_owned();
    }

    let mut real_keystate = [0u8; 256];
    GetKeyboardState(real_keystate.as_mut_ptr());

    let caps_on = real_keystate[VK_CAPITAL] & 0x01 != 0;
    let kana_on = real_keystate[VK_KANA] & 0x01 != 0;

    let scan_space = MapVirtualKeyExW(VK_SPACE, MAPVK_VK_TO_VSC, std::ptr::null_mut());

    let mut result = String::with_capacity(text.len() * 2);

    let mut base_keystate = [0u8; 256];
    if caps_on {
        base_keystate[VK_CAPITAL] = 0x01;
    }
    if kana_on {
        base_keystate[VK_KANA] = 0x01;
    }

    let mut buf = [0u16; 8];
    let mut verify_buf = [0u16; 8];

    for ch in text.chars() {
        if ch as u32 > 0xFFFF {
            result.push(ch);
            continue;
        }

        let wchar = ch as u16;
        let vk_scan = VkKeyScanExW(wchar, src_ptr);

        if vk_scan == -1 {
            result.push(ch);
            continue;
        }

        let vk = (vk_scan as u16 & 0x00FF) as u32;
        let shift_bits = ((vk_scan as u16) >> 8) as u32 & 0x07;

        if vk == 0xFF {
            result.push(ch);
            continue;
        }

        // Ctrl یا Alt+Gr کاراکترها → skip
        if shift_bits & 0x06 != 0 {
            result.push(ch);
            continue;
        }

        let scan_code = MapVirtualKeyExW(vk, MAPVK_VK_TO_VSC, src_ptr);
        if scan_code == 0 {
            result.push(ch);
            continue;
        }

        let mut keystate = base_keystate;
        if shift_bits & 0x01 != 0 {
            keystate[VK_SHIFT] = 0x80;
        }

        let ret = ToUnicodeEx(
            vk,
            scan_code,
            keystate.as_ptr(),
            buf.as_mut_ptr(),
            buf.len() as i32,
            TOUNICODE_DO_NOT_CHANGE_DEAD_KEY_STATE,
            dst_ptr,
        );

        if ret > 0 {
            // ── verify: تبدیل برگشتی رو چک کن ──
            let converted_wchar = buf[0];
            let back_scan = VkKeyScanExW(converted_wchar, dst_ptr);

            if back_scan != -1 {
                let back_vk = (back_scan as u16 & 0x00FF) as u32;
                let back_shift = ((back_scan as u16) >> 8) as u32 & 0x07;

                if back_vk != 0xFF && back_shift & 0x06 == 0 {
                    let back_scan_code = MapVirtualKeyExW(back_vk, MAPVK_VK_TO_VSC, dst_ptr);
                    if back_scan_code != 0 {
                        let mut back_keystate = base_keystate;
                        if back_shift & 0x01 != 0 {
                            back_keystate[VK_SHIFT] = 0x80;
                        }

                        let back_ret = ToUnicodeEx(
                            back_vk,
                            back_scan_code,
                            back_keystate.as_ptr(),
                            verify_buf.as_mut_ptr(),
                            verify_buf.len() as i32,
                            TOUNICODE_DO_NOT_CHANGE_DEAD_KEY_STATE,
                            src_ptr,
                        );

                        if back_ret > 0 && verify_buf[0] == wchar {
                            // ✅ round-trip درسته - تبدیل معتبره
                            for decoded in char::decode_utf16(buf[..ret as usize].iter().cloned()) {
                                result.push(decoded.unwrap_or(char::REPLACEMENT_CHARACTER));
                            }
                            continue;
                        }
                    }
                }
            }

            // ❌ round-trip درست نیست - کاراکتر اصلی رو نگه دار
            result.push(ch);
        } else if ret < 0 {
            flush_dead_key(dst_ptr, scan_space, &base_keystate);
            result.push(ch);
        } else {
            result.push(ch);
        }
    }

    result
}

#[inline]
unsafe fn flush_dead_key(hkl: *mut c_void, scan_space: u32, keystate: &[u8; 256]) {
    let mut buf = [0u16; 8];
    ToUnicodeEx(
        VK_SPACE,
        scan_space,
        keystate.as_ptr(),
        buf.as_mut_ptr(),
        8,
        0,
        hkl,
    );
}

#[inline]
fn is_ime_layout(hkl: isize) -> bool {
    let high = ((hkl as usize) >> 16) as u16;
    if high >= 0xE001 {
        return true;
    }
    let lang = (hkl as usize & 0xFFFF) as u16;
    CJK_LANG_IDS.contains(&lang)
}
