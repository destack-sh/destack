use super::*;

/// Publish one mode-changed monitor event.
pub(in super::super::super) fn publish_mode_changed_event(
    binding: &BindingCallContext,
    display_id: &str,
    mode: DisplayMode,
) {
    let record = display_event_record(DisplayEventRecordKind::ModeChanged {
        id: display_id.to_string(),
        previous: None,
        current: mode,
    });

    let runtime_state = display_event_runtime_state(binding);
    publish_monitor_event(&runtime_state, record);
}

/// Publish one descriptor-changed monitor event.
pub(in super::super::super) fn publish_descriptor_changed_event(
    binding: &BindingCallContext,
    descriptor: &DisplayDescriptorSnapshot,
    changed_mask: u32,
) {
    let record = display_event_record(DisplayEventRecordKind::DescriptorChanged {
        previous: None,
        current: descriptor.clone(),
        changed_mask,
    });

    let runtime_state = display_event_runtime_state(binding);
    publish_monitor_event(&runtime_state, record);
}

/// Refresh the cached monitor topology snapshot from the current host state.
pub(in super::super::super) fn refresh_monitor_topology_cache(
    binding: &BindingCallContext,
) -> RuntimeResult<()> {
    // refresh monitor snapshots from host state
    let snapshots = monitor::enumerate_monitor_snapshots()?;

    // replace cached topology snapshot atomically
    let runtime_state = display_event_runtime_state(binding);
    let mut topology_snapshot = runtime_state
        .monitor_topology_snapshot
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    *topology_snapshot = Some(snapshots);

    Ok(())
}

/// Publish monitor topology events observed since the last cached snapshot.
pub(in super::super::super) fn publish_monitor_topology_deltas(
    runtime_state: &Arc<DisplayEventRuntimeState>,
) -> RuntimeResult<()> {
    let next_snapshots = monitor::enumerate_monitor_snapshots()?;

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

    // iterate this sequence
    for record in records {
        publish_monitor_event(runtime_state, record);
    }

    Ok(())
}

/// Seed one monitor-event stream with current monitor snapshot events.
pub(in super::super) fn seed_monitor_event_stream(
    state: &mut MonitorEventState,
    filter: &MonitorEventFilterState,
) -> RuntimeResult<Vec<MonitorSnapshot>> {
    let snapshots = monitor::enumerate_monitor_snapshots()?;
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
        // evaluate this condition
        if filter.matches(&added) {
            push_monitor_event(state, added);
        }

        let mode_changed = display_event_record(DisplayEventRecordKind::ModeChanged {
            id: snapshot.descriptor.id.clone(),
            previous: None,
            current: snapshot.current_mode,
        });
        // evaluate this condition
        if filter.matches(&mode_changed) {
            push_monitor_event(state, mode_changed);
        }
    }

    let primary_changed = display_event_record(DisplayEventRecordKind::PrimaryChanged {
        previous_id: None,
        current_id: primary_id,
    });
    // evaluate this condition
    if filter.matches(&primary_changed) {
        push_monitor_event(state, primary_changed);
    }

    Ok(snapshots)
}

/// Publish one created window event.
pub(in super::super::super) fn publish_window_created_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::Created { window }),
    );
}

/// Publish one destroyed window event.
pub(in super::super::super) fn publish_window_destroyed_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::Destroyed { window }),
    );
}

/// Publish one close-requested window event.
pub(in super::super::super) fn publish_window_close_requested_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::CloseRequested { window }),
    );
}

/// Publish one refresh-requested window event.
pub(in super::super::super) fn publish_window_refresh_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::RefreshRequested { window }),
    );
}

/// Publish one visibility-changed window event.
fn publish_window_visibility_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
    previous_visibility: WindowVisibility,
    current_visibility: WindowVisibility,
) {
    publish_window_event(
        runtime_state,
        WindowEventRecord {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence: 0,
            dropped_count: 0,
            kind: WindowEventRecordKind::VisibilityChanged {
                window,
                previous_visibility,
                current_visibility,
            },
        },
    );
}

