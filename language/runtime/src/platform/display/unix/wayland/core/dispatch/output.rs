use std::sync::Arc;

use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, WEnum};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_configuration_v1, zwlr_output_head_v1, zwlr_output_manager_v1, zwlr_output_mode_v1,
};

use crate::platform::display::unix::wayland::core::{
    WaylandConnectionDispatchState, WaylandOutputConfigurationOutcome,
    WaylandOutputConfigurationState, WaylandWlrAdaptiveSyncState,
};

impl Dispatch<zwlr_output_manager_v1::ZwlrOutputManagerV1, ()> for WaylandConnectionDispatchState {
    /// Handle wlr-output-manager events.
    fn event(
        state: &mut Self,
        _proxy: &zwlr_output_manager_v1::ZwlrOutputManagerV1,
        event: zwlr_output_manager_v1::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        match event {
            // track newly advertised output-head objects
            zwlr_output_manager_v1::Event::Head { head } => {
                state
                    .output
                    .wlr_output_heads_by_id
                    .entry(head.id())
                    .or_default();
                state.output.mark_monitor_topology_dirty();
            }

            // capture the latest configuration serial for mode-set operations
            zwlr_output_manager_v1::Event::Done { serial } => {
                state.globals.wlr_output_manager_serial = Some(serial);
                state.output.mark_monitor_topology_dirty();
            }

            // clear manager-derived state when the manager becomes invalid
            zwlr_output_manager_v1::Event::Finished => {
                state.globals.wlr_output_manager = None;
                state.globals.wlr_output_manager_serial = None;
                state.output.wlr_output_heads_by_id.clear();
                state.output.wlr_output_modes_by_id.clear();
                state.output.mark_monitor_topology_dirty();
            }
            _ => {}
        }
    }
}

impl Dispatch<zwlr_output_head_v1::ZwlrOutputHeadV1, ()> for WaylandConnectionDispatchState {
    /// Handle wlr-output-head events.
    fn event(
        state: &mut Self,
        proxy: &zwlr_output_head_v1::ZwlrOutputHeadV1,
        event: zwlr_output_head_v1::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        // route this output-head event into the head snapshot map
        match event {
            zwlr_output_head_v1::Event::Mode { mode } => {
                let mode_id = mode.id();

                // record this mode under the owning output head
                let head = state
                    .output
                    .wlr_output_heads_by_id
                    .entry(proxy.id())
                    .or_default();
                if !head.mode_ids.iter().any(|value| value == &mode_id) {
                    head.mode_ids.push(mode_id.clone());
                }

                // allocate one mode snapshot entry
                state
                    .output
                    .wlr_output_modes_by_id
                    .entry(mode_id)
                    .or_default();
                state.output.mark_monitor_topology_dirty();
            }

            zwlr_output_head_v1::Event::Name { name } => {
                if let Some(head) = state.output.wlr_output_heads_by_id.get_mut(&proxy.id()) {
                    head.name = Some(name);
                }
                state.output.mark_monitor_topology_dirty();
            }

            zwlr_output_head_v1::Event::AdaptiveSync {
                state: adaptive_sync,
            } => {
                let adaptive_sync = match adaptive_sync {
                    WEnum::Value(zwlr_output_head_v1::AdaptiveSyncState::Disabled) => {
                        Some(WaylandWlrAdaptiveSyncState::Disabled)
                    }
                    WEnum::Value(zwlr_output_head_v1::AdaptiveSyncState::Enabled) => {
                        Some(WaylandWlrAdaptiveSyncState::Enabled)
                    }
                    WEnum::Unknown(_) => None,
                    _ => None,
                };

                let head = state
                    .output
                    .wlr_output_heads_by_id
                    .entry(proxy.id())
                    .or_default();
                head.adaptive_sync = adaptive_sync;
                state.output.mark_monitor_topology_dirty();
            }

            zwlr_output_head_v1::Event::Finished => {
                let removed_head = state.output.wlr_output_heads_by_id.remove(&proxy.id());
                if let Some(removed_head) = removed_head {
                    for mode_id in removed_head.mode_ids {
                        state.output.wlr_output_modes_by_id.remove(&mode_id);
                    }
                }
                state.output.mark_monitor_topology_dirty();
            }
            _ => {}
        }
    }
}

impl Dispatch<zwlr_output_mode_v1::ZwlrOutputModeV1, ()> for WaylandConnectionDispatchState {
    /// Handle wlr-output-mode events.
    fn event(
        state: &mut Self,
        proxy: &zwlr_output_mode_v1::ZwlrOutputModeV1,
        event: zwlr_output_mode_v1::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        // resolve one mutable mode snapshot entry for this proxy id
        let mode_id = proxy.id();
        let mode = state
            .output
            .wlr_output_modes_by_id
            .entry(mode_id.clone())
            .or_default();

        // apply one event variant to the mode snapshot
        match event {
            zwlr_output_mode_v1::Event::Size { width, height } => {
                if width > 0 {
                    mode.width = Some(width as u32);
                }

                if height > 0 {
                    mode.height = Some(height as u32);
                }
                state.output.mark_monitor_topology_dirty();
            }

            zwlr_output_mode_v1::Event::Refresh { refresh } => {
                if refresh > 0 {
                    mode.refresh_milli_hz = Some(refresh as u32);
                }
                state.output.mark_monitor_topology_dirty();
            }

            zwlr_output_mode_v1::Event::Preferred => {
                mode.is_preferred = true;
                state.output.mark_monitor_topology_dirty();
            }

            zwlr_output_mode_v1::Event::Finished => {
                state.output.wlr_output_modes_by_id.remove(&mode_id);

                // remove stale mode ids from every tracked output head
                for head in state.output.wlr_output_heads_by_id.values_mut() {
                    head.mode_ids.retain(|value| value != &mode_id);
                }
                state.output.mark_monitor_topology_dirty();
            }
            _ => {}
        }
    }
}

impl
    Dispatch<
        zwlr_output_configuration_v1::ZwlrOutputConfigurationV1,
        Arc<WaylandOutputConfigurationState>,
    > for WaylandConnectionDispatchState
{
    /// Handle wlr-output-configuration events.
    fn event(
        _state: &mut Self,
        proxy: &zwlr_output_configuration_v1::ZwlrOutputConfigurationV1,
        event: zwlr_output_configuration_v1::Event,
        data: &Arc<WaylandOutputConfigurationState>,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        // store terminal configuration outcomes for the waiting operation
        let outcome = match event {
            zwlr_output_configuration_v1::Event::Succeeded => {
                Some(WaylandOutputConfigurationOutcome::Succeeded)
            }
            zwlr_output_configuration_v1::Event::Failed => {
                Some(WaylandOutputConfigurationOutcome::Failed)
            }
            zwlr_output_configuration_v1::Event::Cancelled => {
                Some(WaylandOutputConfigurationOutcome::Cancelled)
            }
            _ => None,
        };

        if let Some(outcome) = outcome {
            let mut state = data
                .outcome
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            *state = Some(outcome);
            proxy.destroy();
        }
    }
}
