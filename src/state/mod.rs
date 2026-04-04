pub mod magnifier;
mod mouse;

use wayland_client::delegate_dispatch;
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};

use wayland_client::protocol::wl_callback::WlCallback;
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_display::WlDisplay;
use wayland_client::protocol::wl_keyboard::WlKeyboard;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_pointer::WlPointer;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::protocol::wl_surface::WlSurface;

use wayland_protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::WpCursorShapeDeviceV1;
use wayland_protocols::wp::cursor_shape::v1::client::wp_cursor_shape_manager_v1::WpCursorShapeManagerV1;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::ZwlrLayerSurfaceV1;
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;

use log::{Level, log};

use crate::config;
use crate::error::{MagnifierError, Result};
use crate::render::WgpuState;

macro_rules! delegate_log {
    ($proxy:ty) => {
        impl Dispatch<$proxy, ()> for State {
            fn event(
                _state: &mut Self, _proxy: &$proxy,
                event: <$proxy as Proxy>::Event, _data: &(),
                _conn: &Connection, _qhandle: &QueueHandle<Self>,
            ) {
                log!(target: "magnifier::wl", Level::Trace, "{}: {:?}", std::any::type_name::<$proxy>(), event);
            }
        }
    };
}

/// Collects Wayland globals during the initial roundtrip.
#[derive(Default)]
struct SetupState {
    compositor: Option<WlCompositor>,
    surface: Option<WlSurface>,
    seat: Option<WlSeat>,
    layer_shell: Option<ZwlrLayerShellV1>,
    layer_surface: Option<ZwlrLayerSurfaceV1>,
    cursor_mgr: Option<WpCursorShapeManagerV1>,
    screencopy_mgr: Option<ZwlrScreencopyManagerV1>,
    shm: Option<WlShm>,
    output: Option<WlOutput>,
}

impl SetupState {
    fn into_state(self, connection: Connection, display: WlDisplay) -> Result<Wl> {
        Ok(Wl {
            _connection: connection,
            _display: display,
            _compositor: require_global(self.compositor, "wl_compositor")?,
            surface: require_global(self.surface, "wl_surface")?,
            _seat: require_global(self.seat, "wl_seat")?,
            _layer_shell: require_global(self.layer_shell, "zwlr_layer_shell_v1")?,
            _layer_surface: require_global(self.layer_surface, "zwlr_layer_surface_v1")?,
            cursor_mgr: require_global(self.cursor_mgr, "wp_cursor_shape_manager_v1")?,
            screencopy_mgr: require_global(self.screencopy_mgr, "zwlr_screencopy_manager_v1")?,
            shm: require_global(self.shm, "wl_shm")?,
            output: require_global(self.output, "wl_output")?,
        })
    }
}

/// Helper to produce a clear error when a required global is missing.
fn require_global<T>(opt: Option<T>, name: &'static str) -> Result<T> {
    opt.ok_or(MagnifierError::WaylandGlobalMissing { global: name })
}

impl Dispatch<WlRegistry, QueueHandle<State>> for SetupState {
    fn event(
        s: &mut Self,
        reg: &WlRegistry,
        event: <WlRegistry as Proxy>::Event,
        qh: &QueueHandle<State>,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        use wayland_client::protocol::wl_registry::Event;
        if let Event::Global {
            name,
            interface,
            version: _,
        } = event
        {
            match interface.as_str() {
                "wl_compositor" => {
                    let c = reg.bind::<WlCompositor, _, _>(name, 5, qh, ());
                    let sf = c.create_surface(qh, ());
                    s.compositor = Some(c);
                    s.surface = Some(sf);
                }
                "wl_seat" => {
                    s.seat = Some(reg.bind::<WlSeat, _, _>(name, 9, qh, ()));
                }
                "wl_output" => {
                    if s.output.is_none() {
                        s.output = Some(reg.bind::<WlOutput, _, _>(name, 4, qh, ()));
                    }
                }
                "zwlr_layer_shell_v1" => {
                    use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::Layer;
                    use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::{
                        Anchor, KeyboardInteractivity,
                    };
                    let sf = s.surface.as_ref().expect("surface not available");
                    let ls = reg.bind::<ZwlrLayerShellV1, _, _>(name, 4, qh, ());
                    let lsf = ls.get_layer_surface(
                        sf,
                        None,
                        Layer::Overlay,
                        config::LAYER_SURFACE_NAMESPACE.into(),
                        qh,
                        (),
                    );
                    lsf.set_anchor(Anchor::all());
                    lsf.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
                    lsf.set_exclusive_zone(-1);
                    s.layer_shell = Some(ls);
                    s.layer_surface = Some(lsf);
                }
                "wp_cursor_shape_manager_v1" => {
                    s.cursor_mgr = Some(reg.bind::<WpCursorShapeManagerV1, _, _>(name, 1, qh, ()));
                }
                "zwlr_screencopy_manager_v1" => {
                    s.screencopy_mgr =
                        Some(reg.bind::<ZwlrScreencopyManagerV1, _, _>(name, 3, qh, ()));
                }
                "wl_shm" => {
                    s.shm = Some(reg.bind::<WlShm, _, _>(name, 1, qh, ()));
                }
                _ => {}
            }
        }
    }
}

