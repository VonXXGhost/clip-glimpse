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
}
