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

#[derive(Debug, Clone, Copy)]
pub enum Channel {
    B,
    G,
    R,
}

pub fn extract_channel_from_bgra(bgra: &[u8], channel: Channel) -> Vec<u8> {
    let idx = match channel {
        Channel::B => 0,
        Channel::G => 1,
        Channel::R => 2,
    };
    bgra.chunks(4).map(|p| p[idx]).collect()
}

pub fn stretch_contrast(gray: &[u8]) -> Vec<u8> {
    let (min, max) = match gray.iter().fold((None, None), |(mn, mx), &v| {
        (Some(mn.map_or(v, |m: u8| m.min(v))), Some(mx.map_or(v, |m: u8| m.max(v))))
    }) {
        (Some(min), Some(max)) => (min, max),
        _ => return gray.to_vec(),
    };
    if max <= min || max == 255 || min == 0 {
        return gray.to_vec();
    }
    let range = (max - min) as u32;
    gray.iter().map(|&v| (((v as u32 - min as u32) * 255) / range) as u8).collect()
}

pub fn convert_bgra_to_gray(bgra: &[u8]) -> Vec<u8> {
    bgra.chunks(4).map(|p| {
        let b = p[0] as f32;
        let g = p[1] as f32;
        let r = p[2] as f32;
        (0.299 * r + 0.587 * g + 0.114 * b) as u8
    }).collect()
}

#[allow(dead_code)]
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

    #[test]
    fn test_extract_channel() {
        let bgra = vec![
            255, 0, 0, 255,
            0, 255, 0, 255,
            0, 0, 255, 255,
        ];
        let r = extract_channel_from_bgra(&bgra, Channel::R);
        let g = extract_channel_from_bgra(&bgra, Channel::G);
        let b = extract_channel_from_bgra(&bgra, Channel::B);
        assert_eq!(r, vec![0, 0, 255]);
        assert_eq!(g, vec![0, 255, 0]);
        assert_eq!(b, vec![255, 0, 0]);
    }

    #[test]
    fn test_color_qr_roundtrip() {
        use crate::qr_gen::{self, QrGenParams};
        let params = QrGenParams::default();
        let groups = vec![
            b"red data".to_vec(),
            b"green data".to_vec(),
            b"blue data".to_vec(),
        ];
        let img = qr_gen::generate_color_qr(&groups, &params).unwrap();
        let width = img.width();
        let height = img.height();

        let rgb = img.into_raw();
        let mut bgra = Vec::with_capacity(rgb.len() / 3 * 4);
        for pixel in rgb.chunks(3) {
            bgra.push(pixel[2]);
            bgra.push(pixel[1]);
            bgra.push(pixel[0]);
            bgra.push(255);
        }

        let r_src = extract_channel_from_bgra(&bgra, Channel::R);
        let g_src = extract_channel_from_bgra(&bgra, Channel::G);
        let b_src = extract_channel_from_bgra(&bgra, Channel::B);

        let r_result = decode_qr(&r_src, width, height);
        let g_result = decode_qr(&g_src, width, height);
        let b_result = decode_qr(&b_src, width, height);

        assert!(r_result.is_ok());
        assert!(g_result.is_ok());
        assert!(b_result.is_ok());
        assert_eq!(r_result.unwrap(), b"red data");
        assert_eq!(g_result.unwrap(), b"green data");
        assert_eq!(b_result.unwrap(), b"blue data");
    }

    #[test]
    fn test_stretch_contrast_noop() {
        let data = vec![0, 128, 255];
        let result = stretch_contrast(&data);
        assert_eq!(result, vec![0, 128, 255]);
    }

    #[test]
    fn test_stretch_contrast_shifted() {
        let data = vec![30, 100, 200];
        let result = stretch_contrast(&data);
        assert_eq!(result[0], 0);
        assert_eq!(result[2], 255);
        assert!(result[1] > 0 && result[1] < 255);
    }

    #[test]
    fn test_stretch_contrast_flat() {
        let data = vec![128, 128, 128];
        let result = stretch_contrast(&data);
        assert_eq!(result, vec![128, 128, 128]);
    }

    #[test]
    fn test_color_qr_roundtrip_shifted() {
        use crate::qr_gen::{self, QrGenParams};
        let params = QrGenParams::default();
        let groups = vec![
            b"red channel shifted".to_vec(),
            b"green channel shifted".to_vec(),
            b"blue channel shifted".to_vec(),
        ];
        let img = qr_gen::generate_color_qr(&groups, &params).unwrap();
        let width = img.width();
        let height = img.height();

        let rgb = img.into_raw();
        let mut bgra = Vec::with_capacity(rgb.len() / 3 * 4);
        for pixel in rgb.chunks(3) {
            bgra.push(pixel[2]);
            bgra.push(pixel[1]);
            bgra.push(pixel[0]);
            bgra.push(255);
        }

        // simulate color shift: compress values into [32..=224]
        for v in bgra.iter_mut() {
            *v = ((*v as f32 / 255.0) * 192.0 + 32.0) as u8;
        }

        let r_src = extract_channel_from_bgra(&bgra, Channel::R);
        let g_src = extract_channel_from_bgra(&bgra, Channel::G);
        let b_src = extract_channel_from_bgra(&bgra, Channel::B);

        let r_stretched = stretch_contrast(&r_src);
        let g_stretched = stretch_contrast(&g_src);
        let b_stretched = stretch_contrast(&b_src);

        let r_result = decode_qr(&r_stretched, width, height);
        let g_result = decode_qr(&g_stretched, width, height);
        let b_result = decode_qr(&b_stretched, width, height);

        assert!(r_result.is_ok(), "R decode failed after stretch");
        assert!(g_result.is_ok(), "G decode failed after stretch");
        assert!(b_result.is_ok(), "B decode failed after stretch");
        assert_eq!(r_result.unwrap(), b"red channel shifted");
        assert_eq!(g_result.unwrap(), b"green channel shifted");
        assert_eq!(b_result.unwrap(), b"blue channel shifted");
    }

    #[test]
    fn test_convert_rgb_to_gray() {
        let rgb = vec![0, 0, 0, 255, 255, 255];
        let gray = convert_rgb_to_gray(&rgb);
        assert_eq!(gray.len(), 2);
        assert_eq!(gray[0], 0);
        assert_eq!(gray[1], 255);
    }
}