/// Top-level application state.
pub struct State {
    wl: Option<Wl>,
    pub mag: magnifier::MagState,
    pub mouse: mouse::MouseState,
    wgpu: Option<WgpuState>,
    qhandle: Option<QueueHandle<Self>>,
    screencopy_pending: bool,
    pub quit: bool,
    screen_configured: bool,
    has_clean_capture: bool,
    /// Last frame callback timestamp in seconds (monotonic, from compositor).
    last_frame_time: f32,
}

impl State {
    pub fn setup(zoom: f32) -> (Self, EventQueue<Self>) {
        let conn = Connection::connect_to_env().expect("Wayland connection failed");
        let mut setup_q = conn.new_event_queue();
        let eq = conn.new_event_queue();

        let display = conn.display();
        let _reg = display.get_registry(&setup_q.handle(), eq.handle());

        let mut tmp = SetupState::default();
        setup_q
            .roundtrip(&mut tmp)
            .expect("Initial roundtrip failed");

        let wl = tmp
            .into_state(conn, display)
            .expect("Wayland globals negotiation failed");
        wl.surface.frame(&eq.handle(), ());
        wl.surface.commit();

        let mag = magnifier::MagState::new(zoom, wl.screencopy_mgr.clone(), wl.shm.clone());

        (
            Self {
                wl: Some(wl),
                mag,
                mouse: mouse::MouseState::default(),
                wgpu: None,
                qhandle: Some(eq.handle()),
                screencopy_pending: false,
                quit: false,
                screen_configured: false,
                has_clean_capture: false,
                last_frame_time: 0.0,
            },
            eq,
        )
    }

    fn request_screencopy(&mut self) {
        if self.screencopy_pending {
            return;
        }
        let Some(wl) = &self.wl else { return };
        let Some(qh) = &self.qhandle else { return };
        self.mag.request_frame(qh, &wl.output);
        self.screencopy_pending = true;
    }

    fn render(&mut self) {
        if !self.screen_configured || self.wgpu.is_none() {
            return;
        }

        // Upload screencopy data if available
        if self.mag.buffer_ready {
            self.mag.buffer_ready = false;
            let Some(wgpu) = self.wgpu.as_mut() else {
                return;
            };
            if let Some(screen) = self.mag.screen_data() {
                wgpu.upload_screen_texture(screen.width, screen.height, screen.stride, screen.data);
                self.screencopy_pending = false;
                self.has_clean_capture = true;
            }
        }

        // Don't draw until we have a clean screencopy capture.
        // This prevents showing the red placeholder texture.
        if !self.has_clean_capture {
            let Some(wl) = &self.wl else { return };
            let Some(qh) = &self.qhandle else { return };
            wl.surface.frame(qh, ());
            wl.surface.commit();
            return;
        }

        // Draw with captured screen texture
        let wgpu = self.wgpu.as_ref().unwrap();
        wgpu.render_magnifier(&self.mag.params());

        let Some(wl) = &self.wl else { return };
        let Some(qh) = &self.qhandle else { return };
        wl.surface.frame(qh, ());
        wl.surface.commit();
    }
}

impl Drop for State {
    fn drop(&mut self) {
        self.wgpu.take();
    }
}

