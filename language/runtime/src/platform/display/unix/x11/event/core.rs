use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::ThreadId;

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayEventOverflowPolicy, DisplayMode, DisplayMonitorEventFilter,
    DisplayMonitorEventOpenOptions, WindowEventFilter, WindowEventOpenOptions, WindowLogicalSize,
    WindowModeOptions, WindowOcclusionState, WindowPhysicalSize, WindowPosition, WindowVisibility,
};
use crate::platform::{core as core_platform, resource};

use super::super::core;
use super::super::model::{DisplayDescriptorSnapshot, MonitorSnapshot};
use super::codec::*;
use super::queue::*;

/// Stored monitor-event record payload.
#[derive(Debug, Clone)]
pub(super) struct DisplayEventRecord {
    /// Event timestamp in nanoseconds.
    timestamp_ns: u64,
    /// Event sequence number.
    sequence: u64,
    /// Number of dropped events observed before this event.
    dropped_count: u64,
    /// Event kind payload.
    kind: DisplayEventRecordKind,
}

/// Stored monitor-event variant payload.
#[derive(Debug, Clone)]
pub(super) enum DisplayEventRecordKind {
    /// Added-event payload.
    Added {
        /// Added descriptor payload.
        descriptor: DisplayDescriptorSnapshot,
    },
    /// Removed-event payload.
    Removed {
        /// Removed display identifier.
        id: String,
        /// Last known descriptor before removal.
        descriptor: Option<DisplayDescriptorSnapshot>,
    },
    /// Primary-changed payload.
    PrimaryChanged {
        /// Previous primary display identifier.
        previous_id: Option<String>,
        /// Current primary display identifier.
        current_id: Option<String>,
    },
    /// Descriptor-changed payload.
    DescriptorChanged {
        /// Descriptor payload before mutation.
        previous: Option<DisplayDescriptorSnapshot>,
        /// Descriptor payload after mutation.
        current: DisplayDescriptorSnapshot,
        /// Changed-field bit mask.
        changed_mask: u32,
    },
    /// Mode-changed payload.
    ModeChanged {
        /// Associated display identifier.
        id: String,
        /// Previous display mode payload.
        previous: Option<DisplayMode>,
        /// Current display mode payload.
        current: DisplayMode,
    },
}

impl DisplayEventRecordKind {
    /// Return the event-kind bit for this monitor-event record.
    fn kind_mask(&self) -> u32 {
        // map monitor event variants to kind-mask lanes
        match self {
            DisplayEventRecordKind::Added { .. } => core::DISPLAY_MONITOR_EVENT_KIND_ADDED,
            DisplayEventRecordKind::Removed { .. } => core::DISPLAY_MONITOR_EVENT_KIND_REMOVED,
            DisplayEventRecordKind::PrimaryChanged { .. } => {
                core::DISPLAY_MONITOR_EVENT_KIND_PRIMARY_CHANGED
            }
            DisplayEventRecordKind::DescriptorChanged { .. } => {
                core::DISPLAY_MONITOR_EVENT_KIND_DESCRIPTOR_CHANGED
            }
            DisplayEventRecordKind::ModeChanged { .. } => {
                core::DISPLAY_MONITOR_EVENT_KIND_MODE_CHANGED
            }
        }
    }

    /// Return whether this record matches one display-id filter.
    fn matches_display_id(&self, display_id: &str) -> bool {
        // compare display identity against each event payload shape
        match self {
            DisplayEventRecordKind::Added { descriptor } => descriptor.id == display_id,
            DisplayEventRecordKind::Removed { id, .. } => id == display_id,
            DisplayEventRecordKind::PrimaryChanged {
                previous_id,
                current_id,
            } => {
                previous_id.as_deref() == Some(display_id)
                    || current_id.as_deref() == Some(display_id)
            }
            DisplayEventRecordKind::DescriptorChanged { current, .. } => current.id == display_id,
            DisplayEventRecordKind::ModeChanged { id, .. } => id == display_id,
        }
    }
}

/// Stored window-event record payload.
#[derive(Debug, Clone)]
pub(super) struct WindowEventRecord {
    /// Event timestamp in nanoseconds.
    timestamp_ns: u64,
    /// Event sequence number.
    sequence: u64,
    /// Number of dropped events observed before this event.
    dropped_count: u64,
    /// Event kind payload.
    kind: WindowEventRecordKind,
}

