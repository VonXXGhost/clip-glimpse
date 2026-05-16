use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState,
};

pub const VK_CONTROL: u32 = 0x11;
pub const VK_SHIFT: u32 = 0x10;
pub const VK_ALT: u32 = 0x12;

pub fn is_key_down(virtual_key_code: u32) -> bool {
    unsafe { GetAsyncKeyState(virtual_key_code as i32) < 0 }
}

pub const HOTKEY_ALT: u32 = 1;
pub const HOTKEY_CTRL: u32 = 2;
pub const HOTKEY_SHIFT: u32 = 4;

pub fn is_modifier_down(modifier: u32) -> bool {
    if modifier & HOTKEY_ALT != 0 && !is_key_down(VK_ALT) { return false; }
    if modifier & HOTKEY_CTRL != 0 && !is_key_down(VK_CONTROL) { return false; }
    if modifier & HOTKEY_SHIFT != 0 && !is_key_down(VK_SHIFT) { return false; }
    true
}

pub fn is_hotkey_pressed(modifiers: u32, vk: u32) -> bool {
    if !is_modifier_down(modifiers) {
        return false;
    }
    is_key_down(vk)
}

fn parse_key_name(s: &str) -> Option<u32> {
    let lower = s.to_lowercase();
    match lower.as_str() {
        "backspace" => Some(0x08),
        "tab" => Some(0x09),
        "enter" => Some(0x0D),
        "escape" | "esc" => Some(0x1B),
        "space" => Some(0x20),
        "delete" | "del" => Some(0x2E),
        "numpad0" => Some(0x60),
        "numpad1" => Some(0x61),
        "numpad2" => Some(0x62),
        "numpad3" => Some(0x63),
        "numpad4" => Some(0x64),
        "numpad5" => Some(0x65),
        "numpad6" => Some(0x66),
        "numpad7" => Some(0x67),
        "numpad8" => Some(0x68),
        "numpad9" => Some(0x69),
        "numlock" => Some(0x90),
        "scrolllock" | "scroll" => Some(0x91),
        _ => {
            if let Some(n) = lower.strip_prefix('f') {
                if let Ok(num) = n.parse::<u32>() {
                    if (1..=24).contains(&num) {
                        return Some(0x70 + num - 1);
                    }
                }
            }
            if lower.len() == 1 {
                let c = lower.chars().next().unwrap();
                if c.is_ascii_digit() {
                    return Some(0x30 + (c as u32 - '0' as u32));
                }
                if c.is_ascii_uppercase() || c.is_ascii_lowercase() {
                    return Some(c.to_ascii_uppercase() as u32);
                }
            }
            None
        }
    }
}

pub fn parse_hotkey(s: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return None;
    }
    let mut modifiers: u32 = 0;
    let mut key: Option<u32> = None;
    for part in parts {
        let lower = part.to_lowercase();
        match lower.as_str() {
            "ctrl" | "control" => modifiers |= HOTKEY_CTRL,
            "alt" => modifiers |= HOTKEY_ALT,
            "shift" => modifiers |= HOTKEY_SHIFT,
            other => {
                if let Some(code) = parse_key_name(other) {
                    key = Some(code);
                } else {
                    return None;
                }
            }
        }
    }
    key.map(|k| (modifiers, k))
}

pub fn normalize_hotkey(s: &str) -> String {
    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).filter(|p| !p.is_empty()).collect();
    let mut out: Vec<String> = Vec::new();
    for part in parts {
        let lower = part.to_lowercase();
        match lower.as_str() {
            "ctrl" | "control" => out.push("Ctrl".into()),
            "alt" => out.push("Alt".into()),
            "shift" => out.push("Shift".into()),
            other => {
                if let Some(vk) = parse_key_name(other) {
                    out.push(match vk {
                        0x08 => "Backspace".into(),
                        0x09 => "Tab".into(),
                        0x0D => "Enter".into(),
                        0x1B => "Escape".into(),
                        0x20 => "Space".into(),
                        0x2E => "Delete".into(),
                        0x60..=0x69 => format!("Numpad{}", vk - 0x60),
                        0x70..=0x7B => format!("F{}", vk - 0x70 + 1),
                        0x90 => "NumLock".into(),
                        0x91 => "ScrollLock".into(),
                        _ => ((vk as u8) as char).to_ascii_uppercase().to_string(),
                    });
                } else {
                    out.push(part.to_string());
                }
            }
        }
    }
    out.join("+")
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_default() {
        let (mods, vk) = parse_hotkey("Ctrl+Shift+V").unwrap();
        assert_eq!(mods, HOTKEY_CTRL | HOTKEY_SHIFT);
        assert_eq!(vk, 0x56);
    }

    #[test]
    fn test_parse_lowercase() {
        let (mods, vk) = parse_hotkey("ctrl+shift+v").unwrap();
        assert_eq!(mods, HOTKEY_CTRL | HOTKEY_SHIFT);
        assert_eq!(vk, 0x56);
    }

    #[test]
    fn test_parse_alt_c() {
        let (mods, vk) = parse_hotkey("Alt+C").unwrap();
        assert_eq!(mods, HOTKEY_ALT);
        assert_eq!(vk, 0x43);
    }

    #[test]
    fn test_parse_ctrl_f5() {
        let (mods, vk) = parse_hotkey("Ctrl+F5").unwrap();
        assert_eq!(mods, HOTKEY_CTRL);
        assert_eq!(vk, 0x74);
    }

    #[test]
    fn test_parse_ctrl_shift_3() {
        let (mods, vk) = parse_hotkey("Ctrl+Shift+3").unwrap();
        assert_eq!(mods, HOTKEY_CTRL | HOTKEY_SHIFT);
        assert_eq!(vk, 0x33);
    }

    #[test]
    fn test_parse_space() {
        let (mods, vk) = parse_hotkey("Ctrl+Space").unwrap();
        assert_eq!(mods, HOTKEY_CTRL);
        assert_eq!(vk, 0x20);
    }

    #[test]
    fn test_parse_invalid() {
        assert!(parse_hotkey("").is_none());
        assert!(parse_hotkey("Ctrl+??").is_none());
        assert!(parse_hotkey("Ctrl+Shift+LongKey").is_none());
    }

    #[test]
    fn test_normalize() {
        assert_eq!(normalize_hotkey("ctrl+shift+v"), "Ctrl+Shift+V");
        assert_eq!(normalize_hotkey("ALT+C"), "Alt+C");
        assert_eq!(normalize_hotkey("control+space"), "Ctrl+Space");
    }
}
