pub const MAGIC: [u8; 2] = [0x43, 0x47];
pub const VERSION: u8 = 0x02;

pub const FLAG_COMPRESSED: u8 = 0x01;
#[allow(dead_code)]
pub const FLAG_COLOR: u8 = 0x02;

pub const HEADER_SIZE: usize = 12;

pub const MAX_CHUNKS: u16 = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub version: u8,
    pub flags: u8,
    pub seq: u16,
    pub total: u16,
    pub crc32: u32,
    pub payload: Vec<u8>,
}

impl Chunk {
    pub fn new(seq: u16, total: u16, crc32: u32, flags: u8, payload: Vec<u8>) -> Self {
        Self { version: VERSION, flags, seq, total, crc32, payload }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(HEADER_SIZE + self.payload.len());
        buf.extend_from_slice(&MAGIC);
        buf.push(self.version);
        buf.push(self.flags);
        buf.extend_from_slice(&self.seq.to_be_bytes());
        buf.extend_from_slice(&self.total.to_be_bytes());
        buf.extend_from_slice(&self.crc32.to_be_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    pub fn decode(data: &[u8]) -> Option<Self> {
        if data.len() < HEADER_SIZE {
            return None;
        }
        if data[0] != MAGIC[0] || data[1] != MAGIC[1] {
            return None;
        }
        let version = data[2];
        if version != VERSION {
            return None;
        }
        let flags = data[3];
        let seq = u16::from_be_bytes([data[4], data[5]]);
        let total = u16::from_be_bytes([data[6], data[7]]);
        let crc32 = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
        let payload = data[HEADER_SIZE..].to_vec();
        Some(Self { version, flags, seq, total, crc32, payload })
    }
}

pub fn compute_crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFFFFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

pub fn encode_message(text: &str, max_payload_size: usize) -> Vec<Chunk> {
    if text.is_empty() {
        return vec![];
    }

    let original_bytes = text.as_bytes();
    let crc32 = compute_crc32(original_bytes);
    let compressed = lz4_flex::compress_prepend_size(original_bytes);
    let total_len = compressed.len();

    let data_chunks = (total_len + max_payload_size - 1) / max_payload_size;
    let total = data_chunks as u16;

    let mut chunks = Vec::with_capacity(data_chunks);
    for i in 0..data_chunks {
        let start = i * max_payload_size;
        let end = (start + max_payload_size).min(total_len);
        let payload = compressed[start..end].to_vec();
        chunks.push(Chunk::new(i as u16, total, crc32, FLAG_COMPRESSED, payload));
    }
    chunks
}

#[allow(dead_code)]
pub fn estimate_chunks(text_len: usize, max_payload_size: usize) -> usize {
    if text_len == 0 {
        return 0;
    }
    (text_len + max_payload_size - 1) / max_payload_size
}

#[derive(Debug)]
pub struct MessageAssembler {
    buffer: Vec<Option<Vec<u8>>>,
    total: u16,
    expected_crc: u32,
    flags: u8,
    filled_count: u16,
    active: bool,
}

impl MessageAssembler {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            total: 0,
            expected_crc: 0,
            flags: 0,
            filled_count: 0,
            active: false,
        }
    }