/// Publish one occlusion-changed window event.
fn publish_window_occlusion_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
    previous_occlusion: WindowOcclusionState,
    current_occlusion: WindowOcclusionState,
) {
    publish_window_event(
        runtime_state,
        WindowEventRecord {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence: 0,
            dropped_count: 0,
            kind: WindowEventRecordKind::OcclusionChanged {
                window,
                previous_occlusion,
                current_occlusion,
            },
        },
    );
}

/// Publish one position-changed window event.
fn publish_window_position_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
    previous_position: WindowPosition,
    current_position: WindowPosition,
) {
    publish_window_event(
        runtime_state,
        WindowEventRecord {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence: 0,
            dropped_count: 0,
            kind: WindowEventRecordKind::PositionChanged {
                window,
                previous_position,
                current_position,
            },
        },
    );
}

/// Publish one size-changed window event.
fn publish_window_size_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
    previous_size_logical: WindowLogicalSize,
    previous_size_physical: WindowPhysicalSize,
    current_size_logical: WindowLogicalSize,
    current_size_physical: WindowPhysicalSize,
) {
    publish_window_event(
        runtime_state,
        WindowEventRecord {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence: 0,
            dropped_count: 0,
            kind: WindowEventRecordKind::SizeChanged {
                window,
                previous_size_logical,
                previous_size_physical,
                current_size_logical,
                current_size_physical,
            },
        },
    );
}

/// Publish one scale-factor changed window event.
fn publish_window_scale_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
    previous_scale_factor_milli: u32,
    current_scale_factor_milli: u32,
) {
    publish_window_event(
        runtime_state,
        WindowEventRecord {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence: 0,
            dropped_count: 0,
            kind: WindowEventRecordKind::ScaleFactorChanged {
                window,
                previous_scale_factor_milli,
                current_scale_factor_milli,
            },
        },
    );
}

/// Publish one mode-changed window event.
pub(in super::super::super) fn publish_window_mode_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
    previous_mode: WindowModeOptions,
    current_mode: WindowModeOptions,
) {
    publish_window_event(
        runtime_state,
        WindowEventRecord {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence: 0,
            dropped_count: 0,
            kind: WindowEventRecordKind::ModeChanged {
                window,
                previous_mode,
                current_mode,
            },
        },
    );
}

/// Publish one display-changed window event.
pub(in super::super::super) fn publish_window_display_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
    previous_display: Option<resource::DisplayHandle>,
    current_display: Option<resource::DisplayHandle>,
) {
    publish_window_event(
        runtime_state,
        WindowEventRecord {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence: 0,
            dropped_count: 0,
            kind: WindowEventRecordKind::DisplayChanged {
                window,
                previous_display,
                current_display,
            },
        },
    );
}

/// Publish one focus-changed window event.
fn publish_window_focus_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
    previous_focused: bool,
    current_focused: bool,
) {
    publish_window_event(
        runtime_state,
        WindowEventRecord {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence: 0,
            dropped_count: 0,
            kind: WindowEventRecordKind::FocusChanged {
                window,
                previous_focused,
                current_focused,
            },
        },
    );
}

/// Publish one theme-changed window event.
fn publish_window_theme_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
    previous_theme: WindowTheme,
    current_theme: WindowTheme,
) {
    publish_window_event(
        runtime_state,
        WindowEventRecord {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence: 0,
            dropped_count: 0,
            kind: WindowEventRecordKind::ThemeChanged {
                window,
                previous_theme,
                current_theme,
            },
        },
    );
}

/// Publish one chrome-changed window event.
fn publish_window_chrome_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
    previous_chrome: WindowChromeKind,
    current_chrome: WindowChromeKind,
) {
    publish_window_event(
        runtime_state,
        WindowEventRecord {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence: 0,
            dropped_count: 0,
            kind: WindowEventRecordKind::ChromeChanged {
                window,
                previous_chrome,
                current_chrome,
            },
        },
    );
}

/// Publish one taskbar-visibility-changed window event.
fn publish_window_taskbar_visibility_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
    previous_taskbar_visible: bool,
    current_taskbar_visible: bool,
) {
    publish_window_event(
        runtime_state,
        WindowEventRecord {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence: 0,
            dropped_count: 0,
            kind: WindowEventRecordKind::TaskbarVisibilityChanged {
                window,
                previous_taskbar_visible,
                current_taskbar_visible,
            },
        },
    );
}

