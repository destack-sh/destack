use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayMode, WindowLogicalSize, WindowModeOptions, WindowOcclusionState, WindowPhysicalSize,
    WindowVisibility,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::super::super::{core, monitor};
use super::codec::{descriptor_changed_mask, monitor_topology_records};
use super::core::{
    DisplayDescriptorSnapshot, DisplayEventRecord, DisplayEventRecordKind, MonitorEventBinding,
    MonitorEventFilterState, MonitorEventState, MonitorSnapshot, WindowEventRecord,
    WindowEventRecordKind,
};
use super::queue::{publish_monitor_event, publish_window_event, push_monitor_record};

/// Resolve one occlusion state from one visibility value.
fn occlusion_from_visibility(visibility: WindowVisibility) -> WindowOcclusionState {
    // minimized windows are treated as occluded
    if visibility == WindowVisibility::Minimized {
        return WindowOcclusionState::Occluded;
    }

    WindowOcclusionState::Unknown
}

/// Publish one monitor mode-changed event for one display.
pub(crate) fn publish_monitor_mode_changed(
    runtime_state: &Arc<core::WaylandRuntimeState>,
    display_id: &str,
    previous: Option<DisplayMode>,
    current: DisplayMode,
) {
    let record = DisplayEventRecord {
        timestamp_ns: core_platform::monotonic_now_ns(),
        sequence: 0,
        dropped_count: 0,
        kind: DisplayEventRecordKind::ModeChanged {
            id: display_id.to_string(),
            previous,
            current,
        },
    };
    publish_monitor_event(runtime_state, record);
}

/// Publish one monitor descriptor-changed event for one display.
pub(crate) fn publish_monitor_descriptor_changed(
    runtime_state: &Arc<core::WaylandRuntimeState>,
    previous: Option<DisplayDescriptorSnapshot>,
    current: DisplayDescriptorSnapshot,
) {
    let changed_mask = previous
        .as_ref()
        .map_or(0, |value| descriptor_changed_mask(value, &current));
    // evaluate this condition
    if changed_mask == 0 {
        return;
    }

    let record = DisplayEventRecord {
        timestamp_ns: core_platform::monotonic_now_ns(),
        sequence: 0,
        dropped_count: 0,
        kind: DisplayEventRecordKind::DescriptorChanged {
            previous,
            current,
            changed_mask,
        },
    };
    publish_monitor_event(runtime_state, record);
}

/// Publish one created window event.
pub(crate) fn publish_window_created(
    runtime_state: &Arc<core::WaylandRuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::Created { window }),
    );
}

/// Publish one close-requested window event.
pub(crate) fn publish_window_close_requested(
    runtime_state: &Arc<core::WaylandRuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::CloseRequested { window }),
    );
}

/// Publish one destroyed window event.
pub(crate) fn publish_window_destroyed(
    runtime_state: &Arc<core::WaylandRuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::Destroyed { window }),
    );
}

/// Publish one refresh-requested window event.
pub(crate) fn publish_window_refresh_requested(
    runtime_state: &Arc<core::WaylandRuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::RefreshRequested { window }),
    );
}

/// Publish one visibility-changed window event.
pub(crate) fn publish_window_visibility_changed(
    runtime_state: &Arc<core::WaylandRuntimeState>,
    window: resource::WindowHandle,
    previous_visibility: WindowVisibility,
    current_visibility: WindowVisibility,
) {
    let previous_occlusion = occlusion_from_visibility(previous_visibility);
    let current_occlusion = occlusion_from_visibility(current_visibility);

    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::VisibilityChanged {
            window,
            previous_visibility,
            current_visibility,
        }),
    );

    // publish one occlusion transition when visibility implies one state change
    if previous_occlusion != current_occlusion {
        publish_window_event(
            runtime_state,
            window_event_record(WindowEventRecordKind::OcclusionChanged {
                window,
                previous_occlusion,
                current_occlusion,
            }),
        );
    }
}

/// Publish one size-changed window event.
pub(crate) fn publish_window_size_changed(
    runtime_state: &Arc<core::WaylandRuntimeState>,
    window: resource::WindowHandle,
    previous_size_logical: WindowLogicalSize,
    previous_size_physical: WindowPhysicalSize,
    current_size_logical: WindowLogicalSize,
    current_size_physical: WindowPhysicalSize,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::SizeChanged {
            window,
            previous_size_logical,
            previous_size_physical,
            current_size_logical,
            current_size_physical,
        }),
    );
}

/// Publish one scale-factor-changed window event.
pub(crate) fn publish_window_scale_factor_changed(
    runtime_state: &Arc<core::WaylandRuntimeState>,
    window: resource::WindowHandle,
    previous_scale_factor_milli: u32,
    current_scale_factor_milli: u32,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::ScaleFactorChanged {
            window,
            previous_scale_factor_milli,
            current_scale_factor_milli,
        }),
    );
}

/// Publish one focus-changed window event.
pub(crate) fn publish_window_focus_changed(
    runtime_state: &Arc<core::WaylandRuntimeState>,
    window: resource::WindowHandle,
    previous_focused: bool,
    current_focused: bool,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::FocusChanged {
            window,
            previous_focused,
            current_focused,
        }),
    );
}

