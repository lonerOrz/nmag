//! Shared types passed between state and render modules.
//! This module breaks the render↔state coupling by providing plain data structs.

/// Parameters needed by the renderer for the magnifier effect.
/// Produced by `MagState::params()` and consumed by `WgpuState::render_magnifier()`.
#[derive(Debug, Clone, Copy)]
pub struct MagnifierParams {
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub radius: f32,
    pub zoom: f32,
    /// Accumulated pan offset in screen pixels.
    pub pan_x: f32,
    pub pan_y: f32,
}

/// A borrowed view of a screencopy buffer ready for GPU upload.
/// Carries no allocation — just a slice reference and dimensions.
#[derive(Debug, Clone, Copy)]
pub struct ScreenData<'a> {
    pub data: &'a [u8],
    pub width: u32,
    pub height: u32,
    pub stride: u32,
}