    pub fn feed(&mut self, chunk: &Chunk) -> Option<String> {
        if self.active {
            if chunk.total != self.total || chunk.crc32 != self.expected_crc {
                log_debug!("PROTO", "Message changed (total/crc diff), resetting assembler");
                self.reset();
            }
        }

        if !self.active {
            self.buffer = vec![None; chunk.total as usize];
            self.total = chunk.total;
            self.expected_crc = chunk.crc32;
            self.flags = chunk.flags;
            self.filled_count = 0;
            self.active = true;
        }

        let idx = chunk.seq as usize;
        if idx >= self.buffer.len() || self.buffer[idx].is_some() {
            return None;
        }

        self.buffer[idx] = Some(chunk.payload.clone());
        self.filled_count += 1;

        if self.filled_count == self.total {
            let mut raw = Vec::new();
            for entry in &self.buffer {
                raw.extend_from_slice(entry.as_ref().unwrap());
            }

            let data = if self.flags & FLAG_COMPRESSED != 0 {
                match lz4_flex::decompress_size_prepended(&raw) {
                    Ok(d) => d,
                    Err(e) => {
                        log_debug!("PROTO", "Decompression failed: {:?}", e);
                        self.reset();
                        return None;
                    }
                }
            } else {
                raw
            };

            let message = String::from_utf8_lossy(&data).to_string();

            if compute_crc32(message.as_bytes()) != self.expected_crc {
                self.reset();
                return None;
            }

            self.active = false;
            return Some(message);
        }

        None
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn filled_count(&self) -> u16 {
        self.filled_count
    }

    #[allow(dead_code)]
    pub fn total_chunks(&self) -> u16 {
        self.total
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
        self.total = 0;
        self.expected_crc = 0;
        self.flags = 0;
        self.filled_count = 0;
        self.active = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_encode_decode() {
        let original = Chunk::new(42, 100, 0xDEADBEEF, FLAG_COMPRESSED, b"hello world".to_vec());
        let encoded = original.encode();
        let decoded = Chunk::decode(&encoded).unwrap();
        assert_eq!(original.version, decoded.version);
        assert_eq!(original.flags, decoded.flags);
        assert_eq!(original.seq, decoded.seq);
        assert_eq!(original.total, decoded.total);
        assert_eq!(original.crc32, decoded.crc32);
        assert_eq!(original.payload, decoded.payload);
    }

    #[test]
    fn test_invalid_magic() {
        let result = Chunk::decode(b"XX\x02\x00\x00\x01\x00\x02\x00\x00\x00\x00test");
        assert!(result.is_none());
    }

    #[test]
    fn test_encode_small_message() {
        let chunks = encode_message("hello", 100);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].flags, FLAG_COMPRESSED);
        assert_eq!(chunks[0].total, 1);
        assert_eq!(chunks[0].seq, 0);
    }

    #[test]
    fn test_encode_multi_chunk() {
        let text = "a".repeat(5000);
        let chunks = encode_message(&text, 10);
        assert!(chunks.len() > 1);
        assert!(chunks.iter().all(|c| c.flags == FLAG_COMPRESSED));
        assert_eq!(chunks[0].total, chunks.len() as u16);
    }

