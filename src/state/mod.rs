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
    fn into_state(self, connection: Connection, display: WlDisplay) -> Wl {
        Wl {
            connection,
            display,
            compositor: self.compositor.unwrap(),
            surface: self.surface.unwrap(),
            seat: self.seat.unwrap(),
            layer_shell: self.layer_shell.unwrap(),
            layer_surface: self.layer_surface.unwrap(),
            cursor_mgr: self.cursor_mgr.unwrap(),
            screencopy_mgr: self.screencopy_mgr.unwrap(),
            shm: self.shm.unwrap(),
            output: self.output.unwrap(),
        }
    }
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
                    let sf = s.surface.as_ref().unwrap();
                    let ls = reg.bind::<ZwlrLayerShellV1, _, _>(name, 4, qh, ());
                    let lsf =
                        ls.get_layer_surface(sf, None, Layer::Overlay, "magnifier".into(), qh, ());
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
}

impl State {
    pub fn setup(zoom: f32, radius: f32) -> (Self, EventQueue<Self>) {
        let conn = Connection::connect_to_env().unwrap();
        let mut setup_q = conn.new_event_queue();
        let eq = conn.new_event_queue();

        let display = conn.display();
        let _reg = display.get_registry(&setup_q.handle(), eq.handle());

        let mut tmp = SetupState::default();
        setup_q.roundtrip(&mut tmp).unwrap();

        let wl = tmp.into_state(conn, display);
        wl.surface.frame(&eq.handle(), ());
        wl.surface.commit();

        let mut mag = magnifier::MagState::new();
        mag.zoom = zoom;
        mag.radius = radius;
        mag.screencopy_mgr = Some(wl.screencopy_mgr.clone());
        mag.shm = Some(wl.shm.clone());

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
        if !self.screen_configured {
            return;
        }
        if self.wgpu.is_none() {
            return;
        }

        // Upload screencopy data if available
        if self.mag.buffer_ready {
            self.mag.upload_to_wgpu(self.wgpu.as_mut().unwrap());
            self.mag.buffer_ready = false;
            self.screencopy_pending = false;
            self.has_clean_capture = true;
        }

        // Don't draw until we have a clean screencopy capture.
        // This prevents showing the red placeholder texture.
        if !self.has_clean_capture {
            // Still commit the frame callback so the render loop keeps going
            let Some(wl) = &self.wl else { return };
            let Some(qh) = &self.qhandle else { return };
            wl.surface.frame(qh, ());
            wl.surface.commit();
            return;
        }

        // Draw with captured screen texture
        let wgpu = self.wgpu.as_ref().unwrap();
        wgpu.render_magnifier(&self.mag);

        let Some(wl) = &self.wl else { return };
        let Some(qh) = &self.qhandle else { return };
        wl.surface.frame(qh, ());
        wl.surface.commit();
    }

    #[allow(dead_code)]
    fn exit(&mut self) {
        self.quit = true;
    }
}

impl Drop for State {
    fn drop(&mut self) {
        self.wgpu.take();
    }
}

pub struct Wl {
    #[allow(dead_code)]
    connection: Connection,
    #[allow(dead_code)]
    display: WlDisplay,
    #[allow(dead_code)]
    compositor: WlCompositor,
    surface: WlSurface,
    #[allow(dead_code)]
    seat: WlSeat,
    #[allow(dead_code)]
    layer_shell: ZwlrLayerShellV1,
    #[allow(dead_code)]
    layer_surface: ZwlrLayerSurfaceV1,
    cursor_mgr: WpCursorShapeManagerV1,
    screencopy_mgr: ZwlrScreencopyManagerV1,
    shm: WlShm,
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
        if let wayland_client::protocol::wl_callback::Event::Done { callback_data: _ } = event {
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
                    &state.wl.as_ref().unwrap().display,
                    &state.wl.as_ref().unwrap().surface,
                    width,
                    height,
                ));
                state.screen_configured = true;
                // Request initial screencopy + frame
                state.request_screencopy();
                let wl = state.wl.as_ref().unwrap();
                let qh = state.qhandle.as_ref().unwrap();
                wl.surface.frame(qh, ());
                wl.surface.commit();
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
        {
            // key is a scancode; Escape is typically scancode 1
            if key == 1 {
                log!(target: "magnifier::wl", Level::Info, "Escape pressed, exiting");
                state.quit = true;
            }
        }
    }
}
