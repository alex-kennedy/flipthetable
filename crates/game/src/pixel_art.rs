use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

#[derive(Resource, Clone)]
pub struct ProceduralTextures {
    // Tiled architectural & surface textures (Repeat + Nearest)
    pub carpet_navy: Handle<Image>,
    pub walnut_desk: Handle<Image>,
    pub plywood_floor: Handle<Image>,
    pub parquet_floor: Handle<Image>,
    pub slate_wall: Handle<Image>,
    pub corrugated_wall: Handle<Image>,
    pub mahogany_panel: Handle<Image>,
    pub ceiling_tbar: Handle<Image>,
    pub cubicle_fabric: Handle<Image>,
    pub hazard_stripe: Handle<Image>,
    pub stage_deck: Handle<Image>,
    pub velvet_curtain: Handle<Image>,
    pub linoleum_tile: Handle<Image>,
    pub breakroom_mint: Handle<Image>,
    pub warehouse_concrete: Handle<Image>,

    // Clamped prop / screen / decal pixel-art textures (ClampToEdge + Nearest)
    pub crt_phosphor: Handle<Image>,
    pub crt_cracked: Handle<Image>,
    pub keyboard_top: Handle<Image>,
    pub printed_page: Handle<Image>,
    pub pinned_memos: Handle<Image>,
    pub cat_poster: Handle<Image>,
    pub whiteboard_doodle: Handle<Image>,
    pub blueprint_page: Handle<Image>,
    pub clock_face: Handle<Image>,
    pub stock_crash_tv: Handle<Image>,
    pub townhall_slide: Handle<Image>,
    pub injury_board: Handle<Image>,
    pub city_skyline: Handle<Image>,
    pub splatter_decal: Handle<Image>,
    pub scorch_decal: Handle<Image>,
    pub spiderweb_crack: Handle<Image>,
}

pub fn create_procedural_textures(images: &mut Assets<Image>) -> ProceduralTextures {
    ProceduralTextures {
        carpet_navy: images.add(make_carpet_navy_image()),
        walnut_desk: images.add(make_walnut_desk_image()),
        plywood_floor: images.add(make_plywood_floor_image()),
        parquet_floor: images.add(make_parquet_floor_image()),
        slate_wall: images.add(make_slate_wall_image()),
        corrugated_wall: images.add(make_corrugated_wall_image()),
        mahogany_panel: images.add(make_mahogany_panel_image()),
        ceiling_tbar: images.add(make_ceiling_tbar_image()),
        cubicle_fabric: images.add(make_cubicle_fabric_image()),
        hazard_stripe: images.add(make_hazard_stripe_image()),
        stage_deck: images.add(make_stage_deck_image()),
        velvet_curtain: images.add(make_velvet_curtain_image()),
        linoleum_tile: images.add(make_linoleum_tile_image()),
        breakroom_mint: images.add(make_breakroom_mint_image()),
        warehouse_concrete: images.add(make_warehouse_concrete_image()),

        crt_phosphor: images.add(make_crt_phosphor_image()),
        crt_cracked: images.add(make_crt_cracked_image()),
        keyboard_top: images.add(make_keyboard_top_image()),
        printed_page: images.add(make_printed_page_image()),
        pinned_memos: images.add(make_pinned_memos_image()),
        cat_poster: images.add(make_cat_poster_image()),
        whiteboard_doodle: images.add(make_whiteboard_image()),
        blueprint_page: images.add(make_blueprint_page_image()),
        clock_face: images.add(make_clock_face_image()),
        stock_crash_tv: images.add(make_stock_crash_tv_image()),
        townhall_slide: images.add(make_townhall_slide_image()),
        injury_board: images.add(make_injury_board_image()),
        city_skyline: images.add(make_city_skyline_image()),
        splatter_decal: images.add(make_splatter_decal_image()),
        scorch_decal: images.add(make_scorch_decal_image()),
        spiderweb_crack: images.add(make_spiderweb_crack_image()),
    }
}

fn nearest_clamped_rgba(width: u32, height: u32, data: Vec<u8>) -> Image {
    let mut img = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    img.sampler = ImageSampler::nearest();
    img
}

fn nearest_repeat_rgba(width: u32, height: u32, data: Vec<u8>) -> Image {
    let mut img = nearest_clamped_rgba(width, height, data);
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Nearest,
        min_filter: ImageFilterMode::Nearest,
        mipmap_filter: ImageFilterMode::Nearest,
        ..ImageSamplerDescriptor::nearest()
    });
    img
}

struct PixelCanvas {
    w: i32,
    h: i32,
    buf: Vec<[u8; 4]>,
}

impl PixelCanvas {
    fn new(w: u32, h: u32, fill: [u8; 4]) -> Self {
        Self {
            w: w as i32,
            h: h as i32,
            buf: vec![fill; (w * h) as usize],
        }
    }

    fn set(&mut self, x: i32, y: i32, c: [u8; 4]) {
        if x >= 0 && x < self.w && y >= 0 && y < self.h {
            self.buf[(y * self.w + x) as usize] = c;
        }
    }

    fn set_rgb(&mut self, x: i32, y: i32, rgb: [u8; 3]) {
        self.set(x, y, [rgb[0], rgb[1], rgb[2], 255]);
    }