/// Publish one mode-changed window event.
pub(crate) fn publish_window_mode_changed(
    runtime_state: &Arc<core::WaylandRuntimeState>,
    window: resource::WindowHandle,
    previous_mode: WindowModeOptions,
    current_mode: WindowModeOptions,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::ModeChanged {
            window,
            previous_mode,
            current_mode,
        }),
    );
}

/// Publish one drop-started window event.
pub(crate) fn publish_window_drop_started(
    runtime_state: &Arc<core::WaylandRuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::DropStarted { window }),
    );
}

/// Publish one file-hovered window event.
pub(crate) fn publish_window_file_hovered(
    runtime_state: &Arc<core::WaylandRuntimeState>,
    window: resource::WindowHandle,
    path: Option<String>,
    position: Option<WindowPosition>,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::FileHovered {
            window,
            path,
            position,
        }),
    );
}

/// Publish one drop-cancelled window event.
pub(crate) fn publish_window_drop_cancelled(
    runtime_state: &Arc<core::WaylandRuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::DropCancelled { window }),
    );
}

/// Publish one drop-completed window event.
pub(crate) fn publish_window_drop_completed(
    runtime_state: &Arc<core::WaylandRuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::DropCompleted { window }),
    );
}

/// Publish one file-hover-left window event.
pub(crate) fn publish_window_file_hover_left(
    runtime_state: &Arc<core::WaylandRuntimeState>,
    window: resource::WindowHandle,
    previous_path: Option<String>,
    position: Option<WindowPosition>,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::FileHoverLeft {
            window,
            previous_path,
            position,
        }),
    );
}

/// Publish one file-dropped window event.
pub(crate) fn publish_window_file_dropped(
    runtime_state: &Arc<core::WaylandRuntimeState>,
    window: resource::WindowHandle,
    path: Option<String>,
    position: Option<WindowPosition>,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::FileDropped {
            window,
            path,
            position,
        }),
    );
}

/// Publish one text-dropped window event.
pub(crate) fn publish_window_text_dropped(
    runtime_state: &Arc<core::WaylandRuntimeState>,
    window: resource::WindowHandle,
    text: String,
    position: Option<WindowPosition>,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::TextDropped {
            window,
            text,
            position,
        }),
    );
}
/// Refresh one cached monitor topology snapshot from host state.
pub(crate) fn refresh_monitor_topology_cache(context: &BindingCallContext) -> RuntimeResult<()> {
    // enumerate snapshots and replace cached topology atomically
    let snapshots = monitor::enumerate_monitor_snapshots(context)?;
    let runtime_state = core::runtime_state(context);
    let mut topology_snapshot = runtime_state
        .monitor_topology_snapshot
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    *topology_snapshot = Some(snapshots);

    Ok(())
}

/// Publish monitor topology deltas observed since the last cached snapshot.
pub(crate) fn publish_monitor_topology_deltas(context: &BindingCallContext) -> RuntimeResult<()> {
    // enumerate next monitor topology snapshot
    let next_snapshots = monitor::enumerate_monitor_snapshots(context)?;
    let runtime_state = core::runtime_state(context);

    // compute delta records and replace cached snapshot
    let records = {
        let mut topology_snapshot = runtime_state
            .monitor_topology_snapshot
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let Some(previous_snapshots) = topology_snapshot.as_ref() else {
            *topology_snapshot = Some(next_snapshots);
            return Ok(());
        };

        let records = monitor_topology_records(previous_snapshots, &next_snapshots);
        *topology_snapshot = Some(next_snapshots);
        records
    };

    // publish topology records into all monitor streams
    for record in records {
        publish_monitor_event(&runtime_state, record);
    }

    Ok(())
}

/// Seed one monitor-event stream with current monitor snapshot events.
pub(crate) fn seed_monitor_event_stream(
    context: &BindingCallContext,
    binding: &Arc<MonitorEventBinding>,
) -> RuntimeResult<Vec<MonitorSnapshot>> {
    let snapshots = monitor::enumerate_monitor_snapshots(context)?;
    let mut primary_id = None;

    // iterate this sequence
    for snapshot in &snapshots {
        // evaluate this condition
        if snapshot.descriptor.primary {
            primary_id = Some(snapshot.descriptor.id.clone());
        }

        let added = display_event_record(DisplayEventRecordKind::Added {
            descriptor: snapshot.descriptor.clone(),
        });
        push_monitor_record(binding, added);

        let mode_changed = display_event_record(DisplayEventRecordKind::ModeChanged {
            id: snapshot.descriptor.id.clone(),
            previous: None,
            current: snapshot.current_mode,
        });
        push_monitor_record(binding, mode_changed);
    }

    let primary_changed = display_event_record(DisplayEventRecordKind::PrimaryChanged {
        previous_id: None,
        current_id: primary_id,
    });
    push_monitor_record(binding, primary_changed);

    Ok(snapshots)
}