    #[test]
    fn test_empty_message() {
        let chunks = encode_message("", 100);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_message_assembly() {
        let text = "Hello, ClipGlimpse!";
        let chunks = encode_message(text, 50);
        let mut assembler = MessageAssembler::new();
        let mut result = None;
        for chunk in &chunks {
            if let Some(msg) = assembler.feed(chunk) {
                result = Some(msg);
            }
        }
        assert_eq!(result, Some(text.to_string()));
    }

    #[test]
    fn test_message_assembly_large() {
        let text = "hello ".repeat(200);
        let chunks = encode_message(&text, 100);
        let mut assembler = MessageAssembler::new();
        let mut result = None;
        for chunk in &chunks {
            if let Some(msg) = assembler.feed(chunk) {
                result = Some(msg);
            }
        }
        assert_eq!(result, Some(text));
    }

    #[test]
    fn test_message_assembly_out_of_order() {
        let text = "Hello out of order!";
        let chunks = encode_message(text, 5);
        let total = chunks.len();

        let mut assembler = MessageAssembler::new();
        let mut result = None;
        for i in (0..total).rev() {
            if let Some(msg) = assembler.feed(&chunks[i]) {
                result = Some(msg);
                break;
            }
        }
        assert_eq!(result, Some(text.to_string()));
    }

    #[test]
    fn test_duplicate_during_assembly_ignored() {
        let text = "duplicate test with enough text to span multiple chunks ".repeat(5);
        let chunks = encode_message(&text, 20);
        assert!(chunks.len() >= 2);

        let mut assembler = MessageAssembler::new();
        assembler.feed(&chunks[0]);
        assert!(assembler.is_active());
        assert!(assembler.feed(&chunks[0]).is_none());
    }

    #[test]
    fn test_crc_mismatch_rejected() {
        let text = "crc mismatch test with longer text ".repeat(4);
        let mut chunks = encode_message(&text, 20);
        assert!(chunks.len() >= 2);

        if let Some(c) = chunks.first_mut() {
            if let Some(last) = c.payload.last_mut() {
                *last ^= 0xFF;
            }
        }

        let mut assembler = MessageAssembler::new();
        for chunk in &chunks {
            let _ = assembler.feed(chunk);
        }

        assert!(!assembler.is_active());

        let correct_chunks = encode_message(&text, 20);
        let mut assembler2 = MessageAssembler::new();
        let mut result = None;
        for chunk in &correct_chunks {
            if let Some(msg) = assembler2.feed(chunk) {
                result = Some(msg);
            }
        }
        assert_eq!(result, Some(text));
    }

    #[test]
    fn test_crc32() {
        assert_eq!(compute_crc32(b"hello"), 0x3610A686);
        assert_eq!(compute_crc32(b""), 0);
        assert_eq!(compute_crc32(b"abc"), 0x352441C2);
    }

    #[test]
    fn test_estimate_chunks() {
        assert_eq!(estimate_chunks(0, 100), 0);
        assert_eq!(estimate_chunks(50, 100), 1);
        assert_eq!(estimate_chunks(100, 100), 1);
        assert_eq!(estimate_chunks(101, 100), 2);
        assert_eq!(estimate_chunks(200, 100), 2);
    }

    #[test]
    fn test_decode_too_short() {
        assert!(Chunk::decode(b"CG\x02\x00").is_none());
    }

    #[test]
    fn test_decode_wrong_version() {
        let chunk = Chunk::new(1, 2, 0, FLAG_COMPRESSED, b"test".to_vec());
        let mut data = chunk.encode();
        data[2] = 0xFF;
        assert!(Chunk::decode(&data).is_none());
    }

    #[test]
    fn test_cyclic_consumption() {
        let text = "cyclic test ".repeat(20);
        let chunks = encode_message(&text, 10);
        let total = chunks.len();
        assert!(total >= 2);

        for start in 0..total {
            let mut assembler = MessageAssembler::new();
            let mut result = None;
            for offset in 0..total {
                let idx = (start + offset) % total;
                if let Some(msg) = assembler.feed(&chunks[idx]) {
                    result = Some(msg);
                    break;
                }
            }
            assert_eq!(result, Some(text.clone()), "Failed for start={}", start);
        }
    }

    #[test]
    fn test_message_change_resets_assembler() {
        let text1 = "first message with enough text for many chunks hooray ".repeat(4);
        let text2 = "second message with enough text for many chunks hooray ".repeat(4);
        let chunks1 = encode_message(&text1, 20);
        let chunks2 = encode_message(&text2, 20);
        assert!(chunks1.len() >= 2);
        assert!(chunks2.len() >= 2);

        let mut assembler = MessageAssembler::new();

        assembler.feed(&chunks1[0]);
        assert!(assembler.is_active());
        assert!(assembler.feed(&chunks1[1]).is_none());

        assert!(assembler.feed(&chunks2[0]).is_none());

        let mut result = None;
        for chunk in &chunks2 {
            if let Some(msg) = assembler.feed(chunk) {
                result = Some(msg);
            }
        }
        assert_eq!(result, Some(text2));
    }

    #[test]
    fn test_uncompressed_assembly() {
        let text = "raw data test";
        let crc32 = compute_crc32(text.as_bytes());
        let chunks = vec![Chunk::new(0, 1, crc32, 0, text.as_bytes().to_vec())];
        let mut assembler = MessageAssembler::new();
        assert_eq!(assembler.feed(&chunks[0]), Some(text.to_string()));
    }

    #[test]
    fn test_single_chunk_message() {
        let text = "single";
        let chunks = encode_message(text, 100);
        assert_eq!(chunks.len(), 1);
        let mut assembler = MessageAssembler::new();
        assert_eq!(assembler.feed(&chunks[0]), Some(text.to_string()));
    }
}