    fn rect(&mut self, x0: i32, y0: i32, w: i32, h: i32, rgb: [u8; 3]) {
        for y in y0..(y0 + h) {
            for x in x0..(x0 + w) {
                self.set_rgb(x, y, rgb);
            }
        }
    }

    fn border(&mut self, x0: i32, y0: i32, w: i32, h: i32, rgb: [u8; 3]) {
        for x in x0..(x0 + w) {
            self.set_rgb(x, y0, rgb);
            self.set_rgb(x, y0 + h - 1, rgb);
        }
        for y in y0..(y0 + h) {
            self.set_rgb(x0, y, rgb);
            self.set_rgb(x0 + w - 1, y, rgb);
        }
    }

    fn line(&mut self, mut x0: i32, mut y0: i32, x1: i32, y1: i32, rgb: [u8; 3]) {
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            self.set_rgb(x0, y0, rgb);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    fn into_bytes(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.buf.len() * 4);
        for px in self.buf {
            out.extend_from_slice(&px);
        }
        out
    }
}

fn hash_u32(x: u32, y: u32, seed: u32) -> u32 {
    let mut h = x
        .wrapping_mul(374761393)
        .wrapping_add(y.wrapping_mul(668265263))
        .wrapping_add(seed.wrapping_mul(2246822519));
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^ (h >> 16)
}

/// Moody deep slate-navy commercial carpet tiles matching `loading-desk.jpg`.
fn make_carpet_navy_image() -> Image {
    let (w, h) = (32, 32);
    let mut c = PixelCanvas::new(w, h, [29, 36, 48, 255]);
    for y in 0..h {
        for x in 0..w {
            let tile_seam = x == 0 || y == 0 || x == 16 || y == 16;
            let quad = ((x / 16) + (y / 16)) % 2 == 0;
            let n = (hash_u32(x, y, 11) % 7) as i32 - 3;
            let base = if tile_seam {
                [21, 27, 37]
            } else if quad {
                [31, 39, 52]
            } else {
                [28, 35, 47]
            };
            let rgb = [
                (base[0] + n).clamp(14, 70) as u8,
                (base[1] + n).clamp(18, 82) as u8,
                (base[2] + n).clamp(24, 98) as u8,
            ];
            c.set_rgb(x as i32, y as i32, rgb);
        }
    }
    nearest_repeat_rgba(w, h, c.into_bytes())
}

/// Rich dark warm walnut/mahogany desk planks with subtle grain & dark plank seams (`loading-desk.jpg`).
fn make_walnut_desk_image() -> Image {
    let (w, h) = (32, 32);
    let mut c = PixelCanvas::new(w, h, [104, 58, 28, 255]);
    for y in 0..h {
        let plank = y / 8;
        let seam = y % 8 == 0;
        for x in 0..w {
            let shift = plank * 7;
            let wave = ((x + shift + (y % 8) * 3) % 11) == 0;
            let streak = ((x * 2 + y * 5 + shift) % 13) == 0;
            let highlight = ((x + shift * 3) % 17 == 0) && !seam;
            let rgb = if seam {
                [64, 34, 16]
            } else if wave {
                [84, 45, 21]
            } else if streak {
                [92, 51, 24]
            } else if highlight {
                [128, 74, 36]
            } else {
                [110, 62, 30]
            };
            c.set_rgb(x as i32, y as i32, rgb);
        }
    }
    nearest_repeat_rgba(w, h, c.into_bytes())
}

/// Dusty construction trailer OSB / plywood panels with corner nail heads.
fn make_plywood_floor_image() -> Image {
    let (w, h) = (32, 32);
    let mut c = PixelCanvas::new(w, h, [156, 122, 78, 255]);
    for y in 0..h {
        for x in 0..w {
            let seam = x == 0 || y == 0;
            let flake = hash_u32(x / 2, y, 42) % 5;
            let rgb = if seam {
                [96, 72, 44]
            } else if flake == 0 {
                [140, 108, 68]
            } else if flake == 1 {
                [170, 136, 88]
            } else {
                [156, 122, 78]
            };
            c.set_rgb(x as i32, y as i32, rgb);
        }
    }
    for (nx, ny) in [(2, 2), (29, 2), (2, 29), (29, 29)] {
        c.set_rgb(nx, ny, [58, 62, 68]);
    }
    nearest_repeat_rgba(w, h, c.into_bytes())
}

/// Executive boardroom dark herringbone parquet wood floor.
fn make_parquet_floor_image() -> Image {
    let (w, h) = (32, 32);
    let mut c = PixelCanvas::new(w, h, [72, 42, 26, 255]);
    for y in 0..h {
        for x in 0..w {
            let block = ((x / 8) + (y / 8)) % 2 == 0;
            let local = if block { x % 8 == 0 || y % 4 == 0 } else { y % 8 == 0 || x % 4 == 0 };
            let tone = ((x / 4 + y / 4) % 3) as i32;
            let rgb = if local {
                [46, 26, 16]
            } else if block {
                [(78 + tone * 5) as u8, (46 + tone * 3) as u8, 28]
            } else {
                [(66 + tone * 5) as u8, (38 + tone * 3) as u8, 23]
            };
            c.set_rgb(x as i32, y as i32, rgb);
        }
    }
    nearest_repeat_rgba(w, h, c.into_bytes())
}