/// Stored window-event variant payload.
#[derive(Debug, Clone)]
pub(super) enum WindowEventRecordKind {
    /// Created-event payload.
    Created {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
    },
    /// Close-requested payload.
    CloseRequested {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
    },
    /// Destroyed payload.
    Destroyed {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
    },
    /// Refresh-requested payload.
    RefreshRequested {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
    },
    /// Visibility-changed payload.
    VisibilityChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Visibility state before this event.
        previous_visibility: WindowVisibility,
        /// Visibility state after this event.
        current_visibility: WindowVisibility,
    },
    /// Occlusion-changed payload.
    OcclusionChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Occlusion state before this event.
        previous_occlusion: WindowOcclusionState,
        /// Occlusion state after this event.
        current_occlusion: WindowOcclusionState,
    },
    /// Position-changed payload.
    PositionChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Position before this event.
        previous_position: WindowPosition,
        /// Position after this event.
        current_position: WindowPosition,
    },
    /// Size-changed payload.
    SizeChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Logical size before this event.
        previous_size_logical: WindowLogicalSize,
        /// Physical size before this event.
        previous_size_physical: WindowPhysicalSize,
        /// Logical size after this event.
        current_size_logical: WindowLogicalSize,
        /// Physical size after this event.
        current_size_physical: WindowPhysicalSize,
    },
    /// Focus-changed payload.
    FocusChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Focus state before this event.
        previous_focused: bool,
        /// Focus state after this event.
        current_focused: bool,
    },
    /// Mode-changed payload.
    ModeChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Mode payload before this event.
        previous_mode: WindowModeOptions,
        /// Mode payload after this event.
        current_mode: WindowModeOptions,
    },
    /// Drop-started payload.
    DropStarted {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
    },
    /// File-hovered payload.
    FileHovered {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Hovered path payload.
        path: Option<String>,
        /// Hover position payload.
        position: Option<WindowPosition>,
    },
    /// Drop-cancelled payload.
    DropCancelled {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
    },
    /// Drop-completed payload.
    DropCompleted {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
    },
    /// File-hover-left payload.
    FileHoverLeft {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Previous hovered path payload.
        previous_path: Option<String>,
        /// Last hover position payload.
        position: Option<WindowPosition>,
    },
    /// File-dropped payload.
    FileDropped {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Dropped path payload.
        path: Option<String>,
        /// Drop position payload.
        position: Option<WindowPosition>,
    },
    /// Text-dropped payload.
    TextDropped {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Dropped text payload.
        text: String,
        /// Drop position payload.
        position: Option<WindowPosition>,
    },
}

impl WindowEventRecordKind {
    /// Return the event-kind bit for this window-event record.
    fn kind_mask(&self) -> u64 {
        // map window event variants to kind-mask lanes
        match self {
            WindowEventRecordKind::Created { .. } => core::WINDOW_EVENT_KIND_CREATED,
            WindowEventRecordKind::CloseRequested { .. } => core::WINDOW_EVENT_KIND_CLOSE_REQUESTED,
            WindowEventRecordKind::Destroyed { .. } => core::WINDOW_EVENT_KIND_DESTROYED,
            WindowEventRecordKind::RefreshRequested { .. } => {
                core::WINDOW_EVENT_KIND_REFRESH_REQUESTED
            }
            WindowEventRecordKind::VisibilityChanged { .. } => {
                core::WINDOW_EVENT_KIND_VISIBILITY_CHANGED
            }
            WindowEventRecordKind::OcclusionChanged { .. } => {
                core::WINDOW_EVENT_KIND_OCCLUSION_CHANGED
            }
            WindowEventRecordKind::PositionChanged { .. } => {
                core::WINDOW_EVENT_KIND_POSITION_CHANGED
            }
            WindowEventRecordKind::SizeChanged { .. } => core::WINDOW_EVENT_KIND_SIZE_CHANGED,
            WindowEventRecordKind::FocusChanged { .. } => core::WINDOW_EVENT_KIND_FOCUS_CHANGED,
            WindowEventRecordKind::ModeChanged { .. } => core::WINDOW_EVENT_KIND_MODE_CHANGED,
            WindowEventRecordKind::DropStarted { .. } => core::WINDOW_EVENT_KIND_DROP_STARTED,
            WindowEventRecordKind::FileHovered { .. } => core::WINDOW_EVENT_KIND_FILE_HOVERED,
            WindowEventRecordKind::DropCancelled { .. } => core::WINDOW_EVENT_KIND_DROP_CANCELLED,
            WindowEventRecordKind::DropCompleted { .. } => core::WINDOW_EVENT_KIND_DROP_COMPLETED,
            WindowEventRecordKind::FileHoverLeft { .. } => core::WINDOW_EVENT_KIND_FILE_HOVER_LEFT,
            WindowEventRecordKind::FileDropped { .. } => core::WINDOW_EVENT_KIND_FILE_DROPPED,
            WindowEventRecordKind::TextDropped { .. } => core::WINDOW_EVENT_KIND_TEXT_DROPPED,
        }
    }

