use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeErrorId, RuntimeErrorStore};
use crate::platform::diagnostic::{
    PlatformError as DiagnosticPlatformError, PlatformErrorCode as DiagnosticPlatformErrorCode,
    PlatformErrorContext as DiagnosticPlatformErrorContext,
    PlatformErrorContextKind as DiagnosticPlatformErrorContextKind,
    PlatformPathEncoding as DiagnosticPlatformPathEncoding,
    PlatformPathPayload as DiagnosticPlatformPathPayload,
    PlatformSystemSource as DiagnosticPlatformSystemSource,
    PlatformSystemSourceKind as DiagnosticPlatformSystemSourceKind,
};
use crate::platform::error::{
    PlatformError, PlatformErrorCode, PlatformErrorContext, PlatformErrorContextKind,
    PlatformErrorContextVm, PlatformErrorVm, PlatformPathEncoding, PlatformPathPayload,
    PlatformPathPayloadVm, PlatformSystemSource, PlatformSystemSourceKind, PlatformSystemSourceVm,
};
use crate::platform::{NativeArray, NativeStringRef, VmArray};
use crate::runtime::BindingCallContext;

/// Empty byte array sentinel for absent path payloads in native ABI values.
const EMPTY_NATIVE_BYTE_ARRAY: NativeArray<u8> = NativeArray {
    data: std::ptr::null_mut(),
    len: 0,
    capacity: 0,
};

/// String storage adapter for native runtime calls.
#[derive(Debug)]
pub struct NativeStringStore<'a> {
    /// Runtime call context for the current binding call.
    context: &'a BindingCallContext,
}

impl<'a> NativeStringStore<'a> {
    /// Create a native string store for the runtime context.
    pub fn new(context: &'a BindingCallContext) -> Self {
        Self { context }
    }

    /// Store an optional string.
    fn optional(&self, value: Option<&String>) -> NativeStringRef {
        self.context.store_string_option(value)
    }

    /// Store a byte array.
    fn bytes(&self, data: &[u8]) -> NativeArray<u8> {
        self.context.store_array(data.to_vec())
    }
}

/// String storage adapter for VM runtime calls.
#[derive(Debug)]
pub struct VmStringStore<'a, 'ctx> {
    /// Runtime context for the current VM call.
    context: &'a mut vm::ExternalCallContext<'ctx>,
}

impl<'a, 'ctx> VmStringStore<'a, 'ctx> {
    /// Create a VM string store for the runtime context.
    pub fn new(context: &'a mut vm::ExternalCallContext<'ctx>) -> Self {
        Self { context }
    }

    /// Return the VM sentinel for an absent string.
    fn none_sentinel() -> vm::StringHandle {
        vm::StringHandle::new(vm::Value::VOID)
    }

    /// Store a required string.
    fn required(&mut self, value: &str) -> vm::StringHandle {
        vm::StringHandle::new(self.context.intern_string(value))
    }

    /// Store an optional string.
    fn optional(&mut self, value: Option<&String>) -> vm::StringHandle {
        match value {
            Some(value) => self.required(value),
            None => Self::none_sentinel(),
        }
    }

    /// Store a byte array.
    fn bytes(&mut self, data: &[u8]) -> VmArray<u8> {
        VmArray::from_bytes(self.context, data)
    }
}

/// Take a runtime error by id and normalize it as a platform error.
pub fn take_platform_error(
    errors: &RuntimeErrorStore,
    error_id: RuntimeErrorId,
) -> DiagnosticPlatformError {
    let error = errors.take(error_id).unwrap_or_else(|| {
        RuntimeError::Internal {
            message: "missing runtime error for id".to_string(),
        }
        .boxed()
    });

    DiagnosticPlatformError::from(error.as_ref())
}

/// Convert a diagnostic platform error into a native ABI value.
pub fn platform_error_native(
    store: &NativeStringStore<'_>,
    error: &DiagnosticPlatformError,
) -> PlatformError {
    // map the top-level fields
    let code = map_platform_error_code(error.code);
    let op = store.optional(error.op.as_ref());
    let source = platform_source_native(store, error.source.as_ref());
    let context = platform_context_native(store, error.context.as_ref());
    let message = store.optional(error.message.as_ref());

    PlatformError {
        code,
        op,
        source,
        context,
        message,
    }
}