/// Moody slate-blue office wall with subtle top cornice dither shadow band (matching `loading-desk.jpg`).
fn make_slate_wall_image() -> Image {
    let (w, h) = (32, 32);
    let mut c = PixelCanvas::new(w, h, [44, 55, 72, 255]);
    for y in 0..h {
        for x in 0..w {
            let panel_seam = x == 0;
            let dither_top = y < 5 && ((x + y) % 2 == 0 || y < 2);
            let tex = (hash_u32(x, y, 7) % 3) as i32 - 1;
            let base = if panel_seam || dither_top {
                [31, 39, 52]
            } else {
                [45, 56, 73]
            };
            c.set_rgb(
                x as i32,
                y as i32,
                [
                    (base[0] + tex) as u8,
                    (base[1] + tex) as u8,
                    (base[2] + tex) as u8,
                ],
            );
        }
    }
    nearest_repeat_rgba(w, h, c.into_bytes())
}

/// Ribbed industrial steel siding for the Building Site Trailer.
fn make_corrugated_wall_image() -> Image {
    let (w, h) = (32, 32);
    let mut c = PixelCanvas::new(w, h, [142, 150, 148, 255]);
    for y in 0..h {
        for x in 0..w {
            let phase = x % 4;
            let rgb = match phase {
                0 => [102, 110, 112],
                1 => [162, 170, 168],
                2 => [146, 154, 152],
                _ => [124, 132, 132],
            };
            c.set_rgb(x as i32, y as i32, rgb);
        }
    }
    for rx in (2..32).step_by(8) {
        c.set_rgb(rx, 15, [76, 82, 86]);
    }
    nearest_repeat_rgba(w, h, c.into_bytes())
}

/// Executive dark walnut/slate architectural wall paneling.
fn make_mahogany_panel_image() -> Image {
    let (w, h) = (32, 32);
    let mut c = PixelCanvas::new(w, h, [36, 42, 56, 255]);
    // Lower wood wainscoting + recessed moulding
    c.rect(0, 16, 32, 16, [68, 36, 24]);
    c.rect(0, 15, 32, 2, [158, 124, 62]); // brass trim rail
    c.border(3, 19, 26, 10, [46, 23, 15]);
    c.border(4, 20, 24, 8, [88, 48, 32]);
    nearest_repeat_rgba(w, h, c.into_bytes())
}

/// Dark slate recessed ceiling T-bar grid (`loading-desk.jpg`).
fn make_ceiling_tbar_image() -> Image {
    let (w, h) = (32, 32);
    let mut c = PixelCanvas::new(w, h, [33, 41, 54, 255]);
    for y in 0..h {
        for x in 0..w {
            let tbar = x == 0 || y == 0 || x == 16;
            let bevel = x == 1 || y == 1 || x == 17;
            let pinhole = (x * 5 + y * 11) % 17 == 0;
            let rgb = if tbar {
                [18, 23, 32]
            } else if bevel {
                [42, 52, 68]
            } else if pinhole {
                [25, 32, 43]
            } else {
                [33, 41, 54]
            };
            c.set_rgb(x as i32, y as i32, rgb);
        }
    }
    nearest_repeat_rgba(w, h, c.into_bytes())
}

/// Cross-woven slate-blue cubicle fabric matching `loading-desk.jpg`.
fn make_cubicle_fabric_image() -> Image {
    let (w, h) = (32, 32);
    let mut c = PixelCanvas::new(w, h, [62, 76, 94, 255]);
    for y in 0..h {
        for x in 0..w {
            let weave = (x + y) % 2 == 0;
            let slub = hash_u32(x, y, 99) % 11 == 0;
            let rgb = if slub {
                [74, 89, 108]
            } else if weave {
                [63, 77, 96]
            } else {
                [55, 68, 85]
            };
            c.set_rgb(x as i32, y as i32, rgb);
        }
    }
    nearest_repeat_rgba(w, h, c.into_bytes())
}

/// High-contrast diagonal safety hazard stripes.
fn make_hazard_stripe_image() -> Image {
    let (w, h) = (32, 32);
    let mut c = PixelCanvas::new(w, h, [28, 32, 40, 255]);
    for y in 0..h {
        for x in 0..w {
            let band = ((x + y) / 6) % 2 == 0;
            let edge = ((x + y) % 6) == 0;
            let rgb = if edge {
                [172, 130, 22]
            } else if band {
                [244, 194, 42]
            } else {
                [26, 30, 38]
            };
            c.set_rgb(x as i32, y as i32, rgb);
        }
    }
    nearest_repeat_rgba(w, h, c.into_bytes())
}

