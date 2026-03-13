use std::mem::MaybeUninit;

use super::core::{host_numeric_to_u64, io_error, tty_descriptor};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::{PlatformErrorCode, process_error_code_from_errno};
use crate::platform::tty::core::ensure_out;
use crate::platform::tty::{
    TtyTermiosAttributes, TtyTermiosFlowAction, TtyTermiosQueue, TtyTermiosSetAction,
};
use crate::platform::{PlatformError, core as core_platform, process, resource};
use crate::runtime::BindingCallContext;

/// Build one mapped process-domain error from explicit errno.
fn process_error_with_errno(
    _operation: &'static str,
    syscall: &'static str,
    errno: i32,
    message: &str,
) -> Box<RuntimeError> {
    let code = process_error_code_from_errno(errno).unwrap_or(PlatformErrorCode::Process);
    RuntimeError::from(PlatformError::process_with(
        Some(code),
        Some(errno.to_string()),
        None,
        None,
        Some(syscall.to_string()),
        format!("{syscall} failed: {message}"),
    ))
    .boxed()
}

/// Build one mapped process-domain error from current errno.
fn process_error(
    operation: &'static str,
    syscall: &'static str,
    message: &str,
) -> Box<RuntimeError> {
    let errno = core_platform::get_errno();
    process_error_with_errno(operation, syscall, errno, message)
}

/// Map one termios set-action value into one host selector.
fn set_action_to_host(action: TtyTermiosSetAction) -> libc::c_int {
    match action {
        TtyTermiosSetAction::Now => libc::TCSANOW,
        TtyTermiosSetAction::Drain => libc::TCSADRAIN,
        TtyTermiosSetAction::Flush => libc::TCSAFLUSH,
    }
}

/// Map one termios flush-queue selector into one host selector.
fn flush_queue_to_host(queue: TtyTermiosQueue) -> libc::c_int {
    match queue {
        TtyTermiosQueue::Input => libc::TCIFLUSH,
        TtyTermiosQueue::Output => libc::TCOFLUSH,
        TtyTermiosQueue::InputAndOutput => libc::TCIOFLUSH,
    }
}

/// Map one termios flow-action selector into one host selector.
fn flow_action_to_host(action: TtyTermiosFlowAction) -> libc::c_int {
    match action {
        TtyTermiosFlowAction::SuspendOutput => libc::TCOOFF,
        TtyTermiosFlowAction::ResumeOutput => libc::TCOON,
        TtyTermiosFlowAction::SuspendInput => libc::TCIOFF,
        TtyTermiosFlowAction::ResumeInput => libc::TCION,
    }
}

/// Decode one numeric speed code into one host termios speed value.
fn decode_speed_code(speed_code: u64, field: &str) -> RuntimeResult<libc::speed_t> {
    libc::speed_t::try_from(speed_code).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "speed code exceeds host termios range",
        ))
        .boxed()
    })
}

/// Validate control-character payload length for one termios update.
fn validate_control_character_length(length: usize) -> RuntimeResult<()> {
    let expected_length = libc::NCCS;
    if length != expected_length {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "attributes.controlCharacters",
            format!(
                "control character lane must have exactly {expected_length} bytes, got {length}",
            ),
        ))
        .boxed());
    }

    Ok(())
}

/// Validate one process-group identifier as one host pid_t value.
fn process_group_id_to_host(value: process::ProcessId) -> RuntimeResult<libc::pid_t> {
    if value.0 == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "processGroupId",
            "process group id must be greater than zero",
        ))
        .boxed());
    }
    if value.0 > libc::pid_t::MAX as u32 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "processGroupId",
            "process group id exceeds host pid range",
        ))
        .boxed());
    }

    Ok(value.0 as libc::pid_t)
}

/// Wait for pending output to drain on one terminal handle.
///
/// Block until queued terminal output bytes are transmitted according to host device behavior.
/// This does not flush input data or modify mode flags.
pub(crate) unsafe fn destack_tty_termios_drain(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    // resolve one tty descriptor
    let descriptor = tty_descriptor(binding, handle, "destack.tty.termios.drain")?;

    // issue one host drain operation
    let status = unsafe { libc::tcdrain(descriptor) };
    if status < 0 {
        return Err(io_error(
            "destack.tty.termios.drain",
            "tcdrain",
            "failed to drain tty output",
        ));
    }

    Ok(())
}

