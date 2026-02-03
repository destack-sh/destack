use crate::diagnostic::{RuntimeErrorId, RuntimeResult};
use crate::platform::error::PlatformErrorVm;
use crate::platform::error::core::{VmStringStore, platform_error_fields, take_platform_error};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Take a runtime platform error by id.
pub(super) fn destack_error_take_platform_error(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    errorid: u64,
) -> RuntimeResult<PlatformErrorVm> {
    let error =
        take_platform_error(&runtime.runtime().errors, RuntimeErrorId::from_raw(errorid));
    let mut store = VmStringStore::new(context);
    let fields = platform_error_fields(&mut store, &error);

    Ok(PlatformErrorVm {
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
    })
}