/// Curved teal-cyan phosphor CRT screen with dark inner bezel & bright top-left specular arc (`loading-desk.jpg`!).
fn make_crt_phosphor_image() -> Image {
    let (w, h) = (32, 24);
    let mut c = PixelCanvas::new(w, h, [10, 14, 20, 255]);

    for y in 1..23 {
        for x in 1..31 {
            // Rounded CRT glass corners
            let corner = (x <= 2 || x >= 29) && (y <= 2 || y >= 21);
            if corner {
                continue;
            }
            let scan = y % 2 == 0;
            let dx = (x as f32 - 15.5) / 15.0;
            let dy = (y as f32 - 11.5) / 11.0;
            let r2 = dx * dx + dy * dy;
            let vig = (1.0 - r2 * 0.42).clamp(0.45, 1.0);
            let base_g = if scan { 112.0 } else { 96.0 };
            let base_b = if scan { 116.0 } else { 100.0 };
            c.set_rgb(
                x,
                y,
                [
                    (16.0 * vig) as u8,
                    (base_g * vig) as u8,
                    (base_b * vig) as u8,
                ],
            );
        }
    }

    // Iconic bright cyan-mint curved specular highlight in top-left corner (from loading-desk.jpg!)
    let hi = [132, 255, 236];
    let mid = [56, 198, 186];
    for &(x, y) in &[
        (4, 3), (5, 3), (6, 3), (7, 3), (8, 3), (9, 3), (10, 3), (11, 3), (12, 3),
        (3, 4), (4, 4), (5, 4), (6, 4), (7, 4), (8, 4),
        (3, 5), (4, 5), (3, 6),
    ] {
        c.set_rgb(x, y, hi);
    }
    for &(x, y) in &[
        (13, 3), (14, 3), (15, 3), (9, 4), (10, 4), (5, 5), (6, 5), (4, 6), (3, 7), (3, 8),
    ] {
        c.set_rgb(x, y, mid);
    }

    // Subtle terminal prompt cursor `>_`
    for x in 5..8 {
        c.set_rgb(x, 18, hi);
    }
    nearest_clamped_rgba(w, h, c.into_bytes())
}

/// Cracked & glitching CRT/LCD screen texture when damaged!
fn make_crt_cracked_image() -> Image {
    let (w, h) = (32, 24);
    let mut c = PixelCanvas::new(w, h, [14, 18, 24, 255]);
    for y in 1..23 {
        for x in 1..31 {
            let n = hash_u32(x as u32, y as u32, 313);
            let bar = y == 7 || y == 8 || y == 16;
            let rgb = if bar {
                if n % 3 == 0 {
                    [250, 90, 120]
                } else {
                    [95, 250, 235]
                }
            } else if n % 5 == 0 {
                [48, 86, 94]
            } else {
                [20, 34, 42]
            };
            c.set_rgb(x, y, rgb);
        }
    }
    // Bright white/cyan spiderweb glass fracture lines
    let crack = [240, 252, 255];
    c.line(18, 10, 4, 2, crack);
    c.line(18, 10, 29, 4, crack);
    c.line(18, 10, 27, 21, crack);
    c.line(18, 10, 8, 21, crack);
    c.line(18, 10, 2, 13, crack);
    nearest_clamped_rgba(w, h, c.into_bytes())
}

/// Full 104-key beige mechanical keyboard pixel art (`loading-desk.jpg`!).
fn make_keyboard_top_image() -> Image {
    let (w, h) = (48, 18);
    let chassis = [208, 196, 168];
    let well = [74, 68, 58];
    let key_light = [228, 218, 192];
    let key_mod = [184, 172, 146];

    let mut c = PixelCanvas::new(w, h, [chassis[0], chassis[1], chassis[2], 255]);
    c.border(0, 0, 48, 18, [140, 128, 106]);

    // Function row (Esc + F1..F4 + F5..F8 + F9..F12)
    c.rect(2, 2, 3, 2, [210, 92, 78]); // red/terracotta Esc accent
    for gx in [7, 15, 23] {
        c.rect(gx, 2, 6, 2, well);
        for k in 0..3 {
            c.rect(gx + k * 2, 2, 1, 2, key_light);
        }
    }
    // 3 green status LEDs top-right
    c.set_rgb(40, 2, [90, 238, 120]);
    c.set_rgb(42, 2, [90, 238, 120]);
    c.set_rgb(44, 2, [58, 110, 68]);

    // Main 60% alphanumeric block
    c.rect(2, 5, 27, 11, well);
    for row in 0..5 {
        let y = 5 + row * 2;
        if row == 4 {
            // Bottom modifier + spacebar row
            c.rect(2, y, 4, 2, key_mod);
            c.rect(7, y, 13, 2, key_light); // spacebar!
            c.rect(21, y, 8, 2, key_mod);
        } else {
            c.rect(2, y, 3, 1, key_mod);
            for col in 0..10 {
                c.rect(6 + col * 2, y, 1, 1, key_light);
            }
            c.rect(26, y, 3, 1, key_mod);
        }
    }

    // Navigation + inverted-T arrow cluster
    c.rect(31, 5, 6, 5, well);
    for ky in [5, 7] {
        for kx in [31, 33, 35] {
            c.set_rgb(kx, ky, key_mod);
        }
    }
    c.rect(33, 12, 2, 2, key_mod); // Up
    c.rect(31, 14, 6, 2, key_mod); // Left/Down/Right
    c.set_rgb(32, 14, well);
    c.set_rgb(35, 14, well);

    // Numpad block
    c.rect(39, 5, 7, 11, well);
    for ny in (5..15).step_by(2) {
        for nx in (39..45).step_by(2) {
            c.set_rgb(nx, ny, key_light);
        }
        c.set_rgb(45, ny, key_mod);
    }

    nearest_clamped_rgba(w, h, c.into_bytes())
}

