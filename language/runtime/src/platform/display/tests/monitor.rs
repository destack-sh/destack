#[cfg(windows)]
use super::HarnessValue;
#[cfg(windows)]
use super::decode_display_descriptor_metrics;
use super::{
    decode_display_mode, decode_monitor_list, decode_monitor_modes, default_monitor_list_request,
    default_monitor_open_options, error_code, harness_display_mode, harness_string,
    result_or_skip_not_supported, run_execution_case_or_return, with_harness_context,
};
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(windows)]
use crate::platform::display::DisplayOrientation;
#[cfg(windows)]
use crate::platform::display::{DisplayColorState, DisplayHdrMode};

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_monitor_closest_mode_returns_supported_mode() {
    if run_execution_case_or_return(display_case_name!(
        test_monitor_closest_mode_returns_supported_mode
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let Some(monitor_list) = result_or_skip_not_supported(
            context.destack_display_monitor_list(default_monitor_list_request(&context)),
        )?
        else {
            return Ok(());
        };
        let monitor_list = decode_monitor_list(&mut context, monitor_list)?;
        if monitor_list.is_empty() {
            return Ok(());
        }

        let display_id = monitor_list[0].0.clone();
        let display_id = harness_string(&mut context, &display_id)?;
        let display = context
            .destack_display_monitor_open(display_id, default_monitor_open_options(&context))?;

        let modes = context.destack_display_monitor_modes(display)?;
        let modes = decode_monitor_modes(&mut context, modes)?;
        assert!(!modes.is_empty());

        let requested = harness_display_mode(&context, 1111, 777, 123_000, 16);
        let closest = context.destack_display_monitor_closest_mode(display, requested)?;
        let closest = decode_display_mode(closest);
        assert!(modes.contains(&closest));

        context.destack_display_monitor_close(display)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_monitor_open_unknown_id_reports_not_found() {
    if run_execution_case_or_return(display_case_name!(
        test_monitor_open_unknown_id_reports_not_found
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let missing = harness_string(&mut context, "destack-display-missing")?;
        let result =
            context.destack_display_monitor_open(missing, default_monitor_open_options(&context));
        let error = result.expect_err("unknown monitor id should fail");
        assert_eq!(error_code(&error), Some(PlatformErrorCode::IoNotFound));

        Ok(())
    });
}

#[cfg(windows)]
#[cfg_attr(test, test)]
pub(crate) fn test_monitor_descriptor_reports_orientation_and_capability_fields() {
    if run_execution_case_or_return(display_case_name!(
        test_monitor_descriptor_reports_orientation_and_capability_fields
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let Some(monitor_list) = result_or_skip_not_supported(
            context.destack_display_monitor_list(default_monitor_list_request(&context)),
        )?
        else {
            return Ok(());
        };
        let monitor_list = decode_monitor_list(&mut context, monitor_list)?;
        assert!(!monitor_list.is_empty());

        let display_id = harness_string(&mut context, &monitor_list[0].0)?;
        let display = context
            .destack_display_monitor_open(display_id, default_monitor_open_options(&context))?;
        let descriptor = context.destack_display_monitor_descriptor(display)?;
        let (orientation, _, _, _, width_px, height_px) =
            decode_display_descriptor_metrics(&mut context, descriptor)?;

        assert!(width_px > 0);
        assert!(height_px > 0);
        assert_ne!(orientation, DisplayOrientation::Unknown);

        context.destack_display_monitor_close(display)?;
        Ok(())
    });
}

#[cfg(windows)]
#[cfg_attr(test, test)]
pub(crate) fn test_monitor_color_state_and_hdr_mode_are_consistent() {
    if run_execution_case_or_return(display_case_name!(
        test_monitor_color_state_and_hdr_mode_are_consistent
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let Some(monitor_list) = result_or_skip_not_supported(
            context.destack_display_monitor_list(default_monitor_list_request(&context)),
        )?
        else {
            return Ok(());
        };
        let monitor_list = decode_monitor_list(&mut context, monitor_list)?;
        assert!(!monitor_list.is_empty());

        let display_id = harness_string(&mut context, &monitor_list[0].0)?;
        let display = context
            .destack_display_monitor_open(display_id, default_monitor_open_options(&context))?;

        let color_state = match context.destack_display_monitor_color_state(display) {
            Ok(value) => value,
            Err(error) => {
                let code = error_code(&error);
                if matches!(
                    code,
                    Some(PlatformErrorCode::NotSupported)
                        | Some(PlatformErrorCode::IoPermissionDenied)
                ) {
                    context.destack_display_monitor_close(display)?;
                    return Ok(());
                }
                return Err(error);
            }
        };
        let hdr_mode: DisplayHdrMode = context.destack_display_monitor_hdr_mode(display)?;

        let color_state: DisplayColorState = match color_state {
            HarnessValue::Native(value) => value,
            HarnessValue::Vm(value) => DisplayColorState {
                hdr_mode: value.hdr_mode,
                color_space: value.color_space,
                bits_per_channel: value.bits_per_channel,
            },
        };
        if color_state.hdr_mode != DisplayHdrMode::Unknown {
            assert_eq!(hdr_mode, color_state.hdr_mode);
        }

        context.destack_display_monitor_close(display)?;
        Ok(())
    });
}

#[cfg(windows)]
#[cfg_attr(test, test)]
pub(crate) fn test_monitor_set_hdr_mode_system_is_noop() {
    if run_execution_case_or_return(display_case_name!(test_monitor_set_hdr_mode_system_is_noop)) {
        return;
    }

    with_harness_context(|mut context| {
        let Some(monitor_list) = result_or_skip_not_supported(
            context.destack_display_monitor_list(default_monitor_list_request(&context)),
        )?
        else {
            return Ok(());
        };
        let monitor_list = decode_monitor_list(&mut context, monitor_list)?;
        assert!(!monitor_list.is_empty());

        let display_id = harness_string(&mut context, &monitor_list[0].0)?;
        let display = context
            .destack_display_monitor_open(display_id, default_monitor_open_options(&context))?;

        context.destack_display_monitor_set_hdr_mode(display, DisplayHdrMode::System)?;

        context.destack_display_monitor_close(display)?;
        Ok(())
    });
}

#[cfg(windows)]
#[cfg_attr(test, test)]
pub(crate) fn test_monitor_gamma_ramp_lane_roundtrips_current_values() {
    if run_execution_case_or_return(display_case_name!(
        test_monitor_gamma_ramp_lane_roundtrips_current_values
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let Some(monitor_list) = result_or_skip_not_supported(
            context.destack_display_monitor_list(default_monitor_list_request(&context)),
        )?
        else {
            return Ok(());
        };
        let monitor_list = decode_monitor_list(&mut context, monitor_list)?;
        assert!(!monitor_list.is_empty());

        let display_id = harness_string(&mut context, &monitor_list[0].0)?;
        let display = context
            .destack_display_monitor_open(display_id, default_monitor_open_options(&context))?;

        let ramp = match context.destack_display_monitor_gamma_ramp(display) {
            Ok(value) => value,
            Err(error) => {
                let code = error_code(&error);
                if matches!(
                    code,
                    Some(PlatformErrorCode::NotSupported)
                        | Some(PlatformErrorCode::IoPermissionDenied)
                ) {
                    context.destack_display_monitor_close(display)?;
                    return Ok(());
                }
                return Err(error);
            }
        };

        let ramp_lengths = match ramp {
            HarnessValue::Native(value) => {
                let red = unsafe { value.red.as_slice()? };
                let green = unsafe { value.green.as_slice()? };
                let blue = unsafe { value.blue.as_slice()? };
                (
                    red.len(),
                    green.len(),
                    blue.len(),
                    HarnessValue::Native(value),
                )
            }
            HarnessValue::Vm(value) => {
                let vm_context = context
                    .vm_context
                    .map(|vm_context| unsafe {
                        &mut *(vm_context as *mut destack_vm::BindingContext<'_>)
                    })
                    .expect("vm context should exist for vm harness");
                let red = value.red.read_values(&vm_context.read())?;
                let green = value.green.read_values(&vm_context.read())?;
                let blue = value.blue.read_values(&vm_context.read())?;
                (red.len(), green.len(), blue.len(), HarnessValue::Vm(value))
            }
        };

        assert_eq!(ramp_lengths.0, 256);
        assert_eq!(ramp_lengths.1, 256);
        assert_eq!(ramp_lengths.2, 256);

        let set_result = match ramp_lengths.3 {
            HarnessValue::Native(value) => {
                context.destack_display_monitor_set_gamma_ramp(display, HarnessValue::Native(value))
            }
            HarnessValue::Vm(value) => {
                context.destack_display_monitor_set_gamma_ramp(display, HarnessValue::Vm(value))
            }
        };
        if let Err(error) = set_result {
            let code = error_code(&error);
            if !matches!(
                code,
                Some(PlatformErrorCode::NotSupported) | Some(PlatformErrorCode::IoPermissionDenied)
            ) {
                return Err(error);
            }
        }

        context.destack_display_monitor_close(display)?;
        Ok(())
    });
}
