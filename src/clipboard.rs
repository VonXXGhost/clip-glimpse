use windows::Win32::System::DataExchange::*;
use windows::Win32::System::Memory::*;
use windows::Win32::Foundation::*;

const CF_UNICODETEXT: u32 = 13;

pub fn set_text(text: &str) -> Result<(), String> {
    unsafe {
        if OpenClipboard(HWND::default()).is_err() {
            return Err("OpenClipboard failed".to_string());
        }

        let _ = EmptyClipboard();

        let wide: Vec<u16> = text.encode_utf16().collect();
        let bytes = (wide.len() + 1) * 2;

        let handle = match GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, bytes) {
            Ok(h) => h,
            Err(e) => {
                let _ = CloseClipboard();
                return Err(format!("GlobalAlloc failed: {}", e));
            }
        };

        let ptr = GlobalLock(handle);
        if ptr.is_null() {
            let _ = CloseClipboard();
            return Err("GlobalLock failed".to_string());
        }

        std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr as *mut u16, wide.len());

        let _ = GlobalUnlock(handle);

        if SetClipboardData(CF_UNICODETEXT, HANDLE(handle.0 as *mut _)).is_err() {
            let _ = CloseClipboard();
            return Err("SetClipboardData failed".to_string());
        }

        let _ = CloseClipboard();
        Ok(())
    }
}
