use std::sync::Arc;

use wayland_client::Proxy;
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_head_v1, zwlr_output_mode_v1,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::DisplayMode;
use crate::platform::{PlatformErrorCode, core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::core::{output_head_id_for_display_id, output_mode_id_for_display_mode};
use super::snapshot::{snapshot_by_display_handle, snapshot_by_display_id};
use crate::platform::display::unix::wayland::{core, event};

/// Apply one display mode through wlr output management.
fn apply_monitor_mode_by_display_id(
    context: &BindingCallContext,
    display_id: &str,
    mode: DisplayMode,
    operation: &'static str,
) -> RuntimeResult<()> {
    core::with_connection_dispatch(
        context,
        operation,
        |connection, event_queue, dispatch_state| {
            // require one negotiated wlr output manager and serial lane
            let manager = dispatch_state
                .globals
                .wlr_output_manager
                .as_ref()
                .cloned()
                .ok_or_else(|| core_platform::not_supported(operation))?;
            let serial = dispatch_state
                .globals
                .wlr_output_manager_serial
                .ok_or_else(|| core_platform::not_supported(operation))?;
            let output_head_id = output_head_id_for_display_id(dispatch_state, display_id)
                .ok_or_else(|| core_platform::not_supported(operation))?;

            // resolve the output head proxy for this mode request
            let output_head =
                zwlr_output_head_v1::ZwlrOutputHeadV1::from_id(connection, output_head_id)
                    .map_err(|error| {
                        core::io_error(operation, format!("invalid output head object id: {error}"))
                    })?;

            // allocate one configuration object and configure this output head
            let configuration_state = Arc::new(core::WaylandOutputConfigurationState::default());
            let queue_handle = event_queue.handle();
            let configuration = manager.create_configuration(
                serial,
                &queue_handle,
                Arc::clone(&configuration_state),
            );
            let configuration_head = configuration.enable_head(&output_head, &queue_handle, ());

            // use exact compositor mode objects when available for robust mode selection
            let mode_id = output_mode_id_for_display_mode(dispatch_state, display_id, mode);
            if let Some(mode_id) = mode_id {
                let mode = zwlr_output_mode_v1::ZwlrOutputModeV1::from_id(connection, mode_id)
                    .map_err(|error| {
                        core::io_error(operation, format!("invalid output mode object id: {error}"))
                    })?;
                configuration_head.set_mode(&mode);
            }
            // otherwise fall back to custom mode request lanes
            else {
                let width = i32::try_from(mode.width).map_err(|_| {
                    core_platform::invalid_argument("mode", "mode width must fit in int32")
                })?;
                let height = i32::try_from(mode.height).map_err(|_| {
                    core_platform::invalid_argument("mode", "mode height must fit in int32")
                })?;
                let refresh = i32::try_from(mode.refresh_milli_hz).map_err(|_| {
                    core_platform::invalid_argument("mode", "mode refresh must fit in int32")
                })?;
                configuration_head.set_custom_mode(width, height, refresh);
            }

            // submit one output configuration apply request
            configuration.apply();
            core::flush_queue(event_queue, operation)?;

            // wait for one terminal configuration outcome from compositor events
            for _ in 0..4 {
                let outcome = configuration_state
                    .outcome
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .as_ref()
                    .cloned();
                if outcome.is_some() {
                    break;
                }

                event_queue.roundtrip(dispatch_state).map_err(|error| {
                    core::io_error(
                        operation,
                        format!("wlr output configuration roundtrip failed: {error}"),
                    )
                })?;
            }

            // map one terminal configuration result back into runtime errors
            let outcome = configuration_state
                .outcome
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .take();

            match outcome {
                Some(core::WaylandOutputConfigurationOutcome::Succeeded) => Ok(()),
                Some(core::WaylandOutputConfigurationOutcome::Failed) => {
                    Err(core_platform::io_operation_error(
                        operation,
                        None,
                        "compositor rejected output mode configuration",
                    ))
                }
                Some(core::WaylandOutputConfigurationOutcome::Cancelled) => {
                    Err(core_platform::io_operation_error(
                        operation,
                        Some(PlatformErrorCode::IoInterrupted),
                        "compositor cancelled output mode configuration",
                    ))
                }
                None => Err(core_platform::io_would_block(
                    operation,
                    "timed out waiting for output configuration result",
                )),
            }
        },
    )
}

/// Apply one display mode.
pub(crate) unsafe fn monitor_set_mode(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayMode,
) -> RuntimeResult<()> {
    // resolve the target display snapshot and validate the requested mode
    let previous_snapshot =
        snapshot_by_display_handle(context, handle, "destack.display.monitor.setMode")?;
    if !previous_snapshot.modes.contains(&mode) {
        return Err(core_platform::invalid_argument(
            "mode",
            "display mode is not supported by the target display",
        ));
    }

    // apply the requested mode through compositor output management lanes
    let display_id = previous_snapshot.descriptor.id.clone();
    apply_monitor_mode_by_display_id(
        context,
        display_id.as_str(),
        mode,
        "destack.display.monitor.setMode",
    )?;
    core::dispatch_pending(context, "destack.display.monitor.setMode")?;

    // publish mode and descriptor deltas from updated monitor snapshots
    let current_snapshot = snapshot_by_display_id(
        context,
        display_id.as_str(),
        "destack.display.monitor.setMode",
    )?;
    let runtime_state = core::runtime_state(context);

    event::publish_monitor_mode_changed(
        &runtime_state,
        display_id.as_str(),
        Some(previous_snapshot.current_mode),
        current_snapshot.current_mode,
    );

    if current_snapshot.descriptor != previous_snapshot.descriptor {
        event::publish_monitor_descriptor_changed(
            &runtime_state,
            Some(previous_snapshot.descriptor),
            current_snapshot.descriptor,
        );
    }

    event::refresh_monitor_topology_cache(context)?;

    // clear cached gamma payloads because compositor mode switches can resize ramps
    let mut gamma_cache = runtime_state
        .gamma_ramps_by_display_id
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    gamma_cache.remove(display_id.as_str());

    Ok(())
}
