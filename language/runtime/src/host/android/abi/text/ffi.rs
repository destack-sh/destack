use super::callbacks::call_android_text_callback;
use crate::host::abi::text::{HostTextGeometryRequest, HostTextOpenRequest, HostTextStateRequest};

/// Open one Android text session.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_text_open(
    runtime_id: u64,
    request: HostTextOpenRequest,
) -> u32 {
    call_android_text_callback(
        runtime_id,
        |callbacks| callbacks.open,
        |callback| unsafe { callback(runtime_id, request) },
    )
}

/// Close one Android text session.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_text_close(
    runtime_id: u64,
    session_id: u64,
) -> u32 {
    call_android_text_callback(
        runtime_id,
        |callbacks| callbacks.close,
        |callback| unsafe { callback(runtime_id, session_id) },
    )
}

/// Update one Android text geometry payload.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_text_set_geometry(
    runtime_id: u64,
    request: HostTextGeometryRequest,
) -> u32 {
    call_android_text_callback(
        runtime_id,
        |callbacks| callbacks.set_geometry,
        |callback| unsafe { callback(runtime_id, request) },
    )
}

/// Update one Android text state.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_text_set_state(
    runtime_id: u64,
    request: HostTextStateRequest,
) -> u32 {
    call_android_text_callback(
        runtime_id,
        |callbacks| callbacks.set_state,
        |callback| unsafe { callback(runtime_id, request) },
    )
}
