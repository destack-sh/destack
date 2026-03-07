use wayland_client::protocol::{wl_data_device, wl_data_device_manager, wl_data_offer};
use wayland_client::{Connection, Dispatch, QueueHandle};

use crate::platform::display::unix::wayland::core::WaylandConnectionDispatchState;
use crate::platform::display::unix::wayland::window;

impl Dispatch<wl_data_device_manager::WlDataDeviceManager, ()> for WaylandConnectionDispatchState {
    /// Handle wl_data_device_manager events.
    fn event(
        _state: &mut Self,
        _proxy: &wl_data_device_manager::WlDataDeviceManager,
        _event: wl_data_device_manager::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_data_offer::WlDataOffer, ()> for WaylandConnectionDispatchState {
    /// Handle wl_data_offer events.
    fn event(
        state: &mut Self,
        proxy: &wl_data_offer::WlDataOffer,
        event: wl_data_offer::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        window::handle_data_offer_event(state, proxy, event);
    }
}

impl Dispatch<wl_data_device::WlDataDevice, ()> for WaylandConnectionDispatchState {
    /// Handle wl_data_device events.
    fn event(
        state: &mut Self,
        _proxy: &wl_data_device::WlDataDevice,
        event: wl_data_device::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        window::handle_data_device_event(state, event);
    }
}
