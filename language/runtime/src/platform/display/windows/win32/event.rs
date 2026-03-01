use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, OnceLock, Weak};
use std::time::Duration;

use windows_sys::Win32::System::Threading::GetCurrentThreadId;

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayAddedEvent, DisplayAddedPayload, DisplayBackend, DisplayDescriptorChangedEvent,
    DisplayDescriptorChangedPayload, DisplayEvent, DisplayEventMetadata,
    DisplayEventOverflowPolicy, DisplayMode, DisplayModeChangedEvent, DisplayModeChangedPayload,
    DisplayMonitorEventOpenOptions, DisplayPrimaryChangedEvent, DisplayPrimaryPayload,
    WindowCloseRequestedEvent, WindowCreatedEvent, WindowDestroyedEvent, WindowDisplayChangedEvent,
    WindowDisplayPayload, WindowEvent, WindowEventMetadata, WindowEventOpenOptions,
    WindowFocusChangedEvent, WindowFocusPayload, WindowLogicalSize, WindowModeChangedEvent,
    WindowModeOptions, WindowModePayload, WindowOcclusionChangedEvent, WindowOcclusionPayload,
    WindowPhysicalSize, WindowPosition, WindowPositionChangedEvent, WindowPositionPayload,
    WindowRefreshRequestedEvent, WindowScaleFactorChangedEvent, WindowScaleFactorPayload,
    WindowSizeChangedEvent, WindowSizePayload, WindowTheme, WindowThemeChangedEvent,
    WindowThemePayload, WindowVisibility, WindowVisibilityChangedEvent, WindowVisibilityPayload,
};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{NativeArray, resource};
use crate::runtime::BindingCallContext;

use super::model::{DisplayDescriptorOwned, Win32WindowBinding};
use super::{core, monitor, resource as display_resource, window};

/// Maximum wait slice used while interleaving Win32 message pumping in event reads.
const WINDOW_EVENT_WAIT_SLICE_NS: u64 = 10_000_000;

/// Stored monitor-event record payload.
#[derive(Debug, Clone)]
struct DisplayEventRecord {
    /// Event timestamp in nanoseconds.
    timestamp_ns: u64,
    /// Event sequence number.
    sequence: u64,
    /// Event kind payload.
    kind: DisplayEventRecordKind,
}

/// Stored monitor-event variant payload.
#[derive(Debug, Clone)]
enum DisplayEventRecordKind {
    /// Added-event payload.
    Added {
        /// Added descriptor payload.
        descriptor: DisplayDescriptorOwned,
    },
    /// Primary-changed payload.
    PrimaryChanged {
        /// Current primary display identifier.
        id: Option<String>,
    },
    /// Descriptor-changed payload.
    DescriptorChanged {
        /// Descriptor payload after mutation.
        descriptor: DisplayDescriptorOwned,
        /// Changed-field bit mask.
        changed_mask: u32,
    },
    /// Mode-changed payload.
    ModeChanged {
        /// Associated display identifier.
        id: String,
        /// Current display mode payload.
        mode: DisplayMode,
    },
}

/// Stored window-event record payload.
#[derive(Debug, Clone)]
struct WindowEventRecord {
    /// Event timestamp in nanoseconds.
    timestamp_ns: u64,
    /// Event sequence number.
    sequence: u64,
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
        /// Focus state after this event.
        focused: bool,
    },
    /// Visibility-changed payload.
    VisibilityChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Visibility state after this event.
        visibility: WindowVisibility,
    },
    /// Occlusion-changed payload.
    OcclusionChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Occlusion state after this event.
        occluded: bool,
    },
    /// Position-changed payload.
    PositionChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Position after this event.
        position: WindowPosition,
    },
    /// Size-changed payload.
    SizeChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Logical size after this event.
        size_logical: WindowLogicalSize,
        /// Physical size after this event.
        size_physical: WindowPhysicalSize,
    },
    /// Scale-factor-changed payload.
    ScaleFactorChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Scale factor after this event.
        scale_factor_milli: u32,
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
        /// Mode payload after this event.
        mode: WindowModeOptions,
    },
    /// Display-changed payload.
    DisplayChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Display payload after this event.
        display: Option<resource::DisplayHandle>,
    },
    /// Theme-changed payload.
    ThemeChanged {
        /// Associated runtime window handle.
        window: resource::WindowHandle,
        /// Theme payload after this event.
        theme: WindowTheme,
    },
}

/// Resource payload for one monitor-event stream.
#[derive(Debug)]
pub(super) struct MonitorEventBinding {
    /// Shared mutable stream state.
    state: Mutex<MonitorEventState>,
    /// Wake lane for blocking readers.
    signal: Condvar,
}

