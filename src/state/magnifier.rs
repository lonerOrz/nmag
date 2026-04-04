use std::os::fd::{AsFd, OwnedFd};
use std::ptr::NonNull;

use log::{Level, log};
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::protocol::wl_shm_pool::WlShmPool;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, WEnum};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1;
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;

use crate::config;
use crate::error::{MagnifierError, Result};
use crate::types::{MagnifierParams, ScreenData};

/// Raw pixel buffer backed by a memory-mapped shared memory fd.
pub struct ScreenBuf {
    data: NonNull<u8>,
    len: usize,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    _mmap: memmap2::MmapMut,
}

// SAFETY: ScreenBuf owns the mmap'd memory and the NonNull is derived from it.
// Wayland protocol events are sequential on the main thread.
unsafe impl Send for ScreenBuf {}
unsafe impl Sync for ScreenBuf {}

impl ScreenBuf {
    fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.data.as_ptr(), self.len) }
    }
}

/// State for the magnifier: zoom, position, screencopy frame management.
pub struct MagState {
    /// Displayed zoom — the value actually used for rendering.
    pub zoom: f32,
    /// Target zoom — where the displayed zoom is animating towards.
    target_zoom: f32,
    /// True when displayed_zoom has not yet converged to target_zoom.
    animating: bool,
    pub radius: f32,
    pub mouse_x: f64,
    pub mouse_y: f64,
    pub screen_w: u32,
    pub screen_h: u32,
    buffer: Option<ScreenBuf>,
    /// True when a screencopy frame has been fully written and is ready to read.
    pub buffer_ready: bool,
    pub screencopy_mgr: ZwlrScreencopyManagerV1,
    pub shm: WlShm,
    // Kept alive until the compositor releases them (see hyprmag's PoolBuffer pattern)
    _pool: Option<WlShmPool>,
    _buffer: Option<WlBuffer>,
}

impl MagState {
    pub fn new(
        default_zoom: f32,
        default_radius: f32,
        screencopy_mgr: ZwlrScreencopyManagerV1,
        shm: WlShm,
    ) -> Self {
        Self {
            zoom: default_zoom,
            target_zoom: default_zoom,
            animating: false,
            radius: default_radius,
            mouse_x: 0.0,
            mouse_y: 0.0,
            screen_w: 0,
            screen_h: 0,
            buffer: None,
            buffer_ready: false,
            screencopy_mgr,
            shm,
            _pool: None,
            _buffer: None,
        }
    }

    /// Build the parameters needed by the renderer.
    pub fn params(&self) -> MagnifierParams {
        MagnifierParams {
            mouse_x: self.mouse_x as f32,
            mouse_y: self.mouse_y as f32,
            radius: self.radius,
            zoom: self.zoom,
        }
    }

    /// Returns screen buffer data if a screencopy frame is ready.
    pub fn screen_data(&self) -> Option<ScreenData<'_>> {
        let buf = self.buffer.as_ref()?;
        Some(ScreenData {
            data: buf.as_slice(),
            width: buf.width,
            height: buf.height,
            stride: buf.stride,
        })
    }

    /// Set a new target zoom. This starts (or restarts) the smooth animation.
    pub fn set_target_zoom(&mut self, new_zoom: f32) {
        self.target_zoom = new_zoom;
        self.animating = true;
    }

    /// Returns the current target zoom. Useful for computing new targets relative
    /// to the animation endpoint rather than the in-flight displayed value.
    pub fn target_zoom(&self) -> f32 {
        self.target_zoom
    }

    /// Advance the zoom animation by `dt` seconds.
    /// Uses exponential decay: zoom += (target - zoom) * (1 - e^(-k·dt)).
    /// Should be called once per frame before rendering.
    pub fn tick(&mut self, dt: f32) {
        if !self.animating {
            return;
        }
        let diff = self.target_zoom - self.zoom;
        if diff.abs() < config::ZOOM_LOG_THRESHOLD {
            self.zoom = self.target_zoom;
            self.animating = false;
            return;
        }
        // Exponential ease: each step closes a fraction of the remaining gap.
        self.zoom += diff * (1.0 - (-config::ZOOM_EASE_SPEED * dt).exp());

        // Guard against overshoot (e.g. large dt after window unfocus).
        if (self.target_zoom - self.zoom).signum() != diff.signum() {
            self.zoom = self.target_zoom;
            self.animating = false;
        }
    }

    pub fn request_frame(&mut self, qh: &QueueHandle<super::State>, output: &WlOutput) {
        log!(target: "magnifier::sc", Level::Debug, "requesting screencopy");
        let _frame = self.screencopy_mgr.capture_output(1, output, qh, ());
    }
}