/// Apply terminal flow-control action.
///
/// Pause or resume output transmission, or send start and stop flow-control characters.
/// Remote and local behavior follows host line-discipline configuration.
pub(crate) unsafe fn destack_tty_termios_flow(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    action: TtyTermiosFlowAction,
) -> RuntimeResult<()> {
    // resolve one tty descriptor and map the requested flow action
    let descriptor = tty_descriptor(binding, handle, "destack.tty.termios.flow")?;
    let action = flow_action_to_host(action);

    // issue one host flow-control operation
    let status = unsafe { libc::tcflow(descriptor, action) };
    if status < 0 {
        return Err(io_error(
            "destack.tty.termios.flow",
            "tcflow",
            "failed to apply tty flow-control action",
        ));
    }

    Ok(())
}

/// Flush one terminal queue.
///
/// Discard buffered input, output, or both queues as selected by the queue parameter.
/// Queue semantics follow host terminal driver behavior.
pub(crate) unsafe fn destack_tty_termios_flush(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    queue: TtyTermiosQueue,
) -> RuntimeResult<()> {
    // resolve one tty descriptor and map the requested queue selector
    let descriptor = tty_descriptor(binding, handle, "destack.tty.termios.flush")?;
    let queue = flush_queue_to_host(queue);

    // issue one host queue flush operation
    let status = unsafe { libc::tcflush(descriptor, queue) };
    if status < 0 {
        return Err(io_error(
            "destack.tty.termios.flush",
            "tcflush",
            "failed to flush tty queue",
        ));
    }

    Ok(())
}

/// Read full termios attributes for one terminal handle.
///
/// Read one complete termios snapshot including flag groups, control characters, and speed codes.
/// Control-character ordering follows host `c_cc` layout for the active target.
pub(crate) unsafe fn destack_tty_termios_get_attributes(
    binding: &BindingCallContext,
    out: *mut TtyTermiosAttributes,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    ensure_out(out, "out")?;

    // resolve one tty descriptor
    let descriptor = tty_descriptor(binding, handle, "destack.tty.termios.getAttributes")?;

    // read one host termios payload
    let mut host_attributes = MaybeUninit::<libc::termios>::uninit();
    let status = unsafe { libc::tcgetattr(descriptor, host_attributes.as_mut_ptr()) };
    if status < 0 {
        return Err(io_error(
            "destack.tty.termios.getAttributes",
            "tcgetattr",
            "failed to read tty termios attributes",
        ));
    }
    let host_attributes = unsafe { host_attributes.assume_init() };

    // copy control-character bytes into runtime-owned storage
    let control_characters = host_attributes.c_cc.to_vec();
    let control_characters = binding.store_slice(control_characters);

    // project the host termios payload into platform attributes
    let attributes = TtyTermiosAttributes {
        input_flags: host_numeric_to_u64(host_attributes.c_iflag),
        output_flags: host_numeric_to_u64(host_attributes.c_oflag),
        control_flags: host_numeric_to_u64(host_attributes.c_cflag),
        local_flags: host_numeric_to_u64(host_attributes.c_lflag),
        control_characters,
        input_speed_code: host_numeric_to_u64(unsafe { libc::cfgetispeed(&host_attributes) }),
        output_speed_code: host_numeric_to_u64(unsafe { libc::cfgetospeed(&host_attributes) }),
    };

    // write projected attributes to output storage
    unsafe {
        out.write(attributes);
    }

    Ok(())
}

/// Read controlling-terminal process-group id.
///
/// Read the foreground process-group id currently associated with this terminal.
/// Foreground group semantics follow host session and job-control rules.
pub(crate) unsafe fn destack_tty_termios_get_process_group(
    binding: &BindingCallContext,
    out: *mut process::ProcessId,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    ensure_out(out, "out")?;

    // resolve one tty descriptor
    let descriptor = tty_descriptor(binding, handle, "destack.tty.termios.getProcessGroup")?;

    // read one foreground process-group identifier
    let status = unsafe { libc::tcgetpgrp(descriptor) };
    if status < 0 {
        return Err(process_error(
            "destack.tty.termios.getProcessGroup",
            "tcgetpgrp",
            "failed to read tty foreground process group",
        ));
    }

    // project the process-group identifier into platform range
    let process_group_id = u32::try_from(status).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "result",
            "process group id exceeds platform process id range",
        ))
        .boxed()
    })?;

    // write process-group identifier to output storage
    unsafe {
        out.write(process::ProcessId(process_group_id));
    }

    Ok(())
}

/// Send one terminal break condition.
///
/// Transmit one break condition on the terminal line with one host-defined duration unit.
/// A duration value of zero requests the host default break behavior.
pub(crate) unsafe fn destack_tty_termios_send_break(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    duration: u32,
) -> RuntimeResult<()> {
    // resolve one tty descriptor and decode duration
    let descriptor = tty_descriptor(binding, handle, "destack.tty.termios.sendBreak")?;
    let duration = libc::c_int::try_from(duration).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "duration",
            "duration exceeds host tcsendbreak range",
        ))
        .boxed()
    })?;

    // issue one host break condition operation
    let status = unsafe { libc::tcsendbreak(descriptor, duration) };
    if status < 0 {
        return Err(io_error(
            "destack.tty.termios.sendBreak",
            "tcsendbreak",
            "failed to send tty break condition",
        ));
    }

    Ok(())
}