/// Mutable monitor-event stream state.
#[derive(Debug)]
struct MonitorEventState {
    /// Queue capacity for this stream.
    queue_capacity: usize,
    /// Queue overflow policy for this stream.
    overflow_policy: DisplayEventOverflowPolicy,
    /// Latched overflow-error state.
    overflow_error_pending: bool,
    /// Next sequence number for this stream.
    next_sequence: u64,
    /// Pending queue payload.
    pending: VecDeque<DisplayEventRecord>,
}

/// Resource payload for one window-event stream.
#[derive(Debug)]
pub(super) struct WindowEventBinding {
    /// Shared mutable stream state.
    state: Mutex<WindowEventState>,
    /// Wake lane for blocking readers.
    signal: Condvar,
    /// Owner thread identifier used for message pumping.
    owner_thread_id: u32,
}

/// Mutable window-event stream state.
#[derive(Debug)]
struct WindowEventState {
    /// Queue capacity for this stream.
    queue_capacity: usize,
    /// Queue overflow policy for this stream.
    overflow_policy: DisplayEventOverflowPolicy,
    /// Latched overflow-error state.
    overflow_error_pending: bool,
    /// Next sequence number for this stream.
    next_sequence: u64,
    /// Pending queue payload.
    pending: VecDeque<WindowEventRecord>,
}

/// Shared event subscriber list for monitor-event streams.
static MONITOR_EVENT_REGISTRY: OnceLock<Mutex<Vec<Weak<MonitorEventBinding>>>> = OnceLock::new();
/// Shared event subscriber list for window-event streams.
static WINDOW_EVENT_REGISTRY: OnceLock<Mutex<Vec<Weak<WindowEventBinding>>>> = OnceLock::new();

/// Return one shared monitor-event registry lock.
fn monitor_event_registry() -> &'static Mutex<Vec<Weak<MonitorEventBinding>>> {
    MONITOR_EVENT_REGISTRY.get_or_init(|| Mutex::new(Vec::new()))
}

/// Return one shared window-event registry lock.
fn window_event_registry() -> &'static Mutex<Vec<Weak<WindowEventBinding>>> {
    WINDOW_EVENT_REGISTRY.get_or_init(|| Mutex::new(Vec::new()))
}

/// Convert one remaining timeout payload into a condition wait duration.
fn wait_duration(remaining_ns: u64) -> Duration {
    Duration::from_nanos(remaining_ns)
}

/// Return the current host thread identifier.
fn current_thread_id() -> u32 {
    unsafe { GetCurrentThreadId() }
}

/// Ensure one window-event stream operation is running on its owner thread.
fn ensure_window_event_thread(
    binding: &WindowEventBinding,
    operation: &'static str,
) -> RuntimeResult<()> {
    let current = current_thread_id();
    if current == binding.owner_thread_id {
        return Ok(());
    }

    Err(core::invalid_argument(
        "handle",
        format!(
            "{operation} must run on owner thread {}, current thread is {current}",
            binding.owner_thread_id
        ),
    ))
}

/// Push one monitor-event record into one stream queue.
fn push_monitor_event(state: &mut MonitorEventState, mut event: DisplayEventRecord) {
    while state.pending.len() >= state.queue_capacity {
        match state.overflow_policy {
            DisplayEventOverflowPolicy::DropOldest => {
                let _ = state.pending.pop_front();
            }
            DisplayEventOverflowPolicy::DropNewest => {
                return;
            }
            DisplayEventOverflowPolicy::Error => {
                state.pending.clear();
                state.overflow_error_pending = true;
                return;
            }
        }
    }

    event.sequence = state.next_sequence;
    state.next_sequence = state.next_sequence.saturating_add(1);
    state.pending.push_back(event);
}

/// Push one window-event record into one stream queue.
fn push_window_event(state: &mut WindowEventState, mut event: WindowEventRecord) {
    while state.pending.len() >= state.queue_capacity {
        match state.overflow_policy {
            DisplayEventOverflowPolicy::DropOldest => {
                let _ = state.pending.pop_front();
            }
            DisplayEventOverflowPolicy::DropNewest => {
                return;
            }
            DisplayEventOverflowPolicy::Error => {
                state.pending.clear();
                state.overflow_error_pending = true;
                return;
            }
        }
    }

    event.sequence = state.next_sequence;
    state.next_sequence = state.next_sequence.saturating_add(1);
    state.pending.push_back(event);
}

/// Publish one monitor-event record to all active stream subscribers.
fn publish_monitor_event(record: DisplayEventRecord) {
    let mut registry = monitor_event_registry()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    registry.retain(|binding| {
        let Some(binding) = binding.upgrade() else {
            return false;
        };
        let mut state = binding
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        push_monitor_event(&mut state, record.clone());
        drop(state);
        binding.signal.notify_all();
        true
    });
}

/// Publish one window-event record to all active stream subscribers.
fn publish_window_event(record: WindowEventRecord) {
    let mut registry = window_event_registry()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    registry.retain(|binding| {
        let Some(binding) = binding.upgrade() else {
            return false;
        };
        let mut state = binding
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        push_window_event(&mut state, record.clone());
        drop(state);
        binding.signal.notify_all();
        true
    });
}

