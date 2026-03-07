use std::collections::HashMap;

use crate::platform::core as core_platform;
use crate::platform::display::windows::win32::event::{DisplayEventRecord, DisplayEventRecordKind};
use crate::platform::display::windows::win32::model::{DisplayDescriptorSnapshot, MonitorSnapshot};
use crate::platform::display::windows::win32::{core as win32_core, monitor};
use crate::platform::display::{
    DisplayAddedEvent, DisplayAddedPayload, DisplayBackend, DisplayDescriptorChangedEvent,
    DisplayDescriptorChangedPayload, DisplayMetricChangedMask, DisplayModeChangedEvent,
    DisplayModeChangedPayload, DisplayMonitorEvent, DisplayMonitorEventMetadata,
    DisplayPrimaryChangedEvent, DisplayPrimaryPayload, DisplayRemovedEvent, DisplayRemovedPayload,
};
use crate::runtime::BindingCallContext;

/// Build one monitor-event metadata payload.
fn display_event_metadata(
    binding: &BindingCallContext,
    display_id: Option<&str>,
    timestamp_ns: u64,
    sequence: u64,
    dropped_count: u64,
) -> DisplayMonitorEventMetadata {
    DisplayMonitorEventMetadata {
        backend: DisplayBackend::Win32,
        display_id: display_id.map(|value| binding.store_string(value)),
        timestamp_ns,
        sequence,
        dropped_count,
    }
}
/// Resolve one primary monitor identifier from one snapshot list.
fn primary_display_id(snapshots: &[MonitorSnapshot]) -> Option<String> {
    snapshots
        .iter()
        .find(|snapshot| snapshot.descriptor.primary)
        .map(|snapshot| snapshot.descriptor.id.clone())
}

/// Resolve descriptor changed-mask flags for one descriptor transition.
fn descriptor_changed_mask(
    previous: &DisplayDescriptorSnapshot,
    next: &DisplayDescriptorSnapshot,
) -> u32 {
    let mut changed_mask = 0u32;

    let bounds_changed = previous.x != next.x
        || previous.y != next.y
        || previous.width_px != next.width_px
        || previous.height_px != next.height_px;
    // record bounds changes
    if bounds_changed {
        changed_mask |= win32_core::DISPLAY_CHANGED_MASK_BOUNDS;
    }

    let work_area_changed = previous.work_area_x != next.work_area_x
        || previous.work_area_y != next.work_area_y
        || previous.work_area_width_px != next.work_area_width_px
        || previous.work_area_height_px != next.work_area_height_px;
    // record work-area changes
    if work_area_changed {
        changed_mask |= win32_core::DISPLAY_CHANGED_MASK_WORKAREA;
    }

    // record scale-factor changes
    if previous.scale_factor_milli != next.scale_factor_milli {
        changed_mask |= win32_core::DISPLAY_CHANGED_MASK_SCALE;
    }

    // record orientation changes
    if previous.orientation != next.orientation {
        changed_mask |= win32_core::DISPLAY_CHANGED_MASK_ORIENTATION;
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

    // emit removed displays first
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

    // emit additions and per-display changes
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

        // emit mode changes for existing displays
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

        // emit descriptor changes for existing displays
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

    // emit primary-display transitions
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

/// Convert one stored monitor-event record into one ABI event payload.
pub(crate) fn display_event_from_record(
    binding: &BindingCallContext,
    value: DisplayEventRecord,
) -> DisplayMonitorEvent {
    // map stored monitor-event variants to abi payloads
    match value.kind {
        DisplayEventRecordKind::Added { descriptor } => {
            DisplayMonitorEvent::DisplayAddedEvent(DisplayAddedEvent {
                kind: binding.store_string("added"),
                metadata: display_event_metadata(
                    binding,
                    Some(&descriptor.id),
                    value.timestamp_ns,
                    value.sequence,
                    value.dropped_count,
                ),
                payload: DisplayAddedPayload {
                    descriptor: monitor::descriptor_from_owned(binding, &descriptor),
                },
            })
        }
        DisplayEventRecordKind::Removed { id, descriptor } => {
            DisplayMonitorEvent::DisplayRemovedEvent(DisplayRemovedEvent {
                kind: binding.store_string("removed"),
                metadata: display_event_metadata(
                    binding,
                    Some(&id),
                    value.timestamp_ns,
                    value.sequence,
                    value.dropped_count,
                ),
                payload: DisplayRemovedPayload {
                    id: binding.store_string(&id),
                    descriptor: descriptor
                        .as_ref()
                        .map(|value| monitor::descriptor_from_owned(binding, value)),
                },
            })
        }
        DisplayEventRecordKind::PrimaryChanged {
            previous_id,
            current_id,
        } => DisplayMonitorEvent::DisplayPrimaryChangedEvent(DisplayPrimaryChangedEvent {
            kind: binding.store_string("primaryChanged"),
            metadata: display_event_metadata(
                binding,
                current_id.as_deref(),
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: DisplayPrimaryPayload {
                previous_id: previous_id.map(|value| binding.store_string(&value)),
                current_id: current_id.map(|value| binding.store_string(&value)),
            },
        }),
        DisplayEventRecordKind::DescriptorChanged {
            previous,
            current,
            changed_mask,
        } => DisplayMonitorEvent::DisplayDescriptorChangedEvent(DisplayDescriptorChangedEvent {
            kind: binding.store_string("descriptorChanged"),
            metadata: display_event_metadata(
                binding,
                Some(&current.id),
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: DisplayDescriptorChangedPayload {
                previous: previous
                    .as_ref()
                    .map(|value| monitor::descriptor_from_owned(binding, value)),
                current: monitor::descriptor_from_owned(binding, &current),
                changed_mask: DisplayMetricChangedMask(changed_mask),
            },
        }),
        DisplayEventRecordKind::ModeChanged {
            id,
            previous,
            current,
        } => DisplayMonitorEvent::DisplayModeChangedEvent(DisplayModeChangedEvent {
            kind: binding.store_string("modeChanged"),
            metadata: display_event_metadata(
                binding,
                Some(&id),
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: DisplayModeChangedPayload { previous, current },
        }),
    }
}
