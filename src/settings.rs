use std::fs;
use std::path::PathBuf;

const SETTINGS_FILE: &str = "kalam_settings.json";
const APP_FOLDER: &str = "Kalam";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HotkeySettings {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub win: bool,
    pub key: String, // مثل "Z", "F12", "Space"
}

impl Default for HotkeySettings {
    fn default() -> Self {
        Self {
            ctrl: false,
            shift: true,
            alt: true,
            win: false,
            key: "Z".to_string(),
        }
    }
}

impl HotkeySettings {
    /// تبدیل به MOD_* flags برای RegisterHotKey
    pub fn modifiers(&self) -> u32 {
        let mut mods = 0u32;
        if self.ctrl  { mods |= 0x0002; } // MOD_CONTROL
        if self.shift { mods |= 0x0004; } // MOD_SHIFT
        if self.alt   { mods |= 0x0001; } // MOD_ALT
        if self.win   { mods |= 0x0008; } // MOD_WIN
        mods
    }

    /// تبدیل key string به virtual key code
    pub fn vk_code(&self) -> u32 {
        match self.key.to_uppercase().as_str() {
            // حروف
            "A" => 0x41, "B" => 0x42, "C" => 0x43, "D" => 0x44,
            "E" => 0x45, "F" => 0x46, "G" => 0x47, "H" => 0x48,
            "I" => 0x49, "J" => 0x4A, "K" => 0x4B, "L" => 0x4C,
            "M" => 0x4D, "N" => 0x4E, "O" => 0x4F, "P" => 0x50,
            "Q" => 0x51, "R" => 0x52, "S" => 0x53, "T" => 0x54,
            "U" => 0x55, "V" => 0x56, "W" => 0x57, "X" => 0x58,
            "Y" => 0x59, "Z" => 0x5A,
            // اعداد
            "0" => 0x30, "1" => 0x31, "2" => 0x32, "3" => 0x33,
            "4" => 0x34, "5" => 0x35, "6" => 0x36, "7" => 0x37,
            "8" => 0x38, "9" => 0x39,
            // F keys
            "F1"  => 0x70, "F2"  => 0x71, "F3"  => 0x72, "F4"  => 0x73,
            "F5"  => 0x74, "F6"  => 0x75, "F7"  => 0x76, "F8"  => 0x77,
            "F9"  => 0x78, "F10" => 0x79, "F11" => 0x7A, "F12" => 0x7B,
            // سایر
            "SPACE"  => 0x20,
            "TAB"    => 0x09,
            "ENTER"  => 0x0D,
            "ESC"    => 0x1B,
            "DELETE" => 0x2E,
            "INSERT" => 0x2D,
            "HOME"   => 0x24,
            "END"    => 0x23,
            "PAGEUP" => 0x21,
            "PAGEDOWN" => 0x22,
            // نامعتبر
            _ => 0x5A, // fallback به Z
        }
    }

    /// نمایش خوانا برای کاربر
    pub fn display(&self) -> String {
        let mut parts = Vec::new();
        if self.ctrl  { parts.push("Ctrl"); }
        if self.shift { parts.push("Shift"); }
        if self.alt   { parts.push("Alt"); }
        if self.win   { parts.push("Win"); }
        parts.push(&self.key);
        parts.join("+")
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppSettings {
    pub switch_layout: bool,
    pub autostart: bool,
    pub hotkey: HotkeySettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            switch_layout: true,
            autostart: false,
            hotkey: HotkeySettings::default(),
        }
    }
}

fn settings_path() -> PathBuf {
    let mut dir = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_default();
    dir.push(APP_FOLDER);
    dir
}

pub fn load() -> AppSettings {
    let path = settings_path().join(SETTINGS_FILE);
    if let Ok(json) = fs::read_to_string(&path) {
        if let Ok(settings) = serde_json::from_str(&json) {
            return settings;
        }
    }
    AppSettings::default()
}

pub fn save(settings: &AppSettings) -> std::io::Result<()> {
    let dir = settings_path();
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    let path = dir.join(SETTINGS_FILE);
    let json = serde_json::to_string_pretty(settings).unwrap_or_default();
    fs::write(&path, json)
}

pub fn toggle_switch_layout() -> bool {
    let mut s = load();
    s.switch_layout = !s.switch_layout;
    save(&s).ok();
    s.switch_layout
}

pub fn toggle_autostart() -> bool {
    let mut s = load();
    s.autostart = !s.autostart;
    save(&s).ok();
    s.autostart
}