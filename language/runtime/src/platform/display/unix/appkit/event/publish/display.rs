use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::display::DisplayMode;
use crate::platform::display::unix::appkit::core::AppKitRuntimeState;
use crate::platform::display::unix::appkit::model::{DisplayDescriptorSnapshot, MonitorSnapshot};
use crate::platform::display::unix::appkit::monitor;

use crate::platform::display::unix::appkit::event::codec::{
    descriptor_changed_mask, monitor_topology_records,
};
use crate::platform::display::unix::appkit::event::queue::{
    publish_monitor_event, push_seeded_monitor_record,
};
use crate::platform::display::unix::appkit::event::{
    DisplayEventRecord, DisplayEventRecordKind, MonitorEventStream, display_event_record,
};
/// Publish one monitor mode-changed event for one display.
pub(crate) fn publish_monitor_mode_changed(
    runtime_state: &Arc<AppKitRuntimeState>,
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
#[allow(dead_code)]
pub(crate) fn publish_monitor_descriptor_changed(
    runtime_state: &Arc<AppKitRuntimeState>,
    previous: Option<DisplayDescriptorSnapshot>,
    current: DisplayDescriptorSnapshot,
) {
    let changed_mask = previous
        .as_ref()
        .map_or(0, |value| descriptor_changed_mask(value, &current));
    // skip descriptor publications with no changed fields
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
pub(crate) fn refresh_monitor_topology_cache(
    runtime_state: &Arc<AppKitRuntimeState>,
) -> RuntimeResult<()> {
    // enumerate snapshots and replace cached topology atomically
    let snapshots = monitor::enumerate_monitor_snapshots()?;
    runtime_state.reset_monitor_topology_snapshot(snapshots);

    Ok(())
}

/// Publish monitor topology deltas observed since the last cached snapshot.
pub(crate) fn publish_monitor_topology_deltas(
    runtime_state: &Arc<AppKitRuntimeState>,
) -> RuntimeResult<()> {
    // enumerate next monitor topology snapshot
    let next_snapshots = monitor::enumerate_monitor_snapshots()?;

    // compute delta records and replace cached snapshot
    let Some(records) =
        runtime_state.replace_monitor_topology_snapshot(next_snapshots, monitor_topology_records)
    else {
        return Ok(());
    };

    // publish topology records into all monitor streams
    for record in records {
        publish_monitor_event(runtime_state, record);
    }

    Ok(())
}

/// Seed one monitor-event stream with current monitor snapshot events.
pub(crate) fn seed_monitor_event_stream(
    stream: &Arc<MonitorEventStream>,
) -> RuntimeResult<Vec<MonitorSnapshot>> {
    let snapshots = monitor::enumerate_monitor_snapshots()?;
    let mut primary_id = None;

    // seed added and mode events for each current display
    for snapshot in &snapshots {
        // track the current primary display while seeding
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
