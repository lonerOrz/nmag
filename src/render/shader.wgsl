struct Uniform {
    screen_size: vec2<f32>,
    mouse_pos: vec2<f32>,
    zoom_center: vec2<f32>,
    magnifier_radius: f32,
    zoom: f32,
    _pad: vec2<f32>,
    pan_offset: vec2<f32>,
};

@group(0) @binding(0)
var screen_texture: texture_2d<f32>;

@group(0) @binding(1)
var samp: sampler;

@group(0) @binding(2)
var<uniform> u: Uniform;

struct VSIn {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) screen_pos: vec2<f32>,
};

@vertex
fn vs_main(input: VSIn) -> VSOut {
    var out: VSOut;
    let nx = (input.pos.x * 2.0) / u.screen_size.x - 1.0;
    let ny = (input.pos.y * 2.0) / u.screen_size.y - 1.0;
    out.pos = vec4<f32>(nx, ny, 0.0, 1.0);
    out.uv = input.uv;
    out.screen_pos = input.pos;
    return out;
}

@fragment
fn fs_main(input: VSOut) -> @location(0) vec4<f32> {
    var center: vec2<f32>;
    if (u.zoom_center.x >= 0.0) {
        center = u.zoom_center;
    } else {
        center = u.screen_size * 0.5;
    }
    let screen_px = input.uv * u.screen_size;

    let pan = vec2<f32>(u.pan_offset.x, -u.pan_offset.y);
    let zoomed_px = center + (screen_px - center - pan) / u.zoom;

    let tex_uv = vec2<f32>(
        zoomed_px.x / u.screen_size.x,
        1.0 - zoomed_px.y / u.screen_size.y,
    );

    return textureSample(screen_texture, samp, tex_uv);
}
