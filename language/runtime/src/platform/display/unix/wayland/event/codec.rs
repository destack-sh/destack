use std::collections::HashMap;

use crate::platform::abi::NativeAbi;
use crate::platform::display::{
    DisplayAddedEvent, DisplayAddedPayload, DisplayDescriptorChangedEvent,
    DisplayDescriptorChangedPayload, DisplayMetricChangedMask, DisplayModeChangedEvent,
    DisplayModeChangedPayload, DisplayMonitorEvent, DisplayMonitorEventMetadata,
    DisplayPrimaryChangedEvent, DisplayPrimaryPayload, DisplayRemovedEvent, DisplayRemovedPayload,
    WindowCloseRequestedEvent, WindowCreatedEvent, WindowDestroyedEvent, WindowDropCancelledEvent,
    WindowDropCompletedEvent, WindowDropFilePayload, WindowDropHoverLeavePayload,
    WindowDropHoverPayload, WindowDropStartedEvent, WindowDropTextPayload, WindowEvent,
    WindowEventMetadata, WindowFileDroppedEvent, WindowFileHoverLeftEvent, WindowFileHoveredEvent,
    WindowFocusChangedEvent, WindowFocusPayload, WindowModeChangedEvent, WindowModePayload,
    WindowOcclusionChangedEvent, WindowOcclusionPayload, WindowRefreshRequestedEvent,
    WindowScaleFactorChangedEvent, WindowScaleFactorPayload, WindowSizeChangedEvent,
    WindowSizePayload, WindowTextDroppedEvent, WindowVisibilityChangedEvent,
    WindowVisibilityPayload,
};
use crate::platform::fs::{self as platform_fs, PathBytesAbi, core as core_fs};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::super::super::{core, monitor};
use super::super::model::{DisplayDescriptorSnapshot, MonitorSnapshot};
use super::core::{
    DisplayEventRecord, DisplayEventRecordKind, WindowEventRecord, WindowEventRecordKind,
};

/// Return the primary display identifier from one monitor snapshot list.
fn primary_display_id(snapshots: &[MonitorSnapshot]) -> Option<String> {
    snapshots
        .iter()
        .find(|snapshot| snapshot.descriptor.primary)
        .map(|snapshot| snapshot.descriptor.id.clone())
}

/// Resolve descriptor changed-mask flags for one descriptor transition.
pub(crate) fn descriptor_changed_mask(
    previous: &DisplayDescriptorSnapshot,
    next: &DisplayDescriptorSnapshot,
) -> u32 {
    let mut changed_mask = 0u32;

    let name_changed = previous.name != next.name;
    // evaluate this condition
    if name_changed {
        changed_mask |= core::DISPLAY_CHANGED_MASK_NAME;
    }

    // evaluate this condition
    if previous.primary != next.primary {
        changed_mask |= core::DISPLAY_CHANGED_MASK_PRIMARY;
    }

    let bounds_changed = previous.x != next.x
        || previous.y != next.y
        || previous.width_px != next.width_px
        || previous.height_px != next.height_px;
    // evaluate this condition
    if bounds_changed {
        changed_mask |= core::DISPLAY_CHANGED_MASK_BOUNDS;
    }

    let work_area_changed = previous.work_area_x != next.work_area_x
        || previous.work_area_y != next.work_area_y
        || previous.work_area_width_px != next.work_area_width_px
        || previous.work_area_height_px != next.work_area_height_px;
    // evaluate this condition
    if work_area_changed {
        changed_mask |= core::DISPLAY_CHANGED_MASK_WORKAREA;
    }

    // evaluate this condition
    if previous.scale_factor_milli != next.scale_factor_milli {
        changed_mask |= core::DISPLAY_CHANGED_MASK_SCALE;
    }

    // evaluate this condition
    if previous.orientation != next.orientation {
        changed_mask |= core::DISPLAY_CHANGED_MASK_ORIENTATION;
    }

    changed_mask
}