/// Crisp white TPS report page with header band & printed pixel text lines.
fn make_printed_page_image() -> Image {
    let (w, h) = (24, 32);
    let mut c = PixelCanvas::new(w, h, [238, 242, 246, 255]);
    c.border(0, 0, 24, 32, [172, 182, 196]);

    // Header block + red TPS stamp
    c.rect(3, 3, 11, 2, [52, 64, 84]);
    c.rect(16, 3, 5, 3, [214, 66, 62]);

    // Printed text lines
    for (idx, y) in (8..28).step_by(3).enumerate() {
        let len = if idx % 3 == 2 { 12 } else { 17 };
        c.rect(3, y, len, 1, [118, 130, 148]);
    }
    nearest_clamped_rgba(w, h, c.into_bytes())
}

/// Overlapping white memo + yellow sticky note pinned to cubicle fabric (`loading-desk.jpg`!).
fn make_pinned_memos_image() -> Image {
    let (w, h) = (32, 32);
    let mut c = PixelCanvas::new(w, h, [0, 0, 0, 0]);

    // Main white memo sheet
    for y in 3..26 {
        for x in 3..21 {
            c.set(x, y, [232, 236, 242, 255]);
        }
    }
    c.border(3, 3, 18, 23, [34, 42, 56]);
    c.rect(11, 4, 2, 2, [222, 58, 54]); // red pushpin
    for y in (8..23).step_by(3) {
        c.rect(6, y, 12, 1, [112, 124, 142]);
    }

    // Yellow sticky note bottom-right
    for y in 15..29 {
        for x in 16..29 {
            c.set(x, y, [246, 214, 92, 255]);
        }
    }
    c.border(16, 15, 13, 14, [82, 66, 24]);
    c.rect(22, 16, 2, 2, [56, 142, 232]); // blue pushpin
    for y in [20, 23, 26] {
        c.rect(18, y, 8, 1, [138, 110, 38]);
    }

    nearest_clamped_rgba(w, h, c.into_bytes())
}

/// Framed "HANG IN THERE" orange cat poster.
fn make_cat_poster_image() -> Image {
    let (w, h) = (24, 32);
    let mut c = PixelCanvas::new(w, h, [234, 226, 206, 255]);
    c.border(0, 0, 24, 32, [34, 24, 18]);
    c.border(1, 1, 22, 30, [132, 78, 38]);

    // Sky background + branch
    c.rect(2, 2, 20, 22, [168, 208, 226]);
    c.rect(2, 6, 20, 2, [96, 58, 30]);

    // Orange kitty hanging by paws!
    let orange = [238, 142, 50];
    let dark_orange = [192, 98, 28];
    c.rect(8, 6, 2, 4, orange); // left paw
    c.rect(14, 6, 2, 4, orange); // right paw
    c.rect(7, 10, 10, 8, orange); // head
    c.set_rgb(7, 9, dark_orange);
    c.set_rgb(8, 9, orange);
    c.set_rgb(16, 9, dark_orange);
    c.set_rgb(15, 9, orange);
    c.set_rgb(9, 13, [26, 28, 34]); // eyes
    c.set_rgb(14, 13, [26, 28, 34]);
    c.rect(11, 15, 2, 1, [226, 96, 112]); // pink nose
    c.rect(9, 18, 6, 5, dark_orange); // body

    // Caption bar at bottom
    c.rect(4, 26, 16, 2, [54, 62, 76]);
    nearest_clamped_rgba(w, h, c.into_bytes())
}

/// Rolling site whiteboard with plunging Gantt chart & red markers.
fn make_whiteboard_image() -> Image {
    let (w, h) = (48, 30);
    let mut c = PixelCanvas::new(w, h, [238, 242, 246, 255]);
    c.border(0, 0, 48, 30, [68, 76, 92]);
    c.border(1, 1, 46, 28, [148, 158, 174]);

    // Subtle grid lines
    for gx in (8..44).step_by(8) {
        c.line(gx, 4, gx, 25, [218, 224, 232]);
    }
    for gy in (6..26).step_by(6) {
        c.line(4, gy, 43, gy, [218, 224, 232]);
    }

    // Blue target line vs plunging red actual line
    c.line(5, 8, 38, 8, [46, 126, 224]);
    c.line(5, 9, 18, 11, [224, 52, 52]);
    c.line(18, 11, 30, 18, [224, 52, 52]);
    c.line(30, 18, 41, 25, [224, 52, 52]);
    c.rect(39, 23, 4, 4, [224, 52, 52]);

    // Yellow sticky note on whiteboard
    c.rect(6, 17, 8, 8, [248, 214, 78]);
    nearest_clamped_rgba(w, h, c.into_bytes())
}