    /// Return the associated runtime window handle.
    fn window(&self) -> resource::WindowHandle {
        // extract the shared window handle from each variant payload
        match self {
            WindowEventRecordKind::Created { window }
            | WindowEventRecordKind::CloseRequested { window }
            | WindowEventRecordKind::Destroyed { window }
            | WindowEventRecordKind::RefreshRequested { window }
            | WindowEventRecordKind::VisibilityChanged { window, .. }
            | WindowEventRecordKind::OcclusionChanged { window, .. }
            | WindowEventRecordKind::PositionChanged { window, .. }
            | WindowEventRecordKind::SizeChanged { window, .. }
            | WindowEventRecordKind::FocusChanged { window, .. }
            | WindowEventRecordKind::ModeChanged { window, .. }
            | WindowEventRecordKind::DropStarted { window }
            | WindowEventRecordKind::FileHovered { window, .. }
            | WindowEventRecordKind::DropCancelled { window }
            | WindowEventRecordKind::DropCompleted { window }
            | WindowEventRecordKind::FileHoverLeft { window, .. }
            | WindowEventRecordKind::FileDropped { window, .. }
            | WindowEventRecordKind::TextDropped { window, .. } => *window,
        }
    }
}

/// Parsed monitor-event filter state.
#[derive(Debug, Clone)]
pub(super) struct MonitorEventFilterState {
    /// Optional display id filter.
    display_id: Option<String>,
    /// Enabled kind-mask bits.
    kind_mask: u32,
}

impl MonitorEventFilterState {
    /// Build filter state from monitor-event open options.
    pub(super) fn from_open_options(
        options: DisplayMonitorEventOpenOptions,
    ) -> RuntimeResult<Self> {
        // normalize optional filter payload into one concrete state
        let filter = options.filter.unwrap_or(DisplayMonitorEventFilter {
            display_id: None,
            kind_mask: None,
        });

        // parse optional display id payload into owned string
        let display_id = filter
            .display_id
            .map(|value| unsafe { value.as_str() })
            .transpose()?
            .map(|value| value.to_string());
        let kind_mask = core::monitor_kind_mask(filter.kind_mask);
        core::validate_monitor_event_kind_mask(kind_mask, "options.filter.kindMask")?;

        Ok(Self {
            display_id,
            kind_mask,
        })
    }

    /// Return whether one record matches this filter.
    fn matches(&self, record: &DisplayEventRecord) -> bool {
        // reject records filtered out by kind mask
        if (record.kind.kind_mask() & self.kind_mask) == 0 {
            return false;
        }

        // accept all records when display-id filter is empty
        let Some(display_id) = self.display_id.as_deref() else {
            return true;
        };

        record.kind.matches_display_id(display_id)
    }
}

/// Parsed window-event filter state.
#[derive(Debug, Clone, Copy)]
pub(super) struct WindowEventFilterState {
    /// Optional window filter.
    window: Option<resource::WindowHandle>,
    /// Enabled kind-mask bits.
    kind_mask: u64,
}