/// Publish one opacity-changed window event.
fn publish_window_opacity_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
    previous_opacity: f64,
    current_opacity: f64,
) {
    publish_window_event(
        runtime_state,
        WindowEventRecord {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence: 0,
            dropped_count: 0,
            kind: WindowEventRecordKind::OpacityChanged {
                window,
                previous_opacity,
                current_opacity,
            },
        },
    );
}

/// Publish one parent-changed window event.
fn publish_window_parent_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
    previous_parent: Option<resource::WindowHandle>,
    current_parent: Option<resource::WindowHandle>,
) {
    publish_window_event(
        runtime_state,
        WindowEventRecord {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence: 0,
            dropped_count: 0,
            kind: WindowEventRecordKind::ParentChanged {
                window,
                previous_parent,
                current_parent,
            },
        },
    );
}

/// Publish one transient-owner-changed window event.
fn publish_window_transient_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
    previous_transient_for: Option<resource::WindowHandle>,
    current_transient_for: Option<resource::WindowHandle>,
) {
    publish_window_event(
        runtime_state,
        WindowEventRecord {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence: 0,
            dropped_count: 0,
            kind: WindowEventRecordKind::TransientChanged {
                window,
                previous_transient_for,
                current_transient_for,
            },
        },
    );
}

/// Publish one modal-changed window event.
fn publish_window_modal_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
    previous_modal: bool,
    current_modal: bool,
) {
    publish_window_event(
        runtime_state,
        WindowEventRecord {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence: 0,
            dropped_count: 0,
            kind: WindowEventRecordKind::ModalChanged {
                window,
                previous_modal,
                current_modal,
            },
        },
    );
}

/// Publish one mouse-passthrough-changed window event.
fn publish_window_mouse_passthrough_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
    previous_mouse_passthrough: bool,
    current_mouse_passthrough: bool,
) {
    publish_window_event(
        runtime_state,
        WindowEventRecord {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence: 0,
            dropped_count: 0,
            kind: WindowEventRecordKind::MousePassthroughChanged {
                window,
                previous_mouse_passthrough,
                current_mouse_passthrough,
            },
        },
    );
}

/// Publish one aspect-ratio-changed window event.
fn publish_window_aspect_ratio_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
    previous_aspect_ratio: Option<WindowAspectRatio>,
    current_aspect_ratio: Option<WindowAspectRatio>,
) {
    publish_window_event(
        runtime_state,
        WindowEventRecord {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence: 0,
            dropped_count: 0,
            kind: WindowEventRecordKind::AspectRatioChanged {
                window,
                previous_aspect_ratio,
                current_aspect_ratio,
            },
        },
    );
}

/// Publish one drop-started window event.
pub(in super::super::super) fn publish_window_drop_started_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        WindowEventRecord {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence: 0,
            dropped_count: 0,
            kind: WindowEventRecordKind::DropStarted { window },
        },
    );
}

/// Publish one file-hovered window event.
pub(in super::super::super) fn publish_window_file_hovered_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
    path_utf16: Option<Vec<u16>>,
    position: Option<WindowPosition>,
) {
    publish_window_event(
        runtime_state,
        WindowEventRecord {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence: 0,
            dropped_count: 0,
            kind: WindowEventRecordKind::FileHovered {
                window,
                path_utf16,
                position,
            },
        },
    );
}

/// Publish one drop-cancelled window event.
pub(in super::super::super) fn publish_window_drop_cancelled_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        WindowEventRecord {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence: 0,
            dropped_count: 0,
            kind: WindowEventRecordKind::DropCancelled { window },
        },
    );
}

/// Publish one drop-completed window event.
pub(in super::super::super) fn publish_window_drop_completed_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        WindowEventRecord {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence: 0,
            dropped_count: 0,
            kind: WindowEventRecordKind::DropCompleted { window },
        },
    );
}

/// Publish one file-hover-left window event.
pub(in super::super::super) fn publish_window_file_hover_left_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
    previous_path_utf16: Option<Vec<u16>>,
    position: Option<WindowPosition>,
) {
    publish_window_event(
        runtime_state,
        WindowEventRecord {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence: 0,
            dropped_count: 0,
            kind: WindowEventRecordKind::FileHoverLeft {
                window,
                previous_path_utf16,
                position,
            },
        },
    );
}

