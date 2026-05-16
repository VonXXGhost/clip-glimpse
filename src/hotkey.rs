use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_CONTROL,
};

pub const HOTKEY_ID: i32 = 1;
pub const VK_V: u32 = 0x56;
pub const VK_CONTROL: u32 = 0x11;
pub const VK_SHIFT: u32 = 0x10;
pub const VK_ALT: u32 = 0x12;

pub fn register(hotkey_id: i32, modifiers: HOT_KEY_MODIFIERS, vk: u32) -> Result<(), String> {
    unsafe {
        RegisterHotKey(None, hotkey_id, modifiers, vk).map_err(|e| {
            format!("Failed to register hotkey (id={}, vk=0x{:X}): {}", hotkey_id, vk, e)
        })
    }
}

pub fn unregister(hotkey_id: i32) -> Result<(), String> {
    unsafe {
        UnregisterHotKey(None, hotkey_id).map_err(|e| {
            format!("Failed to unregister hotkey {}: {}", hotkey_id, e)
        })
    }
}

pub fn is_key_down(virtual_key_code: u32) -> bool {
    unsafe { GetAsyncKeyState(virtual_key_code as i32) < 0 }
}

pub fn is_ctrl_shift_v_pressed() -> bool {
    is_key_down(VK_CONTROL) && is_key_down(VK_SHIFT) && is_key_down(VK_V)
}

pub fn get_mod_ctrl() -> HOT_KEY_MODIFIERS {
    MOD_CONTROL
}
