use std::collections::VecDeque;
use std::sync::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::display::windows::win32::core as win32_core;
use crate::platform::display::windows::win32::model::DisplayDescriptorSnapshot;
use crate::platform::display::{
    DisplayEventOverflowPolicy, DisplayMode, DisplayMonitorEventFilter,
    DisplayMonitorEventKindMask, DisplayMonitorEventOpenOptions,
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
        // map monitor event variants to kind mask lanes
        match self {
            DisplayEventRecordKind::Added { .. } => win32_core::DISPLAY_MONITOR_EVENT_KIND_ADDED,
            DisplayEventRecordKind::Removed { .. } => {
                win32_core::DISPLAY_MONITOR_EVENT_KIND_REMOVED
            }
            DisplayEventRecordKind::PrimaryChanged { .. } => {
                win32_core::DISPLAY_MONITOR_EVENT_KIND_PRIMARY_CHANGED
            }
            DisplayEventRecordKind::DescriptorChanged { .. } => {
                win32_core::DISPLAY_MONITOR_EVENT_KIND_DESCRIPTOR_CHANGED
            }
            DisplayEventRecordKind::ModeChanged { .. } => {
                win32_core::DISPLAY_MONITOR_EVENT_KIND_MODE_CHANGED
            }
        }
    }

    /// Return whether this monitor-event record matches one display identifier filter.
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
/// Open-time filter state for one monitor-event stream.
#[derive(Debug, Clone, Default)]
pub(crate) struct MonitorEventFilterState {
    /// Optional display identifier restriction.
    display_id: Option<String>,
    /// Optional monitor-event kind-mask restriction.
    kind_mask: Option<u32>,
}

impl MonitorEventFilterState {
    /// Build one monitor-event filter state from open options.
    pub(crate) unsafe fn from_open_options(
        options: DisplayMonitorEventOpenOptions,
    ) -> RuntimeResult<Self> {
        let Some(filter) = options.filter else {
            return Ok(Self::default());
        };

        let filter = unsafe { parse_monitor_event_filter(filter)? };
        Ok(filter)
    }

    /// Return whether one monitor-event record matches this filter.
    pub(crate) fn matches(&self, record: &DisplayEventRecord) -> bool {
        // reject records outside optional display id lane
        if let Some(display_id) = self.display_id.as_deref()
            && !record.kind.matches_display_id(display_id)
        {
            return false;
        }

        // reject records outside optional kind mask lane
        if let Some(kind_mask) = self.kind_mask
            && record.kind.kind_mask() & kind_mask == 0
        {
            return false;
        }

        true
    }
}

/// Build one monitor-event filter state from ABI filter payload.
unsafe fn parse_monitor_event_filter(
    filter: DisplayMonitorEventFilter,
) -> RuntimeResult<MonitorEventFilterState> {
    let display_id = match filter.display_id {
        Some(value) => Some(unsafe { value.as_str()? }.to_string()),
        None => None,
    };

    let kind_mask = filter
        .kind_mask
        .map(|value: DisplayMonitorEventKindMask| value.0);

    // validate kind-mask bits when provided
    if let Some(kind_mask) = kind_mask {
        win32_core::validate_monitor_event_kind_mask(kind_mask, "options.filter.kindMask")?;
    }

    Ok(MonitorEventFilterState {
        display_id,
        kind_mask,
    })
}
/// Build one monitor-event record with default queue metadata.
pub(crate) fn display_event_record(kind: DisplayEventRecordKind) -> DisplayEventRecord {
    DisplayEventRecord {
        timestamp_ns: core_platform::monotonic_now_ns(),
        sequence: 0,
        dropped_count: 0,
        kind,
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
