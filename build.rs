use std::fs::File;
use std::io::Write;

fn main() {
    let ico_path = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").unwrap(),
    )
    .join("target")
    .join(std::env::var("PROFILE").unwrap())
    .join("clip_glimpse_icon.ico");

    generate_ico(&ico_path);

    let mut res = winresource::WindowsResource::new();
    res.set_icon(&ico_path.to_string_lossy());
    res.compile().unwrap();
}

fn generate_ico(path: &std::path::Path) {
    const S: u32 = 32;
    let mut rgba = vec![255u8; (S * S * 4) as usize];

    let set = |p: &mut [u8], x: u32, y: u32| {
        let i = ((y * S + x) * 4) as usize;
        p[i] = 0;
        p[i + 1] = 0;
        p[i + 2] = 0;
        p[i + 3] = 255;
    };

    let r = |x: u32, rx: std::ops::Range<u32>| rx.contains(&x);

    for y in 0..S {
        for x in 0..S {
            let on = match (x, y) {
                // Border
                _ if x == 0 || y == 0 || x == S - 1 || y == S - 1 => true,
                // Top-left finder outline (2..8 × 2..8, 6×6)
                _ if (y == 2 || y == 7) && r(x, 2..8) => true,
                _ if (x == 2 || x == 7) && r(y, 2..8) => true,
                // Top-right finder (24..30 × 2..8)
                _ if (y == 2 || y == 7) && r(x, 24..30) => true,
                _ if (x == 24 || x == 29) && r(y, 2..8) => true,
                // Bottom-left finder (2..8 × 24..30)
                _ if (y == 24 || y == 29) && r(x, 2..8) => true,
                _ if (x == 2 || x == 7) && r(y, 24..30) => true,
                // Center data 12..20 × 12..20, checkered
                _ if r(x, 12..20) && r(y, 12..20) && (x + y) % 2 == 0 => true,
                // Timing patterns
                _ if x == 12 && r(y, 10..22) => true,
                _ if y == 12 && r(x, 10..22) => true,
                _ => false,
            };
            if on {
                set(&mut rgba, x, y);
            }
        }
    }

    // BMP data: BGRA, bottom-to-top rows, no padding for 32bpp
    let mut bmp: Vec<u8> = Vec::new();
    // BITMAPINFOHEADER (40 bytes)
    bmp.extend_from_slice(&40u32.to_le_bytes()); // header size
    bmp.extend_from_slice(&(S as i32).to_le_bytes()); // width
    bmp.extend_from_slice(&((S * 2) as i32).to_le_bytes()); // height = 2× for XOR+AND
    bmp.extend_from_slice(&1u16.to_le_bytes()); // planes
    bmp.extend_from_slice(&32u16.to_le_bytes()); // bpp
    bmp.extend_from_slice(&[0u8; 4]); // compression (BI_RGB)
    bmp.extend_from_slice(&[0u8; 4]); // image size (0 for BI_RGB)
    bmp.extend_from_slice(&[0u8; 4]); // x pixels per meter
    bmp.extend_from_slice(&[0u8; 4]); // y pixels per meter
    bmp.extend_from_slice(&[0u8; 4]); // colors used
    bmp.extend_from_slice(&[0u8; 4]); // colors important

    // XOR mask: BGRA rows bottom-to-top
    for y in (0..S).rev() {
        for x in 0..S {
            let i = ((y * S + x) * 4) as usize;
            bmp.push(rgba[i + 2]); // R
            bmp.push(rgba[i + 1]); // G
            bmp.push(rgba[i]);     // B
            bmp.push(rgba[i + 3]); // A
        }
    }

    // AND mask: 1bpp, rows bottom-to-top, each row padded to 4 bytes
    let row_bytes = ((S + 31) / 32) * 4;
    for _y in (0..S).rev() {
        for _b in 0..row_bytes {
            bmp.push(0);
        }
    }

    // ICO wrapper
    let image_size = bmp.len() as u32;
    let mut ico: Vec<u8> = Vec::new();
    ico.extend_from_slice(&[0u8; 2]); // reserved
    ico.extend_from_slice(&1u16.to_le_bytes()); // type = icon
    ico.extend_from_slice(&1u16.to_le_bytes()); // count = 1
    // Directory entry
    ico.push(if S >= 256 { 0 } else { S as u8 }); // width
    ico.push(if S >= 256 { 0 } else { S as u8 }); // height
    ico.push(0); // colors
    ico.push(0); // reserved
    ico.extend_from_slice(&1u16.to_le_bytes()); // planes
    ico.extend_from_slice(&32u16.to_le_bytes()); // bpp
    ico.extend_from_slice(&image_size.to_le_bytes()); // image data size
    ico.extend_from_slice(&22u32.to_le_bytes()); // offset
    ico.extend_from_slice(&bmp);

    let mut f = File::create(path).expect("Failed to create icon file");
    f.write_all(&ico).expect("Failed to write icon");
}
