// Protocol module: chunk encode/decode for the wire protocol
pub const MAGIC: [u8; 2] = [0x43, 0x47];
pub const VERSION: u8 = 0x01;

pub const TYPE_SOS: u8 = 0x53;
pub const TYPE_DATA: u8 = 0x44;
pub const TYPE_EOS: u8 = 0x45;

pub const HEADER_SIZE: usize = 8;

pub const MAX_CHUNKS: u16 = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub chunk_type: u8,
    pub version: u8,
    pub seq: u16,
    pub total: u16,
    pub payload: Vec<u8>,
}

impl Chunk {
    pub fn new(chunk_type: u8, seq: u16, total: u16, payload: Vec<u8>) -> Self {
        Self { chunk_type, version: VERSION, seq, total, payload }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(HEADER_SIZE + self.payload.len());
        buf.extend_from_slice(&MAGIC);
        buf.push(self.chunk_type);
        buf.push(self.version);
        buf.extend_from_slice(&self.seq.to_be_bytes());
        buf.extend_from_slice(&self.total.to_be_bytes());
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
        let chunk_type = data[2];
        if chunk_type != TYPE_SOS && chunk_type != TYPE_DATA && chunk_type != TYPE_EOS {
            return None;
        }
        let version = data[3];
        if version != VERSION {
            return None;
        }
        let seq = u16::from_be_bytes([data[4], data[5]]);
        let total = u16::from_be_bytes([data[6], data[7]]);
        let payload = data[HEADER_SIZE..].to_vec();
        Some(Self { chunk_type, version, seq, total, payload })
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
    let text_bytes = text.as_bytes();
    let total_len = text_bytes.len();

    let data_chunks_count = if total_len == 0 {
        0
    } else {
        (total_len + max_payload_size - 1) / max_payload_size
    };

    let total_chunks = data_chunks_count + 2;

    let crc = compute_crc32(text_bytes);

    let mut chunks = Vec::with_capacity(total_chunks);

    chunks.push(Chunk::new(TYPE_SOS, 0, total_chunks as u16, crc.to_be_bytes().to_vec()));

    for i in 0..data_chunks_count {
        let start = i * max_payload_size;
        let end = (start + max_payload_size).min(total_len);
        let payload = text_bytes[start..end].to_vec();
        chunks.push(Chunk::new(TYPE_DATA, (i + 1) as u16, total_chunks as u16, payload));
    }

    if total_len > 0 {
        chunks.push(Chunk::new(TYPE_EOS, (total_chunks - 1) as u16, total_chunks as u16, crc.to_be_bytes().to_vec()));
    } else {
        chunks.push(Chunk::new(TYPE_EOS, (total_chunks - 1) as u16, total_chunks as u16, Vec::new()));
    }

    chunks
}

pub fn estimate_chunks(text_len: usize, max_payload_size: usize) -> usize {
    if text_len == 0 {
        return 2;
    }
    let data_chunks = (text_len + max_payload_size - 1) / max_payload_size;
    data_chunks + 2
}

#[derive(Debug)]
pub struct MessageAssembler {
    buffer: Vec<Option<Vec<u8>>>,
    total: u16,
    started: bool,
    completed: bool,
    sos_seq: u16,
    crc: Option<u32>,
}

impl MessageAssembler {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            total: 0,
            started: false,
            completed: false,
            sos_seq: 0,
            crc: None,
        }
    }

    pub fn feed(&mut self, chunk: &Chunk) -> Option<String> {
        if chunk.chunk_type == TYPE_SOS {
            // Ignore SOS if already assembling the same message,
            // so cycling (which shows SOS repeatedly) doesn't wipe progress.
            if self.started && self.total == chunk.total {
                log_debug!("PROTO", "Ignored SOS (already assembling total={})", chunk.total);
                return None;
            }
            self.buffer = vec![None; chunk.total as usize];
            self.total = chunk.total;
            self.started = true;
            self.completed = false;
            self.sos_seq = chunk.seq;
            if chunk.payload.len() >= 4 {
                self.crc = Some(u32::from_be_bytes([chunk.payload[0], chunk.payload[1], chunk.payload[2], chunk.payload[3]]));
            }
            self.buffer[chunk.seq as usize] = Some(Vec::new());
            return None;
        }

        if !self.started || chunk.total != self.total {
            return None;
        }

        let idx = chunk.seq as usize;
        if idx >= self.buffer.len() {
            return None;
        }

        if self.buffer[idx].is_some() {
            return None;
        }

        if chunk.chunk_type == TYPE_EOS {
            let all_data = (0..self.buffer.len())
                .all(|i| i == idx || self.buffer[i].is_some());
            if all_data {
                self.completed = true;
                let mut result = Vec::new();
                for entry in &self.buffer {
                    if let Some(data) = entry {
                        result.extend_from_slice(data);
                    }
                }
                let message = String::from_utf8_lossy(&result).to_string();

                if let Some(expected_crc) = self.crc {
                    let actual_crc = compute_crc32(message.as_bytes());
                    if actual_crc != expected_crc {
                        return None;
                    }
                }

                self.started = false;
                return Some(message);
            }
            return None;
        }

        if self.buffer[idx].is_some() {
            return None;
        }

        self.buffer[idx] = Some(chunk.payload.clone());
        None
    }

    pub fn is_active(&self) -> bool {
        self.started
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
        self.total = 0;
        self.started = false;
        self.completed = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_encode_decode() {
        let original = Chunk::new(TYPE_DATA, 42, 100, b"hello world".to_vec());
        let encoded = original.encode();
        let decoded = Chunk::decode(&encoded).unwrap();
        assert_eq!(original.chunk_type, decoded.chunk_type);
        assert_eq!(original.seq, decoded.seq);
        assert_eq!(original.total, decoded.total);
        assert_eq!(original.payload, decoded.payload);
    }

    #[test]
    fn test_invalid_magic() {
        let result = Chunk::decode(b"XX\x44\x01\x00\x01\x00\x02test");
        assert!(result.is_none());
    }

    #[test]
    fn test_encode_small_message() {
        let chunks = encode_message("hello", 100);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].chunk_type, TYPE_SOS);
        assert_eq!(chunks[1].chunk_type, TYPE_DATA);
        assert_eq!(chunks[2].chunk_type, TYPE_EOS);
    }

    #[test]
    fn test_encode_large_message() {
        let text = "a".repeat(1000);
        let chunks = encode_message(&text, 300);
        assert!(chunks.len() > 3);
        assert_eq!(chunks[0].chunk_type, TYPE_SOS);
        assert_eq!(chunks[chunks.len()-1].chunk_type, TYPE_EOS);
    }

    #[test]
    fn test_empty_message() {
        let chunks = encode_message("", 100);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].chunk_type, TYPE_SOS);
        assert_eq!(chunks[1].chunk_type, TYPE_EOS);
    }

    #[test]
    fn test_message_assembly() {
        let text = "Hello, ClipGlimpse!";
        let chunks = encode_message(text, 5);

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
    fn test_crc32() {
        assert_eq!(compute_crc32(b"hello"), 0x3610A686);
        assert_eq!(compute_crc32(b""), 0);
        assert_eq!(compute_crc32(b"abc"), 0x352441C2);
    }
}
