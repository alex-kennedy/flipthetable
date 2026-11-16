use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};

pub const PIXEL_WIDTH: u32 = 480;
pub const PIXEL_HEIGHT: u32 = 270;

#[derive(Resource)]
pub struct PixelRenderTarget {
    pub image: Handle<Image>,
}

#[derive(Resource)]
pub struct ProceduralTextures {
    pub carpet: Handle<Image>,
    pub wood: Handle<Image>,
    pub crt_angry: Handle<Image>,
    pub cat_poster: Handle<Image>,
}

pub fn create_pixel_render_target(images: &mut Assets<Image>) -> Handle<Image> {
    let size = Extent3d {
        width: PIXEL_WIDTH,
        height: PIXEL_HEIGHT,
        depth_or_array_layers: 1,
    };
    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
        | TextureUsages::COPY_DST
        | TextureUsages::RENDER_ATTACHMENT;
    image.sampler = ImageSampler::nearest();
    images.add(image)
}

pub fn create_procedural_textures(images: &mut Assets<Image>) -> ProceduralTextures {
    ProceduralTextures {
        carpet: images.add(make_carpet_image()),
        wood: images.add(make_wood_image()),
        crt_angry: images.add(make_crt_angry_image()),
        cat_poster: images.add(make_cat_poster_image()),
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

    // Subtle scanline tint
    for y in 0..h {
        if y % 2 == 0 {
            for x in 0..w {
                set(x as i32, y as i32, [18, 76, 78]);
            }
        }
    }

    let glow = [110_u8, 250, 225];
    // Angry eyebrows
    for i in 0..4 {
        set(5 + i, 4 + (i / 2), glow);
        set(18 - i, 4 + (i / 2), glow);
    }
    // Eyes
    for dy in 0..2 {
        for dx in 0..2 {
            set(7 + dx, 7 + dy, glow);
            set(15 + dx, 7 + dy, glow);
        }
    }
    // Frown
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

    // Wooden frame border
    for i in 0..16 {
        set(i, 0, [130, 78, 38]);
        set(i, 15, [130, 78, 38]);
        set(0, i, [130, 78, 38]);
        set(15, i, [130, 78, 38]);
    }

    // Orange cat head & ears
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

    // Eyes & nose
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