/// Prussian-blue construction blueprint with crisp white floorplan lines.
fn make_blueprint_page_image() -> Image {
    let (w, h) = (32, 24);
    let mut c = PixelCanvas::new(w, h, [28, 78, 156, 255]);
    c.border(1, 1, 30, 22, [218, 238, 255]);
    c.border(5, 4, 16, 14, [218, 238, 255]);
    c.line(13, 4, 13, 17, [168, 210, 250]);
    c.line(5, 11, 20, 11, [168, 210, 250]);
    c.rect(22, 15, 7, 6, [168, 210, 250]);
    nearest_clamped_rgba(w, h, c.into_bytes())
}

/// Analog office wall clock (`4:59` PM!) matching the pillar clock in `loading-desk.jpg`.
fn make_clock_face_image() -> Image {
    let (w, h) = (24, 24);
    let mut c = PixelCanvas::new(w, h, [0, 0, 0, 0]);
    for y in 0..24 {
        for x in 0..24 {
            let dx = x as f32 - 11.5;
            let dy = y as f32 - 11.5;
            let r = (dx * dx + dy * dy).sqrt();
            if r <= 11.2 {
                if r >= 9.3 {
                    c.set(x, y, [26, 32, 44, 255]);
                } else {
                    c.set(x, y, [224, 226, 218, 255]);
                }
            }
        }
    }
    // 12, 3, 6, 9 ticks + hands at 4:59!
    for &(tx, ty) in &[(11, 3), (12, 3), (20, 11), (11, 20), (3, 11)] {
        c.set_rgb(tx, ty, [38, 44, 58]);
    }
    c.line(11, 11, 11, 5, [32, 38, 50]); // minute hand near 12
    c.line(11, 11, 16, 15, [32, 38, 50]); // hour hand at 5
    c.set_rgb(11, 11, [204, 54, 54]);
    nearest_clamped_rgba(w, h, c.into_bytes())
}

/// Executive Boardroom TV showing dramatic red Q4 stock crash chart!
fn make_stock_crash_tv_image() -> Image {
    let (w, h) = (48, 28);
    let mut c = PixelCanvas::new(w, h, [14, 18, 28, 255]);
    c.border(0, 0, 48, 28, [34, 42, 58]);
    for gx in (6..44).step_by(8) {
        c.line(gx, 3, gx, 24, [26, 34, 48]);
    }
    for gy in (5..25).step_by(5) {
        c.line(4, gy, 44, gy, [26, 34, 48]);
    }
    // Jagged crashing red stock line + glowing fill bars
    let pts = [(4, 7), (11, 6), (17, 10), (22, 8), (28, 15), (34, 14), (40, 22), (44, 25)];
    for w_pair in pts.windows(2) {
        let (x0, y0) = w_pair[0];
        let (x1, y1) = w_pair[1];
        c.line(x0, y0, x1, y1, [255, 72, 72]);
        c.line(x0, y0 + 1, x1, y1 + 1, [180, 34, 44]);
    }
    c.rect(5, 3, 12, 2, [90, 224, 242]);
    c.rect(35, 4, 9, 3, [242, 58, 58]);
    nearest_clamped_rgba(w, h, c.into_bytes())
}

/// Moody twilight/sunset pixel skyscraper skyline for Executive Boardroom panoramic window.
fn make_city_skyline_image() -> Image {
    let (w, h) = (64, 36);
    let mut c = PixelCanvas::new(w, h, [26, 30, 54, 255]);
    for y in 0..36 {
        let t = y as f32 / 35.0;
        let sky = [
            (28.0 + t * 198.0).clamp(0.0, 255.0) as u8,
            (32.0 + t * 106.0).clamp(0.0, 255.0) as u8,
            (66.0 + t * 28.0).clamp(0.0, 255.0) as u8,
        ];
        for x in 0..64 {
            c.set_rgb(x, y, sky);
        }
    }
    // Silhouette high-rises with lit golden pixel windows
    let towers = [
        (0, 16, 9),
        (8, 10, 10),
        (17, 18, 8),
        (24, 7, 12),
        (35, 14, 9),
        (43, 9, 11),
        (53, 15, 11),
    ];
    for (tx, top_y, tw) in towers {
        c.rect(tx, top_y, tw, 36 - top_y, [18, 22, 34]);
        for wy in ((top_y + 2)..34).step_by(3) {
            for wx in ((tx + 2)..(tx + tw - 1)).step_by(2) {
                if hash_u32(wx as u32, wy as u32, 77) % 3 != 0 {
                    c.set_rgb(wx, wy, [252, 206, 108]);
                }
            }
        }
    }
    nearest_clamped_rgba(w, h, c.into_bytes())
}

/// White pixel-art organic liquid splatter / puddle mask (`AlphaMode::Mask(0.5)`), tintable to coffee, water, or paint!
fn make_splatter_decal_image() -> Image {
    let (w, h) = (32, 32);
    let mut c = PixelCanvas::new(w, h, [255, 255, 255, 0]);
    for y in 0..32 {
        for x in 0..32 {
            let dx = (x as f32 - 15.5) / 11.5;
            let dy = (y as f32 - 15.5) / 11.5;
            let angle_bucket = ((dy.atan2(dx) + 3.15) * 3.2) as u32;
            let lobe = ((hash_u32(angle_bucket, 1, 55) % 10) as f32) * 0.045;
            let r = (dx * dx + dy * dy).sqrt();
            if r < 0.78 + lobe {
                let shade = if r > 0.62 + lobe { 215 } else { 255 };
                c.set(x, y, [shade, shade, shade, 255]);
            }
        }
    }
    // Satellite droplets around main puddle!
    for &(sx, sy, sr) in &[(5, 8, 2), (26, 7, 2), (27, 24, 2), (6, 25, 1), (16, 3, 1)] {
        for dy in -sr..=sr {
            for dx in -sr..=sr {
                if dx * dx + dy * dy <= sr * sr {
                    c.set(sx + dx, sy + dy, [245, 245, 245, 255]);
                }
            }
        }
    }
    nearest_clamped_rgba(w, h, c.into_bytes())
}

