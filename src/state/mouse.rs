use log::{Level, log};
use wayland_client::protocol::wl_pointer::WlPointer;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, WEnum};
use wayland_protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::{
    Shape, WpCursorShapeDeviceV1,
};

use crate::config;

/// Tracks cursor shape and delegates pointer events to the magnifier state.
#[derive(Default)]
pub struct MouseState {
    cursor_dev: Option<WpCursorShapeDeviceV1>,
}

impl MouseState {
    pub fn set_cursor_shape_device(&mut self, dev: WpCursorShapeDeviceV1) {
        self.cursor_dev = Some(dev);
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
                state.mag.mouse_x = surface_x;
                state.mag.mouse_y = surface_y;
                log!(target: "magnifier::mouse", Level::Debug, "Mouse enter: {surface_x},{surface_y}");
                if let Some(ref dev) = state.mouse.cursor_dev {
                    dev.set_shape(serial, Shape::Crosshair);
                }
            }
            Event::Leave { .. } => {}
            Event::Motion {
                surface_x,
                surface_y,
                ..
            } => {
                state.mag.mouse_x = surface_x;
                state.mag.mouse_y = surface_y;
            }
            Event::Button {
                button,
                state: WEnum::Value(ButtonState::Pressed),
                ..
            } => {
                if button == config::BTN_LEFT {
                    log!(target: "magnifier::mouse", Level::Info, "Exit on click");
                    state.quit = true;
                }
            }
            Event::Axis { value, .. } => {
                // Scroll relative to the animation target, not the in-flight displayed value.
                // Prevents jumps when the user scrolls mid-animation.
                let base = state.mag.target_zoom() as f64;
                let factor = config::ZOOM_FACTOR_BASE.powf(-value / config::ZOOM_DIVISOR);
                let new_zoom =
                    (base * factor).clamp(config::ZOOM_MIN as f64, config::ZOOM_MAX as f64) as f32;
                state.mag.set_target_zoom(new_zoom);
            }
            Event::Frame => {}
            _ => {}
        }
    }
}
