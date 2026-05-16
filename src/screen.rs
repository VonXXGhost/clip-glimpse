use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits,
    GetDC, GetDeviceCaps, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER,
    BI_RGB, DIB_RGB_COLORS, SRCCOPY, DESKTOPHORZRES, DESKTOPVERTRES,
};
use windows::Win32::UI::WindowsAndMessaging::GetDesktopWindow;
use std::mem;

#[derive(Debug, Clone, Copy)]
pub struct CaptureRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl CaptureRegion {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    #[allow(dead_code)]
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x
            && x < self.x + self.width as i32
            && y >= self.y
            && y < self.y + self.height as i32
    }
}

pub fn get_screen_size() -> Result<(u32, u32), String> {
    unsafe {
        let hdc = GetDesktopWindow();
        let dc = GetDC(hdc);
        if dc.is_invalid() {
            return Err("Failed to get screen DC".to_string());
        }
        let w = GetDeviceCaps(dc, DESKTOPHORZRES);
        let h = GetDeviceCaps(dc, DESKTOPVERTRES);
        let _ = ReleaseDC(hdc, dc);
        Ok((w as u32, h as u32))
    }
}

pub fn capture_region(region: &CaptureRegion) -> Result<Vec<u8>, String> {
    unsafe {
        let width = region.width as i32;
        let height = region.height as i32;

        let desktop_hwnd = GetDesktopWindow();
        let hdc_screen = GetDC(desktop_hwnd);
        if hdc_screen.is_invalid() {
            return Err("Failed to get screen DC".to_string());
        }

        let hdc_mem = CreateCompatibleDC(hdc_screen);
        if hdc_mem.is_invalid() {
            let _ = ReleaseDC(desktop_hwnd, hdc_screen);
            return Err("Failed to create compatible DC".to_string());
        }

        let hbitmap = CreateCompatibleBitmap(hdc_screen, width, height);
        if hbitmap.is_invalid() {
            let _ = DeleteDC(hdc_mem);
            let _ = ReleaseDC(desktop_hwnd, hdc_screen);
            return Err("Failed to create compatible bitmap".to_string());
        }

        SelectObject(hdc_mem, hbitmap);

        if let Err(e) = BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, region.x, region.y, SRCCOPY) {
            let _ = DeleteObject(hbitmap);
            let _ = DeleteDC(hdc_mem);
            let _ = ReleaseDC(desktop_hwnd, hdc_screen);
            return Err(format!("BitBlt failed: {}", e));
        }

        let mut bmi: BITMAPINFO = mem::zeroed();
        bmi.bmiHeader.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = width;
        bmi.bmiHeader.biHeight = -height;
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB.0;

        let pixel_count = (width * height) as usize;
        let mut pixels = vec![0u8; pixel_count * 4];

        let result = GetDIBits(
            hdc_mem,
            hbitmap,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        let _ = DeleteObject(hbitmap);
        let _ = DeleteDC(hdc_mem);
        let _ = ReleaseDC(desktop_hwnd, hdc_screen);

        if result == 0 {
            return Err("GetDIBits failed".to_string());
        }

        Ok(pixels)
    }
}