/// Dark pixel-art explosion soot / scorch decal for electronics & hazard blasts.
fn make_scorch_decal_image() -> Image {
    let (w, h) = (32, 32);
    let mut c = PixelCanvas::new(w, h, [18, 20, 26, 0]);
    for y in 0..32 {
        for x in 0..32 {
            let dx = (x as f32 - 15.5) / 13.0;
            let dy = (y as f32 - 15.5) / 13.0;
            let r = (dx * dx + dy * dy).sqrt();
            let n = (hash_u32(x as u32, y as u32, 909) % 10) as f32 * 0.04;
            if r + n < 0.88 {
                let core = if r < 0.38 { 12 } else { 28 };
                c.set(x, y, [core, core + 2, core + 6, 235]);
            }
        }
    }
    nearest_clamped_rgba(w, h, c.into_bytes())
}

/// Crisp white/cyan pixel spiderweb crack decal for glass partitions & frames.
fn make_spiderweb_crack_image() -> Image {
    let (w, h) = (32, 32);
    let mut c = PixelCanvas::new(w, h, [255, 255, 255, 0]);
    let mut draw = |x0: i32, y0: i32, x1: i32, y1: i32| {
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let (mut cx, mut cy) = (x0, y0);
        loop {
            c.set(cx, cy, [235, 248, 255, 240]);
            if cx == x1 && cy == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                cx += sx;
            }
            if e2 <= dx {
                err += dx;
                cy += sy;
            }
        }
    };
    draw(16, 16, 3, 4);
    draw(16, 16, 28, 5);
    draw(16, 16, 30, 19);
    draw(16, 16, 22, 29);
    draw(16, 16, 6, 27);
    draw(16, 16, 2, 15);
    draw(10, 10, 22, 10);
    draw(22, 10, 23, 22);
    draw(23, 22, 10, 21);
    draw(10, 21, 10, 10);
    nearest_clamped_rgba(w, h, c.into_bytes())
}

/// Matte charcoal studio stage decking with subtle wood-plank seams & neon gaffer spike marks.
fn make_stage_deck_image() -> Image {
    let (w, h) = (32, 32);
    let mut c = PixelCanvas::new(w, h, [22, 20, 30, 255]);
    for y in 0..32 {
        let plank = (y / 8) as u32;
        for x in 0..32 {
            let grain = (hash_u32(x as u32, plank, 311) % 7) as u8;
            let mut rgb = [20 + grain, 18 + grain, 28 + grain];
            if y % 8 == 0 || (x == ((plank as i32 * 11) % 32) && y % 8 < 7) {
                rgb = [13, 12, 19];
            }
            c.set_rgb(x, y, rgb);
        }
    }
    // Neon magenta/cyan stage gaffer T-mark!
    c.rect(14, 15, 5, 1, [235, 62, 178]);
    c.rect(16, 15, 1, 4, [235, 62, 178]);
    nearest_repeat_rgba(w, h, c.into_bytes())
}