/// Build one display-event metadata payload.
fn display_event_metadata(
    context: &BindingCallContext,
    display_id: Option<&str>,
    timestamp_ns: u64,
    sequence: u64,
) -> DisplayEventMetadata {
    DisplayEventMetadata {
        backend: DisplayBackend::Win32,
        display_id: display_id.map(|value| context.store_string(value)),
        timestamp_ns,
        sequence,
    }
}

/// Build one window-event metadata payload.
fn window_event_metadata(
    window: resource::WindowHandle,
    timestamp_ns: u64,
    sequence: u64,
) -> WindowEventMetadata {
    WindowEventMetadata {
        backend: DisplayBackend::Win32,
        window,
        timestamp_ns,
        sequence,
    }
}

/// Convert one stored monitor-event record into one ABI event payload.
fn display_event_from_record(
    context: &BindingCallContext,
    value: DisplayEventRecord,
) -> DisplayEvent {
    match value.kind {
        DisplayEventRecordKind::Added { descriptor } => {
            DisplayEvent::DisplayAddedEvent(DisplayAddedEvent {
                kind: context.store_string("added"),
                metadata: display_event_metadata(
                    context,
                    Some(&descriptor.id),
                    value.timestamp_ns,
                    value.sequence,
                ),
                payload: DisplayAddedPayload {
                    descriptor: monitor::descriptor_from_owned(context, &descriptor),
                },
            })
        }
        DisplayEventRecordKind::PrimaryChanged { id } => {
            DisplayEvent::DisplayPrimaryChangedEvent(DisplayPrimaryChangedEvent {
                kind: context.store_string("primaryChanged"),
                metadata: display_event_metadata(
                    context,
                    id.as_deref(),
                    value.timestamp_ns,
                    value.sequence,
                ),
                payload: DisplayPrimaryPayload {
                    id: id.map(|value| context.store_string(&value)),
                },
            })
        }
        DisplayEventRecordKind::DescriptorChanged {
            descriptor,
            changed_mask,
        } => DisplayEvent::DisplayDescriptorChangedEvent(DisplayDescriptorChangedEvent {
            kind: context.store_string("descriptorChanged"),
            metadata: display_event_metadata(
                context,
                Some(&descriptor.id),
                value.timestamp_ns,
                value.sequence,
            ),
            payload: DisplayDescriptorChangedPayload {
                descriptor: monitor::descriptor_from_owned(context, &descriptor),
                changed_mask,
            },
        }),
        DisplayEventRecordKind::ModeChanged { id, mode } => {
            DisplayEvent::DisplayModeChangedEvent(DisplayModeChangedEvent {
                kind: context.store_string("modeChanged"),
                metadata: display_event_metadata(
                    context,
                    Some(&id),
                    value.timestamp_ns,
                    value.sequence,
                ),
                payload: DisplayModeChangedPayload { mode },
            })
        }
    }
}

