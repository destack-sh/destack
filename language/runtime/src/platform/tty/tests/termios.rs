use super::{
    HarnessValue, TtyHarnessContext, assert_ok_or_expected_error, assert_platform_error_codes,
    close_tty_worker_resource, decode_harness_value, with_harness_context,
};
use crate::diagnostic::RuntimeResult;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::process;
use crate::platform::resource::{ResourceId, TtyHandle};
use crate::platform::tty::{
    TtyTermiosAttributes, TtyTermiosAttributesVm, TtyTermiosFlowAction, TtyTermiosQueue,
    TtyTermiosSetAction,
};

/// Build one minimal termios attributes payload with an empty control-character lane.
fn empty_termios_attributes(
    context: &mut TtyHarnessContext<'_>,
) -> RuntimeResult<HarnessValue<TtyTermiosAttributes, TtyTermiosAttributesVm>> {
    // build one backend-typed empty control-character lane
    let control_characters = context.bytes_value(&[])?;

    // wrap one typed termios payload for the active harness backend
    let attributes = match control_characters {
        HarnessValue::Native(control_characters) => context.harness_value(TtyTermiosAttributes {
            input_flags: 0,
            output_flags: 0,
            control_flags: 0,
            local_flags: 0,
            control_characters,
            input_speed_code: 0,
            output_speed_code: 0,
        }),
        HarnessValue::Vm(control_characters) => context.harness_value_vm(TtyTermiosAttributesVm {
            input_flags: 0,
            output_flags: 0,
            control_flags: 0,
            local_flags: 0,
            control_characters,
            input_speed_code: 0,
            output_speed_code: 0,
        }),
    };

    Ok(attributes)
}