/// Convert a diagnostic platform error into a VM ABI value.
pub fn platform_error_vm(
    store: &mut VmStringStore<'_, '_>,
    error: &DiagnosticPlatformError,
) -> PlatformErrorVm {
    // map the top-level fields
    let code = map_platform_error_code(error.code);
    let op = store.optional(error.op.as_ref());
    let source = platform_source_vm(store, error.source.as_ref());
    let context = platform_context_vm(store, error.context.as_ref());
    let message = store.optional(error.message.as_ref());

    PlatformErrorVm {
        code,
        op,
        source,
        context,
        message,
    }
}

/// Convert an optional source object into a native ABI source.
fn platform_source_native(
    store: &NativeStringStore<'_>,
    source: Option<&DiagnosticPlatformSystemSource>,
) -> PlatformSystemSource {
    // map present source fields
    if let Some(source) = source {
        let kind = map_platform_source_kind(source.kind);
        let value = source.value;
        let name = store.optional(source.name.as_ref());

        return PlatformSystemSource { kind, value, name };
    }

    // use an explicit empty sentinel source when absent
    PlatformSystemSource {
        kind: PlatformSystemSourceKind::Other,
        value: 0,
        name: store.optional(None),
    }
}

/// Convert an optional source object into a VM ABI source.
fn platform_source_vm(
    store: &mut VmStringStore<'_, '_>,
    source: Option<&DiagnosticPlatformSystemSource>,
) -> PlatformSystemSourceVm {
    // map present source fields
    if let Some(source) = source {
        let kind = map_platform_source_kind(source.kind);
        let value = source.value;
        let name = store.optional(source.name.as_ref());

        return PlatformSystemSourceVm { kind, value, name };
    }

    // use an explicit empty sentinel source when absent
    PlatformSystemSourceVm {
        kind: PlatformSystemSourceKind::Other,
        value: 0,
        name: store.optional(None),
    }
}

/// Convert an optional context object into a native ABI context.
fn platform_context_native(
    store: &NativeStringStore<'_>,
    context: Option<&DiagnosticPlatformErrorContext>,
) -> PlatformErrorContext {
    // map present context fields
    if let Some(context) = context {
        return PlatformErrorContext {
            kind: map_platform_context_kind(context.kind),
            syscall: store.optional(context.syscall.as_ref()),
            path: platform_path_native(store, context.path.as_ref()),
            dest: platform_path_native(store, context.dest.as_ref()),
            path_text: store.optional(context.path_text.as_ref()),
            dest_text: store.optional(context.dest_text.as_ref()),
            fd: context.fd.unwrap_or(0),
            address: store.optional(context.address.as_ref()),
            port: context.port.unwrap_or(0),
            hostname: store.optional(context.hostname.as_ref()),
            pid: context.pid.unwrap_or(0),
            signal: store.optional(context.signal.as_ref()),
            exit_code: context.exit_code.unwrap_or(0),
            timer_id: context.timer_id.unwrap_or(0),
            deadline_ns: context.deadline_ns.unwrap_or(0),
            resource_id: context.resource_id.unwrap_or(0),
            resource_kind: store.optional(context.resource_kind.as_ref()),
            capability: store.optional(context.capability.as_ref()),
            policy: store.optional(context.policy.as_ref()),
            library: store.optional(context.library.as_ref()),
            symbol: store.optional(context.symbol.as_ref()),
            thread_id: context.thread_id.unwrap_or(0),
            argument: store.optional(context.argument.as_ref()),
            pointer: store.optional(context.pointer.as_ref()),
            feature: store.optional(context.feature.as_ref()),
        };
    }

    // use an explicit empty sentinel context when absent
    PlatformErrorContext {
        kind: PlatformErrorContextKind::Generic,
        syscall: store.optional(None),
        path: platform_path_native(store, None),
        dest: platform_path_native(store, None),
        path_text: store.optional(None),
        dest_text: store.optional(None),
        fd: 0,
        address: store.optional(None),
        port: 0,
        hostname: store.optional(None),
        pid: 0,
        signal: store.optional(None),
        exit_code: 0,
        timer_id: 0,
        deadline_ns: 0,
        resource_id: 0,
        resource_kind: store.optional(None),
        capability: store.optional(None),
        policy: store.optional(None),
        library: store.optional(None),
        symbol: store.optional(None),
        thread_id: 0,
        argument: store.optional(None),
        pointer: store.optional(None),
        feature: store.optional(None),
    }
}

