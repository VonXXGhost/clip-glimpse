use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::time::{Duration, Instant};
use std::thread;

use crate::protocol::{Chunk, MessageAssembler};
use crate::qr_read;
use crate::screen::{self, CaptureRegion};
use crate::history::History;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanState {
    Idle,
    Scanning,
}

#[derive(Debug, Clone, Copy)]
pub struct ScanConfig {
    pub interval_ms: u64,
    pub timeout_secs: u64,
}

impl ScanConfig {
    pub fn new(interval_ms: u64) -> Self {
        Self {
            interval_ms,
            timeout_secs: 30,
        }
    }
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            interval_ms: 200,
            timeout_secs: 30,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScanStats {
    pub frames_captured: u64,
    pub frames_decoded: u64,
    pub chunks_received: u64,
    pub messages_completed: u64,
    pub last_message_time: Option<String>,
    pub current_total_chunks: u16,
    pub current_received_data_chunks: u16,
    pub current_message_active: bool,
}

impl Default for ScanStats {
    fn default() -> Self {
        Self {
            frames_captured: 0,
            frames_decoded: 0,
            chunks_received: 0,
            messages_completed: 0,
            last_message_time: None,
            current_total_chunks: 0,
            current_received_data_chunks: 0,
            current_message_active: false,
        }
    }
}

pub struct Scanner {
    region: CaptureRegion,
    config: ScanConfig,
    history: Arc<Mutex<History>>,
    scan_state: Arc<Mutex<ScanState>>,
    stats: Arc<Mutex<ScanStats>>,
    running: Arc<AtomicBool>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

unsafe impl Send for Scanner {}
unsafe impl Sync for Scanner {}

impl Scanner {
    pub fn new(
        region: CaptureRegion,
        history: Arc<Mutex<History>>,
        scan_state: Arc<Mutex<ScanState>>,
        scan_interval_ms: u64,
    ) -> Self {
        Self {
            region,
            config: ScanConfig::new(scan_interval_ms),
            history,
            scan_state,
            stats: Arc::new(Mutex::new(ScanStats::default())),
            running: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
        }
    }

    pub fn stats(&self) -> Arc<Mutex<ScanStats>> {
        self.stats.clone()
    }

    pub fn start(&mut self) {
        if self.running.load(Ordering::SeqCst) {
            log_debug!("SCAN", "start() called but already running");
            return;
        }

        log_debug!("SCAN", "Scanner thread starting, region=({},{}) {}x{}",
            self.region.x, self.region.y, self.region.width, self.region.height);
        self.running.store(true, Ordering::SeqCst);

        let region = self.region;
        let config = self.config;
        let history = self.history.clone();
        let scan_state = self.scan_state.clone();
        let stats = self.stats.clone();
        let running = self.running.clone();

        let handle = thread::Builder::new()
            .name("scanner".into())
            .spawn(move || {
                let mut assembler = MessageAssembler::new();
                let mut last_scan = Instant::now();
                let mut sos_time: Option<Instant> = None;

                while running.load(Ordering::SeqCst) {
                    let is_scanning = *scan_state.lock().unwrap() == ScanState::Scanning;

                    if !is_scanning {
                        thread::sleep(Duration::from_millis(50));
                        continue;
                    }

                    let now = Instant::now();
                    if now.duration_since(last_scan) < Duration::from_millis(config.interval_ms) {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    last_scan = now;

                    let pixels = match screen::capture_region(&region) {
                        Ok(p) => p,
                        Err(e) => {
                            log_debug!("SCAN", "Capture failed: {}", e);
                            continue;
                        },
                    };

                    {
                        let mut s = stats.lock().unwrap();
                        s.frames_captured += 1;
                    }

                    let gray = qr_read::convert_bgra_to_gray(&pixels);

                    let decoded = match qr_read::decode_qr(&gray, region.width, region.height) {
                        Ok(d) => d,
                        Err(_) => {
                            log_debug!("SCAN", "QR decode failed");
                            continue;
                        },
                    };

                    {
                        let mut s = stats.lock().unwrap();
                        s.frames_decoded += 1;
                    }

                    let chunk = match Chunk::decode(&decoded) {
                        Some(c) => c,
                        None => {
                            log_debug!("SCAN", "Chunk::decode failed (bad magic/type/version), raw_len={}", decoded.len());
                            continue;
                        },
                    };

                    let type_str = if chunk.chunk_type == crate::protocol::TYPE_SOS { "SOS" }
                        else if chunk.chunk_type == crate::protocol::TYPE_DATA { "DATA" }
                        else if chunk.chunk_type == crate::protocol::TYPE_EOS { "EOS" }
                        else { "???" };
                    log_debug!("SCAN", "Chunk {} seq={} total={} payload_len={}",
                        type_str, chunk.seq, chunk.total, chunk.payload.len());

                    {
                        let mut s = stats.lock().unwrap();
                        s.chunks_received += 1;
                    }

                    if chunk.chunk_type == crate::protocol::TYPE_SOS {
                        sos_time = Some(Instant::now());
                        let mut s = stats.lock().unwrap();
                        s.current_total_chunks = chunk.total;
                        s.current_received_data_chunks = 0;
                        s.current_message_active = true;
                    } else if chunk.chunk_type == crate::protocol::TYPE_DATA {
                        let mut s = stats.lock().unwrap();
                        s.current_received_data_chunks =
                            (s.current_received_data_chunks + 1).min(s.current_total_chunks);
                    }

                    if let Some(start) = sos_time {
                        if start.elapsed() > Duration::from_secs(config.timeout_secs) {
                            log_debug!("SCAN", "SOS timeout, resetting assembler");
                            assembler.reset();
                            sos_time = None;
                            let mut s = stats.lock().unwrap();
                            s.current_message_active = false;
                            continue;
                        }
                    }

                    if let Some(message) = assembler.feed(&chunk) {
                        log_debug!("SCAN", "Message completed! len={}", message.len());

                        // Auto-copy to clipboard
                        if let Err(e) = crate::clipboard::set_text(&message) {
                            log_debug!("CLIP", "Failed to set clipboard: {}", e);
                        } else {
                            log_debug!("CLIP", "Copied message to clipboard ({} bytes)", message.len());
                        }

                        // System notification
                        crate::notify::show("ClipGlimpse", "Message received and copied to clipboard");

                        let mut h = history.lock().unwrap();
                        h.add(message);
                        drop(h);

                        sos_time = None;

                        let mut s = stats.lock().unwrap();
                        s.messages_completed += 1;
                        s.last_message_time = Some(
                            chrono::Local::now().format("%H:%M:%S").to_string()
                        );
                        s.current_message_active = false;
                        drop(s);

                        // Auto-stop scanning after a complete message is received
                        *scan_state.lock().unwrap() = ScanState::Idle;
                        log_debug!("SCAN", "Scanning auto-stopped after message completion");
                    } else {
                        log_debug!("SCAN", "feed returned None (duplicate or incomplete)");
                    }
                }
            });

        match handle {
            Ok(h) => self.thread_handle = Some(h),
            Err(e) => eprintln!("Failed to start scanner thread: {}", e),
        }
    }

    pub fn stop(&mut self) {
        log_debug!("SCAN", "Scanner stopping");
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
        log_debug!("SCAN", "Scanner stopped");
    }
}

impl Drop for Scanner {
    fn drop(&mut self) {
        self.stop();
    }
}
