/// Configuration constants for the magnifier.

// Zoom range limits
pub const ZOOM_MIN: f32 = 0.1;
pub const ZOOM_MAX: f32 = 50.0;

// Zoom scroll factor: new_zoom = old_zoom * ZOOM_FACTOR_BASE ^ (-delta / ZOOM_DIVISOR)
pub const ZOOM_FACTOR_BASE: f64 = 2.0;
pub const ZOOM_DIVISOR: f64 = 100.0;

// Zoom delta threshold for logging (avoids spam on tiny changes)
pub const ZOOM_LOG_THRESHOLD: f32 = 0.001;

// Shader visual parameters (must match shader.wgsl values)
// NOTE: border and soft-edge are currently hardcoded in shader.wgsl;
// these constants are here for future uniform injection.
#[allow(dead_code)]
pub const MAGNIFIER_BORDER: f32 = 3.0;
#[allow(dead_code)]
pub const MAGNIFIER_SOFT_EDGE: f32 = 8.0;

// Default CLI values
pub const DEFAULT_ZOOM: f32 = 2.0;
pub const DEFAULT_RADIUS: f32 = 150.0;

// Input scancodes / button codes
pub const BTN_LEFT: u32 = 272;
pub const KEY_ESCAPE_SCANCODE: u32 = 1;

// Shared memory
pub const SHM_FD_NAME: &str = "magnifier";

// Layer shell
pub const LAYER_SURFACE_NAMESPACE: &str = "magnifier";
