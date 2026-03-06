use std::collections::VecDeque;
use std::sync::Mutex;
use std::thread::ThreadId;

use crate::diagnostic::RuntimeResult;
use crate::platform::display::host::unix::appkit::core as appkit_core;
use crate::platform::display::{
    DisplayEventOverflowPolicy, WindowAspectRatio, WindowChromeKind, WindowEventFilter,
    WindowEventOpenOptions, WindowLogicalSize, WindowModeOptions, WindowOcclusionState,
    WindowPhysicalSize, WindowPosition, WindowTheme, WindowVisibility,
};
use crate::platform::{core as core_platform, resource};

/// Stored window-event record payload.
#[derive(Debug, Clone)]
pub(crate) struct WindowEventRecord {
    /// Event timestamp in nanoseconds.
    pub(crate) timestamp_ns: u64,
    /// Event sequence number.
    pub(crate) sequence: u64,
    /// Number of dropped events observed before this event.
    pub(crate) dropped_count: u64,
    /// Event kind payload.
    pub(crate) kind: WindowEventRecordKind,
}

/// Stored window-event variant payload.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) enum WindowEventRecordKind {
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
    /// Scale-factor-changed payload.
    ScaleFactorChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Scale factor before this event.
        previous_scale_factor_milli: u32,
        /// Scale factor after this event.
        current_scale_factor_milli: u32,
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
    /// Opacity-changed payload.
    OpacityChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Opacity payload before this event.
        previous_opacity: f64,
        /// Opacity payload after this event.
        current_opacity: f64,
    },
    /// Parent-changed payload.
    ParentChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Parent payload before this event.
        previous_parent: Option<resource::WindowHandle>,
        /// Parent payload after this event.
        current_parent: Option<resource::WindowHandle>,
    },
    /// Transient-owner-changed payload.
    TransientChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Transient-owner payload before this event.
        previous_transient_for: Option<resource::WindowHandle>,
        /// Transient-owner payload after this event.
        current_transient_for: Option<resource::WindowHandle>,
    },
    /// Modal-changed payload.
    ModalChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Modal payload before this event.
        previous_modal: bool,
        /// Modal payload after this event.
        current_modal: bool,
    },
    /// Mouse-passthrough-changed payload.
    MousePassthroughChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Mouse-passthrough payload before this event.
        previous_mouse_passthrough: bool,
        /// Mouse-passthrough payload after this event.
        current_mouse_passthrough: bool,
    },
    /// Aspect-ratio-changed payload.
    AspectRatioChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Aspect-ratio payload before this event.
        previous_aspect_ratio: Option<WindowAspectRatio>,
        /// Aspect-ratio payload after this event.
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
    pub(crate) fn kind_mask(&self) -> u64 {
        // map window event variants to kind-mask lanes
        match self {
            WindowEventRecordKind::Created { .. } => appkit_core::WINDOW_EVENT_KIND_CREATED,
            WindowEventRecordKind::CloseRequested { .. } => {
                appkit_core::WINDOW_EVENT_KIND_CLOSE_REQUESTED
            }
            WindowEventRecordKind::Destroyed { .. } => appkit_core::WINDOW_EVENT_KIND_DESTROYED,
            WindowEventRecordKind::RefreshRequested { .. } => {
                appkit_core::WINDOW_EVENT_KIND_REFRESH_REQUESTED
            }
            WindowEventRecordKind::VisibilityChanged { .. } => {
                appkit_core::WINDOW_EVENT_KIND_VISIBILITY_CHANGED
            }
            WindowEventRecordKind::OcclusionChanged { .. } => {
                appkit_core::WINDOW_EVENT_KIND_OCCLUSION_CHANGED
            }
            WindowEventRecordKind::PositionChanged { .. } => {
                appkit_core::WINDOW_EVENT_KIND_POSITION_CHANGED
            }
            WindowEventRecordKind::SizeChanged { .. } => {
                appkit_core::WINDOW_EVENT_KIND_SIZE_CHANGED
            }
            WindowEventRecordKind::FocusChanged { .. } => {
                appkit_core::WINDOW_EVENT_KIND_FOCUS_CHANGED
            }
            WindowEventRecordKind::ModeChanged { .. } => {
                appkit_core::WINDOW_EVENT_KIND_MODE_CHANGED
            }
            WindowEventRecordKind::ScaleFactorChanged { .. } => {
                appkit_core::WINDOW_EVENT_KIND_SCALE_FACTOR_CHANGED
            }
            WindowEventRecordKind::DisplayChanged { .. } => {
                appkit_core::WINDOW_EVENT_KIND_DISPLAY_CHANGED
            }
            WindowEventRecordKind::ThemeChanged { .. } => {
                appkit_core::WINDOW_EVENT_KIND_THEME_CHANGED
            }
            WindowEventRecordKind::ChromeChanged { .. } => {
                appkit_core::WINDOW_EVENT_KIND_CHROME_CHANGED
            }
            WindowEventRecordKind::OpacityChanged { .. } => {
                appkit_core::WINDOW_EVENT_KIND_OPACITY_CHANGED
            }
            WindowEventRecordKind::ParentChanged { .. } => {
                appkit_core::WINDOW_EVENT_KIND_PARENT_CHANGED
            }
            WindowEventRecordKind::TransientChanged { .. } => {
                appkit_core::WINDOW_EVENT_KIND_TRANSIENT_CHANGED
            }
            WindowEventRecordKind::ModalChanged { .. } => {
                appkit_core::WINDOW_EVENT_KIND_MODAL_CHANGED
            }
            WindowEventRecordKind::MousePassthroughChanged { .. } => {
                appkit_core::WINDOW_EVENT_KIND_MOUSE_PASSTHROUGH_CHANGED
            }
            WindowEventRecordKind::AspectRatioChanged { .. } => {
                appkit_core::WINDOW_EVENT_KIND_ASPECT_RATIO_CHANGED
            }
            WindowEventRecordKind::DropStarted { .. } => {
                appkit_core::WINDOW_EVENT_KIND_DROP_STARTED
            }
            WindowEventRecordKind::FileHovered { .. } => {
                appkit_core::WINDOW_EVENT_KIND_FILE_HOVERED
            }
            WindowEventRecordKind::DropCancelled { .. } => {
                appkit_core::WINDOW_EVENT_KIND_DROP_CANCELLED
            }
            WindowEventRecordKind::DropCompleted { .. } => {
                appkit_core::WINDOW_EVENT_KIND_DROP_COMPLETED
            }
            WindowEventRecordKind::FileHoverLeft { .. } => {
                appkit_core::WINDOW_EVENT_KIND_FILE_HOVER_LEFT
            }
            WindowEventRecordKind::FileDropped { .. } => {
                appkit_core::WINDOW_EVENT_KIND_FILE_DROPPED
            }
            WindowEventRecordKind::TextDropped { .. } => {
                appkit_core::WINDOW_EVENT_KIND_TEXT_DROPPED
            }
        }
    }

    /// Return the associated runtime window handle.
    pub(crate) fn window(&self) -> resource::WindowHandle {
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
            | WindowEventRecordKind::ScaleFactorChanged { window, .. }
            | WindowEventRecordKind::DisplayChanged { window, .. }
            | WindowEventRecordKind::ThemeChanged { window, .. }
            | WindowEventRecordKind::ChromeChanged { window, .. }
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

/// Parsed window-event filter state.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WindowEventFilterState {
    /// Optional window filter.
    pub(crate) window: Option<resource::WindowHandle>,
    /// Enabled kind-mask bits.
    pub(crate) kind_mask: u64,
}

