<div align="center">
<img src="https://raw.githubusercontent.com/Mr-EZAD/kalam/refs/heads/main/assets/kalam.png" alt="Kalam Logo" width="120"/>

# کَلَم / Kalam

**Real-time keyboard layout text converter for Windows**

_Typed in the wrong layout? Convert it instantly._

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Platform: Windows](https://img.shields.io/badge/Platform-Windows-blue.svg)]()
[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust-orange.svg)](https://www.rust-lang.org/)

</div>

---

## What is Kalam?

Have you ever typed a whole sentence and realized your keyboard layout was wrong?

**Kalam** fixes that. Select the text, press your hotkey (default: `Alt+Shift+Z`), and it converts
the text to what you meant to type — then switches your layout automatically.

Before: اثممخ (typed with Persian layout active, meant English)  
After: hello (converted to correct layout)

---

## Features

- **One hotkey** — Default `Alt` + `Shift` + `Z`, fully customizable from tray menu
- **Works everywhere** — Notepad, Word, browsers, VS Code, Telegram, terminals
- **Any two layouts** — Works with any pair of installed Windows keyboard layouts
- **Non-destructive** — Clipboard is saved and restored after conversion
- **System tray** — Lives quietly in your tray, always ready
- **Autostart** — Optional start with Windows
- **Lightweight** — Single ~1MB executable, no installation needed

---

## Supported Applications

| Application                          | Status |
| ------------------------------------ | ------ |
| Notepad / WordPad                    | ✅     |
| Microsoft Word                       | ✅     |
| Web Browsers (Chrome, Firefox, Edge) | ✅     |
| VS Code                              | ✅     |
| Visual Studio                        | ✅     |
| Telegram Desktop                     | ✅     |
| Windows Terminal / CMD               | ✅     |
| Any standard Win32 app               | ✅     |

---

## Installation

### Option 1: Download Release (Recommended)

1. Go to [Releases](../../releases/latest)
2. Download `kalam.exe`
3. Run it — no installation needed

### Option 2: Build from Source

**Requirements:**

- [Rust](https://rustup.rs/) (1.70+)
- Windows 10 or later

```powershell
git clone https://github.com/YOUR_USERNAME/kalam.git
cd kalam
cargo build --release
./target/release/kalam.exe
```

---

## Usage

### Basic

You have English (US) and Persian layouts installed.  
Your active layout is Persian, but you type English words:
You type: اثممخ
You meant: hello

Select اثممخ → Press `Alt`+ `Shift` + `Z` → Text becomes hello ✓

### Tray Menu

Right-click the tray icon to access:

| Menu Item                   | Description                         |
| --------------------------- | ----------------------------------- |
| ✓ Enabled / ✗ Disabled      | Toggle Kalam on/off                 |
| Start with Windows          | Enable/disable autostart            |
| Switch layout after convert | Auto-switch layout after conversion |
| Set Hotkey                  | Change the conversion hotkey        |
| About Kalam                 | Version info                        |
| Exit                        | Quit Kalam                          |
|                             |                                     |

### Setting a Custom Hotkey

1. Right-click tray icon → **Set Hotkey**
2. Press your desired key combination (e.g. `Ctrl+Alt+Z`)
3. Click **Apply**
   The new hotkey is saved and takes effect immediately — no restart needed.

---

## How It Works

Hotkey pressed
│
▼
Detect foreground app type
(Electron / VisualStudio / Standard)
│
▼
Read current keyboard layout (HKL)
│
▼
Copy selected text via Ctrl+C
│
▼
For each character in text:
VkKeyScanExW(char, src_layout)
│ → which key produces this char on source layout?
▼
MapVirtualKeyExW(vk, src_layout)
│ → convert to hardware scan code
▼
ToUnicodeEx(vk, scancode, dst_layout)
│ → what does that key produce on destination layout?
▼
Paste converted text via Ctrl+V
│
▼
Switch keyboard layout (if enabled)

---

## Configuration

All settings are saved automatically to:

```text
%LOCALAPPDATA%\Kalam\kalam_settings.json
```

| Setting         | Default       | Description                    |
| --------------- | ------------- | ------------------------------ |
| `hotkey`        | `Alt+Shift+Z` | Conversion hotkey              |
| `switch_layout` | `true`        | Switch layout after conversion |
| `autostart`     | `false`       | Start with Windows             |

---

## Limitations

- Requires at least **2 keyboard layouts** installed in Windows
- IME layouts (Chinese, Japanese, Korean) are not supported for conversion
- Some apps with custom text rendering may not support UIA — clipboard fallback is used

---

## License

MIT — see [LICENSE](https://arena.ai/c/LICENSE)

---

<div align="center">

Made with ❤️ and Rust

[github.com/Mr-EZAD/kalam](https://github.com/Mr-EZAD/kalam)

</div>
