pub(crate) use super::backend::{
    destack_display_monitor_event_close, destack_display_monitor_event_open,
    destack_display_monitor_event_read, destack_display_monitor_event_read_batch,
    destack_display_monitor_event_try_read, destack_display_monitor_event_try_read_batch,
    destack_display_window_event_close, destack_display_window_event_open,
    destack_display_window_event_read, destack_display_window_event_read_batch,
    destack_display_window_event_try_read, destack_display_window_event_try_read_batch,
};
pub(crate) use crate::platform::display::unsupported::{
    destack_display_begin_frame_close, destack_display_begin_frame_open,
    destack_display_begin_frame_read, destack_display_begin_frame_read_batch,
    destack_display_begin_frame_try_read, destack_display_begin_frame_try_read_batch,
};