fn create_shm_fd() -> Result<OwnedFd> {
    use std::os::unix::io::FromRawFd;
    let name = format!("{}\0", config::SHM_FD_NAME);
    let fd = unsafe { nix::libc::memfd_create(name.as_ptr() as *const _, 0) };
    if fd < 0 {
        return Err(MagnifierError::ShmFdCreateFailed);
    }
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

// ---- Dispatch ----

impl Dispatch<ZwlrScreencopyFrameV1, ()> for super::State {
    fn event(
        state: &mut super::State,
        frame: &ZwlrScreencopyFrameV1,
        event: <ZwlrScreencopyFrameV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<super::State>,
    ) {
        log!(target: "magnifier::sc", Level::Debug, "Frame: {:?}", event);
        use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::Event;
        match event {
            Event::Buffer {
                format,
                width,
                height,
                stride,
            } => {
                let fmt = match format {
                    WEnum::Value(f) => f,
                    WEnum::Unknown(v) => {
                        log!(target: "magnifier::sc", Level::Error, "bad format: {v}");
                        return;
                    }
                };

                let len = (stride * height) as usize;
                let fd = match create_shm_fd() {
                    Ok(fd) => fd,
                    Err(e) => {
                        log!(target: "magnifier::sc", Level::Error, "{e}");
                        return;
                    }
                };
                if let Err(e) = nix::unistd::ftruncate(&fd, len as i64) {
                    log!(target: "magnifier::sc", Level::Error, "ftruncate: {e}");
                    return;
                }

                // Create wl_buffer and keep it alive until Ready
                let pool = state.mag.shm.create_pool(fd.as_fd(), len as i32, qh, ());
                let buf =
                    pool.create_buffer(0, width as i32, height as i32, stride as i32, fmt, qh, ());
                frame.copy(&buf);
                // DON'T destroy — compositor writes asynchronously
                state.mag._pool = Some(pool);
                state.mag._buffer = Some(buf);

                // Now mmap
                let mmap = match unsafe { memmap2::MmapOptions::new().len(len).map_mut(&fd) } {
                    Ok(mmap) => mmap,
                    Err(e) => {
                        log!(target: "magnifier::sc", Level::Error, "mmap: {e}");
                        return;
                    }
                };
                let data = match NonNull::new(mmap.as_ptr() as *mut u8) {
                    Some(p) => p,
                    None => {
                        log!(target: "magnifier::sc", Level::Error, "mmap returned null pointer");
                        return;
                    }
                };

                state.mag.buffer = Some(ScreenBuf {
                    data,
                    len,
                    width,
                    height,
                    stride,
                    _mmap: mmap,
                });
                state.mag.screen_w = width;
                state.mag.screen_h = height;
            }
            Event::Ready {
                tv_sec_hi: _,
                tv_sec_lo: _,
                tv_nsec: _,
            } => {
                log!(target: "magnifier::sc", Level::Debug, "frame ready");
                state.mag.buffer_ready = true;
                // Clean up wayland objects — compositor is done writing
                state.mag._buffer.take();
                state.mag._pool.take();
            }
            Event::Failed => {
                log!(target: "magnifier::sc", Level::Error, "screencopy failed");
            }
            Event::Damage { .. } | Event::BufferDone => {}
            _ => {}
        }
    }
}

impl Dispatch<WlShmPool, ()> for super::State {
    fn event(
        _s: &mut super::State,
        _p: &WlShmPool,
        event: <WlShmPool as Proxy>::Event,
        _d: &(),
        _c: &Connection,
        _q: &QueueHandle<super::State>,
    ) {
        log!(target: "magnifier::sc", Level::Debug, "ShmPool: {:?}", event);
    }
}

impl Dispatch<WlBuffer, ()> for super::State {
    fn event(
        _s: &mut super::State,
        _p: &WlBuffer,
        event: <WlBuffer as Proxy>::Event,
        _d: &(),
        _c: &Connection,
        _q: &QueueHandle<super::State>,
    ) {
        log!(target: "magnifier::sc", Level::Debug, "Buffer: {:?}", event);
    }
}
