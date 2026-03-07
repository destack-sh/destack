use std::collections::HashMap;

use wayland_client::protocol::{wl_data_device, wl_data_device_manager, wl_pointer, wl_seat};
use wayland_protocols::wp::cursor_shape::v1::client::{
    wp_cursor_shape_device_v1, wp_cursor_shape_manager_v1,
};
use wayland_protocols::wp::pointer_constraints::zv1::client::zwp_pointer_constraints_v1;
use wayland_protocols::wp::pointer_warp::v1::client::wp_pointer_warp_v1;
use wayland_protocols::wp::relative_pointer::zv1::client::{
    zwp_relative_pointer_manager_v1, zwp_relative_pointer_v1,
};

use crate::platform::display::WindowPosition;

/// Mutable payload for one advertised data offer.
#[derive(Debug, Clone, Default)]
pub(crate) struct WaylandDataOfferState {
    /// Offered mime-type set.
    pub(crate) mime_types: Vec<String>,
}

/// Mutable payload for one active drop session.
#[derive(Debug, Clone, Default)]
pub(crate) struct WaylandDropSessionState {
    /// Offer object id for this active session.
    pub(crate) offer: Option<wayland_client::backend::ObjectId>,
    /// Target surface object id for this active session.
    pub(crate) surface: Option<wayland_client::backend::ObjectId>,
    /// Last known drop position in window coordinates.
    pub(crate) position: Option<WindowPosition>,
    /// Last accepted mime type for this session.
    pub(crate) accepted_mime_type: Option<String>,
    /// Whether one drop-started event was emitted for this session.
    pub(crate) started: bool,
    /// Whether one compositor drop event is pending payload transfer.
    pub(crate) drop_pending: bool,
    /// Last hovered file path used for hover-leave payloads.
    pub(crate) last_hovered_path: Option<String>,
}

/// Mutable input and data-device state for one wayland connection lane.
#[derive(Debug, Default)]
pub(crate) struct WaylandInputState {
    /// Bound wl seat global used for pointer serial lanes.
    pub(crate) seat: Option<wl_seat::WlSeat>,
    /// Bound wl_data_device_manager global used for drop events.
    pub(crate) data_device_manager: Option<wl_data_device_manager::WlDataDeviceManager>,
    /// Bound wl_data_device object for the active seat.
    pub(crate) data_device: Option<wl_data_device::WlDataDevice>,
    /// Bound wl_pointer object for the active seat when available.
    pub(crate) pointer: Option<wl_pointer::WlPointer>,
    /// Bound cursor-shape manager for compositor-provided cursor themes.
    pub(crate) cursor_shape_manager: Option<wp_cursor_shape_manager_v1::WpCursorShapeManagerV1>,
    /// Bound cursor-shape device for the active pointer.
    pub(crate) cursor_shape_device: Option<wp_cursor_shape_device_v1::WpCursorShapeDeviceV1>,
    /// Bound pointer-constraints manager for lock and confine lanes.
    pub(crate) pointer_constraints_manager:
        Option<zwp_pointer_constraints_v1::ZwpPointerConstraintsV1>,
    /// Bound relative-pointer manager for locked-pointer motion lanes.
    pub(crate) relative_pointer_manager:
        Option<zwp_relative_pointer_manager_v1::ZwpRelativePointerManagerV1>,
    /// Bound pointer-warp manager for surface-local cursor warp requests.
    pub(crate) pointer_warp_manager: Option<wp_pointer_warp_v1::WpPointerWarpV1>,
    /// Bound relative-pointer object for the active pointer when available.
    pub(crate) relative_pointer: Option<zwp_relative_pointer_v1::ZwpRelativePointerV1>,
    /// Last entered wl_surface object id for pointer focus.
    pub(crate) pointer_focus_surface: Option<wayland_client::backend::ObjectId>,
    /// Last pointer-enter serial used for cursor shape updates.
    pub(crate) last_pointer_enter_serial: Option<u32>,
    /// Last known pointer-button serial for interactive move and resize lanes.
    pub(crate) last_pointer_button_serial: Option<u32>,
    /// Data-offer state keyed by offer object id.
    pub(crate) data_offer_state_by_id:
        HashMap<wayland_client::backend::ObjectId, WaylandDataOfferState>,
    /// Active drop-session state.
    pub(crate) drop_session_state: WaylandDropSessionState,
}