/// Convert an optional context object into a VM ABI context.
fn platform_context_vm(
    store: &mut VmStringStore<'_, '_>,
    context: Option<&DiagnosticPlatformErrorContext>,
) -> PlatformErrorContextVm {
    // map present context fields
    if let Some(context) = context {
        return PlatformErrorContextVm {
            kind: map_platform_context_kind(context.kind),
            syscall: store.optional(context.syscall.as_ref()),
            path: platform_path_vm(store, context.path.as_ref()),
            dest: platform_path_vm(store, context.dest.as_ref()),
            path_text: store.optional(context.path_text.as_ref()),
            dest_text: store.optional(context.dest_text.as_ref()),
            fd: context.fd.unwrap_or(0),
            address: store.optional(context.address.as_ref()),
            port: context.port.unwrap_or(0),
            hostname: store.optional(context.hostname.as_ref()),
            pid: context.pid.unwrap_or(0),
            signal: store.optional(context.signal.as_ref()),
            exit_code: context.exit_code.unwrap_or(0),
            timer_id: context.timer_id.unwrap_or(0),
            deadline_ns: context.deadline_ns.unwrap_or(0),
            resource_id: context.resource_id.unwrap_or(0),
            resource_kind: store.optional(context.resource_kind.as_ref()),
            capability: store.optional(context.capability.as_ref()),
            policy: store.optional(context.policy.as_ref()),
            library: store.optional(context.library.as_ref()),
            symbol: store.optional(context.symbol.as_ref()),
            thread_id: context.thread_id.unwrap_or(0),
            argument: store.optional(context.argument.as_ref()),
            pointer: store.optional(context.pointer.as_ref()),
            feature: store.optional(context.feature.as_ref()),
        };
    }

    // use an explicit empty sentinel context when absent
    PlatformErrorContextVm {
        kind: PlatformErrorContextKind::Generic,
        syscall: store.optional(None),
        path: platform_path_vm(store, None),
        dest: platform_path_vm(store, None),
        path_text: store.optional(None),
        dest_text: store.optional(None),
        fd: 0,
        address: store.optional(None),
        port: 0,
        hostname: store.optional(None),
        pid: 0,
        signal: store.optional(None),
        exit_code: 0,
        timer_id: 0,
        deadline_ns: 0,
        resource_id: 0,
        resource_kind: store.optional(None),
        capability: store.optional(None),
        policy: store.optional(None),
        library: store.optional(None),
        symbol: store.optional(None),
        thread_id: 0,
        argument: store.optional(None),
        pointer: store.optional(None),
        feature: store.optional(None),
    }
}

/// Convert an optional path payload into a native ABI payload.
fn platform_path_native(
    store: &NativeStringStore<'_>,
    payload: Option<&DiagnosticPlatformPathPayload>,
) -> PlatformPathPayload {
    // map present payload fields
    if let Some(payload) = payload {
        return PlatformPathPayload {
            encoding: map_platform_path_encoding(payload.encoding),
            data: store.bytes(&payload.data),
        };
    }

    // use an explicit empty path sentinel when absent
    PlatformPathPayload {
        encoding: PlatformPathEncoding::Bytes,
        data: EMPTY_NATIVE_BYTE_ARRAY,
    }
}

