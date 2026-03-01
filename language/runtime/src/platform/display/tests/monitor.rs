#[cfg(windows)]
use super::decode_display_descriptor_metrics;
use super::{
    decode_display_mode, decode_monitor_list, decode_monitor_modes, default_monitor_list_request,
    default_monitor_open_options, error_code, harness_display_mode, harness_string,
    with_harness_context,
};
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(windows)]
use crate::platform::display::DisplayOrientation;

#[cfg(any(unix, windows))]
#[test]
fn test_monitor_closest_mode_returns_supported_mode() {
    with_harness_context(|mut context| {
        let monitor_list =
            match context.destack_display_monitor_list(default_monitor_list_request(&context)) {
                Ok(value) => value,
                Err(error) => {
                    if error_code(&error) == Some(PlatformErrorCode::NotSupported) {
                        return Ok(());
                    }

                    return Err(error);
                }
            };
        let monitor_list = decode_monitor_list(&mut context, monitor_list)?;
        assert!(!monitor_list.is_empty());

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
#[test]
fn test_monitor_open_unknown_id_reports_not_found() {
    with_harness_context(|mut context| {
        let monitor_list =
            match context.destack_display_monitor_list(default_monitor_list_request(&context)) {
                Ok(value) => value,
                Err(error) => {
                    if error_code(&error) == Some(PlatformErrorCode::NotSupported) {
                        return Ok(());
                    }

                    return Err(error);
                }
            };
        let monitor_list = decode_monitor_list(&mut context, monitor_list)?;
        assert!(!monitor_list.is_empty());

        let missing = harness_string(&mut context, "destack-display-missing")?;
        let result =
            context.destack_display_monitor_open(missing, default_monitor_open_options(&context));
        let error = result.expect_err("unknown monitor id should fail");
        assert_eq!(error_code(&error), Some(PlatformErrorCode::IoNotFound));

        Ok(())
    });
}

#[cfg(windows)]
#[test]
fn test_monitor_descriptor_reports_orientation_and_capability_fields() {
    with_harness_context(|mut context| {
        let monitor_list =
            match context.destack_display_monitor_list(default_monitor_list_request(&context)) {
                Ok(value) => value,
                Err(error) => {
                    if error_code(&error) == Some(PlatformErrorCode::NotSupported) {
                        return Ok(());
                    }

                    return Err(error);
                }
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
