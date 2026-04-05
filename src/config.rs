// Configuration constants for the magnifier.

// Zoom range limits
pub const ZOOM_MIN: f32 = 0.1;
pub const ZOOM_MAX: f32 = 50.0;

// Zoom scroll: new_zoom = old_zoom * 2^(±1/ZOOM_DIVISOR) per notch.
// 30 → ~2.3% per notch, responsive but not too aggressive.
pub const ZOOM_DIVISOR: f64 = 30.0;

// Zoom delta threshold for logging (avoids spam on tiny changes)
pub const ZOOM_LOG_THRESHOLD: f32 = 0.001;

// Zoom animation: exponential ease speed (units per second).
// Higher = snappier. ~12 gives a natural ~300ms transition for a 2× zoom jump.
pub const ZOOM_EASE_SPEED: f32 = 20.0;

// Assumed frame interval for fixed-dt animation (60 fps).
// The exponential ease is insensitive to small dt variations.
pub const ASSUMED_DT: f32 = 1.0 / 60.0;

// Default CLI values
pub const DEFAULT_ZOOM: f32 = 2.0;

// Input scancodes / button codes (from linux/input-event-codes.h)
pub const BTN_LEFT: u32 = 272; // BTN_LEFT
pub const BTN_RIGHT: u32 = 273; // BTN_RIGHT
pub const BTN_MIDDLE: u32 = 274; // BTN_MIDDLE
pub const KEY_ESCAPE_SCANCODE: u32 = 1;

// Shared memory
pub const SHM_FD_NAME: &str = "nmag";

// Layer shell
pub const LAYER_SURFACE_NAMESPACE: &str = "nmag";
