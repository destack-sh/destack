use super::{
    decode_display_descriptor, decode_harness_value, decode_monitor_list, decode_monitor_modes,
    decode_window_descriptor, default_monitor_event_open_options, default_monitor_list_request,
    default_monitor_open_options, default_window_event_open_options, default_window_options,
    harness_string, with_harness_context,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::display::{DisplayMode, DisplayModeVm, WindowVisibility};

#[cfg(any(unix, windows))]
#[test]
fn test_display_monitor_surface_works_end_to_end() {
    with_harness_context(|mut context| {
        let monitor_list =
            match context.destack_display_monitor_list(default_monitor_list_request(&context)) {
                Ok(monitor_list) => monitor_list,
                Err(error) => {
                    let code = error.platform_error().map(|platform| platform.code);
                    if code == Some(PlatformErrorCode::NotSupported) {
                        return Ok(());
                    }

                    return Err(error);
                }
            };
        let monitor_list = decode_monitor_list(&mut context, monitor_list)?;
        assert!(!monitor_list.is_empty());

        let (display_id, _, is_primary) = monitor_list[0].clone();
        assert!(is_primary);

        let display_id_value = harness_string(&mut context, &display_id)?;
        let display = context.destack_display_monitor_open(
            display_id_value,
            default_monitor_open_options(&context),
        )?;
        let descriptor = context.destack_display_monitor_descriptor(display)?;
        let (descriptor_id, _, descriptor_primary) =
            decode_display_descriptor(&mut context, descriptor)?;
        assert_eq!(descriptor_id, display_id);
        assert!(descriptor_primary);

        let modes = context.destack_display_monitor_modes(display)?;
        let modes = decode_monitor_modes(&mut context, modes)?;
        assert!(!modes.is_empty());

        let event_stream = context
            .destack_display_monitor_event_open(default_monitor_event_open_options(&context))?;
        let requested_mode = modes[modes.len().saturating_sub(1)];
        let requested_mode = if context.vm_context.is_some() {
            context.harness_value_vm::<DisplayMode, DisplayModeVm>(requested_mode)
        } else {
            context.harness_value::<DisplayMode, DisplayModeVm>(requested_mode)
        };
        context.destack_display_monitor_set_mode(display, requested_mode)?;

        let event = context.destack_display_monitor_event_read(event_stream, 100_000_000)?;
        assert!(matches!(
            event,
            super::HarnessValue::Native(
                crate::platform::display::DisplayEvent::DisplayModeChangedEvent(_)
            ) | super::HarnessValue::Vm(
                crate::platform::display::DisplayEventVm::DisplayModeChangedEvent(_)
            )
        ));

        let primary =
            context.destack_display_monitor_primary(default_monitor_list_request(&context))?;
        assert!(primary.is_some());

        context.destack_display_monitor_event_close(event_stream)?;
        context.destack_display_monitor_close(display)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_display_window_surface_works_end_to_end() {
    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "alpha")?;
        let window = match context.destack_display_window_open(options) {
            Ok(window) => window,
            Err(error) => {
                let code = error.platform_error().map(|platform| platform.code);
                if code == Some(PlatformErrorCode::NotSupported) {
                    return Ok(());
                }

                return Err(error);
            }
        };

        let descriptor = context.destack_display_window_descriptor(window)?;
        let (_, title) = decode_window_descriptor(&mut context, descriptor)?;
        assert_eq!(title, "alpha");

        let title = harness_string(&mut context, "beta")?;
        context.destack_display_window_set_title(window, title)?;
        let descriptor = context.destack_display_window_descriptor(window)?;
        let (_, title) = decode_window_descriptor(&mut context, descriptor)?;
        assert_eq!(title, "beta");

        let event_stream = context
            .destack_display_window_event_open(default_window_event_open_options(&context))?;
        context.destack_display_window_request_refresh(window)?;

        let event = context.destack_display_window_event_read(event_stream, 100_000_000)?;
        assert!(matches!(
            event,
            super::HarnessValue::Native(
                crate::platform::display::WindowEvent::WindowRefreshRequestedEvent(_)
            ) | super::HarnessValue::Native(
                crate::platform::display::WindowEvent::WindowCreatedEvent(_)
            ) | super::HarnessValue::Vm(
                crate::platform::display::WindowEventVm::WindowRefreshRequestedEvent(_)
            ) | super::HarnessValue::Vm(
                crate::platform::display::WindowEventVm::WindowCreatedEvent(_)
            )
        ));

        context.destack_display_window_set_visibility(window, WindowVisibility::Minimized)?;
        let state = context.destack_display_window_state(window)?;
        let state = decode_harness_value(state);
        assert_eq!(state.visibility, WindowVisibility::Minimized);

        let vsync = context.destack_display_window_vsync_wait(window, 1_000_000);
        if let Err(error) = vsync {
            let code = error.platform_error().map(|platform| platform.code);
            assert_eq!(code, Some(PlatformErrorCode::IoWouldBlock));
        }

        context.destack_display_window_event_close(event_stream)?;
        context.destack_display_window_close(window)?;
        Ok(())
    });
}