/// Publish one file-dropped window event.
pub(in super::super::super) fn publish_window_file_dropped_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
    path_utf16: Option<Vec<u16>>,
    position: Option<WindowPosition>,
) {
    publish_window_event(
        runtime_state,
        WindowEventRecord {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence: 0,
            dropped_count: 0,
            kind: WindowEventRecordKind::FileDropped {
                window,
                path_utf16,
                position,
            },
        },
    );
}

/// Publish one text-dropped window event.
pub(in super::super::super) fn publish_window_text_dropped_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
    text: String,
    position: Option<WindowPosition>,
) {
    publish_window_event(
        runtime_state,
        WindowEventRecord {
            timestamp_ns: core_platform::monotonic_now_ns(),
            sequence: 0,
            dropped_count: 0,
            kind: WindowEventRecordKind::TextDropped {
                window,
                text,
                position,
            },
        },
    );
}

/// Publish all state transitions observed between two window snapshots.
pub(in super::super::super) fn publish_state_deltas(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    window: resource::WindowHandle,
    previous: &Win32WindowBinding,
    next: &Win32WindowBinding,
) {
    // evaluate this condition
    if previous.visibility != next.visibility {
        publish_window_visibility_event(
            runtime_state,
            window,
            previous.visibility,
            next.visibility,
        );
    }

    let previous_occlusion = window::occlusion_from_visibility(previous.visibility);
    let current_occlusion = window::occlusion_from_visibility(next.visibility);
    // evaluate this condition
    if previous_occlusion != current_occlusion {
        publish_window_occlusion_event(
            runtime_state,
            window,
            previous_occlusion,
            current_occlusion,
        );
    }

    // evaluate this condition
    if previous.position != next.position {
        publish_window_position_event(runtime_state, window, previous.position, next.position);
    }

    // evaluate this condition
    if previous.size_logical != next.size_logical || previous.size_physical != next.size_physical {
        publish_window_size_event(
            runtime_state,
            window,
            previous.size_logical,
            previous.size_physical,
            next.size_logical,
            next.size_physical,
        );
    }

    // evaluate this condition
    if previous.scale_factor_milli != next.scale_factor_milli {
        publish_window_scale_event(
            runtime_state,
            window,
            previous.scale_factor_milli,
            next.scale_factor_milli,
        );
    }

    // evaluate this condition
    if previous.focused != next.focused {
        publish_window_focus_event(runtime_state, window, previous.focused, next.focused);
    }

    // evaluate this condition
    if previous.theme != next.theme {
        publish_window_theme_event(runtime_state, window, previous.theme, next.theme);
    }

    // evaluate this condition
    if previous.chrome != next.chrome {
        publish_window_chrome_event(runtime_state, window, previous.chrome, next.chrome);
    }

    // evaluate this condition
    if previous.taskbar_visible != next.taskbar_visible {
        publish_window_taskbar_visibility_event(
            runtime_state,
            window,
            previous.taskbar_visible,
            next.taskbar_visible,
        );
    }

    // evaluate this condition
    if previous.opacity != next.opacity {
        publish_window_opacity_event(runtime_state, window, previous.opacity, next.opacity);
    }

    // evaluate this condition
    if previous.parent != next.parent {
        publish_window_parent_event(runtime_state, window, previous.parent, next.parent);
    }

    // evaluate this condition
    if previous.transient_for != next.transient_for {
        publish_window_transient_event(
            runtime_state,
            window,
            previous.transient_for,
            next.transient_for,
        );
    }

    // evaluate this condition
    if previous.modal != next.modal {
        publish_window_modal_event(runtime_state, window, previous.modal, next.modal);
    }

    // evaluate this condition
    if previous.mouse_passthrough != next.mouse_passthrough {
        publish_window_mouse_passthrough_event(
            runtime_state,
            window,
            previous.mouse_passthrough,
            next.mouse_passthrough,
        );
    }

    // evaluate this condition
    if previous.aspect_ratio != next.aspect_ratio {
        publish_window_aspect_ratio_event(
            runtime_state,
            window,
            previous.aspect_ratio,
            next.aspect_ratio,
        );
    }
}