/// Convert one stored window-event record into one ABI event payload.
fn window_event_from_record(value: WindowEventRecord, context: &BindingCallContext) -> WindowEvent {
    match value.kind {
        WindowEventRecordKind::Created { window } => {
            WindowEvent::WindowCreatedEvent(WindowCreatedEvent {
                kind: context.store_string("created"),
                metadata: window_event_metadata(window, value.timestamp_ns, value.sequence),
            })
        }
        WindowEventRecordKind::CloseRequested { window } => {
            WindowEvent::WindowCloseRequestedEvent(WindowCloseRequestedEvent {
                kind: context.store_string("closeRequested"),
                metadata: window_event_metadata(window, value.timestamp_ns, value.sequence),
            })
        }
        WindowEventRecordKind::Destroyed { window } => {
            WindowEvent::WindowDestroyedEvent(WindowDestroyedEvent {
                kind: context.store_string("destroyed"),
                metadata: window_event_metadata(window, value.timestamp_ns, value.sequence),
            })
        }
        WindowEventRecordKind::FocusChanged { window, focused } => {
            WindowEvent::WindowFocusChangedEvent(WindowFocusChangedEvent {
                kind: context.store_string("focusChanged"),
                metadata: window_event_metadata(window, value.timestamp_ns, value.sequence),
                payload: WindowFocusPayload { focused },
            })
        }
        WindowEventRecordKind::VisibilityChanged { window, visibility } => {
            WindowEvent::WindowVisibilityChangedEvent(WindowVisibilityChangedEvent {
                kind: context.store_string("visibilityChanged"),
                metadata: window_event_metadata(window, value.timestamp_ns, value.sequence),
                payload: WindowVisibilityPayload { visibility },
            })
        }
        WindowEventRecordKind::OcclusionChanged { window, occluded } => {
            WindowEvent::WindowOcclusionChangedEvent(WindowOcclusionChangedEvent {
                kind: context.store_string("occlusionChanged"),
                metadata: window_event_metadata(window, value.timestamp_ns, value.sequence),
                payload: WindowOcclusionPayload { occluded },
            })
        }
        WindowEventRecordKind::PositionChanged { window, position } => {
            WindowEvent::WindowPositionChangedEvent(WindowPositionChangedEvent {
                kind: context.store_string("positionChanged"),
                metadata: window_event_metadata(window, value.timestamp_ns, value.sequence),
                payload: WindowPositionPayload { position },
            })
        }
        WindowEventRecordKind::SizeChanged {
            window,
            size_logical,
            size_physical,
        } => WindowEvent::WindowSizeChangedEvent(WindowSizeChangedEvent {
            kind: context.store_string("sizeChanged"),
            metadata: window_event_metadata(window, value.timestamp_ns, value.sequence),
            payload: WindowSizePayload {
                size_logical,
                size_physical,
            },
        }),
        WindowEventRecordKind::ScaleFactorChanged {
            window,
            scale_factor_milli,
        } => WindowEvent::WindowScaleFactorChangedEvent(WindowScaleFactorChangedEvent {
            kind: context.store_string("scaleFactorChanged"),
            metadata: window_event_metadata(window, value.timestamp_ns, value.sequence),
            payload: WindowScaleFactorPayload { scale_factor_milli },
        }),
        WindowEventRecordKind::RefreshRequested { window } => {
            WindowEvent::WindowRefreshRequestedEvent(WindowRefreshRequestedEvent {
                kind: context.store_string("refreshRequested"),
                metadata: window_event_metadata(window, value.timestamp_ns, value.sequence),
            })
        }
        WindowEventRecordKind::ModeChanged { window, mode } => {
            WindowEvent::WindowModeChangedEvent(WindowModeChangedEvent {
                kind: context.store_string("modeChanged"),
                metadata: window_event_metadata(window, value.timestamp_ns, value.sequence),
                payload: WindowModePayload { mode },
            })
        }
        WindowEventRecordKind::DisplayChanged { window, display } => {
            WindowEvent::WindowDisplayChangedEvent(WindowDisplayChangedEvent {
                kind: context.store_string("displayChanged"),
                metadata: window_event_metadata(window, value.timestamp_ns, value.sequence),
                payload: WindowDisplayPayload { display },
            })
        }
        WindowEventRecordKind::ThemeChanged { window, theme } => {
            WindowEvent::WindowThemeChangedEvent(WindowThemeChangedEvent {
                kind: context.store_string("themeChanged"),
                metadata: window_event_metadata(window, value.timestamp_ns, value.sequence),
                payload: WindowThemePayload { theme },
            })
        }
    }
}

/// Publish one mode-changed monitor event.
pub(super) fn publish_mode_changed_event(display_id: &str, mode: DisplayMode) {
    let record = DisplayEventRecord {
        timestamp_ns: core::now_timestamp_ns(),
        sequence: 0,
        kind: DisplayEventRecordKind::ModeChanged {
            id: display_id.to_string(),
            mode,
        },
    };

    publish_monitor_event(record);
}

/// Publish one descriptor-changed monitor event.
pub(super) fn publish_descriptor_changed_event(
    descriptor: &DisplayDescriptorOwned,
    changed_mask: u32,
) {
    let record = DisplayEventRecord {
        timestamp_ns: core::now_timestamp_ns(),
        sequence: 0,
        kind: DisplayEventRecordKind::DescriptorChanged {
            descriptor: descriptor.clone(),
            changed_mask,
        },
    };

    publish_monitor_event(record);
}

/// Seed one monitor-event stream with current monitor snapshot events.
fn seed_monitor_event_stream(state: &mut MonitorEventState) -> RuntimeResult<()> {
    let snapshots = monitor::enumerate_monitor_snapshots()?;
    let mut primary_id = None;

    for snapshot in snapshots {
        if snapshot.descriptor.primary {
            primary_id = Some(snapshot.descriptor.id.clone());
        }

        let added = DisplayEventRecord {
            timestamp_ns: core::now_timestamp_ns(),
            sequence: 0,
            kind: DisplayEventRecordKind::Added {
                descriptor: snapshot.descriptor.clone(),
            },
        };
        push_monitor_event(state, added);

        let mode_changed = DisplayEventRecord {
            timestamp_ns: core::now_timestamp_ns(),
            sequence: 0,
            kind: DisplayEventRecordKind::ModeChanged {
                id: snapshot.descriptor.id,
                mode: snapshot.current_mode,
            },
        };
        push_monitor_event(state, mode_changed);
    }

    let primary_changed = DisplayEventRecord {
        timestamp_ns: core::now_timestamp_ns(),
        sequence: 0,
        kind: DisplayEventRecordKind::PrimaryChanged { id: primary_id },
    };
    push_monitor_event(state, primary_changed);

    Ok(())
}

