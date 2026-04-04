use log::{Level, log};
use wayland_client::protocol::wl_pointer::WlPointer;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, WEnum};
use wayland_protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::{
    Shape, WpCursorShapeDeviceV1,
};

#[derive(Default)]
pub struct MouseState {
    cursor_dev: Option<WpCursorShapeDeviceV1>,
    pub mouse_x: f64,
    pub mouse_y: f64,
    last_enter_serial: Option<u32>,
}

impl MouseState {
    pub fn set_cursor_shape_device(&mut self, d: WpCursorShapeDeviceV1) {
        self.cursor_dev = Some(d);
    }
}

impl Dispatch<WlPointer, (), super::State> for MouseState {
    fn event(
        state: &mut super::State,
        _ptr: &WlPointer,
        event: <WlPointer as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<super::State>,
    ) {
        use wayland_client::protocol::wl_pointer::{ButtonState, Event};
        match event {
            Event::Enter {
                serial,
                surface_x,
                surface_y,
                ..
            } => {
                state.mouse.mouse_x = surface_x;
                state.mouse.mouse_y = surface_y;
                state.mag.mouse_x = surface_x;
                state.mag.mouse_y = surface_y;
                state.mouse.last_enter_serial = Some(serial);
                log!(target: "magnifier::mouse", Level::Debug, "Mouse enter: {surface_x},{surface_y}");
                if let Some(ref d) = state.mouse.cursor_dev {
                    d.set_shape(serial, Shape::Crosshair);
                }
            }
            Event::Leave { .. } => {}
            Event::Motion {
                surface_x,
                surface_y,
                ..
            } => {
                state.mouse.mouse_x = surface_x;
                state.mouse.mouse_y = surface_y;
                state.mag.mouse_x = surface_x;
                state.mag.mouse_y = surface_y;
            }
            Event::Button {
                button,
                state: WEnum::Value(ButtonState::Pressed),
                ..
            } => {
                if button == 272 {
                    log!(target: "magnifier::mouse", Level::Info, "Exit on click");
                    state.quit = true;
                }
            }
            Event::Axis { value, .. } => {
                // wayland-client already converts wl_fixed_t to f64
                // Negative value = scroll up (content down) → zoom in
                // Positive value = scroll down (content up) → zoom out
                let old_zoom = state.mag.zoom;
                let factor = 2.0_f64.powf(-value / 100.0);
                state.mag.zoom = ((old_zoom as f64) * factor).clamp(0.1, 50.0) as f32;
                if (state.mag.zoom - old_zoom).abs() > 0.001 {
                    log!(target: "magnifier::mouse", Level::Debug, "Zoom: {} -> {}", old_zoom, state.mag.zoom);
                }
            }
            Event::Frame => {}
            _ => {}
        }
    }
}