/// Reject unknown tty handles for termios operations.
#[cfg(unix)]
#[test]
fn test_tty_termios_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let unknown = TtyHandle(ResourceId::local(0));

        let drain_result = context.destack_tty_termios_drain(unknown);
        assert_platform_error_codes(drain_result, &[PlatformErrorCode::InvalidArgumentValue])?;

        let flow_result =
            context.destack_tty_termios_flow(unknown, TtyTermiosFlowAction::ResumeOutput);
        assert_platform_error_codes(flow_result, &[PlatformErrorCode::InvalidArgumentValue])?;

        let flush_result =
            context.destack_tty_termios_flush(unknown, TtyTermiosQueue::InputAndOutput);
        assert_platform_error_codes(flush_result, &[PlatformErrorCode::InvalidArgumentValue])?;

        let attributes_result = context.destack_tty_termios_get_attributes(unknown);
        assert_platform_error_codes(
            attributes_result,
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        let send_break_result = context.destack_tty_termios_send_break(unknown, 0);
        assert_platform_error_codes(
            send_break_result,
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        let attributes = empty_termios_attributes(&mut context)?;
        let set_attributes_result = context.destack_tty_termios_set_attributes(
            unknown,
            attributes,
            TtyTermiosSetAction::Now,
        );
        assert_platform_error_codes(
            set_attributes_result,
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        let get_process_group_result = context.destack_tty_termios_get_process_group(unknown);
        assert_platform_error_codes(
            get_process_group_result,
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        let set_process_group_result =
            context.destack_tty_termios_set_process_group(unknown, process::ProcessId(1));
        assert_platform_error_codes(
            set_process_group_result,
            &[PlatformErrorCode::InvalidArgumentValue],
        )
    });
}

/// Roundtrip and exercise termios operations on one opened tty worker.
#[cfg(unix)]
#[test]
fn test_tty_termios_roundtrip() {
    with_harness_context(|mut context| {
        // open one pty pair
        let pair = context.destack_tty_pty_open(24, 80, 0)?;
        let pair = decode_harness_value(pair);

        // read one termios snapshot and reapply it
        let attributes = context.destack_tty_termios_get_attributes(pair.worker)?;
        let control_characters = match &attributes {
            HarnessValue::Native(value) => context.harness_value(value.control_characters),
            HarnessValue::Vm(value) => context.harness_value_vm(value.control_characters),
        };
        let control_characters = context.bytes_from_value(control_characters)?;
        assert_eq!(control_characters.len(), libc::NCCS);

        context.destack_tty_termios_set_attributes(
            pair.worker,
            attributes,
            TtyTermiosSetAction::Now,
        )?;

        // exercise drain, flow, flush, and break operations
        let _ = assert_ok_or_expected_error(
            context.destack_tty_termios_drain(pair.worker),
            &[
                PlatformErrorCode::IoWouldBlock,
                PlatformErrorCode::IoInterrupted,
                PlatformErrorCode::IoInvalidData,
            ],
        )?;
        let _ = assert_ok_or_expected_error(
            context.destack_tty_termios_flow(pair.worker, TtyTermiosFlowAction::ResumeOutput),
            &[
                PlatformErrorCode::IoWouldBlock,
                PlatformErrorCode::IoInvalidData,
                PlatformErrorCode::IoPermissionDenied,
            ],
        )?;
        let _ = assert_ok_or_expected_error(
            context.destack_tty_termios_flush(pair.worker, TtyTermiosQueue::InputAndOutput),
            &[
                PlatformErrorCode::IoWouldBlock,
                PlatformErrorCode::IoInvalidData,
                PlatformErrorCode::IoPermissionDenied,
            ],
        )?;
        let _ = assert_ok_or_expected_error(
            context.destack_tty_termios_send_break(pair.worker, 0),
            &[
                PlatformErrorCode::IoWouldBlock,
                PlatformErrorCode::IoInvalidData,
                PlatformErrorCode::IoPermissionDenied,
            ],
        )?;

        // probe process-group controls and roundtrip when allowed
        let process_group = assert_ok_or_expected_error(
            context.destack_tty_termios_get_process_group(pair.worker),
            &[
                PlatformErrorCode::Process,
                PlatformErrorCode::ProcessNotFound,
                PlatformErrorCode::ProcessPermissionDenied,
            ],
        )?;
        if let Some(process_group) = process_group {
            let _ = assert_ok_or_expected_error(
                context.destack_tty_termios_set_process_group(pair.worker, process_group),
                &[
                    PlatformErrorCode::Process,
                    PlatformErrorCode::ProcessNotFound,
                    PlatformErrorCode::ProcessPermissionDenied,
                ],
            )?;
        }

        // close resources
        context.destack_tty_pty_close(pair.controller)?;
        close_tty_worker_resource(context.call_context, pair.worker)?;

        Ok(())
    });
}

/// Reject invalid termios control-character lane length on supported backends.
#[cfg(unix)]
#[test]
fn test_tty_termios_set_attributes_rejects_invalid_control_character_lane() {
    with_harness_context(|mut context| {
        // open one pty pair
        let pair = context.destack_tty_pty_open(24, 80, 0)?;
        let pair = decode_harness_value(pair);

        // apply one intentionally invalid termios payload
        let attributes = empty_termios_attributes(&mut context)?;
        let result = context.destack_tty_termios_set_attributes(
            pair.worker,
            attributes,
            TtyTermiosSetAction::Now,
        );
        assert_platform_error_codes(result, &[PlatformErrorCode::InvalidArgumentValue])?;

        // close resources
        context.destack_tty_pty_close(pair.controller)?;
        close_tty_worker_resource(context.call_context, pair.worker)
    });
}

/// Exercise overflow speed lanes and accept host-specific outcomes.
#[cfg(unix)]
#[test]
fn test_tty_termios_set_attributes_speed_overflow_host_dependent() {
    with_harness_context(|mut context| {
        // open one pty pair
        let pair = context.destack_tty_pty_open(24, 80, 0)?;
        let pair = decode_harness_value(pair);

        // read one baseline termios payload
        let attributes = context.destack_tty_termios_get_attributes(pair.worker)?;

        // apply one overflow speed code while preserving all other lanes
        let attributes = match attributes {
            HarnessValue::Native(mut value) => {
                value.input_speed_code = u64::MAX;
                context.harness_value(value)
            }
            HarnessValue::Vm(mut value) => {
                value.input_speed_code = u64::MAX;
                context.harness_value_vm(value)
            }
        };
        let result = context.destack_tty_termios_set_attributes(
            pair.worker,
            attributes,
            TtyTermiosSetAction::Now,
        );
        let _ = assert_ok_or_expected_error(
            result,
            &[
                PlatformErrorCode::InvalidArgumentValue,
                PlatformErrorCode::IoInvalidData,
            ],
        )?;

        // close resources
        context.destack_tty_pty_close(pair.controller)?;
        close_tty_worker_resource(context.call_context, pair.worker)
    });
}

/// Reject zero process-group ids for termios process-group updates.
#[cfg(unix)]
#[test]
fn test_tty_termios_set_process_group_rejects_zero() {
    with_harness_context(|mut context| {
        // open one pty pair
        let pair = context.destack_tty_pty_open(24, 80, 0)?;
        let pair = decode_harness_value(pair);

        // reject one zero process-group id
        let result =
            context.destack_tty_termios_set_process_group(pair.worker, process::ProcessId(0));
        assert_platform_error_codes(result, &[PlatformErrorCode::InvalidArgumentValue])?;

        // close resources
        context.destack_tty_pty_close(pair.controller)?;
        close_tty_worker_resource(context.call_context, pair.worker)
    });
}

/// Reject break durations that exceed host syscall argument range.
#[cfg(unix)]
#[test]
fn test_tty_termios_send_break_rejects_duration_overflow() {
    with_harness_context(|mut context| {
        // open one pty pair
        let pair = context.destack_tty_pty_open(24, 80, 0)?;
        let pair = decode_harness_value(pair);

        // reject one overflow duration payload
        let result = context.destack_tty_termios_send_break(pair.worker, u32::MAX);
        assert_platform_error_codes(result, &[PlatformErrorCode::InvalidArgumentValue])?;

        // close resources
        context.destack_tty_pty_close(pair.controller)?;
        close_tty_worker_resource(context.call_context, pair.worker)
    });
}

/// Repeatedly roundtrip termios attributes to stress conversion and host calls.
#[cfg(unix)]
#[test]
fn test_tty_termios_repeated_roundtrip_stress() {
    with_harness_context(|mut context| {
        // open one pty pair
        let pair = context.destack_tty_pty_open(24, 80, 0)?;
        let pair = decode_harness_value(pair);

        // repeatedly read and reapply one termios payload
        for _ in 0..32 {
            let attributes = context.destack_tty_termios_get_attributes(pair.worker)?;

            context.destack_tty_termios_set_attributes(
                pair.worker,
                attributes,
                TtyTermiosSetAction::Now,
            )?;
        }

        // close resources
        context.destack_tty_pty_close(pair.controller)?;
        close_tty_worker_resource(context.call_context, pair.worker)
    });
}