/// Convert an optional path payload into a VM ABI payload.
fn platform_path_vm(
    store: &mut VmStringStore<'_, '_>,
    payload: Option<&DiagnosticPlatformPathPayload>,
) -> PlatformPathPayloadVm {
    // map present payload fields
    if let Some(payload) = payload {
        return PlatformPathPayloadVm {
            encoding: map_platform_path_encoding(payload.encoding),
            data: store.bytes(&payload.data),
        };
    }

    // use an explicit empty path sentinel when absent
    PlatformPathPayloadVm {
        encoding: PlatformPathEncoding::Bytes,
        data: store.bytes(&[]),
    }
}

/// Map diagnostic source kind values into ABI source kind values.
fn map_platform_source_kind(kind: DiagnosticPlatformSystemSourceKind) -> PlatformSystemSourceKind {
    match kind {
        DiagnosticPlatformSystemSourceKind::Errno => PlatformSystemSourceKind::Errno,
        DiagnosticPlatformSystemSourceKind::Winsock => PlatformSystemSourceKind::Winsock,
        DiagnosticPlatformSystemSourceKind::HResult => PlatformSystemSourceKind::HResult,
        DiagnosticPlatformSystemSourceKind::Eai => PlatformSystemSourceKind::Eai,
        DiagnosticPlatformSystemSourceKind::Signal => PlatformSystemSourceKind::Signal,
        DiagnosticPlatformSystemSourceKind::Other => PlatformSystemSourceKind::Other,
    }
}

/// Map diagnostic context kind values into ABI context kind values.
fn map_platform_context_kind(kind: DiagnosticPlatformErrorContextKind) -> PlatformErrorContextKind {
    match kind {
        DiagnosticPlatformErrorContextKind::Generic => PlatformErrorContextKind::Generic,
        DiagnosticPlatformErrorContextKind::Io => PlatformErrorContextKind::Io,
        DiagnosticPlatformErrorContextKind::Net => PlatformErrorContextKind::Net,
        DiagnosticPlatformErrorContextKind::Process => PlatformErrorContextKind::Process,
        DiagnosticPlatformErrorContextKind::Timer => PlatformErrorContextKind::Timer,
        DiagnosticPlatformErrorContextKind::Resource => PlatformErrorContextKind::Resource,
        DiagnosticPlatformErrorContextKind::Security => PlatformErrorContextKind::Security,
        DiagnosticPlatformErrorContextKind::Ffi => PlatformErrorContextKind::Ffi,
        DiagnosticPlatformErrorContextKind::Thread => PlatformErrorContextKind::Thread,
        DiagnosticPlatformErrorContextKind::Ipc => PlatformErrorContextKind::Ipc,
        DiagnosticPlatformErrorContextKind::Device => PlatformErrorContextKind::Device,
        DiagnosticPlatformErrorContextKind::Display => PlatformErrorContextKind::Display,
        DiagnosticPlatformErrorContextKind::Audio => PlatformErrorContextKind::Audio,
        DiagnosticPlatformErrorContextKind::Gpu => PlatformErrorContextKind::Gpu,
        DiagnosticPlatformErrorContextKind::IoDriver => PlatformErrorContextKind::IoDriver,
    }
}

/// Map diagnostic path encoding values into ABI path encoding values.
fn map_platform_path_encoding(encoding: DiagnosticPlatformPathEncoding) -> PlatformPathEncoding {
    match encoding {
        DiagnosticPlatformPathEncoding::Bytes => PlatformPathEncoding::Bytes,
        DiagnosticPlatformPathEncoding::Utf16 => PlatformPathEncoding::Utf16,
    }
}

/// Map diagnostic error codes into ABI platform error codes.
fn map_platform_error_code(code: DiagnosticPlatformErrorCode) -> PlatformErrorCode {
    // map numeric discriminants across parallel enums generated from one source
    //
    // safety: both enums are repr(u16) and derive from `platform/error/error.ds`
    unsafe { std::mem::transmute::<DiagnosticPlatformErrorCode, PlatformErrorCode>(code) }
}

#[cfg(test)]
mod tests {
    use super::VmStringStore;

    #[test]
    fn test_vm_string_none_sentinel_is_void() {
        let sentinel = VmStringStore::none_sentinel();
        assert!(sentinel.value().is_void_value());
    }
}
