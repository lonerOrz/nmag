struct Uniform {
    screen_size: vec2<f32>,
    mouse_pos: vec2<f32>,
    magnifier_radius: f32,
    zoom: f32,
    _pad: vec2<f32>,
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
    // Circle position: flip Y because Wayland Y-down ≠ NDC Y-up
    let mouse = vec2<f32>(u.mouse_pos.x, u.screen_size.y - u.mouse_pos.y);

    let dist = distance(input.screen_pos, mouse);

    if (dist > u.magnifier_radius) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    let border = 3.0;
    let inner_r = u.magnifier_radius - border;
    if (dist > inner_r) {
        return vec4<f32>(0.0, 1.0, 1.0, 1.0);
    }

    let rel = (input.screen_pos - mouse) / u.magnifier_radius;
    let zoomed = rel / u.zoom;
    let sample_px = mouse + zoomed * u.magnifier_radius;

    // Texture UV: flip Y to fix mirrored content
    let tex_uv = vec2<f32>(
        sample_px.x / u.screen_size.x,
        1.0 - sample_px.y / u.screen_size.y,
    );

    let color = textureSample(screen_texture, samp, tex_uv);

    let a = 1.0 - smoothstep(inner_r - 8.0, inner_r, dist);
    return vec4<f32>(color.rgb * a, a);
}
