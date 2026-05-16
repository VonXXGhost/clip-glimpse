use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::Foundation::*;
use windows::core::GUID;
use std::mem::size_of;

static NOTIFY_GUID: GUID = GUID::from_u128(0x3A2B1C0D_4E5F_6A7B_8C9D_0E1F2A3B4C5D);
static INITIALIZED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn ensure_init() {
    if !INITIALIZED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        unsafe {
            let _ = SetCurrentProcessExplicitAppUserModelID(
                &windows::core::HSTRING::from("ClipGlimpse.Notifications")
            );
        }
    }
}

pub fn show(title: &str, body: &str) {
    let title = title.to_owned();
    let body = body.to_owned();
    std::thread::Builder::new()
        .name("notify".into())
        .spawn(move || {
            ensure_init();

            let dummy_hwnd = unsafe {
                match CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    windows::core::w!("STATIC"),
                    windows::core::w!(""),
                    WINDOW_STYLE::default(),
                    0, 0, 0, 0,
                    HWND(-3isize as *mut _),
                    HMENU::default(),
                    None,
                    None,
                ) {
                    Ok(h) => h,
                    Err(_) => return,
                }
            };

            let icon = unsafe {
                match LoadIconW(None, IDI_APPLICATION) {
                    Ok(ico) => ico,
                    Err(_) => HICON::default(),
                }
            };

            let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
            nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = dummy_hwnd;
            nid.uID = 0;
            nid.uFlags = NIF_INFO | NIF_GUID | NIF_ICON;
            nid.guidItem = NOTIFY_GUID;
            nid.hIcon = icon;

            let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
            let title_len = title_wide.len().min(64);
            unsafe {
                std::ptr::copy_nonoverlapping(title_wide.as_ptr(), nid.szInfoTitle.as_mut_ptr(), title_len);
            }

            let body_wide: Vec<u16> = body.encode_utf16().chain(std::iter::once(0)).collect();
            let body_len = body_wide.len().min(256);
            unsafe {
                std::ptr::copy_nonoverlapping(body_wide.as_ptr(), nid.szInfo.as_mut_ptr(), body_len);
            }

            nid.dwInfoFlags = NIIF_INFO;
            nid.Anonymous.uTimeout = 5000;

            unsafe {
                if Shell_NotifyIconW(NIM_ADD, &nid).as_bool() {
                    std::thread::sleep(std::time::Duration::from_millis(6000));
                    let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
                } else {
                    let err = std::io::Error::last_os_error();
                    log_debug!("NOTIFY", "Shell_NotifyIconW failed: {} (kind: {:?})", err, err.kind());
                }
            }

            unsafe {
                let _ = DestroyWindow(dummy_hwnd);
            }
        });
}