/// Publish one created window event.
pub(super) fn publish_window_created_event(window: resource::WindowHandle) {
    publish_window_event(WindowEventRecord {
        timestamp_ns: core::now_timestamp_ns(),
        sequence: 0,
        kind: WindowEventRecordKind::Created { window },
    });
}

/// Publish one destroyed window event.
pub(super) fn publish_window_destroyed_event(window: resource::WindowHandle) {
    publish_window_event(WindowEventRecord {
        timestamp_ns: core::now_timestamp_ns(),
        sequence: 0,
        kind: WindowEventRecordKind::Destroyed { window },
    });
}

/// Publish one close-requested window event.
pub(super) fn publish_window_close_requested_event(window: resource::WindowHandle) {
    publish_window_event(WindowEventRecord {
        timestamp_ns: core::now_timestamp_ns(),
        sequence: 0,
        kind: WindowEventRecordKind::CloseRequested { window },
    });
}

/// Publish one refresh-requested window event.
pub(super) fn publish_window_refresh_event(window: resource::WindowHandle) {
    publish_window_event(WindowEventRecord {
        timestamp_ns: core::now_timestamp_ns(),
        sequence: 0,
        kind: WindowEventRecordKind::RefreshRequested { window },
    });
}

/// Publish one visibility-changed window event.
fn publish_window_visibility_event(window: resource::WindowHandle, visibility: WindowVisibility) {
    publish_window_event(WindowEventRecord {
        timestamp_ns: core::now_timestamp_ns(),
        sequence: 0,
        kind: WindowEventRecordKind::VisibilityChanged { window, visibility },
    });
}

/// Publish one position-changed window event.
fn publish_window_position_event(window: resource::WindowHandle, position: WindowPosition) {
    publish_window_event(WindowEventRecord {
        timestamp_ns: core::now_timestamp_ns(),
        sequence: 0,
        kind: WindowEventRecordKind::PositionChanged { window, position },
    });
}

/// Publish one size-changed window event.
fn publish_window_size_event(
    window: resource::WindowHandle,
    size_logical: WindowLogicalSize,
    size_physical: WindowPhysicalSize,
) {
    publish_window_event(WindowEventRecord {
        timestamp_ns: core::now_timestamp_ns(),
        sequence: 0,
        kind: WindowEventRecordKind::SizeChanged {
            window,
            size_logical,
            size_physical,
        },
    });
}

/// Publish one scale-factor changed window event.
fn publish_window_scale_event(window: resource::WindowHandle, scale_factor_milli: u32) {
    publish_window_event(WindowEventRecord {
        timestamp_ns: core::now_timestamp_ns(),
        sequence: 0,
        kind: WindowEventRecordKind::ScaleFactorChanged {
            window,
            scale_factor_milli,
        },
    });
}

/// Publish one mode-changed window event.
pub(super) fn publish_window_mode_event(window: resource::WindowHandle, mode: WindowModeOptions) {
    publish_window_event(WindowEventRecord {
        timestamp_ns: core::now_timestamp_ns(),
        sequence: 0,
        kind: WindowEventRecordKind::ModeChanged { window, mode },
    });
}

/// Publish one display-changed window event.
pub(super) fn publish_window_display_event(
    window: resource::WindowHandle,
    display: Option<resource::DisplayHandle>,
) {
    publish_window_event(WindowEventRecord {
        timestamp_ns: core::now_timestamp_ns(),
        sequence: 0,
        kind: WindowEventRecordKind::DisplayChanged { window, display },
    });
}

/// Publish one focus-changed window event.
fn publish_window_focus_event(window: resource::WindowHandle, focused: bool) {
    publish_window_event(WindowEventRecord {
        timestamp_ns: core::now_timestamp_ns(),
        sequence: 0,
        kind: WindowEventRecordKind::FocusChanged { window, focused },
    });
}

/// Publish one occlusion-changed window event.
fn publish_window_occlusion_event(window: resource::WindowHandle, occluded: bool) {
    publish_window_event(WindowEventRecord {
        timestamp_ns: core::now_timestamp_ns(),
        sequence: 0,
        kind: WindowEventRecordKind::OcclusionChanged { window, occluded },
    });
}

/// Publish one theme-changed window event.
fn publish_window_theme_event(window: resource::WindowHandle, theme: WindowTheme) {
    publish_window_event(WindowEventRecord {
        timestamp_ns: core::now_timestamp_ns(),
        sequence: 0,
        kind: WindowEventRecordKind::ThemeChanged { window, theme },
    });
}

