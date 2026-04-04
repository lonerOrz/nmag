/// Shared types passed between state and render modules.
/// This module breaks the render↔state coupling by providing plain data structs.

/// Parameters needed by the renderer for the magnifier effect.
/// Produced by `MagState::params()` and consumed by `WgpuState::render_magnifier()`.
#[derive(Debug, Clone, Copy)]
pub struct MagnifierParams {
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub radius: f32,
    pub zoom: f32,
}

/// Raw screencopy buffer data to be uploaded to the GPU.
/// Returned by `MagState::take_screen_buffer()` instead of passing `WgpuState` into state.
pub struct ScreenData {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
}
