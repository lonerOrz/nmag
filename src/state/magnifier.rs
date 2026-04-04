use std::os::fd::{AsFd, OwnedFd};
use std::ptr::NonNull;

use log::{Level, log};
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::protocol::wl_shm_pool::WlShmPool;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, WEnum};

use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1,
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

use crate::render::WgpuState;

pub struct ScreenBuf {
    pub data: NonNull<u8>,
    pub len: usize,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    _mmap: memmap2::MmapMut,
}

unsafe impl Send for ScreenBuf {}
unsafe impl Sync for ScreenBuf {}

pub struct MagState {
    pub zoom: f32,
    pub radius: f32,
    pub mouse_x: f64,
    pub mouse_y: f64,
    pub screen_w: u32,
    pub screen_h: u32,
    pub buffer: Option<ScreenBuf>,
    pub buffer_ready: bool,
    pub screencopy_mgr: Option<ZwlrScreencopyManagerV1>,
    pub shm: Option<WlShm>,
    // Keep alive until compositor releases them (see hyprmagnifier PoolBuffer)
    _pool: Option<WlShmPool>,
    _buffer: Option<WlBuffer>,
}

impl MagState {
    pub fn new() -> Self {
        Self {
            zoom: 2.0,
            radius: 150.0,
            mouse_x: 0.0,
            mouse_y: 0.0,
            screen_w: 0,
            screen_h: 0,
            buffer: None,
            buffer_ready: false,
            screencopy_mgr: None,
            shm: None,
            _pool: None,
            _buffer: None,
        }
    }

    pub fn request_frame(&mut self, qh: &QueueHandle<super::State>, output: &WlOutput) {
        let Some(mgr) = &self.screencopy_mgr else {
            return;
        };
        log!(target: "magnifier::sc", Level::Debug, "requesting screencopy");
        let _frame = mgr.capture_output(1, output, qh, ());
    }

    pub fn upload_to_wgpu(&self, wgpu: &mut WgpuState) {
        let Some(ref buf) = self.buffer else { return };
        let slice = unsafe { std::slice::from_raw_parts(buf.data.as_ptr(), buf.len) };
        wgpu.upload_screen_texture(buf.width, buf.height, buf.stride, slice);
    }
}

fn create_shm_fd() -> std::io::Result<OwnedFd> {
    use std::os::unix::io::FromRawFd;
    let name = b"magnifier\0";
    let fd = unsafe { nix::libc::memfd_create(name.as_ptr() as *const _, 0) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
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
                        log!(target: "magnifier::sc", Level::Error, "bad format: {}", v);
                        return;
                    }
                };

                let len = (stride * height) as usize;
                let fd = create_shm_fd().expect("shm fd");
                nix::unistd::ftruncate(&fd, len as i64).expect("ftruncate");

                // Create wl_buffer and keep it alive until Ready (see hyprmagnifier PoolBuffer)
                if let Some(ref shm) = state.mag.shm {
                    let pool = shm.create_pool(fd.as_fd(), len as i32, qh, ());
                    let buf = pool.create_buffer(
                        0,
                        width as i32,
                        height as i32,
                        stride as i32,
                        fmt,
                        qh,
                        (),
                    );
                    frame.copy(&buf);
                    // DON'T destroy — compositor writes asynchronously
                    state.mag._pool = Some(pool);
                    state.mag._buffer = Some(buf);
                }

                // Now mmap
                let mmap = unsafe {
                    memmap2::MmapOptions::new()
                        .len(len)
                        .map_mut(&fd)
                        .expect("mmap")
                };
                let data = NonNull::new(mmap.as_ptr() as *mut u8).unwrap();

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