impl WindowEventFilterState {
    /// Build filter state from window-event open options.
    pub(super) fn from_open_options(options: WindowEventOpenOptions) -> RuntimeResult<Self> {
        // map optional filter to normalized state
        let filter = options.filter.unwrap_or(WindowEventFilter {
            window: None,
            kind_mask: None,
        });
        let kind_mask = core::window_kind_mask(filter.kind_mask);
        core::validate_window_event_kind_mask(kind_mask, "options.filter.kindMask")?;

        Ok(Self {
            window: filter.window,
            kind_mask,
        })
    }

    /// Return whether one record matches this filter.
    fn matches(&self, record: &WindowEventRecord) -> bool {
        // reject records filtered out by kind mask
        if (record.kind.kind_mask() & self.kind_mask) == 0 {
            return false;
        }

        // accept all records when window filter is empty
        let Some(window) = self.window else {
            return true;
        };

        record.kind.window() == window
    }
}

/// Mutable monitor-event stream queue state.
#[derive(Debug)]
pub(super) struct MonitorEventState {
    /// Configured queue capacity.
    pub(super) queue_capacity: usize,
    /// Overflow policy for full queues.
    pub(super) overflow_policy: DisplayEventOverflowPolicy,
    /// Overflow error pending marker.
    pub(super) overflow_error_pending: bool,
    /// Next event sequence value.
    pub(super) next_sequence: u64,
    /// Total dropped event count.
    pub(super) dropped_count: u64,
    /// Pending monitor-event records.
    pub(super) pending: VecDeque<DisplayEventRecord>,
}

/// Shared monitor-event stream binding.
#[derive(Debug)]
pub(in super::super) struct MonitorEventBinding {
    /// Mutable queue state.
    pub(super) state: Mutex<MonitorEventState>,
    /// Stream filter configuration.
    pub(super) filter: MonitorEventFilterState,
    /// Queue wake signal.
    pub(super) signal: Condvar,
}

/// Mutable window-event stream queue state.
#[derive(Debug)]
pub(super) struct WindowEventState {
    /// Configured queue capacity.
    pub(super) queue_capacity: usize,
    /// Overflow policy for full queues.
    pub(super) overflow_policy: DisplayEventOverflowPolicy,
    /// Overflow error pending marker.
    pub(super) overflow_error_pending: bool,
    /// Next event sequence value.
    pub(super) next_sequence: u64,
    /// Total dropped event count.
    pub(super) dropped_count: u64,
    /// Pending window-event records.
    pub(super) pending: VecDeque<WindowEventRecord>,
}

/// Shared window-event stream binding.
#[derive(Debug)]
pub(in super::super) struct WindowEventBinding {
    /// Mutable queue state.
    pub(super) state: Mutex<WindowEventState>,
    /// Stream filter configuration.
    pub(super) filter: WindowEventFilterState,
    /// Queue wake signal.
    pub(super) signal: Condvar,
    /// Owner thread id for x11 event pumping operations.
    pub(super) owner_thread_id: ThreadId,
}

/// Register one x11 window id mapping.
pub(in super::super) fn register_xid(
    runtime_state: &Arc<core::X11RuntimeState>,
    xid: u32,
    window: resource::WindowHandle,
) {
    let mut map = runtime_state
        .windows_by_xid
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    map.insert(xid, window);
}

/// Unregister one x11 window id mapping.
pub(in super::super) fn unregister_xid(runtime_state: &Arc<core::X11RuntimeState>, xid: u32) {
    let mut map = runtime_state
        .windows_by_xid
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    map.remove(&xid);
}

/// Resolve one runtime window handle from one x11 window id.
pub(in super::super) fn resolve_window_by_xid(
    runtime_state: &Arc<core::X11RuntimeState>,
    xid: u32,
) -> Option<resource::WindowHandle> {
    let map = runtime_state
        .windows_by_xid
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    map.get(&xid).copied()
}

/// Build one monitor-event record with default metadata fields.
fn display_event_record(kind: DisplayEventRecordKind) -> DisplayEventRecord {
    DisplayEventRecord {
        timestamp_ns: core_platform::monotonic_now_ns(),
        sequence: 0,
        dropped_count: 0,
        kind,
    }
}

/// Build one window-event record with default metadata fields.
fn window_event_record(kind: WindowEventRecordKind) -> WindowEventRecord {
    WindowEventRecord {
        timestamp_ns: core_platform::monotonic_now_ns(),
        sequence: 0,
        dropped_count: 0,
        kind,
    }
}

