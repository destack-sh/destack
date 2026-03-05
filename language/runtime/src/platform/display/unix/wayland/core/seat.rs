use wayland_client::protocol::{wl_pointer, wl_seat};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, WEnum};

use super::{
    WaylandConnectionDispatchState, apply_pointer_cursor_state, window_token_from_surface,
};

impl Dispatch<wl_seat::WlSeat, ()> for WaylandConnectionDispatchState {
    /// Handle wl_seat events.
    fn event(
        state: &mut Self,
        proxy: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _data: &(),
        _connection: &Connection,
        queue_handle: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities { capabilities } = event {
            let supports_pointer = match capabilities {
                WEnum::Value(capabilities) => capabilities.contains(wl_seat::Capability::Pointer),
                WEnum::Unknown(_) => false,
            };
            if supports_pointer && state.pointer.is_none() {
                let pointer = proxy.get_pointer(queue_handle, ());
                if let Some(manager) = state.cursor_shape_manager.as_ref().cloned() {
                    let shape_device = manager.get_pointer(&pointer, queue_handle, ());
                    state.cursor_shape_device = Some(shape_device);
                }
                if let Some(manager) = state.relative_pointer_manager.as_ref().cloned() {
                    let relative_pointer = manager.get_relative_pointer(&pointer, queue_handle, ());
                    state.relative_pointer = Some(relative_pointer);
                }
                state.pointer = Some(pointer);
            } else if !supports_pointer {
                if let Some(pointer) = state.pointer.take() {
                    pointer.release();
                }
                if let Some(shape_device) = state.cursor_shape_device.take() {
                    shape_device.destroy();
                }
                if let Some(relative_pointer) = state.relative_pointer.take() {
                    relative_pointer.destroy();
                }
                state.pointer_focus_surface = None;
                state.last_pointer_enter_serial = None;
                state.last_pointer_button_serial = None;
            }
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for WaylandConnectionDispatchState {
    /// Handle wl_pointer events.
    fn event(
        state: &mut Self,
        _proxy: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Enter {
                serial,
                surface,
                surface_x: _,
                surface_y: _,
            } => {
                state.pointer_focus_surface = Some(surface.id());
                state.last_pointer_enter_serial = Some(serial);

                if let Some(runtime_state) = state.runtime_state.upgrade()
                    && let Some(token) = window_token_from_surface(&runtime_state, &surface.id())
                    && let Some(binding) = token.binding.upgrade()
                {
                    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());

                    if binding.destroyed_emitted {
                        return;
                    }

                    apply_pointer_cursor_state(
                        state,
                        binding.cursor_visible,
                        binding.cursor_mode,
                        binding.cursor_icon,
                    );
                }
            }
            wl_pointer::Event::Leave { serial: _, surface } => {
                let is_current_surface = state
                    .pointer_focus_surface
                    .as_ref()
                    .is_some_and(|value| value == &surface.id());
                if is_current_surface {
                    state.pointer_focus_surface = None;
                    state.last_pointer_enter_serial = None;
                }
            }
            wl_pointer::Event::Button {
                serial,
                time: _,
                button: _,
                state: button_state,
            } => {
                if button_state == WEnum::Value(wl_pointer::ButtonState::Pressed) {
                    state.last_pointer_button_serial = Some(serial);
                }
            }
            _ => {}
        }
    }
}