/// Deep corporate purple/indigo pleated theater acoustic curtain wall.
fn make_velvet_curtain_image() -> Image {
    let (w, h) = (32, 32);
    let mut c = PixelCanvas::new(w, h, [34, 20, 52, 255]);
    for y in 0..32 {
        for x in 0..32 {
            let fold = ((x as f32 * std::f32::consts::TAU / 8.0).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
            let tex = (hash_u32(x as u32, y as u32, 419) % 5) as f32;
            let r = (22.0 + fold * 36.0 + tex) as u8;
            let g = (12.0 + fold * 18.0 + tex * 0.5) as u8;
            let b = (38.0 + fold * 52.0 + tex) as u8;
            c.set_rgb(x, y, [r, g, b]);
        }
    }
    nearest_repeat_rgba(w, h, c.into_bytes())
}

/// Sad institutional breakroom linoleum checkerboard tile (hospital mint + dingy cream + scuffs).
fn make_linoleum_tile_image() -> Image {
    let (w, h) = (32, 32);
    let mut c = PixelCanvas::new(w, h, [196, 204, 188, 255]);
    for y in 0..32 {
        for x in 0..32 {
            let cell_alt = ((x / 16) + (y / 16)) % 2 == 0;
            let mut base = if cell_alt {
                [198_u8, 206, 190]
            } else {
                [162_u8, 184, 170]
            };
            if x % 16 == 0 || y % 16 == 0 {
                base = [126, 136, 128];
            } else {
                let speck = hash_u32(x as u32, y as u32, 521) % 19;
                if speck == 0 {
                    base = [132, 142, 134];
                } else if speck == 1 {
                    base = [218, 224, 210];
                }
            }
            c.set_rgb(x, y, base);
        }
    }
    nearest_repeat_rgba(w, h, c.into_bytes())
}

/// Institutional two-tone breakroom drywall (cream upper, hospital-sage dado rail & lower wall).
fn make_breakroom_mint_image() -> Image {
    let (w, h) = (32, 32);
    let mut c = PixelCanvas::new(w, h, [212, 216, 198, 255]);
    for y in 0..32 {
        for x in 0..32 {
            let n = (hash_u32(x as u32, y as u32, 613) % 6) as u8;
            let rgb = if y < 19 {
                [210 + n, 214 + n, 196 + n]
            } else if y == 19 || y == 20 {
                [112, 132, 122]
            } else {
                [146 + n, 174 + n, 160 + n]
            };
            c.set_rgb(x, y, rgb);
        }
    }
    nearest_repeat_rgba(w, h, c.into_bytes())
}

/// Cracked industrial fulfillment-warehouse concrete with yellow safety lane stripes & tire marks.
fn make_warehouse_concrete_image() -> Image {
    let (w, h) = (32, 32);
    let mut c = PixelCanvas::new(w, h, [88, 92, 98, 255]);
    for y in 0..32 {
        for x in 0..32 {
            let n = (hash_u32(x as u32, y as u32, 733) % 12) as u8;
            let mut rgb = [80 + n, 84 + n, 90 + n];
            if x == 0 || y == 0 {
                rgb = [58, 62, 68];
            } else if y == 29 || y == 30 {
                rgb = [224, 178, 34];
            }
            c.set_rgb(x, y, rgb);
        }
    }
    // Hairline concrete crack
    c.set_rgb(9, 11, [54, 58, 64]);
    c.set_rgb(10, 12, [54, 58, 64]);
    c.set_rgb(11, 12, [54, 58, 64]);
    c.set_rgb(12, 13, [54, 58, 64]);
    nearest_repeat_rgba(w, h, c.into_bytes())
}

/// All-Hands Town Hall presentation slide: soaring green BUYBACKS bar next to plunging red -15% RIF bar!
fn make_townhall_slide_image() -> Image {
    let (w, h) = (64, 36);
    let mut c = PixelCanvas::new(w, h, [20, 14, 38, 255]);
    c.border(0, 0, 64, 36, [232, 68, 184]);
    // Header ribbon ("WE ARE FAMILY // Q4 UPDATE")
    c.rect(2, 2, 60, 6, [68, 28, 112]);
    for x in (5..48).step_by(4) {
        c.rect(x, 4, 3, 2, [248, 236, 255]);
    }
    // Left chart card: RECORD EPS / BUYBACKS (tall glowing green columns)
    c.rect(4, 10, 26, 22, [30, 22, 54]);
    c.rect(7, 22, 4, 8, [64, 218, 128]);
    c.rect(13, 18, 4, 12, [64, 218, 128]);
    c.rect(19, 12, 5, 18, [92, 248, 152]);
    // Right chart card: HEADCOUNT -15% RIF (plunging crimson arrow & bars)
    c.rect(34, 10, 26, 22, [38, 18, 42]);
    c.rect(37, 13, 4, 17, [238, 64, 72]);
    c.rect(43, 19, 4, 11, [238, 64, 72]);
    c.rect(49, 25, 5, 5, [255, 86, 86]);
    // Downward red crash diagonal
    for i in 0..14 {
        c.rect(38 + i, 12 + i, 2, 2, [255, 196, 64]);
    }
    nearest_clamped_rgba(w, h, c.into_bytes())
}

/// Warehouse safety & pick-rate LED scoreboard ("00 DAYS SINCE INJURY // QUOTA: RED").
fn make_injury_board_image() -> Image {
    let (w, h) = (64, 36);
    let mut c = PixelCanvas::new(w, h, [18, 20, 24, 255]);
    // Hazard border
    for x in 0..64 {
        let yel = ((x / 4) % 2) == 0;
        let col = if yel { [238, 186, 28] } else { [24, 26, 30] };
        c.set_rgb(x, 0, col);
        c.set_rgb(x, 1, col);
        c.set_rgb(x, 34, col);
        c.set_rgb(x, 35, col);
    }
    // Left LED box: big glowing red "0" (DAYS WITHOUT INJURY)
    c.rect(4, 5, 22, 25, [10, 12, 15]);
    c.border(4, 5, 22, 25, [238, 186, 28]);
    c.rect(10, 9, 10, 3, [255, 52, 48]);
    c.rect(10, 23, 10, 3, [255, 52, 48]);
    c.rect(10, 9, 3, 17, [255, 52, 48]);
    c.rect(17, 9, 3, 17, [255, 52, 48]);
    // Right LED box: pick-rate bars flashing red/amber
    c.rect(30, 5, 30, 25, [10, 12, 15]);
    c.border(30, 5, 30, 25, [92, 102, 118]);
    c.rect(33, 8, 22, 4, [255, 64, 52]);
    c.rect(33, 15, 16, 4, [246, 182, 36]);
    c.rect(33, 22, 24, 4, [255, 64, 52]);
    nearest_clamped_rgba(w, h, c.into_bytes())
}
