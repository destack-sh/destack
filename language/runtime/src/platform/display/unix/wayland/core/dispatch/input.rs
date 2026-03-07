use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::wp::pointer_constraints::zv1::client::{
    zwp_confined_pointer_v1, zwp_locked_pointer_v1,
};
use wayland_protocols::wp::relative_pointer::zv1::client::zwp_relative_pointer_v1;

use crate::platform::display::unix::wayland::core::WaylandConnectionDispatchState;

impl Dispatch<zwp_locked_pointer_v1::ZwpLockedPointerV1, ()> for WaylandConnectionDispatchState {
    /// Handle zwp_locked_pointer events.
    fn event(
        _state: &mut Self,
        _proxy: &zwp_locked_pointer_v1::ZwpLockedPointerV1,
        _event: zwp_locked_pointer_v1::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwp_confined_pointer_v1::ZwpConfinedPointerV1, ()>
    for WaylandConnectionDispatchState
{
    /// Handle zwp_confined_pointer events.
    fn event(
        _state: &mut Self,
        _proxy: &zwp_confined_pointer_v1::ZwpConfinedPointerV1,
        _event: zwp_confined_pointer_v1::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwp_relative_pointer_v1::ZwpRelativePointerV1, ()>
    for WaylandConnectionDispatchState
{
    /// Handle zwp_relative_pointer events.
    fn event(
        _state: &mut Self,
        _proxy: &zwp_relative_pointer_v1::ZwpRelativePointerV1,
        _event: zwp_relative_pointer_v1::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
    }
}
