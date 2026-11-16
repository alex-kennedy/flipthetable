import bevy_sprite_render::mesh2d::vertex_output::VertexOutput;
@if(SRGB_OUTPUT)
import bevy_render::color_operations::linear_to_srgb;

struct RetroPostUniform {
    // x = tex_w, y = tex_h, z = color_levels, w = dither_strength
    params: vec4<f32>,
    // xyz = shadow tint (linear), w = bloom_strength
    shadow_tint: vec4<f32>,
    // xyz = highlight tint (linear), w = vignette_strength
    highlight_tint: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> material: RetroPostUniform;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var scene_tex: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var scene_sampler: sampler;

fn bayer4x4(px: u32, py: u32) -> f32 {
    let x = px & 3u;
    let y = py & 3u;
    let idx = y * 4u + x;
    var m: f32 = 0.0;
    switch idx {
        case 0u:  { m = 0.0; }
        case 1u:  { m = 8.0; }
        case 2u:  { m = 2.0; }
        case 3u:  { m = 10.0; }
        case 4u:  { m = 12.0; }
        case 5u:  { m = 4.0; }
        case 6u:  { m = 14.0; }
        case 7u:  { m = 6.0; }
        case 8u:  { m = 3.0; }
        case 9u:  { m = 11.0; }
        case 10u: { m = 1.0; }
        case 11u: { m = 9.0; }
        case 12u: { m = 15.0; }
        case 13u: { m = 7.0; }
        case 14u: { m = 13.0; }
        default:  { m = 5.0; }
    }
    return (m / 16.0) - 0.5;
}

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn extract_bright(c: vec3<f32>) -> vec3<f32> {
    let l = luma(c);
    let knee = smoothstep(0.42, 0.92, l);
    return c * knee;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let res = max(material.params.xy, vec2<f32>(64.0, 36.0));
    let inv_res = 1.0 / res;

    // Snap UV strictly to low-res pixel centers so upscaled pixels are razor-sharp.
    let pixel_coord = floor(clamp(in.uv, vec2<f32>(0.0), vec2<f32>(0.99999)) * res);
    let center_uv = (pixel_coord + vec2<f32>(0.5, 0.5)) * inv_res;

    let base = textureSample(scene_tex, scene_sampler, center_uv).rgb;

    // Sample immediate cardinal + diagonal low-res neighbors for pixel phosphor glow & edge crispness.
    let n_e  = textureSample(scene_tex, scene_sampler, clamp(center_uv + vec2<f32>( inv_res.x, 0.0), vec2<f32>(0.001), vec2<f32>(0.999))).rgb;
    let n_w  = textureSample(scene_tex, scene_sampler, clamp(center_uv + vec2<f32>(-inv_res.x, 0.0), vec2<f32>(0.001), vec2<f32>(0.999))).rgb;
    let n_n  = textureSample(scene_tex, scene_sampler, clamp(center_uv + vec2<f32>(0.0, -inv_res.y), vec2<f32>(0.001), vec2<f32>(0.999))).rgb;
    let n_s  = textureSample(scene_tex, scene_sampler, clamp(center_uv + vec2<f32>(0.0,  inv_res.y), vec2<f32>(0.001), vec2<f32>(0.999))).rgb;
    let n_ne = textureSample(scene_tex, scene_sampler, clamp(center_uv + vec2<f32>( inv_res.x * 2.0, -inv_res.y * 2.0), vec2<f32>(0.001), vec2<f32>(0.999))).rgb;
    let n_nw = textureSample(scene_tex, scene_sampler, clamp(center_uv + vec2<f32>(-inv_res.x * 2.0, -inv_res.y * 2.0), vec2<f32>(0.001), vec2<f32>(0.999))).rgb;
    let n_se = textureSample(scene_tex, scene_sampler, clamp(center_uv + vec2<f32>( inv_res.x * 2.0,  inv_res.y * 2.0), vec2<f32>(0.001), vec2<f32>(0.999))).rgb;
    let n_sw = textureSample(scene_tex, scene_sampler, clamp(center_uv + vec2<f32>(-inv_res.x * 2.0,  inv_res.y * 2.0), vec2<f32>(0.001), vec2<f32>(0.999))).rgb;

    // Pixel-space phosphor highlight halation (CRT cyan, golden blinds, electric sparks, explosions).
    let glow = (
        extract_bright(n_e) * 0.18 +
        extract_bright(n_w) * 0.18 +
        extract_bright(n_n) * 0.18 +
        extract_bright(n_s) * 0.18 +
        extract_bright(n_ne) * 0.07 +
        extract_bright(n_nw) * 0.07 +
        extract_bright(n_se) * 0.07 +
        extract_bright(n_sw) * 0.07
    );
    var col = base + glow * material.shadow_tint.w;

    // Subtle pixel-art dark-edge accentuation when adjacent to brighter surfaces.
    let base_l = luma(base);
    let max_neighbor_l = max(max(luma(n_e), luma(n_w)), max(luma(n_n), luma(n_s)));
    let edge_contrast = clamp((max_neighbor_l - base_l) * 1.35, 0.0, 0.35);
    col = col * (1.0 - edge_contrast * 0.28);

    // Split-tone color grading: moody slate-navy shadows & warm golden highlights (like loading-desk.jpg).
    let lum = clamp(luma(col), 0.0, 1.0);
    let shadow_w = (1.0 - smoothstep(0.0, 0.36, lum)) * 0.34;
    let highlight_w = smoothstep(0.38, 0.95, lum) * 0.18;
    col = mix(col, col * material.shadow_tint.xyz + material.shadow_tint.xyz * 0.035, shadow_w);
    col = mix(col, col * material.highlight_tint.xyz, highlight_w);

    // Convert linear -> perceptual gamma (~sRGB) BEFORE dithering & palette quantization so steps are perceptually even!
    var srgb = pow(max(col, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2));

    let levels = max(material.params.z, 8.0);
    let dither = bayer4x4(u32(pixel_coord.x), u32(pixel_coord.y)) * (material.params.w / levels);
    srgb = clamp(floor((srgb + vec3<f32>(dither)) * levels + 0.5) / levels, vec3<f32>(0.0), vec3<f32>(1.0));

    // Subtle retro corner vignette
    let centered = in.uv - vec2<f32>(0.5, 0.5);
    let vig = clamp(1.0 - dot(centered, centered) * material.highlight_tint.w, 0.58, 1.0);
    srgb = srgb * vig;

    // Back to linear for Bevy's output stage.
    var out_lin = pow(srgb, vec3<f32>(2.2));
@if(SRGB_OUTPUT)
    out_lin = linear_to_srgb(out_lin);

    return vec4<f32>(out_lin, 1.0);
}