/// Build one monitor-topology event delta list between two snapshots.
pub(crate) fn monitor_topology_records(
    previous: &[MonitorSnapshot],
    next: &[MonitorSnapshot],
) -> Vec<DisplayEventRecord> {
    let mut records = Vec::new();
    let timestamp_ns = core_platform::monotonic_now_ns();

    let previous_by_id = previous
        .iter()
        .map(|snapshot| (snapshot.descriptor.id.as_str(), snapshot))
        .collect::<HashMap<_, _>>();
    let next_by_id = next
        .iter()
        .map(|snapshot| (snapshot.descriptor.id.as_str(), snapshot))
        .collect::<HashMap<_, _>>();

    // iterate this sequence
    for snapshot in previous {
        // evaluate this condition
        if next_by_id.contains_key(snapshot.descriptor.id.as_str()) {
            continue;
        }

        records.push(DisplayEventRecord {
            timestamp_ns,
            sequence: 0,
            dropped_count: 0,
            kind: DisplayEventRecordKind::Removed {
                id: snapshot.descriptor.id.clone(),
                descriptor: Some(snapshot.descriptor.clone()),
            },
        });
    }

    // iterate this sequence
    for snapshot in next {
        let id = snapshot.descriptor.id.as_str();
        let Some(previous_snapshot) = previous_by_id.get(id) else {
            records.push(DisplayEventRecord {
                timestamp_ns,
                sequence: 0,
                dropped_count: 0,
                kind: DisplayEventRecordKind::Added {
                    descriptor: snapshot.descriptor.clone(),
                },
            });
            records.push(DisplayEventRecord {
                timestamp_ns,
                sequence: 0,
                dropped_count: 0,
                kind: DisplayEventRecordKind::ModeChanged {
                    id: snapshot.descriptor.id.clone(),
                    previous: None,
                    current: snapshot.current_mode,
                },
            });
            continue;
        };

        // evaluate this condition
        if previous_snapshot.current_mode != snapshot.current_mode {
            records.push(DisplayEventRecord {
                timestamp_ns,
                sequence: 0,
                dropped_count: 0,
                kind: DisplayEventRecordKind::ModeChanged {
                    id: snapshot.descriptor.id.clone(),
                    previous: Some(previous_snapshot.current_mode),
                    current: snapshot.current_mode,
                },
            });
        }

        // evaluate this condition
        if previous_snapshot.descriptor != snapshot.descriptor {
            let changed_mask =
                descriptor_changed_mask(&previous_snapshot.descriptor, &snapshot.descriptor);
            records.push(DisplayEventRecord {
                timestamp_ns,
                sequence: 0,
                dropped_count: 0,
                kind: DisplayEventRecordKind::DescriptorChanged {
                    previous: Some(previous_snapshot.descriptor.clone()),
                    current: snapshot.descriptor.clone(),
                    changed_mask,
                },
            });
        }
    }

    // evaluate this condition
    if primary_display_id(previous) != primary_display_id(next) {
        let previous_id = primary_display_id(previous);
        let current_id = primary_display_id(next);
        records.push(DisplayEventRecord {
            timestamp_ns,
            sequence: 0,
            dropped_count: 0,
            kind: DisplayEventRecordKind::PrimaryChanged {
                previous_id,
                current_id,
            },
        });
    }

    records
}

/// Build one monitor-event metadata payload.
fn display_event_metadata(
    context: &BindingCallContext,
    display_id: Option<&str>,
    timestamp_ns: u64,
    sequence: u64,
    dropped_count: u64,
) -> DisplayMonitorEventMetadata {
    DisplayMonitorEventMetadata {
        backend: core::selected_backend(),
        display_id: display_id.map(|value| context.store_string(value)),
        timestamp_ns,
        sequence,
        dropped_count,
    }
}

/// Build one window-event metadata payload.
fn window_event_metadata(
    window: resource::WindowHandle,
    timestamp_ns: u64,
    sequence: u64,
    dropped_count: u64,
) -> WindowEventMetadata {
    WindowEventMetadata {
        backend: core::selected_backend(),
        window,
        timestamp_ns,
        sequence,
        dropped_count,
    }
}

/// Build one `OsPath` payload from one UTF-8 string path.
fn os_path_from_utf8(context: &BindingCallContext, value: &str) -> platform_fs::OsPath {
    let bytes = PathBytesAbi::<NativeAbi>(context.store_array(value.as_bytes().to_vec()));
    core_fs::path_ref_from_bytes(bytes)
}

