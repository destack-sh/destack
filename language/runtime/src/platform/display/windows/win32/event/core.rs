use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Condvar, Mutex, Weak};
use std::time::Duration;

use windows_sys::Win32::System::Threading::GetCurrentThreadId;

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayAddedEvent, DisplayAddedPayload, DisplayBackend, DisplayDescriptorChangedEvent,
    DisplayDescriptorChangedPayload, DisplayEventOverflowPolicy, DisplayMetricChangedMask,
    DisplayMode, DisplayModeChangedEvent, DisplayModeChangedPayload, DisplayMonitorEvent,
    DisplayMonitorEventFilter, DisplayMonitorEventKindMask, DisplayMonitorEventMetadata,
    DisplayMonitorEventOpenOptions, DisplayPrimaryChangedEvent, DisplayPrimaryPayload,
    DisplayRemovedEvent, DisplayRemovedPayload, WindowAspectRatio, WindowAspectRatioChangedEvent,
    WindowAspectRatioPayload, WindowChromeChangedEvent, WindowChromeKind, WindowChromePayload,
    WindowCloseRequestedEvent, WindowCreatedEvent, WindowDestroyedEvent, WindowDisplayChangedEvent,
    WindowDisplayPayload, WindowDropCancelledEvent, WindowDropCompletedEvent,
    WindowDropFilePayload, WindowDropHoverLeavePayload, WindowDropHoverPayload,
    WindowDropStartedEvent, WindowDropTextPayload, WindowEvent, WindowEventFilter,
    WindowEventKindMask, WindowEventMetadata, WindowEventOpenOptions, WindowFileDroppedEvent,
    WindowFileHoverLeftEvent, WindowFileHoveredEvent, WindowFocusChangedEvent, WindowFocusPayload,
    WindowLogicalSize, WindowModalChangedEvent, WindowModalPayload, WindowModeChangedEvent,
    WindowModeOptions, WindowModePayload, WindowMousePassthroughChangedEvent,
    WindowMousePassthroughPayload, WindowOcclusionChangedEvent, WindowOcclusionPayload,
    WindowOcclusionState, WindowOpacityChangedEvent, WindowOpacityPayload,
    WindowParentChangedEvent, WindowParentPayload, WindowPhysicalSize, WindowPosition,
    WindowPositionChangedEvent, WindowPositionPayload, WindowRefreshRequestedEvent,
    WindowScaleFactorChangedEvent, WindowScaleFactorPayload, WindowSizeChangedEvent,
    WindowSizePayload, WindowTaskbarVisibilityChangedEvent, WindowTaskbarVisibilityPayload,
    WindowTextDroppedEvent, WindowTheme, WindowThemeChangedEvent, WindowThemePayload,
    WindowTransientChangedEvent, WindowTransientPayload, WindowVisibility,
    WindowVisibilityChangedEvent, WindowVisibilityPayload,
};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{NativeArray, core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::super::model::{DisplayDescriptorSnapshot, MonitorSnapshot, Win32WindowBinding};
use super::super::{core, monitor, resource as display_resource, window};
use super::codec::*;
use super::queue::*;

/// Stored monitor-event record payload.
#[derive(Debug, Clone)]
struct DisplayEventRecord {
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
enum DisplayEventRecordKind {
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

/// Stored window-event record payload.
#[derive(Debug, Clone)]
struct WindowEventRecord {
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
enum WindowEventRecordKind {
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
    /// Focus-changed payload.
    FocusChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Focus state before this event.
        previous_focused: bool,
        /// Focus state after this event.
        current_focused: bool,
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
    /// Scale-factor-changed payload.
    ScaleFactorChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Scale factor before this event.
        previous_scale_factor_milli: u32,
        /// Scale factor after this event.
        current_scale_factor_milli: u32,
    },
    /// Refresh-requested payload.
    RefreshRequested {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
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
    /// Display-changed payload.
    DisplayChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Display payload before this event.
        previous_display: Option<resource::DisplayHandle>,
        /// Display payload after this event.
        current_display: Option<resource::DisplayHandle>,
    },
    /// Theme-changed payload.
    ThemeChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Theme payload before this event.
        previous_theme: WindowTheme,
        /// Theme payload after this event.
        current_theme: WindowTheme,
    },
    /// Chrome-changed payload.
    ChromeChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Chrome payload before this event.
        previous_chrome: WindowChromeKind,
        /// Chrome payload after this event.
        current_chrome: WindowChromeKind,
    },
    /// Taskbar-visibility-changed payload.
    TaskbarVisibilityChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Previous taskbar visibility.
        previous_taskbar_visible: bool,
        /// Current taskbar visibility.
        current_taskbar_visible: bool,
    },
    /// Opacity-changed payload.
    OpacityChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Previous opacity.
        previous_opacity: f64,
        /// Current opacity.
        current_opacity: f64,
    },
    /// Parent-changed payload.
    ParentChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Previous parent.
        previous_parent: Option<resource::WindowHandle>,
        /// Current parent.
        current_parent: Option<resource::WindowHandle>,
    },
    /// Transient-owner-changed payload.
    TransientChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Previous transient owner.
        previous_transient_for: Option<resource::WindowHandle>,
        /// Current transient owner.
        current_transient_for: Option<resource::WindowHandle>,
    },
    /// Modal-changed payload.
    ModalChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Previous modal state.
        previous_modal: bool,
        /// Current modal state.
        current_modal: bool,
    },
    /// Mouse-passthrough-changed payload.
    MousePassthroughChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Previous passthrough state.
        previous_mouse_passthrough: bool,
        /// Current passthrough state.
        current_mouse_passthrough: bool,
    },
    /// Aspect-ratio-changed payload.
    AspectRatioChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Previous aspect ratio.
        previous_aspect_ratio: Option<WindowAspectRatio>,
        /// Current aspect ratio.
        current_aspect_ratio: Option<WindowAspectRatio>,
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
        /// Hovered path payload as UTF-16 code units.
        path_utf16: Option<Vec<u16>>,
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
        /// Previous hovered path payload as UTF-16 code units.
        previous_path_utf16: Option<Vec<u16>>,
        /// Last hover position payload.
        position: Option<WindowPosition>,
    },
    /// File-dropped payload.
    FileDropped {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Dropped path payload as UTF-16 code units.
        path_utf16: Option<Vec<u16>>,
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

impl DisplayEventRecordKind {
    /// Return the event-kind bit for this monitor-event record.
    fn kind_mask(&self) -> u32 {
        // map monitor event variants to kind mask lanes
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

impl WindowEventRecordKind {
    /// Return the event-kind bit for this window-event record.
    fn kind_mask(&self) -> u64 {
        // map window event variants to kind mask lanes
        match self {
            WindowEventRecordKind::Created { .. } => core::WINDOW_EVENT_KIND_CREATED,
            WindowEventRecordKind::CloseRequested { .. } => core::WINDOW_EVENT_KIND_CLOSE_REQUESTED,
            WindowEventRecordKind::Destroyed { .. } => core::WINDOW_EVENT_KIND_DESTROYED,
            WindowEventRecordKind::FocusChanged { .. } => core::WINDOW_EVENT_KIND_FOCUS_CHANGED,
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
            WindowEventRecordKind::ScaleFactorChanged { .. } => {
                core::WINDOW_EVENT_KIND_SCALE_FACTOR_CHANGED
            }
            WindowEventRecordKind::RefreshRequested { .. } => {
                core::WINDOW_EVENT_KIND_REFRESH_REQUESTED
            }
            WindowEventRecordKind::ModeChanged { .. } => core::WINDOW_EVENT_KIND_MODE_CHANGED,
            WindowEventRecordKind::DisplayChanged { .. } => core::WINDOW_EVENT_KIND_DISPLAY_CHANGED,
            WindowEventRecordKind::ThemeChanged { .. } => core::WINDOW_EVENT_KIND_THEME_CHANGED,
            WindowEventRecordKind::ChromeChanged { .. } => core::WINDOW_EVENT_KIND_CHROME_CHANGED,
            WindowEventRecordKind::TaskbarVisibilityChanged { .. } => {
                core::WINDOW_EVENT_KIND_TASKBAR_VISIBILITY_CHANGED
            }
            WindowEventRecordKind::OpacityChanged { .. } => core::WINDOW_EVENT_KIND_OPACITY_CHANGED,
            WindowEventRecordKind::ParentChanged { .. } => core::WINDOW_EVENT_KIND_PARENT_CHANGED,
            WindowEventRecordKind::TransientChanged { .. } => {
                core::WINDOW_EVENT_KIND_TRANSIENT_CHANGED
            }
            WindowEventRecordKind::ModalChanged { .. } => core::WINDOW_EVENT_KIND_MODAL_CHANGED,
            WindowEventRecordKind::MousePassthroughChanged { .. } => {
                core::WINDOW_EVENT_KIND_MOUSE_PASSTHROUGH_CHANGED
            }
            WindowEventRecordKind::AspectRatioChanged { .. } => {
                core::WINDOW_EVENT_KIND_ASPECT_RATIO_CHANGED
            }
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
        // project associated window handle from each event variant
        match self {
            WindowEventRecordKind::Created { window }
            | WindowEventRecordKind::CloseRequested { window }
            | WindowEventRecordKind::Destroyed { window }
            | WindowEventRecordKind::FocusChanged { window, .. }
            | WindowEventRecordKind::VisibilityChanged { window, .. }
            | WindowEventRecordKind::OcclusionChanged { window, .. }
            | WindowEventRecordKind::PositionChanged { window, .. }
            | WindowEventRecordKind::SizeChanged { window, .. }
            | WindowEventRecordKind::ScaleFactorChanged { window, .. }
            | WindowEventRecordKind::RefreshRequested { window }
            | WindowEventRecordKind::ModeChanged { window, .. }
            | WindowEventRecordKind::DisplayChanged { window, .. }
            | WindowEventRecordKind::ThemeChanged { window, .. }
            | WindowEventRecordKind::ChromeChanged { window, .. }
            | WindowEventRecordKind::TaskbarVisibilityChanged { window, .. }
            | WindowEventRecordKind::OpacityChanged { window, .. }
            | WindowEventRecordKind::ParentChanged { window, .. }
            | WindowEventRecordKind::TransientChanged { window, .. }
            | WindowEventRecordKind::ModalChanged { window, .. }
            | WindowEventRecordKind::MousePassthroughChanged { window, .. }
            | WindowEventRecordKind::AspectRatioChanged { window, .. }
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

/// Open-time filter state for one monitor-event stream.
#[derive(Debug, Clone, Default)]
pub(super) struct MonitorEventFilterState {
    /// Optional display identifier restriction.
    display_id: Option<String>,
    /// Optional monitor-event kind-mask restriction.
    kind_mask: Option<u32>,
}

impl MonitorEventFilterState {
    /// Build one monitor-event filter state from open options.
    pub(super) unsafe fn from_open_options(
        options: DisplayMonitorEventOpenOptions,
    ) -> RuntimeResult<Self> {
        let Some(filter) = options.filter else {
            return Ok(Self::default());
        };

        let filter = unsafe { parse_monitor_event_filter(filter)? };
        Ok(filter)
    }

    /// Return whether one monitor-event record matches this filter.
    fn matches(&self, record: &DisplayEventRecord) -> bool {
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

/// Open-time filter state for one window-event stream.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct WindowEventFilterState {
    /// Optional window-handle restriction.
    window: Option<resource::WindowHandle>,
    /// Optional window-event kind-mask restriction.
    kind_mask: Option<u64>,
}

impl WindowEventFilterState {
    /// Build one window-event filter state from open options.
    pub(super) fn from_open_options(options: WindowEventOpenOptions) -> RuntimeResult<Self> {
        let Some(filter) = options.filter else {
            return Ok(Self::default());
        };

        let filter = parse_window_event_filter(filter)?;
        Ok(filter)
    }

    /// Return whether one window-event record matches this filter.
    fn matches(&self, record: &WindowEventRecord) -> bool {
        // reject records outside optional window lane
        if let Some(window) = self.window
            && record.kind.window() != window
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
    // evaluate this condition
    if let Some(kind_mask) = kind_mask {
        core::validate_monitor_event_kind_mask(kind_mask, "options.filter.kindMask")?;
    }

    Ok(MonitorEventFilterState {
        display_id,
        kind_mask,
    })
}

/// Build one window-event filter state from ABI filter payload.
fn parse_window_event_filter(filter: WindowEventFilter) -> RuntimeResult<WindowEventFilterState> {
    // decode optional kind mask from abi payload
    let kind_mask = filter.kind_mask.map(|value: WindowEventKindMask| value.0);

    // validate kind-mask bits when provided
    if let Some(kind_mask) = kind_mask {
        core::validate_window_event_kind_mask(kind_mask, "options.filter.kindMask")?;
    }

    // build normalized runtime filter state
    Ok(WindowEventFilterState {
        window: filter.window,
        kind_mask,
    })
}

/// Build one monitor-event record with default queue metadata.
fn display_event_record(kind: DisplayEventRecordKind) -> DisplayEventRecord {
    DisplayEventRecord {
        timestamp_ns: core_platform::monotonic_now_ns(),
        sequence: 0,
        dropped_count: 0,
        kind,
    }
}

/// Build one window-event record with default queue metadata.
fn window_event_record(kind: WindowEventRecordKind) -> WindowEventRecord {
    WindowEventRecord {
        timestamp_ns: core_platform::monotonic_now_ns(),
        sequence: 0,
        dropped_count: 0,
        kind,
    }
}

/// Resource payload for one monitor-event stream.
#[derive(Debug)]
pub(in super::super) struct MonitorEventBinding {
    /// Shared mutable stream state.
    state: Mutex<MonitorEventState>,
    /// Stream-level event filter payload.
    filter: MonitorEventFilterState,
    /// Wake lane for blocking readers.
    signal: Condvar,
}

/// Mutable monitor-event stream state.
#[derive(Debug)]
pub(super) struct MonitorEventState {
    /// Queue capacity for this stream.
    queue_capacity: usize,
    /// Queue overflow policy for this stream.
    overflow_policy: DisplayEventOverflowPolicy,
    /// Latched overflow-error state.
    overflow_error_pending: bool,
    /// Next sequence number for this stream.
    next_sequence: u64,
    /// Total number of dropped events observed by this stream.
    dropped_count: u64,
    /// Pending queue payload.
    pending: VecDeque<DisplayEventRecord>,
}

/// Resource payload for one window-event stream.
#[derive(Debug)]
pub(in super::super) struct WindowEventBinding {
    /// Shared mutable stream state.
    state: Mutex<WindowEventState>,
    /// Stream-level event filter payload.
    filter: WindowEventFilterState,
    /// Wake lane for blocking readers.
    signal: Condvar,
    /// Owner thread identifier used for message pumping.
    owner_thread_id: u32,
}

/// Mutable window-event stream state.
#[derive(Debug)]
pub(super) struct WindowEventState {
    /// Queue capacity for this stream.
    queue_capacity: usize,
    /// Queue overflow policy for this stream.
    overflow_policy: DisplayEventOverflowPolicy,
    /// Latched overflow-error state.
    overflow_error_pending: bool,
    /// Next sequence number for this stream.
    next_sequence: u64,
    /// Total number of dropped events observed by this stream.
    dropped_count: u64,
    /// Pending queue payload.
    pending: VecDeque<WindowEventRecord>,
}

/// Shared queue-state interface for monitor and window event streams.
trait EventQueueState<Record> {
    /// Return the configured queue capacity for this stream.
    fn queue_capacity(&self) -> usize;

    /// Return the configured queue overflow policy for this stream.
    fn overflow_policy(&self) -> DisplayEventOverflowPolicy;

    /// Return the overflow-error latch.
    fn overflow_error_pending(&mut self) -> &mut bool;

    /// Return the next event sequence counter.
    fn next_sequence(&mut self) -> &mut u64;

    /// Return the stream dropped-event counter.
    fn dropped_count(&mut self) -> &mut u64;

    /// Return the pending queue payload.
    fn pending(&mut self) -> &mut VecDeque<Record>;
}

impl EventQueueState<DisplayEventRecord> for MonitorEventState {
    /// Return this stream queue capacity.
    fn queue_capacity(&self) -> usize {
        self.queue_capacity
    }

    /// Return this stream overflow policy.
    fn overflow_policy(&self) -> DisplayEventOverflowPolicy {
        self.overflow_policy
    }

    /// Return mutable access to overflow latch.
    fn overflow_error_pending(&mut self) -> &mut bool {
        &mut self.overflow_error_pending
    }

    /// Return mutable access to next sequence counter.
    fn next_sequence(&mut self) -> &mut u64 {
        &mut self.next_sequence
    }

    /// Return mutable access to dropped counter.
    fn dropped_count(&mut self) -> &mut u64 {
        &mut self.dropped_count
    }

    /// Return mutable access to pending queue.
    fn pending(&mut self) -> &mut VecDeque<DisplayEventRecord> {
        &mut self.pending
    }
}

impl EventQueueState<WindowEventRecord> for WindowEventState {
    /// Return this stream queue capacity.
    fn queue_capacity(&self) -> usize {
        self.queue_capacity
    }

    /// Return this stream overflow policy.
    fn overflow_policy(&self) -> DisplayEventOverflowPolicy {
        self.overflow_policy
    }

    /// Return mutable access to overflow latch.
    fn overflow_error_pending(&mut self) -> &mut bool {
        &mut self.overflow_error_pending
    }

    /// Return mutable access to next sequence counter.
    fn next_sequence(&mut self) -> &mut u64 {
        &mut self.next_sequence
    }

    /// Return mutable access to dropped counter.
    fn dropped_count(&mut self) -> &mut u64 {
        &mut self.dropped_count
    }

    /// Return mutable access to pending queue.
    fn pending(&mut self) -> &mut VecDeque<WindowEventRecord> {
        &mut self.pending
    }
}

/// Runtime-owned mutable state for display event streams.
#[derive(Debug)]
pub(crate) struct DisplayEventRuntimeState {
    /// Event subscriber list for monitor-event streams.
    monitor_event_registry: Mutex<Vec<Weak<MonitorEventBinding>>>,
    /// Last observed monitor topology snapshot.
    monitor_topology_snapshot: Mutex<Option<Vec<MonitorSnapshot>>>,
    /// Event subscriber list for window-event streams.
    window_event_registry: Mutex<Vec<Weak<WindowEventBinding>>>,
}

impl Default for DisplayEventRuntimeState {
    /// Create one default display-event runtime state.
    fn default() -> Self {
        Self::new()
    }
}

impl DisplayEventRuntimeState {
    /// Create one display-event runtime state.
    fn new() -> Self {
        Self {
            monitor_event_registry: Mutex::new(Vec::new()),
            monitor_topology_snapshot: Mutex::new(None),
            window_event_registry: Mutex::new(Vec::new()),
        }
    }
}

/// Return runtime-owned display-event state.
pub(in super::super) fn display_event_runtime_state(
    binding: &BindingCallContext,
) -> Arc<DisplayEventRuntimeState> {
    binding
        .agent()
        .platform_state
        .display
        .display_event_runtime_state(DisplayEventRuntimeState::new)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Condvar, Mutex};

    use super::{
        DisplayEventRecordKind, DisplayEventRuntimeState, MonitorEventBinding,
        MonitorEventFilterState, MonitorEventState, WindowEventBinding, WindowEventFilterState,
        WindowEventRecordKind, WindowEventState, display_event_record, monitor_topology_records,
        publish_monitor_event, publish_window_drop_cancelled_event,
        publish_window_drop_completed_event, publish_window_drop_started_event,
        publish_window_file_dropped_event, publish_window_text_dropped_event,
    };
    use crate::platform::display::{
        DisplayBackend, DisplayEventOverflowPolicy, DisplayMode, DisplayOrientation,
        DisplaySupportStatus, WindowPosition,
    };
    use crate::platform::resource::{ResourceId, WindowHandle};

    use super::super::super::model::{DisplayDescriptorSnapshot, MonitorSnapshot};

    /// Build one test descriptor payload with explicit geometry fields.
    fn descriptor(
        id: &str,
        primary: bool,
        width_px: u32,
        height_px: u32,
    ) -> DisplayDescriptorSnapshot {
        DisplayDescriptorSnapshot {
            backend: DisplayBackend::Win32,
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

    /// Build one test mode payload.
    fn mode(width: u32, height: u32, refresh_milli_hz: u32) -> DisplayMode {
        DisplayMode {
            width,
            height,
            refresh_milli_hz,
            format: 0,
            bit_depth: 8,
        }
    }

    /// Build one monitor snapshot payload for tests.
    fn snapshot(descriptor: DisplayDescriptorSnapshot, mode: DisplayMode) -> MonitorSnapshot {
        MonitorSnapshot {
            descriptor,
            current_mode: mode,
            desktop_mode: mode,
            modes: vec![mode],
        }
    }

    /// Monitor topology records should include removed, added, mode, and primary events.
    #[test]
    fn test_monitor_topology_records_emit_removed_added_and_primary_changed() {
        let previous = vec![snapshot(
            descriptor(r"\\.\DISPLAY1", true, 1920, 1080),
            mode(1920, 1080, 60_000),
        )];
        let next = vec![snapshot(
            descriptor(r"\\.\DISPLAY2", true, 2560, 1440),
            mode(2560, 1440, 144_000),
        )];

        let records = monitor_topology_records(&previous, &next);

        assert!(records.iter().any(|record| matches!(
            record.kind,
            DisplayEventRecordKind::Removed { ref id, .. } if id == r"\\.\DISPLAY1"
        )));
        assert!(records.iter().any(|record| matches!(
            record.kind,
            DisplayEventRecordKind::Added { ref descriptor } if descriptor.id == r"\\.\DISPLAY2"
        )));
        assert!(records.iter().any(|record| matches!(
            record.kind,
            DisplayEventRecordKind::ModeChanged { ref id, current: mode, .. } if id == r"\\.\DISPLAY2" && mode.refresh_milli_hz == 144_000
        )));
        assert!(records.iter().any(|record| matches!(
            record.kind,
            DisplayEventRecordKind::PrimaryChanged { current_id: Some(ref id), .. } if id == r"\\.\DISPLAY2"
        )));
    }

    /// Monitor topology records should include descriptor and mode deltas for one existing display.
    #[test]
    fn test_monitor_topology_records_emit_descriptor_and_mode_changes() {
        let previous = vec![snapshot(
            descriptor(r"\\.\DISPLAY1", true, 1920, 1080),
            mode(1920, 1080, 60_000),
        )];
        let next = vec![snapshot(
            descriptor(r"\\.\DISPLAY1", true, 2560, 1440),
            mode(2560, 1440, 120_000),
        )];

        let records = monitor_topology_records(&previous, &next);

        assert!(records.iter().any(|record| matches!(
            record.kind,
            DisplayEventRecordKind::DescriptorChanged { ref current, changed_mask, .. }
                // evaluate this condition
                if current.id == r"\\.\DISPLAY1" && (changed_mask & DISPLAY_CHANGED_MASK_BOUNDS) != 0
        )));
        assert!(records.iter().any(|record| matches!(
            record.kind,
            DisplayEventRecordKind::ModeChanged { ref id, current: mode, .. }
                // evaluate this condition
                if id == r"\\.\DISPLAY1" && mode.refresh_milli_hz == 120_000
        )));
    }

    /// Build one runtime state and one subscribed window-event stream for publication tests.
    fn subscribed_window_stream_with_filter(
        filter: WindowEventFilterState,
    ) -> (
        Arc<DisplayEventRuntimeState>,
        Arc<WindowEventBinding>,
        WindowHandle,
    ) {
        let runtime_state = Arc::new(DisplayEventRuntimeState::default());
        let window_event_binding = Arc::new(WindowEventBinding {
            state: Mutex::new(WindowEventState {
                queue_capacity: 32,
                overflow_policy: DisplayEventOverflowPolicy::DropOldest,
                overflow_error_pending: false,
                next_sequence: 1,
                dropped_count: 0,
                pending: std::collections::VecDeque::new(),
            }),
            filter,
            signal: Condvar::new(),
            owner_thread_id: 0,
        });
        runtime_state
            .window_event_registry
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .push(Arc::downgrade(&window_event_binding));
        let window = WindowHandle(ResourceId(123));

        (runtime_state, window_event_binding, window)
    }

    /// Build one runtime state and one subscribed window-event stream for publication tests.
    fn subscribed_window_stream() -> (
        Arc<DisplayEventRuntimeState>,
        Arc<WindowEventBinding>,
        WindowHandle,
    ) {
        subscribed_window_stream_with_filter(WindowEventFilterState::default())
    }

    /// Build one runtime state and one subscribed monitor-event stream for publication tests.
    fn subscribed_monitor_stream_with_filter(
        filter: MonitorEventFilterState,
    ) -> (Arc<DisplayEventRuntimeState>, Arc<MonitorEventBinding>) {
        let runtime_state = Arc::new(DisplayEventRuntimeState::default());
        let monitor_event_binding = Arc::new(MonitorEventBinding {
            state: Mutex::new(MonitorEventState {
                queue_capacity: 32,
                overflow_policy: DisplayEventOverflowPolicy::DropOldest,
                overflow_error_pending: false,
                next_sequence: 1,
                dropped_count: 0,
                pending: std::collections::VecDeque::new(),
            }),
            filter,
            signal: Condvar::new(),
        });
        runtime_state
            .monitor_event_registry
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .push(Arc::downgrade(&window_event_binding));

        (runtime_state, binding)
    }

    /// Drop lifecycle publishers should enqueue one started then completed event sequence.
    #[test]
    fn test_drop_lifecycle_publishers_enqueue_expected_record_kinds() {
        let (runtime_state, window_event_binding, window) = subscribed_window_stream();

        publish_window_drop_started_event(&runtime_state, window);
        publish_window_drop_completed_event(&runtime_state, window);
        publish_window_drop_cancelled_event(&runtime_state, window);

        let mut state = binding
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let first = state
            .pending
            .pop_front()
            .expect("missing dropStarted record");
        let second = state
            .pending
            .pop_front()
            .expect("missing dropCompleted record");
        let third = state
            .pending
            .pop_front()
            .expect("missing dropCancelled record");

        assert!(matches!(
            first.kind,
            WindowEventRecordKind::DropStarted { window: value } if value == window
        ));
        assert!(matches!(
            second.kind,
            WindowEventRecordKind::DropCompleted { window: value } if value == window
        ));
        assert!(matches!(
            third.kind,
            WindowEventRecordKind::DropCancelled { window: value } if value == window
        ));
    }

    /// File-drop publisher should preserve UTF-16 path payload and position metadata.
    #[test]
    fn test_file_drop_publisher_preserves_path_and_position_payloads() {
        let (runtime_state, window_event_binding, window) = subscribed_window_stream();
        let path_utf16 = "C:\\drop\\asset.txt".encode_utf16().collect::<Vec<_>>();
        let position = Some(WindowPosition { x: 480, y: 320 });

        publish_window_file_dropped_event(
            &runtime_state,
            window,
            Some(path_utf16.clone()),
            position,
        );

        let mut state = binding
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let record = state
            .pending
            .pop_front()
            .expect("missing fileDropped record");
        assert!(matches!(
            record.kind,
            WindowEventRecordKind::FileDropped {
                window: value,
                path_utf16: Some(path),
                position: payload_position,
            } if value == window && path == path_utf16 && payload_position == position
        ));
    }

    /// Text-drop publisher should preserve text payload and position metadata.
    #[test]
    fn test_text_drop_publisher_preserves_text_and_position_payloads() {
        let (runtime_state, window_event_binding, window) = subscribed_window_stream();
        let text_payload = String::from("dropped-text");
        let position = Some(WindowPosition { x: 640, y: 480 });

        publish_window_text_dropped_event(&runtime_state, window, text_payload.clone(), position);

        let mut state = binding
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let record = state
            .pending
            .pop_front()
            .expect("missing textDropped record");
        assert!(matches!(
            record.kind,
            WindowEventRecordKind::TextDropped {
                window: value,
                text,
                position: payload_position,
            } if value == window && text == text_payload && payload_position == position
        ));
    }

    /// Window-event filters should restrict delivery to one target window.
    #[test]
    fn test_window_event_filter_restricts_window_handle() {
        let target_window = WindowHandle(ResourceId(200));
        let other_window = WindowHandle(ResourceId(201));
        let (runtime_state, binding, _) =
            subscribed_window_stream_with_filter(WindowEventFilterState {
                window: Some(target_window),
                kind_mask: None,
            });

        publish_window_drop_started_event(&runtime_state, other_window);
        publish_window_drop_started_event(&runtime_state, target_window);

        let mut state = binding
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        assert_eq!(state.pending.len(), 1);
        assert!(matches!(
            state.pending.pop_front().map(|value| value.kind),
            Some(WindowEventRecordKind::DropStarted { window }) if window == target_window
        ));
    }

    /// Window-event filters should restrict delivery by event-kind mask.
    #[test]
    fn test_window_event_filter_restricts_kind_mask() {
        let (runtime_state, window_event_binding, window) =
            subscribed_window_stream_with_filter(WindowEventFilterState {
                window: None,
                kind_mask: Some(WINDOW_EVENT_KIND_DROP_STARTED),
            });

        publish_window_drop_started_event(&runtime_state, window);
        publish_window_drop_completed_event(&runtime_state, window);

        let mut state = binding
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        assert_eq!(state.pending.len(), 1);
        assert!(matches!(
            state.pending.pop_front().map(|value| value.kind),
            Some(WindowEventRecordKind::DropStarted { window: value }) if value == window
        ));
    }

    /// Window-event kind filters should route only text-dropped records.
    #[test]
    fn test_window_event_filter_accepts_text_dropped_kind() {
        let (runtime_state, window_event_binding, window) =
            subscribed_window_stream_with_filter(WindowEventFilterState {
                window: None,
                kind_mask: Some(WINDOW_EVENT_KIND_TEXT_DROPPED),
            });

        publish_window_drop_started_event(&runtime_state, window);
        publish_window_text_dropped_event(
            &runtime_state,
            window,
            String::from("accepted-text"),
            Some(WindowPosition { x: 11, y: 22 }),
        );

        let mut state = binding
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        assert_eq!(state.pending.len(), 1);
        assert!(matches!(
            state.pending.pop_front().map(|value| value.kind),
            Some(WindowEventRecordKind::TextDropped {
                window: value,
                text,
                position: Some(WindowPosition { x: 11, y: 22 })
            }) if value == window && text == "accepted-text"
        ));
    }

    /// Monitor-event filters should restrict delivery by event-kind mask.
    #[test]
    fn test_monitor_event_filter_restricts_kind_mask() {
        let (runtime_state, binding) =
            subscribed_monitor_stream_with_filter(MonitorEventFilterState {
                display_id: None,
                kind_mask: Some(DISPLAY_MONITOR_EVENT_KIND_MODE_CHANGED),
            });

        publish_monitor_event(
            &runtime_state,
            display_event_record(DisplayEventRecordKind::PrimaryChanged {
                previous_id: Some(String::from(r"\\.\DISPLAY1")),
                current_id: Some(String::from(r"\\.\DISPLAY2")),
            }),
        );
        publish_monitor_event(
            &runtime_state,
            display_event_record(DisplayEventRecordKind::ModeChanged {
                id: String::from(r"\\.\DISPLAY2"),
                previous: None,
                current: mode(2560, 1440, 144_000),
            }),
        );

        let mut state = binding
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        assert_eq!(state.pending.len(), 1);
        assert!(matches!(
            state.pending.pop_front().map(|value| value.kind),
            Some(DisplayEventRecordKind::ModeChanged { ref id, .. }) if id == r"\\.\DISPLAY2"
        ));
    }

    /// Monitor-event filters should restrict delivery by display identifier.
    #[test]
    fn test_monitor_event_filter_restricts_display_identifier() {
        let (runtime_state, binding) =
            subscribed_monitor_stream_with_filter(MonitorEventFilterState {
                display_id: Some(String::from(r"\\.\DISPLAY2")),
                kind_mask: None,
            });

        publish_monitor_event(
            &runtime_state,
            display_event_record(DisplayEventRecordKind::Added {
                descriptor: descriptor(r"\\.\DISPLAY1", false, 1920, 1080),
            }),
        );
        publish_monitor_event(
            &runtime_state,
            display_event_record(DisplayEventRecordKind::Added {
                descriptor: descriptor(r"\\.\DISPLAY2", true, 2560, 1440),
            }),
        );

        let mut state = binding
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        assert_eq!(state.pending.len(), 1);
        assert!(matches!(
            state.pending.pop_front().map(|value| value.kind),
            Some(DisplayEventRecordKind::Added { descriptor }) if descriptor.id == r"\\.\DISPLAY2"
        ));
    }
}
