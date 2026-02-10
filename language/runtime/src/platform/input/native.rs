#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::bindings_generated as bindings;
use crate::platform::{NativeSlice, NativeStringRef, PlatformError};

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::input::{InputDeviceInfo, InputEvent};
use crate::platform::resource;

/// Stub for destack.input.device.close.
pub unsafe fn destack_input_close(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    context.check_policy(INPUT_DEVICE_CLOSE)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.device.close")).boxed())
}

/// Stub for destack.input.device.list.
pub unsafe fn destack_input_list(
    context: &RuntimeCallContext,
    out: *mut NativeSlice<InputDeviceInfo>,
) -> RuntimeResult<()> {
    context.check_policy(INPUT_DEVICE_LIST)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.device.list")).boxed())
}

/// Stub for destack.input.device.open.
pub unsafe fn destack_input_open(
    context: &RuntimeCallContext,
    out: *mut resource::InputDeviceHandle,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    context.check_policy(INPUT_DEVICE_OPEN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, id);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.device.open")).boxed())
}

/// Stub for destack.input.event.read.
pub unsafe fn destack_input_read(
    context: &RuntimeCallContext,
    out: *mut InputEvent,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    context.check_policy(INPUT_EVENT_READ)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.event.read")).boxed())
}

/// Stub for destack.input.event.setGrab.
pub unsafe fn destack_input_set_grab(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    enable: bool,
) -> RuntimeResult<()> {
    context.check_policy(INPUT_EVENT_SET_GRAB)?;
    let _ = (handle, enable);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.event.setGrab")).boxed())
}

/// Stub for destack.input.event.tryRead.
pub unsafe fn destack_input_try_read(
    context: &RuntimeCallContext,
    out: *mut InputEvent,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    context.check_policy(INPUT_EVENT_TRY_READ)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.input.event.tryRead")).boxed())
}
