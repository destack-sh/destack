use crate::diagnostic::{RuntimeErrorId, RuntimeResult};
use crate::platform::error::PlatformError;
use crate::platform::error::core::{
    NativeStringStore, platform_error_fields, take_platform_error,
};
use crate::runtime::RuntimeCallContext;

/// Take a runtime platform error by id.
pub unsafe fn destack_error_take_platform_error(
    context: &RuntimeCallContext,
    out: *mut PlatformError,
    error_id: u64,
) -> RuntimeResult<()> {
    let error = take_platform_error(&context.runtime().errors, RuntimeErrorId::from_raw(error_id));
    let mut store = NativeStringStore::new(context);
    let fields = platform_error_fields(&mut store, &error);
    let platform_error = PlatformError {
        kind: fields.kind,
        message: fields.message,
        name: fields.name,
        code: fields.code,
        system_code: fields.system_code,
        errno: fields.errno,
        syscall: fields.syscall,
        path: fields.path,
        dest: fields.dest,
        fd: fields.fd,
        address: fields.address,
        port: fields.port,
        hostname: fields.hostname,
        signal: fields.signal,
        exit_code: fields.exit_code,
        cause: fields.cause,
        argument: fields.argument,
        pointer: fields.pointer,
        feature: fields.feature,
    };

    unsafe {
        out.write(platform_error);
    }

    Ok(())
}