impl WindowEventFilterState {
    /// Build filter state from window-event open options.
    pub(crate) fn from_open_options(options: WindowEventOpenOptions) -> RuntimeResult<Self> {
        // map optional filter to normalized state
        let filter = options.filter.unwrap_or(WindowEventFilter {
            window: None,
            kind_mask: None,
        });
        let kind_mask = appkit_core::window_kind_mask(filter.kind_mask);
        appkit_core::validate_window_event_kind_mask(kind_mask, "options.filter.kindMask")?;

        Ok(Self {
            window: filter.window,
            kind_mask,
        })
    }

    /// Return whether one record matches this filter.
    pub(crate) fn matches(&self, record: &WindowEventRecord) -> bool {
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

/// Mutable window-event stream queue state.
#[derive(Debug)]
pub(crate) struct WindowEventState {
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
    pub(crate) seeded: VecDeque<WindowEventRecord>,
}

/// Shared window-event stream binding.
#[derive(Debug)]
pub(crate) struct WindowEventBinding {
    /// Mutable stream state.
    pub(crate) state: Mutex<WindowEventState>,
    /// Stream filter configuration.
    pub(crate) filter: WindowEventFilterState,
    /// Owner thread id for AppKit event pumping operations.
    pub(crate) owner_thread_id: ThreadId,
}

/// Build one window-event record with default metadata fields.
pub(crate) fn window_event_record(kind: WindowEventRecordKind) -> WindowEventRecord {
    WindowEventRecord {
        timestamp_ns: core_platform::monotonic_now_ns(),
        sequence: 0,
        dropped_count: 0,
        kind,
    }
}