/// Publish all state transitions observed between two window snapshots.
pub(super) fn publish_state_deltas(
    window: resource::WindowHandle,
    previous: &Win32WindowBinding,
    next: &Win32WindowBinding,
) {
    if previous.visibility != next.visibility {
        publish_window_visibility_event(window, next.visibility);
    }

    if previous.position != next.position {
        publish_window_position_event(window, next.position);
    }

    if previous.size_logical != next.size_logical || previous.size_physical != next.size_physical {
        publish_window_size_event(window, next.size_logical, next.size_physical);
    }

    if previous.scale_factor_milli != next.scale_factor_milli {
        publish_window_scale_event(window, next.scale_factor_milli);
    }

    if previous.focused != next.focused {
        publish_window_focus_event(window, next.focused);
    }

    if previous.occluded != next.occluded {
        publish_window_occlusion_event(window, next.occluded);
    }

    if previous.theme != next.theme {
        publish_window_theme_event(window, next.theme);
    }
}

/// Open one global monitor-event stream.
pub(crate) unsafe fn monitor_event_open(
    context: &BindingCallContext,
    out: *mut resource::DisplayEventHandle,
    options: DisplayMonitorEventOpenOptions,
) -> RuntimeResult<()> {
    core::ensure_out(out, "out")?;

    let binding = Arc::new(MonitorEventBinding {
        state: Mutex::new(MonitorEventState {
            queue_capacity: core::queue_capacity(options.queue.queue_capacity),
            overflow_policy: options.queue.overflow_policy,
            overflow_error_pending: false,
            next_sequence: 1,
            pending: VecDeque::new(),
        }),
        signal: Condvar::new(),
    });

    {
        let mut state = binding
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        seed_monitor_event_stream(&mut state)?;
    }

    let resource_id = context.runtime().resources.insert(
        ResourceEntry::new(ResourceKind::Display)
            .with_label(display_resource::DISPLAY_EVENT_RESOURCE_LABEL)
            .with_payload(Arc::clone(&binding)),
    );
    monitor_event_registry()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .push(Arc::downgrade(&binding));

    unsafe {
        *out = resource::DisplayEventHandle(resource_id);
    }

    Ok(())
}

/// Close one global monitor-event stream.
pub(crate) unsafe fn monitor_event_close(
    context: &BindingCallContext,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    let binding = display_resource::resolve_monitor_event_binding(
        context,
        handle,
        "destack.display.monitor.eventClose",
    )?;

    let identity = Arc::as_ptr(&binding) as usize;
    monitor_event_registry()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .retain(|value| {
            let Some(active) = value.upgrade() else {
                return false;
            };
            Arc::as_ptr(&active) as usize != identity
        });

    let removed = context.runtime().resources.remove(handle.0).is_some();
    if !removed {
        return Err(core::not_found(
            "destack.display.monitor.eventClose",
            format!("display event handle {} was not found", handle.0.0),
        ));
    }

    Ok(())
}

/// Wait for one monitor event.
pub(crate) unsafe fn monitor_event_read(
    context: &BindingCallContext,
    out: *mut DisplayEvent,
    handle: resource::DisplayEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core::ensure_out(out, "out")?;

    let binding = display_resource::resolve_monitor_event_binding(
        context,
        handle,
        "destack.display.monitor.eventRead",
    )?;

    let deadline = core::now_timestamp_ns().saturating_add(timeoutns);
    let mut state = binding
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    loop {
        if state.overflow_error_pending {
            state.overflow_error_pending = false;
            return Err(core::io_busy(
                "destack.display.monitor.eventRead",
                "event queue overflowed",
            ));
        }

        if let Some(record) = state.pending.pop_front() {
            unsafe {
                *out = display_event_from_record(context, record);
            }
            return Ok(());
        }

        let now = core::now_timestamp_ns();
        if now >= deadline {
            return Err(core::io_would_block(
                "destack.display.monitor.eventRead",
                "event read timed out",
            ));
        }

        let remaining = deadline.saturating_sub(now);
        let wait_duration = wait_duration(remaining);
        let (next_state, _) = binding
            .signal
            .wait_timeout(state, wait_duration)
            .unwrap_or_else(|error| error.into_inner());
        state = next_state;
    }
}

/// Wait for one batch of monitor events.
pub(crate) unsafe fn monitor_event_read_batch(
    context: &BindingCallContext,
    out: *mut NativeArray<DisplayEvent>,
    handle: resource::DisplayEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core::ensure_out(out, "out")?;
    let maxevents = core::validate_max_events(maxevents, "maxevents")?;

    let binding = display_resource::resolve_monitor_event_binding(
        context,
        handle,
        "destack.display.monitor.eventReadBatch",
    )?;

    let deadline = core::now_timestamp_ns().saturating_add(timeoutns);
    let mut state = binding
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    loop {
        if state.overflow_error_pending {
            state.overflow_error_pending = false;
            return Err(core::io_busy(
                "destack.display.monitor.eventReadBatch",
                "event queue overflowed",
            ));
        }

        if !state.pending.is_empty() {
            let take = maxevents.min(state.pending.len());
            let mut events = Vec::with_capacity(take);
            for _ in 0..take {
                if let Some(record) = state.pending.pop_front() {
                    events.push(display_event_from_record(context, record));
                }
            }

            unsafe {
                *out = context.store_array(events);
            }
            return Ok(());
        }

        let now = core::now_timestamp_ns();
        if now >= deadline {
            return Err(core::io_would_block(
                "destack.display.monitor.eventReadBatch",
                "event read timed out",
            ));
        }

        let remaining = deadline.saturating_sub(now);
        let wait_duration = wait_duration(remaining);
        let (next_state, _) = binding
            .signal
            .wait_timeout(state, wait_duration)
            .unwrap_or_else(|error| error.into_inner());
        state = next_state;
    }
}