/// Convert one stored monitor-event record into one ABI event payload.
pub(crate) fn display_event_from_record(
    context: &BindingCallContext,
    value: DisplayEventRecord,
) -> DisplayMonitorEvent {
    // resolve this variant
    match value.kind {
        DisplayEventRecordKind::Added { descriptor } => {
            DisplayMonitorEvent::DisplayAddedEvent(DisplayAddedEvent {
                kind: context.store_string("added"),
                metadata: display_event_metadata(
                    context,
                    Some(&descriptor.id),
                    value.timestamp_ns,
                    value.sequence,
                    value.dropped_count,
                ),
                payload: DisplayAddedPayload {
                    descriptor: monitor::descriptor_from_owned(context, &descriptor),
                },
            })
        }
        DisplayEventRecordKind::Removed { id, descriptor } => {
            DisplayMonitorEvent::DisplayRemovedEvent(DisplayRemovedEvent {
                kind: context.store_string("removed"),
                metadata: display_event_metadata(
                    context,
                    Some(&id),
                    value.timestamp_ns,
                    value.sequence,
                    value.dropped_count,
                ),
                payload: DisplayRemovedPayload {
                    id: context.store_string(&id),
                    descriptor: descriptor
                        .as_ref()
                        .map(|value| monitor::descriptor_from_owned(context, value)),
                },
            })
        }
        DisplayEventRecordKind::PrimaryChanged {
            previous_id,
            current_id,
        } => DisplayMonitorEvent::DisplayPrimaryChangedEvent(DisplayPrimaryChangedEvent {
            kind: context.store_string("primaryChanged"),
            metadata: display_event_metadata(
                context,
                current_id.as_deref(),
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: DisplayPrimaryPayload {
                previous_id: previous_id.map(|value| context.store_string(&value)),
                current_id: current_id.map(|value| context.store_string(&value)),
            },
        }),
        DisplayEventRecordKind::DescriptorChanged {
            previous,
            current,
            changed_mask,
        } => DisplayMonitorEvent::DisplayDescriptorChangedEvent(DisplayDescriptorChangedEvent {
            kind: context.store_string("descriptorChanged"),
            metadata: display_event_metadata(
                context,
                Some(&current.id),
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: DisplayDescriptorChangedPayload {
                previous: previous
                    .as_ref()
                    .map(|value| monitor::descriptor_from_owned(context, value)),
                current: monitor::descriptor_from_owned(context, &current),
                changed_mask: DisplayMetricChangedMask(changed_mask),
            },
        }),
        DisplayEventRecordKind::ModeChanged {
            id,
            previous,
            current,
        } => DisplayMonitorEvent::DisplayModeChangedEvent(DisplayModeChangedEvent {
            kind: context.store_string("modeChanged"),
            metadata: display_event_metadata(
                context,
                Some(&id),
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: DisplayModeChangedPayload { previous, current },
        }),
    }
}

/// Convert one stored window-event record into one ABI event payload.
pub(crate) fn window_event_from_record(
    context: &BindingCallContext,
    value: WindowEventRecord,
) -> WindowEvent {
    // resolve this variant
    match value.kind {
        WindowEventRecordKind::Created { window } => {
            WindowEvent::WindowCreatedEvent(WindowCreatedEvent {
                kind: context.store_string("created"),
                metadata: window_event_metadata(
                    window,
                    value.timestamp_ns,
                    value.sequence,
                    value.dropped_count,
                ),
            })
        }
        WindowEventRecordKind::CloseRequested { window } => {
            WindowEvent::WindowCloseRequestedEvent(WindowCloseRequestedEvent {
                kind: context.store_string("closeRequested"),
                metadata: window_event_metadata(
                    window,
                    value.timestamp_ns,
                    value.sequence,
                    value.dropped_count,
                ),
            })
        }
        WindowEventRecordKind::Destroyed { window } => {
            WindowEvent::WindowDestroyedEvent(WindowDestroyedEvent {
                kind: context.store_string("destroyed"),
                metadata: window_event_metadata(
                    window,
                    value.timestamp_ns,
                    value.sequence,
                    value.dropped_count,
                ),
            })
        }
        WindowEventRecordKind::RefreshRequested { window } => {
            WindowEvent::WindowRefreshRequestedEvent(WindowRefreshRequestedEvent {
                kind: context.store_string("refreshRequested"),
                metadata: window_event_metadata(
                    window,
                    value.timestamp_ns,
                    value.sequence,
                    value.dropped_count,
                ),
            })
        }
        WindowEventRecordKind::VisibilityChanged {
            window,
            previous_visibility,
            current_visibility,
        } => WindowEvent::WindowVisibilityChangedEvent(WindowVisibilityChangedEvent {
            kind: context.store_string("visibilityChanged"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowVisibilityPayload {
                previous_visibility,
                current_visibility,
            },
        }),
        WindowEventRecordKind::OcclusionChanged {
            window,
            previous_occlusion,
            current_occlusion,
        } => WindowEvent::WindowOcclusionChangedEvent(WindowOcclusionChangedEvent {
            kind: context.store_string("occlusionChanged"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowOcclusionPayload {
                previous_occlusion,
                current_occlusion,
            },
        }),
        WindowEventRecordKind::SizeChanged {
            window,
            previous_size_logical,
            previous_size_physical,
            current_size_logical,
            current_size_physical,
        } => WindowEvent::WindowSizeChangedEvent(WindowSizeChangedEvent {
            kind: context.store_string("sizeChanged"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowSizePayload {
                previous_size_logical,
                previous_size_physical,
                current_size_logical,
                current_size_physical,
            },
        }),
        WindowEventRecordKind::ScaleFactorChanged {
            window,
            previous_scale_factor_milli,
            current_scale_factor_milli,
        } => WindowEvent::WindowScaleFactorChangedEvent(WindowScaleFactorChangedEvent {
            kind: context.store_string("scaleFactorChanged"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowScaleFactorPayload {
                previous_scale_factor_milli,
                current_scale_factor_milli,
            },
        }),
        WindowEventRecordKind::FocusChanged {
            window,
            previous_focused,
            current_focused,
        } => WindowEvent::WindowFocusChangedEvent(WindowFocusChangedEvent {
            kind: context.store_string("focusChanged"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowFocusPayload {
                previous_focused,
                current_focused,
            },
        }),
        WindowEventRecordKind::ModeChanged {
            window,
            previous_mode,
            current_mode,
        } => WindowEvent::WindowModeChangedEvent(WindowModeChangedEvent {
            kind: context.store_string("modeChanged"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowModePayload {
                previous_mode,
                current_mode,
            },
        }),
        WindowEventRecordKind::DropStarted { window } => {
            WindowEvent::WindowDropStartedEvent(WindowDropStartedEvent {
                kind: context.store_string("dropStarted"),
                metadata: window_event_metadata(
                    window,
                    value.timestamp_ns,
                    value.sequence,
                    value.dropped_count,
                ),
            })
        }
        WindowEventRecordKind::FileHovered {
            window,
            path,
            position,
        } => WindowEvent::WindowFileHoveredEvent(WindowFileHoveredEvent {
            kind: context.store_string("fileHovered"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowDropHoverPayload {
                path: path.as_ref().map(|value| os_path_from_utf8(context, value)),
                position,
            },
        }),
        WindowEventRecordKind::DropCancelled { window } => {
            WindowEvent::WindowDropCancelledEvent(WindowDropCancelledEvent {
                kind: context.store_string("dropCancelled"),
                metadata: window_event_metadata(
                    window,
                    value.timestamp_ns,
                    value.sequence,
                    value.dropped_count,
                ),
            })
        }
        WindowEventRecordKind::DropCompleted { window } => {
            WindowEvent::WindowDropCompletedEvent(WindowDropCompletedEvent {
                kind: context.store_string("dropCompleted"),
                metadata: window_event_metadata(
                    window,
                    value.timestamp_ns,
                    value.sequence,
                    value.dropped_count,
                ),
            })
        }
        WindowEventRecordKind::FileHoverLeft {
            window,
            previous_path,
            position,
        } => WindowEvent::WindowFileHoverLeftEvent(WindowFileHoverLeftEvent {
            kind: context.store_string("fileHoverLeft"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowDropHoverLeavePayload {
                previous_path: previous_path
                    .as_ref()
                    .map(|value| os_path_from_utf8(context, value)),
                position,
            },
        }),
        WindowEventRecordKind::FileDropped {
            window,
            path,
            position,
        } => WindowEvent::WindowFileDroppedEvent(WindowFileDroppedEvent {
            kind: context.store_string("fileDropped"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowDropFilePayload {
                path: path.as_ref().map(|value| os_path_from_utf8(context, value)),
                position,
            },
        }),
        WindowEventRecordKind::TextDropped {
            window,
            text,
            position,
        } => WindowEvent::WindowTextDroppedEvent(WindowTextDroppedEvent {
            kind: context.store_string("textDropped"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowDropTextPayload {
                text: context.store_string(&text),
                position,
            },
        }),
    }
}
