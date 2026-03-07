use std::collections::VecDeque;
use std::sync::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::display::windows::win32::core as win32_core;
use crate::platform::display::{
    DisplayEventOverflowPolicy, WindowAspectRatio, WindowChromeKind, WindowEventKindMask,
    WindowEventOpenOptions, WindowLogicalSize, WindowModeOptions, WindowOcclusionState,
    WindowPhysicalSize, WindowPosition, WindowSafeAreaInsets, WindowTheme, WindowVisibility,
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
    /// Safe-area-changed payload.
    SafeAreaChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Previous safe-area insets.
        previous_safe_area_insets: Option<WindowSafeAreaInsets>,
        /// Current safe-area insets.
        current_safe_area_insets: Option<WindowSafeAreaInsets>,
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

impl WindowEventRecordKind {
    /// Return the event-kind bit for this window-event record.
    pub(crate) fn kind_mask(&self) -> u64 {
        // map window event variants to kind mask lanes
        match self {
            WindowEventRecordKind::Created { .. } => win32_core::WINDOW_EVENT_KIND_CREATED,
            WindowEventRecordKind::CloseRequested { .. } => {
                win32_core::WINDOW_EVENT_KIND_CLOSE_REQUESTED
            }
            WindowEventRecordKind::Destroyed { .. } => win32_core::WINDOW_EVENT_KIND_DESTROYED,
            WindowEventRecordKind::FocusChanged { .. } => {
                win32_core::WINDOW_EVENT_KIND_FOCUS_CHANGED
            }
            WindowEventRecordKind::VisibilityChanged { .. } => {
                win32_core::WINDOW_EVENT_KIND_VISIBILITY_CHANGED
            }
            WindowEventRecordKind::OcclusionChanged { .. } => {
                win32_core::WINDOW_EVENT_KIND_OCCLUSION_CHANGED
            }
            WindowEventRecordKind::PositionChanged { .. } => {
                win32_core::WINDOW_EVENT_KIND_POSITION_CHANGED
            }
            WindowEventRecordKind::SizeChanged { .. } => win32_core::WINDOW_EVENT_KIND_SIZE_CHANGED,
            WindowEventRecordKind::ScaleFactorChanged { .. } => {
                win32_core::WINDOW_EVENT_KIND_SCALE_FACTOR_CHANGED
            }
            WindowEventRecordKind::RefreshRequested { .. } => {
                win32_core::WINDOW_EVENT_KIND_REFRESH_REQUESTED
            }
            WindowEventRecordKind::ModeChanged { .. } => win32_core::WINDOW_EVENT_KIND_MODE_CHANGED,
            WindowEventRecordKind::DisplayChanged { .. } => {
                win32_core::WINDOW_EVENT_KIND_DISPLAY_CHANGED
            }
            WindowEventRecordKind::ThemeChanged { .. } => {
                win32_core::WINDOW_EVENT_KIND_THEME_CHANGED
            }
            WindowEventRecordKind::ChromeChanged { .. } => {
                win32_core::WINDOW_EVENT_KIND_CHROME_CHANGED
            }
            WindowEventRecordKind::TaskbarVisibilityChanged { .. } => {
                win32_core::WINDOW_EVENT_KIND_TASKBAR_VISIBILITY_CHANGED
            }
            WindowEventRecordKind::SafeAreaChanged { .. } => {
                win32_core::WINDOW_EVENT_KIND_SAFE_AREA_CHANGED
            }
            WindowEventRecordKind::OpacityChanged { .. } => {
                win32_core::WINDOW_EVENT_KIND_OPACITY_CHANGED
            }
            WindowEventRecordKind::ParentChanged { .. } => {
                win32_core::WINDOW_EVENT_KIND_PARENT_CHANGED
            }
            WindowEventRecordKind::TransientChanged { .. } => {
                win32_core::WINDOW_EVENT_KIND_TRANSIENT_CHANGED
            }
            WindowEventRecordKind::ModalChanged { .. } => {
                win32_core::WINDOW_EVENT_KIND_MODAL_CHANGED
            }
            WindowEventRecordKind::MousePassthroughChanged { .. } => {
                win32_core::WINDOW_EVENT_KIND_MOUSE_PASSTHROUGH_CHANGED
            }
            WindowEventRecordKind::AspectRatioChanged { .. } => {
                win32_core::WINDOW_EVENT_KIND_ASPECT_RATIO_CHANGED
            }
            WindowEventRecordKind::DropStarted { .. } => win32_core::WINDOW_EVENT_KIND_DROP_STARTED,
            WindowEventRecordKind::FileHovered { .. } => win32_core::WINDOW_EVENT_KIND_FILE_HOVERED,
            WindowEventRecordKind::DropCancelled { .. } => {
                win32_core::WINDOW_EVENT_KIND_DROP_CANCELLED
            }
            WindowEventRecordKind::DropCompleted { .. } => {
                win32_core::WINDOW_EVENT_KIND_DROP_COMPLETED
            }
            WindowEventRecordKind::FileHoverLeft { .. } => {
                win32_core::WINDOW_EVENT_KIND_FILE_HOVER_LEFT
            }
            WindowEventRecordKind::FileDropped { .. } => win32_core::WINDOW_EVENT_KIND_FILE_DROPPED,
            WindowEventRecordKind::TextDropped { .. } => win32_core::WINDOW_EVENT_KIND_TEXT_DROPPED,
        }
    }

    /// Return the associated runtime window handle.
    pub(crate) fn window(&self) -> resource::WindowHandle {
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
            | WindowEventRecordKind::SafeAreaChanged { window, .. }
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
/// Open-time filter state for one window-event stream.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct WindowEventFilterState {
    /// Optional window-handle restriction.
    window: Option<resource::WindowHandle>,
    /// Optional window-event kind-mask restriction.
    kind_mask: Option<u64>,
}

impl WindowEventFilterState {
    /// Build one window-event filter state from open options.
    pub(crate) fn from_open_options(options: WindowEventOpenOptions) -> RuntimeResult<Self> {
        let Some(filter) = options.filter else {
            return Ok(Self::default());
        };

        // decode optional kind mask from abi payload
        let kind_mask = filter.kind_mask.map(|value: WindowEventKindMask| value.0);

        // validate kind-mask bits when provided
        if let Some(kind_mask) = kind_mask {
            win32_core::validate_window_event_kind_mask(kind_mask, "options.filter.kindMask")?;
        }

        let filter = Self {
            window: filter.window,
            kind_mask,
        };
        Ok(filter)
    }

    /// Return whether one window-event record matches this filter.
    pub(crate) fn matches(&self, record: &WindowEventRecord) -> bool {
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

/// Build one window-event record with default queue metadata.
pub(crate) fn window_event_record(kind: WindowEventRecordKind) -> WindowEventRecord {
    WindowEventRecord {
        timestamp_ns: core_platform::monotonic_now_ns(),
        sequence: 0,
        dropped_count: 0,
        kind,
    }
}
/// Mutable window-event stream state.
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
/// Shared window-event stream payload.
#[derive(Debug)]
pub(crate) struct WindowEventStream {
    /// Stable runtime stream identifier.
    pub(crate) stream_id: u64,
    /// Mutable stream state.
    pub(crate) state: Mutex<WindowEventState>,
    /// Stream filter configuration.
    pub(crate) filter: WindowEventFilterState,
}