/// Poll one monitor event without blocking.
pub(crate) unsafe fn monitor_event_try_read(
    context: &BindingCallContext,
    out: *mut DisplayEvent,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    core::ensure_out(out, "out")?;

    let binding = display_resource::resolve_monitor_event_binding(
        context,
        handle,
        "destack.display.monitor.eventTryRead",
    )?;
    let mut state = binding
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if state.overflow_error_pending {
        state.overflow_error_pending = false;
        return Err(core::io_busy(
            "destack.display.monitor.eventTryRead",
            "event queue overflowed",
        ));
    }

    let Some(record) = state.pending.pop_front() else {
        return Err(core::io_would_block(
            "destack.display.monitor.eventTryRead",
            "no monitor event is currently queued",
        ));
    };

    unsafe {
        *out = display_event_from_record(context, record);
    }

    Ok(())
}

/// Poll one batch of monitor events without blocking.
pub(crate) unsafe fn monitor_event_try_read_batch(
    context: &BindingCallContext,
    out: *mut NativeArray<DisplayEvent>,
    handle: resource::DisplayEventHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    core::ensure_out(out, "out")?;
    let maxevents = core::validate_max_events(maxevents, "maxevents")?;

    let binding = display_resource::resolve_monitor_event_binding(
        context,
        handle,
        "destack.display.monitor.eventTryReadBatch",
    )?;
    let mut state = binding
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if state.overflow_error_pending {
        state.overflow_error_pending = false;
        return Err(core::io_busy(
            "destack.display.monitor.eventTryReadBatch",
            "event queue overflowed",
        ));
    }

    if state.pending.is_empty() {
        return Err(core::io_would_block(
            "destack.display.monitor.eventTryReadBatch",
            "no monitor event is currently queued",
        ));
    }

    let take = maxevents.min(state.pending.len());
    let mut events = Vec::with_capacity(take);
    for _ in 0..take {
        if let Some(record) = state.pending.pop_front() {
            events.push(display_event_from_record(context, record));
        }
    }

    unsafe {
        *out = context.store_array(events);
    }

    Ok(())
}

/// Open one global window-event stream.
pub(crate) unsafe fn window_event_open(
    context: &BindingCallContext,
    out: *mut resource::WindowEventHandle,
    options: WindowEventOpenOptions,
) -> RuntimeResult<()> {
    core::ensure_out(out, "out")?;

    let binding = Arc::new(WindowEventBinding {
        state: Mutex::new(WindowEventState {
            queue_capacity: core::queue_capacity(options.queue.queue_capacity),
            overflow_policy: options.queue.overflow_policy,
            overflow_error_pending: false,
            next_sequence: 1,
            pending: VecDeque::new(),
        }),
        signal: Condvar::new(),
        owner_thread_id: current_thread_id(),
    });

    let resource_id = context.runtime().resources.insert(
        ResourceEntry::new(ResourceKind::Window)
            .with_label(display_resource::WINDOW_EVENT_RESOURCE_LABEL)
            .with_payload(Arc::clone(&binding)),
    );
    window_event_registry()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .push(Arc::downgrade(&binding));

    unsafe {
        *out = resource::WindowEventHandle(resource_id);
    }

    Ok(())
}

/// Close one global window-event stream.
pub(crate) unsafe fn window_event_close(
    context: &BindingCallContext,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    let binding = display_resource::resolve_window_event_binding(
        context,
        handle,
        "destack.display.window.eventClose",
    )?;

    let identity = Arc::as_ptr(&binding) as usize;
    window_event_registry()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .retain(|value| {
            let Some(active) = value.upgrade() else {
                return false;
            };
            Arc::as_ptr(&active) as usize != identity
        });

    let removed = context.runtime().resources.remove(handle.0).is_some();
    if !removed {
        return Err(core::not_found(
            "destack.display.window.eventClose",
            format!("window event handle {} was not found", handle.0.0),
        ));
    }

    Ok(())
}