/// Apply full termios attributes to one terminal handle.
///
/// Apply one complete termios snapshot with caller-selected update timing semantics.
/// Unsupported flag bits and control-character lanes follow host kernel behavior.
pub(crate) unsafe fn destack_tty_termios_set_attributes(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    attributes: TtyTermiosAttributes,
    action: TtyTermiosSetAction,
) -> RuntimeResult<()> {
    // resolve one tty descriptor and decode caller-provided lane values
    let descriptor = tty_descriptor(binding, handle, "destack.tty.termios.setAttributes")?;
    let input_flags = libc::tcflag_t::try_from(attributes.input_flags).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "attributes.inputFlags",
            "attributes.inputFlags exceeds host termios range",
        ))
        .boxed()
    })?;
    let output_flags = libc::tcflag_t::try_from(attributes.output_flags).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "attributes.outputFlags",
            "attributes.outputFlags exceeds host termios range",
        ))
        .boxed()
    })?;
    let control_flags = libc::tcflag_t::try_from(attributes.control_flags).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "attributes.controlFlags",
            "attributes.controlFlags exceeds host termios range",
        ))
        .boxed()
    })?;
    let local_flags = libc::tcflag_t::try_from(attributes.local_flags).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "attributes.localFlags",
            "attributes.localFlags exceeds host termios range",
        ))
        .boxed()
    })?;
    let input_speed = decode_speed_code(attributes.input_speed_code, "attributes.inputSpeedCode")?;
    let output_speed =
        decode_speed_code(attributes.output_speed_code, "attributes.outputSpeedCode")?;
    let control_characters = unsafe { attributes.control_characters.as_slice()? };
    validate_control_character_length(control_characters.len())?;
    let action = set_action_to_host(action);

    // read one host termios payload as a mutation baseline
    let mut host_attributes = MaybeUninit::<libc::termios>::uninit();
    let get_status = unsafe { libc::tcgetattr(descriptor, host_attributes.as_mut_ptr()) };
    if get_status < 0 {
        return Err(io_error(
            "destack.tty.termios.setAttributes",
            "tcgetattr",
            "failed to read tty termios attributes before update",
        ));
    }
    let mut host_attributes = unsafe { host_attributes.assume_init() };

    // apply caller-provided lane values
    host_attributes.c_iflag = input_flags;
    host_attributes.c_oflag = output_flags;
    host_attributes.c_cflag = control_flags;
    host_attributes.c_lflag = local_flags;
    for (index, value) in control_characters.iter().copied().enumerate() {
        host_attributes.c_cc[index] = value;
    }
    let set_input_speed_status = unsafe { libc::cfsetispeed(&mut host_attributes, input_speed) };
    if set_input_speed_status < 0 {
        return Err(io_error(
            "destack.tty.termios.setAttributes",
            "cfsetispeed",
            "failed to set tty input speed code",
        ));
    }
    let set_output_speed_status = unsafe { libc::cfsetospeed(&mut host_attributes, output_speed) };
    if set_output_speed_status < 0 {
        return Err(io_error(
            "destack.tty.termios.setAttributes",
            "cfsetospeed",
            "failed to set tty output speed code",
        ));
    }

    // commit one host termios update
    let set_status = unsafe { libc::tcsetattr(descriptor, action, &host_attributes) };
    if set_status < 0 {
        return Err(io_error(
            "destack.tty.termios.setAttributes",
            "tcsetattr",
            "failed to apply tty termios attributes",
        ));
    }

    Ok(())
}

/// Set controlling-terminal process-group id.
///
/// Set the foreground process-group id for this terminal to one existing process group.
/// Permission and session checks follow host kernel job-control rules.
pub(crate) unsafe fn destack_tty_termios_set_process_group(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    processgroupid: process::ProcessId,
) -> RuntimeResult<()> {
    // resolve one tty descriptor and validate requested process-group identifier
    let descriptor = tty_descriptor(binding, handle, "destack.tty.termios.setProcessGroup")?;
    let process_group_id = process_group_id_to_host(processgroupid)?;

    // apply one foreground process-group update
    let status = unsafe { libc::tcsetpgrp(descriptor, process_group_id) };
    if status < 0 {
        return Err(process_error(
            "destack.tty.termios.setProcessGroup",
            "tcsetpgrp",
            "failed to set tty foreground process group",
        ));
    }

    Ok(())
}
