pub fn create_app_icon() -> egui::IconData {
    let size = 16u32;
    let mut rgba = vec![255u8; (size * size * 4) as usize];

    let set = |rgba: &mut [u8], x: u32, y: u32| {
        let i = ((y * size + x) * 4) as usize;
        rgba[i] = 0;
        rgba[i + 1] = 0;
        rgba[i + 2] = 0;
        rgba[i + 3] = 255;
    };

    let is_in = |x: u32, y: u32, rx: std::ops::Range<u32>, ry: std::ops::Range<u32>| -> bool {
        rx.contains(&x) && ry.contains(&y)
    };

    for y in 0..size {
        for x in 0..size {
            // Border
            if x == 0 || y == 0 || x == size - 1 || y == size - 1 {
                set(&mut rgba, x, y);
            }
            // Top-left finder (3x3 outline at 1,1)
            if is_in(x, y, 1..4, 1..2) || is_in(x, y, 1..4, 3..4) || is_in(x, y, 1..2, 1..4) || is_in(x, y, 3..4, 1..4) {
                set(&mut rgba, x, y);
            }
            // Top-right finder (3x3 at 12,1)
            if is_in(x, y, 12..15, 1..2) || is_in(x, y, 12..15, 3..4) || is_in(x, y, 12..13, 1..4) || is_in(x, y, 14..15, 1..4) {
                set(&mut rgba, x, y);
            }
            // Bottom-left finder (3x3 at 1,12)
            if is_in(x, y, 1..4, 12..13) || is_in(x, y, 1..4, 14..15) || is_in(x, y, 1..2, 12..15) || is_in(x, y, 3..4, 12..15) {
                set(&mut rgba, x, y);
            }
            // Center data pattern
            if is_in(x, y, 6..10, 6..10) && (x + y) % 2 == 0 {
                set(&mut rgba, x, y);
            }
            // Timing patterns
            if (x == 6 && y > 4 && y < 11) || (y == 6 && x > 4 && x < 11) {
                set(&mut rgba, x, y);
            }
        }
    }

    egui::IconData {
        width: size,
        height: size,
        rgba,
    }
}
