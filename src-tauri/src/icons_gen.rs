/// 32×32 RGBA tray ikonu üreticisi — harici kütüphane gerektirmez.
/// Shield şekli + durum sembolü (check / pause / X) çizer.

pub fn make_tray_rgba(state: &str) -> Vec<u8> {
    let (r, g, b) = match state {
        "paused" => (0xf5u8, 0xa6u8, 0x23u8), // sarı
        "error" => (0xffu8, 0x4du8, 0x4du8),  // kırmızı
        _ => (0x4au8, 0x7cu8, 0xffu8),        // mavi
    };

    let mut img = vec![0u8; 32 * 32 * 4];

    // Shield alanı doldur
    for y in 0i32..32 {
        if let Some((x0, x1)) = shield_range(y) {
            for x in x0..=x1 {
                set_px(&mut img, x, y, r, g, b, 255);
            }
        }
    }

    // Durum sembolü (beyaz)
    match state {
        "paused" => draw_pause(&mut img),
        "error" => draw_x(&mut img),
        _ => draw_check(&mut img),
    }

    img
}

// Shield şekli: 32×32 içinde beşgen (üst yuvarlak, alt sivri)
fn shield_range(y: i32) -> Option<(i32, i32)> {
    match y {
        2..=4 => Some((7 + (4 - y), 24 - (4 - y))), // üst hafif girintili
        5..=19 => Some((5, 26)),                    // gövde
        20 => Some((6, 25)),
        21 => Some((7, 24)),
        22 => Some((8, 23)),
        23 => Some((9, 22)),
        24 => Some((10, 21)),
        25 => Some((11, 20)),
        26 => Some((12, 19)),
        27 => Some((13, 18)),
        28 => Some((14, 17)),
        29 => Some((15, 16)),
        _ => None,
    }
}

// Onay işareti (✓)
fn draw_check(img: &mut Vec<u8>) {
    // Sol kol: (10,16)→(13,19)
    for i in 0i32..4 {
        thick_px(img, 10 + i, 16 + i);
    }
    // Sağ kol: (13,19)→(22,10)
    for i in 0i32..10 {
        thick_px(img, 13 + i, 19 - i);
    }
}

// Duraklatma çubukları (⏸)
fn draw_pause(img: &mut Vec<u8>) {
    for y in 11i32..22 {
        for x in 11i32..14 {
            set_px(img, x, y, 255, 255, 255, 255);
        }
        for x in 18i32..21 {
            set_px(img, x, y, 255, 255, 255, 255);
        }
    }
}

// X işareti (✕)
fn draw_x(img: &mut Vec<u8>) {
    for i in 0i32..10 {
        thick_px(img, 10 + i, 11 + i);
        thick_px(img, 21 - i, 11 + i);
    }
}

// 1 piksel beyaz + sağına da beyaz (2px kalınlık)
fn thick_px(img: &mut Vec<u8>, x: i32, y: i32) {
    set_px(img, x, y, 255, 255, 255, 255);
    set_px(img, x + 1, y, 255, 255, 255, 255);
}

fn set_px(img: &mut Vec<u8>, x: i32, y: i32, r: u8, g: u8, b: u8, a: u8) {
    if x < 0 || x >= 32 || y < 0 || y >= 32 {
        return;
    }
    let idx = ((y * 32 + x) * 4) as usize;
    img[idx] = r;
    img[idx + 1] = g;
    img[idx + 2] = b;
    img[idx + 3] = a;
}
