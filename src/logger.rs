use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;

static LOGGER: Mutex<Option<std::fs::File>> = Mutex::new(None);

fn ensure_init() {
    let mut guard = LOGGER.lock().unwrap();
    if guard.is_none() {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("clip_glimpse.log")
            .expect("Failed to open log file");
        *guard = Some(file);
    }
}

pub fn log_msg(tag: &str, msg: &str) {
    ensure_init();
    let now = chrono::Local::now();
    let line = format!("[{}] [{}] {}\n", now.format("%H:%M:%S%.3f"), tag, msg);
    if let Ok(mut guard) = LOGGER.lock() {
        if let Some(ref mut f) = *guard {
            let _ = f.write_all(line.as_bytes());
            let _ = f.flush();
        }
    }
}

#[macro_export]
macro_rules! log_debug {
    ($tag:expr, $($arg:tt)+) => {
        $crate::logger::log_msg($tag, &format!($($arg)+));
    };
}
