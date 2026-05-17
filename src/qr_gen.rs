use image::{RgbImage, Rgb};
use qrcode::{QrCode, EcLevel, Version, types::Color};

pub struct QrGenParams {
    pub version: Version,
    pub ec_level: EcLevel,
    pub module_size_px: u32,
}

impl Default for QrGenParams {
    fn default() -> Self {
        Self {
            version: Version::Normal(25),
            ec_level: EcLevel::M,
            module_size_px: 3,
        }
    }
}

pub fn generate_qr(data: &[u8], params: &QrGenParams) -> Option<RgbImage> {
    let code = QrCode::with_version(data, params.version, params.ec_level).ok()?;
    let modules = code.width();

    let border = params.module_size_px as usize;
    let full_size = modules * params.module_size_px as usize + border * 2;
    let mut full_img = RgbImage::new(full_size as u32, full_size as u32);

    for pixel in full_img.pixels_mut() {
        *pixel = Rgb([255, 255, 255]);
    }

    for y in 0..modules {
        for x in 0..modules {
            let is_dark = code[(x, y)] == Color::Dark;
            let color = if is_dark {
                Rgb([0, 0, 0])
            } else {
                Rgb([255, 255, 255])
            };
            for dy in 0..params.module_size_px {
                for dx in 0..params.module_size_px {
                    let px = (x as u32 * params.module_size_px + dx) as usize + border;
                    let py = (y as u32 * params.module_size_px + dy) as usize + border;
                    if px < full_size && py < full_size {
                        full_img.put_pixel(px as u32, py as u32, color);
                    }
                }
            }
        }
    }

    Some(full_img)
}

pub fn generate_color_qr(group: &[Vec<u8>], params: &QrGenParams) -> Option<RgbImage> {
    if group.is_empty() {
        return None;
    }

    let count = group.len().min(3);
    let mut codes: [Option<QrCode>; 3] = [None, None, None];
    let mut modules = 0usize;

    for i in 0..count {
        match QrCode::with_version(&group[i], params.version, params.ec_level) {
            Ok(code) => {
                if modules == 0 {
                    modules = code.width();
                }
                codes[i] = Some(code);
            }
            Err(e) => {
                log_debug!("GEN_COLOR", "Channel {} QrCode::with_version failed: {:?}", i, e);
            }
        }
    }

    if codes.iter().all(|c| c.is_none()) {
        return None;
    }
    if modules == 0 {
        return None;
    }

    let border = params.module_size_px as usize;
    let full_size = modules * params.module_size_px as usize + border * 2;
    let mut full_img = RgbImage::new(full_size as u32, full_size as u32);

    for pixel in full_img.pixels_mut() {
        *pixel = Rgb([255, 255, 255]);
    }

    for y in 0..modules {
        for x in 0..modules {
            let mut channels = [255u8; 3];
            for (i, code) in codes.iter().enumerate() {
                if let Some(ref code) = code {
                    if code[(x, y)] == Color::Dark {
                        channels[i] = 0;
                    }
                }
            }
            let color = Rgb(channels);
            for dy in 0..params.module_size_px {
                for dx in 0..params.module_size_px {
                    let px = (x as u32 * params.module_size_px + dx) as usize + border;
                    let py = (y as u32 * params.module_size_px + dy) as usize + border;
                    if px < full_size && py < full_size {
                        full_img.put_pixel(px as u32, py as u32, color);
                    }
                }
            }
        }
    }

    Some(full_img)
}

pub fn qr_to_egui_color_image(img: &RgbImage) -> egui::ColorImage {
    let size = [img.width() as usize, img.height() as usize];
    let pixels = img
        .pixels()
        .map(|p| egui::Color32::from_rgb(p[0], p[1], p[2]))
        .collect();
    egui::ColorImage { size, pixels }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_qr_default() {
        let params = QrGenParams::default();
        let img = generate_qr(b"test data", &params);
        assert!(img.is_some());
        let img = img.unwrap();
        assert!(img.width() > 0);
        assert!(img.height() > 0);
    }

    #[test]
    fn test_generate_qr_different_params() {
        let params = QrGenParams {
            version: Version::Normal(20),
            ec_level: EcLevel::Q,
            module_size_px: 4,
        };
        let img = generate_qr(b"hello qr", &params);
        assert!(img.is_some());
    }

    #[test]
    fn test_generate_color_qr_single() {
        let params = QrGenParams::default();
        let img = generate_color_qr(&[b"chunk 1 data".to_vec()], &params);
        assert!(img.is_some());
    }

    #[test]
    fn test_generate_color_qr_three() {
        let params = QrGenParams::default();
        let img = generate_color_qr(
            &[b"red channel".to_vec(), b"green channel".to_vec(), b"blue channel".to_vec()],
            &params,
        );
        assert!(img.is_some());
    }

    #[test]
    fn test_generate_color_qr_empty() {
        let params = QrGenParams::default();
        assert!(generate_color_qr(&[], &params).is_none());
    }

    #[test]
    fn test_color_qr_same_size_as_bw() {
        let params = QrGenParams::default();
        let bw = generate_qr(b"test", &params).unwrap();
        let color = generate_color_qr(&[b"test".to_vec()], &params).unwrap();
        assert_eq!(bw.width(), color.width());
        assert_eq!(bw.height(), color.height());
    }
}