/// Wait for one window event.
pub(crate) unsafe fn window_event_read(
    context: &BindingCallContext,
    out: *mut WindowEvent,
    handle: resource::WindowEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core::ensure_out(out, "out")?;

    let binding = display_resource::resolve_window_event_binding(
        context,
        handle,
        "destack.display.window.eventRead",
    )?;
    ensure_window_event_thread(&binding, "destack.display.window.eventRead")?;

    let deadline = core::now_timestamp_ns().saturating_add(timeoutns);
    loop {
        window::pump_window_messages();

        let mut state = binding
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if state.overflow_error_pending {
            state.overflow_error_pending = false;
            return Err(core::io_busy(
                "destack.display.window.eventRead",
                "event queue overflowed",
            ));
        }

        if let Some(record) = state.pending.pop_front() {
            unsafe {
                *out = window_event_from_record(record, context);
            }
            return Ok(());
        }

        let now = core::now_timestamp_ns();
        if now >= deadline {
            return Err(core::io_would_block(
                "destack.display.window.eventRead",
                "event read timed out",
            ));
        }

        let remaining_ns = deadline.saturating_sub(now).min(WINDOW_EVENT_WAIT_SLICE_NS);
        let wait_duration = wait_duration(remaining_ns);
        let _ = binding
            .signal
            .wait_timeout(state, wait_duration)
            .unwrap_or_else(|error| error.into_inner());
    }
}

/// Wait for one batch of window events.
pub(crate) unsafe fn window_event_read_batch(
    context: &BindingCallContext,
    out: *mut NativeArray<WindowEvent>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core::ensure_out(out, "out")?;
    let maxevents = core::validate_max_events(maxevents, "maxevents")?;

    let binding = display_resource::resolve_window_event_binding(
        context,
        handle,
        "destack.display.window.eventReadBatch",
    )?;
    ensure_window_event_thread(&binding, "destack.display.window.eventReadBatch")?;

    let deadline = core::now_timestamp_ns().saturating_add(timeoutns);
    loop {
        window::pump_window_messages();

        let mut state = binding
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if state.overflow_error_pending {
            state.overflow_error_pending = false;
            return Err(core::io_busy(
                "destack.display.window.eventReadBatch",
                "event queue overflowed",
            ));
        }

        if !state.pending.is_empty() {
            let take = maxevents.min(state.pending.len());
            let mut events = Vec::with_capacity(take);
            for _ in 0..take {
                if let Some(record) = state.pending.pop_front() {
                    events.push(window_event_from_record(record, context));
                }
            }

            unsafe {
                *out = context.store_array(events);
            }
            return Ok(());
        }

        let now = core::now_timestamp_ns();
        if now >= deadline {
            return Err(core::io_would_block(
                "destack.display.window.eventReadBatch",
                "event read timed out",
            ));
        }

        let remaining_ns = deadline.saturating_sub(now).min(WINDOW_EVENT_WAIT_SLICE_NS);
        let wait_duration = wait_duration(remaining_ns);
        let _ = binding
            .signal
            .wait_timeout(state, wait_duration)
            .unwrap_or_else(|error| error.into_inner());
    }
}

/// Poll one window event without blocking.
pub(crate) unsafe fn window_event_try_read(
    context: &BindingCallContext,
    out: *mut WindowEvent,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    core::ensure_out(out, "out")?;
    window::pump_window_messages();

    let binding = display_resource::resolve_window_event_binding(
        context,
        handle,
        "destack.display.window.eventTryRead",
    )?;
    ensure_window_event_thread(&binding, "destack.display.window.eventTryRead")?;
    let mut state = binding
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if state.overflow_error_pending {
        state.overflow_error_pending = false;
        return Err(core::io_busy(
            "destack.display.window.eventTryRead",
            "event queue overflowed",
        ));
    }

    let Some(record) = state.pending.pop_front() else {
        return Err(core::io_would_block(
            "destack.display.window.eventTryRead",
            "no window event is currently queued",
        ));
    };

    unsafe {
        *out = window_event_from_record(record, context);
    }

    Ok(())
}

/// Poll one batch of window events without blocking.
pub(crate) unsafe fn window_event_try_read_batch(
    context: &BindingCallContext,
    out: *mut NativeArray<WindowEvent>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    core::ensure_out(out, "out")?;
    let maxevents = core::validate_max_events(maxevents, "maxevents")?;
    window::pump_window_messages();

    let binding = display_resource::resolve_window_event_binding(
        context,
        handle,
        "destack.display.window.eventTryReadBatch",
    )?;
    ensure_window_event_thread(&binding, "destack.display.window.eventTryReadBatch")?;
    let mut state = binding
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if state.overflow_error_pending {
        state.overflow_error_pending = false;
        return Err(core::io_busy(
            "destack.display.window.eventTryReadBatch",
            "event queue overflowed",
        ));
    }

    if state.pending.is_empty() {
        return Err(core::io_would_block(
            "destack.display.window.eventTryReadBatch",
            "no window event is currently queued",
        ));
    }

    let take = maxevents.min(state.pending.len());
    let mut events = Vec::with_capacity(take);
    for _ in 0..take {
        if let Some(record) = state.pending.pop_front() {
            events.push(window_event_from_record(record, context));
        }
    }

    unsafe {
        *out = context.store_array(events);
    }

    Ok(())
}
