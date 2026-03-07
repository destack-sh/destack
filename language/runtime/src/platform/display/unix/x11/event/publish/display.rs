use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::display::DisplayMode;
use crate::runtime::BindingCallContext;

use crate::platform::display::unix::x11::event::codec::{
    descriptor_changed_mask, monitor_topology_records,
};
use crate::platform::display::unix::x11::event::queue::{
    publish_monitor_event, push_seeded_monitor_record,
};
use crate::platform::display::unix::x11::event::{
    DisplayEventRecord, DisplayEventRecordKind, MonitorEventStream, display_event_record,
};
use crate::platform::display::unix::x11::model::{DisplayDescriptorSnapshot, MonitorSnapshot};
use crate::platform::display::unix::x11::{core as x11_core, monitor};

/// Publish one monitor mode-changed event for one display.
pub(crate) fn publish_monitor_mode_changed(
    runtime_state: &Arc<x11_core::X11RuntimeState>,
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
    runtime_state: &Arc<x11_core::X11RuntimeState>,
    previous: Option<DisplayDescriptorSnapshot>,
    current: DisplayDescriptorSnapshot,
) {
    let changed_mask = previous
        .as_ref()
        .map_or(0, |value| descriptor_changed_mask(value, &current));

    // skip descriptor events that do not change any published field
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

/// Refresh one cached monitor topology snapshot from host state.
pub(crate) fn refresh_monitor_topology_cache(binding: &BindingCallContext) -> RuntimeResult<()> {
    let snapshots = monitor::enumerate_monitor_snapshots(binding)?;
    let runtime_state = x11_core::runtime_state(binding);
    runtime_state.reset_monitor_topology_snapshot(snapshots);

    Ok(())
}

/// Publish monitor topology deltas observed since the last cached snapshot.
pub(crate) fn publish_monitor_topology_deltas(
    runtime_state: &Arc<x11_core::X11RuntimeState>,
) -> RuntimeResult<()> {
    let next_snapshots =
        monitor::enumerate_monitor_snapshots_for_runtime(runtime_state, "destack.display")?;

    let Some(records) =
        runtime_state.replace_monitor_topology_snapshot(next_snapshots, monitor_topology_records)
    else {
        return Ok(());
    };

    for record in records {
        publish_monitor_event(runtime_state, record);
    }

    Ok(())
}

/// Seed one monitor-event stream with current monitor snapshot events.
pub(crate) fn seed_monitor_event_stream(
    binding: &BindingCallContext,
    stream: &Arc<MonitorEventStream>,
) -> RuntimeResult<Vec<MonitorSnapshot>> {
    let snapshots = monitor::enumerate_monitor_snapshots(binding)?;
    let mut primary_id = None;

    // seed added and mode events for each current display
    for snapshot in &snapshots {
        if snapshot.descriptor.primary {
            primary_id = Some(snapshot.descriptor.id.clone());
        }

        let added = display_event_record(DisplayEventRecordKind::Added {
            descriptor: snapshot.descriptor.clone(),
        });
        push_seeded_monitor_record(stream, added);

        let mode_changed = display_event_record(DisplayEventRecordKind::ModeChanged {
            id: snapshot.descriptor.id.clone(),
            previous: None,
            current: snapshot.current_mode,
        });
        push_seeded_monitor_record(stream, mode_changed);
    }

    let primary_changed = display_event_record(DisplayEventRecordKind::PrimaryChanged {
        previous_id: None,
        current_id: primary_id,
    });
    push_seeded_monitor_record(stream, primary_changed);

    Ok(snapshots)
}
