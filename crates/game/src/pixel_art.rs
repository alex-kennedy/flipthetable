use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

#[derive(Resource)]
pub struct ProceduralTextures {
    pub carpet: Handle<Image>,
    pub wood: Handle<Image>,
    pub crt_angry: Handle<Image>,
    pub cat_poster: Handle<Image>,
    pub hazard_stripe: Handle<Image>,
    pub whiteboard_doodle: Handle<Image>,
    pub ceiling_tile: Handle<Image>,
    pub wall_grid: Handle<Image>,
}

pub fn create_procedural_textures(images: &mut Assets<Image>) -> ProceduralTextures {
    ProceduralTextures {
        carpet: images.add(make_carpet_image()),
        wood: images.add(make_wood_image()),
        crt_angry: images.add(make_crt_angry_image()),
        cat_poster: images.add(make_cat_poster_image()),
        hazard_stripe: images.add(make_hazard_stripe_image()),
        whiteboard_doodle: images.add(make_whiteboard_image()),
        ceiling_tile: images.add(make_ceiling_tile_image()),
        wall_grid: images.add(make_wall_grid_image()),
    }
}

fn nearest_rgba_image(width: u32, height: u32, data: Vec<u8>) -> Image {
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

fn make_carpet_image() -> Image {
    let (w, h) = (16, 16);
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let seam = x == 0 || y == 0;
            let checker = ((x / 4) + (y / 4)) % 2 == 0;
            let rgb = if seam {
                [38, 44, 56]
            } else if checker {
                [54, 62, 78]
            } else {
                [49, 57, 72]
            };
            data.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
        }
    }
    nearest_rgba_image(w, h, data)
}

fn make_wood_image() -> Image {
    let (w, h) = (16, 16);
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let grain = (x + y * 2) % 7 == 0;
            let knot = (x == 5 || x == 6) && (y == 9 || y == 10);
            let rgb = if knot {
                [122, 68, 32]
            } else if grain {
                [146, 86, 42]
            } else {
                [168, 102, 52]
            };
            data.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
        }
    }
    nearest_rgba_image(w, h, data)
}

fn make_crt_angry_image() -> Image {
    let (w, h) = (24, 18);
    let mut pixels = vec![[14_u8, 62, 64]; (w * h) as usize];

    let mut set = |x: i32, y: i32, rgb: [u8; 3]| {
        if x >= 0 && x < w as i32 && y >= 0 && y < h as i32 {
            pixels[(y as u32 * w + x as u32) as usize] = rgb;
        }
    };

    for y in 0..h {
        if y % 2 == 0 {
            for x in 0..w {
                set(x as i32, y as i32, [18, 76, 78]);
            }
        }
    }

    let glow = [110_u8, 250, 225];
    for i in 0..4 {
        set(5 + i, 4 + (i / 2), glow);
        set(18 - i, 4 + (i / 2), glow);
    }
    for dy in 0..2 {
        for dx in 0..2 {
            set(7 + dx, 7 + dy, glow);
            set(15 + dx, 7 + dy, glow);
        }
    }
    for x in 8..=15 {
        let curve = if x <= 9 || x >= 14 { 13 } else { 12 };
        set(x, curve, glow);
    }

    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for rgb in pixels {
        data.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
    }
    nearest_rgba_image(w, h, data)
}

fn make_cat_poster_image() -> Image {
    let (w, h) = (16, 16);
    let mut pixels = vec![[235_u8, 228, 210]; (w * h) as usize];
    let mut set = |x: i32, y: i32, rgb: [u8; 3]| {
        if x >= 0 && x < w as i32 && y >= 0 && y < h as i32 {
            pixels[(y as u32 * w + x as u32) as usize] = rgb;
        }
    };

    for i in 0..16 {
        set(i, 0, [130, 78, 38]);
        set(i, 15, [130, 78, 38]);
        set(0, i, [130, 78, 38]);
        set(15, i, [130, 78, 38]);
    }

    let orange = [235_u8, 140, 52];
    for y in 5..=12 {
        for x in 4..=11 {
            set(x, y, orange);
        }
    }
    set(4, 3, orange);
    set(4, 4, orange);
    set(5, 4, orange);
    set(11, 3, orange);
    set(11, 4, orange);
    set(10, 4, orange);

    set(6, 8, [30, 30, 35]);
    set(9, 8, [30, 30, 35]);
    set(7, 10, [220, 95, 110]);
    set(8, 10, [220, 95, 110]);

    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for rgb in pixels {
        data.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
    }
    nearest_rgba_image(w, h, data)
}

fn make_hazard_stripe_image() -> Image {
    let (w, h) = (16, 16);
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let stripe = ((x + y) / 4) % 2 == 0;
            let rgb = if stripe { [242, 191, 42] } else { [28, 32, 40] };
            data.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
        }
    }
    nearest_rgba_image(w, h, data)
}

fn make_whiteboard_image() -> Image {
    let (w, h) = (32, 20);
    let mut pixels = vec![[240_u8, 244, 248]; (w * h) as usize];
    let mut set = |x: i32, y: i32, rgb: [u8; 3]| {
        if x >= 0 && x < w as i32 && y >= 0 && y < h as i32 {
            pixels[(y as u32 * w + x as u32) as usize] = rgb;
        }
    };

    // Border
    for x in 0..32 {
        set(x, 0, [70, 78, 92]);
        set(x, 19, [70, 78, 92]);
    }
    for y in 0..20 {
        set(0, y, [70, 78, 92]);
        set(31, y, [70, 78, 92]);
    }

    // Blue chart line going down
    for x in 4..26 {
        let y = if x < 14 { 6 } else { 6 + (x - 14) / 2 };
        set(x, y.min(16), [40, 120, 220]);
    }
    // Red downward arrow & X
    set(25, 13, [220, 50, 50]);
    set(26, 14, [220, 50, 50]);
    set(27, 13, [220, 50, 50]);
    set(6, 12, [220, 50, 50]);
    set(8, 12, [220, 50, 50]);
    set(7, 13, [220, 50, 50]);
    set(6, 14, [220, 50, 50]);
    set(8, 14, [220, 50, 50]);

    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for rgb in pixels {
        data.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
    }
    nearest_rgba_image(w, h, data)
}

fn make_ceiling_tile_image() -> Image {
    let (w, h) = (16, 16);
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let grid = x == 0 || y == 0;
            let speck = (x * 7 + y * 13) % 19 == 0;
            let rgb = if grid {
                [150, 154, 160]
            } else if speck {
                [198, 200, 204]
            } else {
                [214, 216, 218]
            };
            data.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
        }
    }
    nearest_rgba_image(w, h, data)
}

fn make_wall_grid_image() -> Image {
    let (w, h) = (16, 16);
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let seam = x == 0;
            let rib = x % 4 == 0;
            let rgb = if seam {
                [168, 164, 152]
            } else if rib {
                [194, 189, 176]
            } else {
                [204, 198, 184]
            };
            data.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
        }
    }
    nearest_rgba_image(w, h, data)
}
