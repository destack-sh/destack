use crate::diagnostic::{RuntimeErrorId, RuntimeResult};
use crate::platform::error::PlatformError;
use crate::platform::error::core::{NativeStringStore, platform_error_native, take_platform_error};
use crate::runtime::RuntimeCallContext;

/// Take a runtime platform error by id.
pub unsafe fn destack_error_take_platform_error(
    context: &RuntimeCallContext,
    out: *mut PlatformError,
    error_id: u64,
) -> RuntimeResult<()> {
    let error = take_platform_error(
        &context.runtime().errors,
        RuntimeErrorId::from_raw(error_id),
    );
    let store = NativeStringStore::new(context);
    let platform_error = platform_error_native(&store, &error);

    unsafe {
        out.write(platform_error);
    }

    Ok(())
}
