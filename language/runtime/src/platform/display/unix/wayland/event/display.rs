use std::collections::VecDeque;
use std::sync::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::display::unix::wayland::core as wayland_core;
use crate::platform::display::unix::wayland::model::DisplayDescriptorSnapshot;
use crate::platform::display::{
    DisplayEventOverflowPolicy, DisplayMode, DisplayMonitorEventFilter,
    DisplayMonitorEventOpenOptions,
};

/// Stored monitor-event record payload.
#[derive(Debug, Clone)]
pub(crate) struct DisplayEventRecord {
    /// Event timestamp in nanoseconds.
    pub(crate) timestamp_ns: u64,
    /// Event sequence number.
    pub(crate) sequence: u64,
    /// Number of dropped events observed before this event.
    pub(crate) dropped_count: u64,
    /// Event kind payload.
    pub(crate) kind: DisplayEventRecordKind,
}

/// Stored monitor-event variant payload.
#[derive(Debug, Clone)]
pub(crate) enum DisplayEventRecordKind {
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
    pub(crate) fn kind_mask(&self) -> u32 {
        // map monitor event variants to kind-mask lanes
        match self {
            DisplayEventRecordKind::Added { .. } => wayland_core::DISPLAY_MONITOR_EVENT_KIND_ADDED,
            DisplayEventRecordKind::Removed { .. } => {
                wayland_core::DISPLAY_MONITOR_EVENT_KIND_REMOVED
            }
            DisplayEventRecordKind::PrimaryChanged { .. } => {
                wayland_core::DISPLAY_MONITOR_EVENT_KIND_PRIMARY_CHANGED
            }
            DisplayEventRecordKind::DescriptorChanged { .. } => {
                wayland_core::DISPLAY_MONITOR_EVENT_KIND_DESCRIPTOR_CHANGED
            }
            DisplayEventRecordKind::ModeChanged { .. } => {
                wayland_core::DISPLAY_MONITOR_EVENT_KIND_MODE_CHANGED
            }
        }
    }

    /// Return whether this record matches one display-id filter.
    pub(crate) fn matches_display_id(&self, display_id: &str) -> bool {
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

/// Parsed monitor-event filter state.
#[derive(Debug, Clone)]
pub(crate) struct MonitorEventFilterState {
    /// Optional display id filter.
    display_id: Option<String>,
    /// Enabled kind-mask bits.
    kind_mask: u32,
}

impl MonitorEventFilterState {
    /// Build filter state from monitor-event open options.
    pub(crate) fn from_open_options(
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
        let kind_mask = wayland_core::monitor_kind_mask(filter.kind_mask);
        wayland_core::validate_monitor_event_kind_mask(kind_mask, "options.filter.kindMask")?;

        Ok(Self {
            display_id,
            kind_mask,
        })
    }

    /// Return whether one record matches this filter.
    pub(crate) fn matches(&self, record: &DisplayEventRecord) -> bool {
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

/// Mutable monitor-event stream state.
#[derive(Debug)]
pub(crate) struct MonitorEventState {
    /// Configured queue capacity.
    pub(crate) queue_capacity: usize,
    /// Overflow policy for full queues.
    pub(crate) overflow_policy: DisplayEventOverflowPolicy,
    /// Overflow error pending marker.
    pub(crate) overflow_error_pending: bool,
    /// Next delivered event sequence value.
    pub(crate) next_output_sequence: u64,
    /// Total dropped event count.
    pub(crate) dropped_count: u64,
    /// Frontier for unread live records in the runtime event log.
    pub(crate) next_live_sequence: u64,
    /// Count of unread live records matching this stream filter.
    pub(crate) unread_live_count: usize,
    /// Seeded snapshot records delivered before live records.
    pub(crate) seeded: VecDeque<DisplayEventRecord>,
}

/// Shared monitor-event stream payload.
#[derive(Debug)]
pub(crate) struct MonitorEventStream {
    /// Stable runtime stream identifier.
    pub(crate) stream_id: u64,
    /// Mutable stream state.
    pub(crate) state: Mutex<MonitorEventState>,
    /// Stream filter configuration.
    pub(crate) filter: MonitorEventFilterState,
}

/// Build one monitor-event record with default metadata fields.
pub(crate) fn display_event_record(kind: DisplayEventRecordKind) -> DisplayEventRecord {
    DisplayEventRecord {
        timestamp_ns: core_platform::monotonic_now_ns(),
        sequence: 0,
        dropped_count: 0,
        kind,
    }
}

#[cfg(test)]
mod tests {
    use crate::platform::display::DisplayMode;

    use super::{
        DisplayDescriptorSnapshot, DisplayEventRecordKind, MonitorSnapshot,
        monitor_topology_records, wayland_core,
    };

    /// Build one descriptor payload for monitor topology tests.
    fn descriptor(
        id: &str,
        primary: bool,
        width_px: u32,
        height_px: u32,
    ) -> DisplayDescriptorSnapshot {
        DisplayDescriptorSnapshot {
            backend: wayland_core::selected_backend(),
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
            descriptor("wayland-output-1", true, 1920, 1080),
            mode(1920, 1080, 60_000),
        )];
        let next = vec![snapshot(
            descriptor("wayland-output-2", true, 2560, 1440),
            mode(2560, 1440, 144_000),
        )];

        let records = monitor_topology_records(&previous, &next);

        assert!(records.iter().any(|record| matches!(
            record.kind,
            DisplayEventRecordKind::Removed { ref id, .. } if id == "wayland-output-1"
        )));
        assert!(records.iter().any(|record| matches!(
            record.kind,
            DisplayEventRecordKind::Added { ref descriptor } if descriptor.id == "wayland-output-2"
        )));
        assert!(records.iter().any(|record| matches!(
            record.kind,
            DisplayEventRecordKind::ModeChanged { ref id, current: mode, .. } if id == "wayland-output-2" && mode.refresh_milli_hz == 144_000
        )));
        assert!(records.iter().any(|record| matches!(
            record.kind,
            DisplayEventRecordKind::PrimaryChanged { current_id: Some(ref id), .. } if id == "wayland-output-2"
        )));
    }

    /// Emit descriptor and mode-change records for one updated monitor.
    #[test]
    fn test_monitor_topology_records_emit_descriptor_and_mode_changes() {
        let previous = vec![snapshot(
            descriptor("wayland-output-1", true, 1920, 1080),
            mode(1920, 1080, 60_000),
        )];
        let next = vec![snapshot(
            descriptor("wayland-output-1", true, 2560, 1440),
            mode(2560, 1440, 120_000),
        )];

        let records = monitor_topology_records(&previous, &next);

        assert!(records.iter().any(|record| matches!(
            record.kind,
            DisplayEventRecordKind::DescriptorChanged { ref current, changed_mask, .. }
                if current.id == "wayland-output-1" && (changed_mask & wayland_core::DISPLAY_CHANGED_MASK_BOUNDS) != 0
        )));
        assert!(records.iter().any(|record| matches!(
            record.kind,
            DisplayEventRecordKind::ModeChanged { ref id, current: mode, .. }
                if id == "wayland-output-1" && mode.refresh_milli_hz == 120_000
        )));
    }
}