#[cfg(test)]
mod tests {
    use crate::platform::display::{DisplayMode, DisplayOrientation, DisplaySupportStatus};

    use super::{
        DisplayDescriptorSnapshot, DisplayEventRecordKind, MonitorSnapshot, core,
        monitor_topology_records,
    };

    /// Build one descriptor payload for monitor topology tests.
    fn descriptor(
        id: &str,
        primary: bool,
        width_px: u32,
        height_px: u32,
    ) -> DisplayDescriptorSnapshot {
        DisplayDescriptorSnapshot {
            backend: core::selected_backend(),
            id: id.to_string(),
            name: id.to_string(),
            primary,
            x: 0,
            y: 0,
            width_px,
            height_px,
            work_area_x: 0,
            work_area_y: 0,
            work_area_width_px: width_px,
            work_area_height_px: height_px,
            width_mm: 500,
            height_mm: 300,
            scale_factor_milli: 1000,
            orientation: DisplayOrientation::Landscape,
            builtin_panel: DisplaySupportStatus::Unsupported,
            variable_refresh_support: DisplaySupportStatus::Unknown,
            hdr_support: DisplaySupportStatus::Unsupported,
        }
    }

    /// Build one mode payload for monitor topology tests.
    fn mode(width: u32, height: u32, refresh_milli_hz: u32) -> DisplayMode {
        DisplayMode {
            width,
            height,
            refresh_milli_hz,
            format: 0,
            bit_depth: 8,
        }
    }

    /// Build one monitor snapshot payload for monitor topology tests.
    fn snapshot(descriptor: DisplayDescriptorSnapshot, mode: DisplayMode) -> MonitorSnapshot {
        MonitorSnapshot {
            descriptor,
            current_mode: mode,
            desktop_mode: mode,
            modes: vec![mode],
        }
    }

    /// Emit removed, added, mode and primary-change records for topology replacement.
    #[test]
    fn test_monitor_topology_records_emit_removed_added_mode_and_primary() {
        let previous = vec![snapshot(
            descriptor("x11-output-1", true, 1920, 1080),
            mode(1920, 1080, 60_000),
        )];
        let next = vec![snapshot(
            descriptor("x11-output-2", true, 2560, 1440),
            mode(2560, 1440, 144_000),
        )];

        let records = monitor_topology_records(&previous, &next);

        assert!(records.iter().any(|record| matches!(
            record.kind,
            DisplayEventRecordKind::Removed { ref id, .. } if id == "x11-output-1"
        )));
        assert!(records.iter().any(|record| matches!(
            record.kind,
            DisplayEventRecordKind::Added { ref descriptor } if descriptor.id == "x11-output-2"
        )));
        assert!(records.iter().any(|record| matches!(
            record.kind,
            DisplayEventRecordKind::ModeChanged { ref id, current: mode, .. } if id == "x11-output-2" && mode.refresh_milli_hz == 144_000
        )));
        assert!(records.iter().any(|record| matches!(
            record.kind,
            DisplayEventRecordKind::PrimaryChanged { current_id: Some(ref id), .. } if id == "x11-output-2"
        )));
    }

    /// Emit descriptor and mode-change records for one updated monitor.
    #[test]
    fn test_monitor_topology_records_emit_descriptor_and_mode_changes() {
        let previous = vec![snapshot(
            descriptor("x11-output-1", true, 1920, 1080),
            mode(1920, 1080, 60_000),
        )];
        let next = vec![snapshot(
            descriptor("x11-output-1", true, 2560, 1440),
            mode(2560, 1440, 120_000),
        )];

        let records = monitor_topology_records(&previous, &next);

        assert!(records.iter().any(|record| matches!(
            record.kind,
            DisplayEventRecordKind::DescriptorChanged { ref current, changed_mask, .. }
                // evaluate this condition
                if current.id == "x11-output-1" && (changed_mask & core::DISPLAY_CHANGED_MASK_BOUNDS) != 0
        )));
        assert!(records.iter().any(|record| matches!(
            record.kind,
            DisplayEventRecordKind::ModeChanged { ref id, current: mode, .. }
                // evaluate this condition
                if id == "x11-output-1" && mode.refresh_milli_hz == 120_000
        )));
    }
}
