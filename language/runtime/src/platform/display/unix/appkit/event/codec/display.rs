use std::collections::HashMap;

use crate::platform::core as core_platform;
use crate::platform::display::host::unix::appkit::model::{
    DisplayDescriptorSnapshot, MonitorSnapshot,
};
use crate::platform::display::host::unix::appkit::{constants, core as appkit_core, monitor};
use crate::platform::display::{
    DisplayAddedEvent, DisplayAddedPayload, DisplayDescriptorChangedEvent,
    DisplayDescriptorChangedPayload, DisplayMetricChangedMask, DisplayModeChangedEvent,
    DisplayModeChangedPayload, DisplayMonitorEvent, DisplayMonitorEventMetadata,
    DisplayPrimaryChangedEvent, DisplayPrimaryPayload, DisplayRemovedEvent, DisplayRemovedPayload,
};
use crate::runtime::BindingCallContext;

use super::super::{DisplayEventRecord, DisplayEventRecordKind};

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
    // track name changes
    if name_changed {
        changed_mask |= constants::DISPLAY_CHANGED_MASK_NAME;
    }

    // track primary-display changes
    if previous.primary != next.primary {
        changed_mask |= constants::DISPLAY_CHANGED_MASK_PRIMARY;
    }

    let bounds_changed = previous.x != next.x
        || previous.y != next.y
        || previous.width_px != next.width_px
        || previous.height_px != next.height_px;
    // track bounds changes
    if bounds_changed {
        changed_mask |= constants::DISPLAY_CHANGED_MASK_BOUNDS;
    }

    let work_area_changed = previous.work_area_x != next.work_area_x
        || previous.work_area_y != next.work_area_y
        || previous.work_area_width_px != next.work_area_width_px
        || previous.work_area_height_px != next.work_area_height_px;
    // track work-area changes
    if work_area_changed {
        changed_mask |= constants::DISPLAY_CHANGED_MASK_WORKAREA;
    }

    // track scale-factor changes
    if previous.scale_factor_milli != next.scale_factor_milli {
        changed_mask |= constants::DISPLAY_CHANGED_MASK_SCALE;
    }

    // track orientation changes
    if previous.orientation != next.orientation {
        changed_mask |= constants::DISPLAY_CHANGED_MASK_ORIENTATION;
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

    // emit removals for displays missing from the new topology
    for snapshot in previous {
        // skip displays that still exist
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

    // compare each next snapshot against the previous topology
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

        // emit mode changes
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

        // emit descriptor changes
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

    // emit primary-display changes
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
        backend: appkit_core::selected_backend(),
        display_id: display_id.map(|value| context.store_string(value)),
        timestamp_ns,
        sequence,
        dropped_count,
    }
}

/// Convert one stored monitor-event record into one ABI event payload.
pub(crate) fn display_event_from_record(
    context: &BindingCallContext,
    value: DisplayEventRecord,
) -> DisplayMonitorEvent {
    // decode this monitor-event variant
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
