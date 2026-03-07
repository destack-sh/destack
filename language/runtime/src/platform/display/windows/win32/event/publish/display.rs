use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::display::DisplayMode;
use crate::platform::display::windows::win32::core::{self as win32_core, Win32RuntimeState};
use crate::platform::display::windows::win32::event::codec::monitor_topology_records;
use crate::platform::display::windows::win32::event::queue::{
    publish_monitor_event, push_seeded_monitor_record,
};
use crate::platform::display::windows::win32::event::{
    DisplayEventRecordKind, MonitorEventStream, display_event_record,
};
use crate::platform::display::windows::win32::model::{DisplayDescriptorSnapshot, MonitorSnapshot};
use crate::platform::display::windows::win32::monitor;
use crate::runtime::BindingCallContext;
/// Publish one mode-changed monitor event.
pub(crate) fn publish_mode_changed_event(
    binding: &BindingCallContext,
    display_id: &str,
    mode: DisplayMode,
) {
    let record = display_event_record(DisplayEventRecordKind::ModeChanged {
        id: display_id.to_string(),
        previous: None,
        current: mode,
    });

    let runtime_state = win32_core::runtime_state(binding);
    publish_monitor_event(&runtime_state, record);
}

/// Publish one descriptor-changed monitor event.
pub(crate) fn publish_descriptor_changed_event(
    binding: &BindingCallContext,
    descriptor: &DisplayDescriptorSnapshot,
    changed_mask: u32,
) {
    let record = display_event_record(DisplayEventRecordKind::DescriptorChanged {
        previous: None,
        current: descriptor.clone(),
        changed_mask,
    });

    let runtime_state = win32_core::runtime_state(binding);
    publish_monitor_event(&runtime_state, record);
}

/// Refresh the cached monitor topology snapshot from the current host state.
pub(crate) fn refresh_monitor_topology_cache(binding: &BindingCallContext) -> RuntimeResult<()> {
    // refresh monitor snapshots from host state
    let snapshots = monitor::enumerate_monitor_snapshots()?;

    // replace cached topology snapshot atomically
    let runtime_state = win32_core::runtime_state(binding);
    runtime_state.reset_monitor_topology_snapshot(snapshots);

    Ok(())
}

/// Publish monitor topology events observed since the last cached snapshot.
pub(crate) fn publish_monitor_topology_deltas(
    runtime_state: &Arc<Win32RuntimeState>,
) -> RuntimeResult<()> {
    let next_snapshots = monitor::enumerate_monitor_snapshots()?;

    let Some(records) =
        runtime_state.replace_monitor_topology_snapshot(next_snapshots, monitor_topology_records)
    else {
        return Ok(());
    };

    // publish each monitor topology delta in order
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

    // seed one added and mode-changed record per current display
    for snapshot in &snapshots {
        // remember the current primary display while seeding
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