/// Wayland protocol objects kept alive for the lifetime of the application.
/// Fields prefixed with `_` are stored solely to prevent the proxy from
/// being dropped (which would send a destroy request to the compositor).
pub struct Wl {
    _connection: Connection,
    _display: WlDisplay,
    _compositor: WlCompositor,
    pub surface: WlSurface,
    _seat: WlSeat,
    _layer_shell: ZwlrLayerShellV1,
    _layer_surface: ZwlrLayerSurfaceV1,
    pub cursor_mgr: WpCursorShapeManagerV1,
    pub screencopy_mgr: ZwlrScreencopyManagerV1,
    pub shm: WlShm,
    pub output: WlOutput,
}

delegate_log!(WlCompositor);
delegate_log!(WlSurface);
delegate_log!(WlOutput);

impl Dispatch<WlSeat, ()> for State {
    fn event(
        state: &mut Self,
        seat: &WlSeat,
        event: <WlSeat as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        log!(target: "magnifier::wl", Level::Debug, "WlSeat: {:?}", event);
        use wayland_client::protocol::wl_seat::{Capability, Event};
        if let Event::Capabilities {
            capabilities: wayland_client::WEnum::Value(caps),
        } = event
            && caps.contains(Capability::Pointer)
        {
            let ptr = seat.get_pointer(qh, ());
            let dev = state
                .wl
                .as_ref()
                .unwrap()
                .cursor_mgr
                .get_pointer(&ptr, qh, ());
            state.mouse.set_cursor_shape_device(dev);
        }
        if let Event::Capabilities {
            capabilities: wayland_client::WEnum::Value(caps),
        } = event
            && caps.contains(Capability::Keyboard)
        {
            seat.get_keyboard(qh, ());
        }
    }
}

impl Dispatch<WlCallback, ()> for State {
    fn event(
        state: &mut Self,
        _cb: &WlCallback,
        event: <WlCallback as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        log!(target: "magnifier::wl", Level::Info, "WlCallback: {:?}", event);
        if let wayland_client::protocol::wl_callback::Event::Done { callback_data } = event {
            let now_s = callback_data as f32 / 1000.0;
            let dt = if state.last_frame_time > 0.0 {
                // Clamp to avoid huge jumps (e.g. after focus loss).
                (now_s - state.last_frame_time).clamp(0.001, 0.1)
            } else {
                config::ASSUMED_DT
            };
            state.last_frame_time = now_s;

            // Advance zoom animation before rendering.
            state.mag.tick(dt);
            state.render();
        }
    }
}

delegate_log!(ZwlrLayerShellV1);
impl Dispatch<ZwlrLayerSurfaceV1, ()> for State {
    fn event(
        state: &mut Self,
        lsf: &ZwlrLayerSurfaceV1,
        event: <ZwlrLayerSurfaceV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        log!(target: "magnifier::wl", Level::Info, "LayerSurface: {:?}", event);
        use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::Event;
        if let Event::Configure {
            serial,
            width,
            height,
        } = event
        {
            lsf.ack_configure(serial);
            if state.wgpu.is_none() {
                state.wgpu = Some(WgpuState::new(
                    &state.wl.as_ref().unwrap()._display,
                    &state.wl.as_ref().unwrap().surface,
                    width,
                    height,
                ));
                state.screen_configured = true;
                state.request_screencopy();
                let qh = state.qhandle.as_ref().unwrap().clone();
                state.wl.as_ref().unwrap().surface.frame(&qh, ());
                state.wl.as_ref().unwrap().surface.commit();
            }
        }
    }
}

delegate_dispatch!(State: [WlPointer: ()] => mouse::MouseState);
delegate_log!(WpCursorShapeManagerV1);
delegate_log!(WpCursorShapeDeviceV1);
delegate_log!(ZwlrScreencopyManagerV1);
delegate_log!(WlShm);

impl Dispatch<WlKeyboard, ()> for State {
    fn event(
        state: &mut Self,
        _kbd: &WlKeyboard,
        event: <WlKeyboard as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        use wayland_client::protocol::wl_keyboard::Event;
        if let Event::Key {
            key,
            state: key_state,
            ..
        } = event
            && let wayland_client::WEnum::Value(
                wayland_client::protocol::wl_keyboard::KeyState::Pressed,
            ) = key_state
            && key == config::KEY_ESCAPE_SCANCODE
        {
            log!(target: "magnifier::wl", Level::Info, "Escape pressed, exiting");
            state.quit = true;
        }
    }
}
