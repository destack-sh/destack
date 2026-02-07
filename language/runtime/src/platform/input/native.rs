#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::RuntimeError;
use crate::platform::bindings::native_call;
use crate::platform::input::bindings_generated as bindings;
use crate::platform::{
    PlatformError,
    NativeSlice,
    RuntimeStatus,
    NativeStringRef,
};

use crate::platform::{resource};
use crate::platform::input::{InputDeviceInfo, InputDeviceKind, InputEvent, InputEventKind};

/// Stub for destack.input.close.
#[unsafe(export_name = "destack.input.close")]
pub unsafe extern "C" fn destack_input_close(handle: resource::InputDeviceHandle) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(CLOSE)?;
        let _ = handle;

        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.input.close",
        ))
        .boxed())
    })
}

/// Stub for destack.input.list.
#[unsafe(export_name = "destack.input.list")]
pub unsafe extern "C" fn destack_input_list(out: *mut NativeSlice<InputDeviceInfo>) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(LIST)?;
        if out.is_null() {
            return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
        }
        let _ = out;

        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.input.list",
        ))
        .boxed())
    })
}

/// Stub for destack.input.open.
#[unsafe(export_name = "destack.input.open")]
pub unsafe extern "C" fn destack_input_open(out: *mut resource::InputDeviceHandle, id: NativeStringRef) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(OPEN)?;
        if out.is_null() {
            return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
        }
        let _ = (out, id);

        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.input.open",
        ))
        .boxed())
    })
}

/// Stub for destack.input.read.
#[unsafe(export_name = "destack.input.read")]
pub unsafe extern "C" fn destack_input_read(out: *mut InputEvent, handle: resource::InputDeviceHandle) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(READ)?;
        if out.is_null() {
            return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
        }
        let _ = (out, handle);

        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.input.read",
        ))
        .boxed())
    })
}

/// Stub for destack.input.setGrab.
#[unsafe(export_name = "destack.input.setGrab")]
pub unsafe extern "C" fn destack_input_set_grab(handle: resource::InputDeviceHandle, enable: bool) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(SET_GRAB)?;
        let _ = (handle, enable);

        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.input.setGrab",
        ))
        .boxed())
    })
}

/// Stub for destack.input.tryRead.
#[unsafe(export_name = "destack.input.tryRead")]
pub unsafe extern "C" fn destack_input_try_read(out: *mut InputEvent, handle: resource::InputDeviceHandle) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(TRY_READ)?;
        if out.is_null() {
            return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
        }
        let _ = (out, handle);

        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.input.tryRead",
        ))
        .boxed())
    })
}

