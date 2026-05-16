use rxing::{
    common::HybridBinarizer,
    BinaryBitmap, MultiFormatReader, Reader,
    PlanarYUVLuminanceSource,
};

pub fn decode_qr(gray_data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    let source = PlanarYUVLuminanceSource::new_with_all(
        gray_data.to_vec(),
        width as usize,
        height as usize,
        0,
        0,
        width as usize,
        height as usize,
        false,
        false,
    ).map_err(|e| format!("Failed to create luminance source: {}", e))?;

    let binarizer = HybridBinarizer::new(source);
    let mut bitmap = BinaryBitmap::new(binarizer);
    let mut reader = MultiFormatReader::default();

    match reader.decode(&mut bitmap) {
        Ok(result) => Ok(result.getRawBytes().to_vec()),
        Err(e) => Err(format!("QR decode failed: {}", e)),
    }
}

pub fn convert_bgra_to_gray(bgra: &[u8]) -> Vec<u8> {
    bgra.chunks(4).map(|p| {
        let b = p[0] as f32;
        let g = p[1] as f32;
        let r = p[2] as f32;
        (0.299 * r + 0.587 * g + 0.114 * b) as u8
    }).collect()
}

pub fn convert_rgb_to_gray(rgb: &[u8]) -> Vec<u8> {
    rgb.chunks(3).map(|p| {
        let r = p[0] as f32;
        let g = p[1] as f32;
        let b = p[2] as f32;
        (0.299 * r + 0.587 * g + 0.114 * b) as u8
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qr_gen::{self, QrGenParams};

    #[test]
    fn test_roundtrip() {
        let params = QrGenParams::default();
        let img = qr_gen::generate_qr(b"ClipGlimpse roundtrip test", &params).unwrap();

        let gray = convert_rgb_to_gray(&img);
        let width = img.width();
        let height = img.height();

        let result = decode_qr(&gray, width, height);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), b"ClipGlimpse roundtrip test");
    }

    #[test]
    fn test_convert_bgra_to_gray() {
        let bgra = vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255];
        let gray = convert_bgra_to_gray(&bgra);
        assert_eq!(gray.len(), 3);
    }
}
