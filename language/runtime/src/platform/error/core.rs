use destack_vm;

use crate::diagnostic::{DiagnosticId, DiagnosticStore, RuntimeError, RuntimeResult};
use crate::platform::abi::NativeStringRef;
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
    PlatformError, PlatformErrorCode, PlatformErrorContext, PlatformErrorContextAudio,
    PlatformErrorContextAudioVm, PlatformErrorContextDevice, PlatformErrorContextDeviceVm,
    PlatformErrorContextDisplay, PlatformErrorContextDisplayVm, PlatformErrorContextFfi,
    PlatformErrorContextFfiVm, PlatformErrorContextGeneric, PlatformErrorContextGenericVm,
    PlatformErrorContextGpu, PlatformErrorContextGpuVm, PlatformErrorContextIo,
    PlatformErrorContextIoDriver, PlatformErrorContextIoDriverVm, PlatformErrorContextIoVm,
    PlatformErrorContextIpc, PlatformErrorContextIpcVm, PlatformErrorContextNet,
    PlatformErrorContextNetVm, PlatformErrorContextProcess, PlatformErrorContextProcessVm,
    PlatformErrorContextResource, PlatformErrorContextResourceVm, PlatformErrorContextSecurity,
    PlatformErrorContextSecurityVm, PlatformErrorContextThread, PlatformErrorContextThreadVm,
    PlatformErrorContextTimer, PlatformErrorContextTimerVm, PlatformErrorContextVm,
    PlatformErrorVm, PlatformPathPayload, PlatformPathPayloadBytes, PlatformPathPayloadBytesVm,
    PlatformPathPayloadUtf16, PlatformPathPayloadUtf16Vm, PlatformPathPayloadVm,
    PlatformSystemSource, PlatformSystemSourceEai, PlatformSystemSourceEaiVm,
    PlatformSystemSourceErrno, PlatformSystemSourceErrnoVm, PlatformSystemSourceHResult,
    PlatformSystemSourceHResultVm, PlatformSystemSourceOther, PlatformSystemSourceOtherVm,
    PlatformSystemSourceSignal, PlatformSystemSourceSignalVm, PlatformSystemSourceVm,
    PlatformSystemSourceWinsock, PlatformSystemSourceWinsockVm,
};
use crate::platform::{NativeArray, VmArray};
use crate::runtime::BindingCallContext;

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

    /// Store a required string.
    fn required(&self, value: &str) -> NativeStringRef {
        self.context.store_string(value)
    }

    /// Store an optional string.
    fn optional(&self, value: Option<&String>) -> Option<NativeStringRef> {
        value.map(|value| self.required(value))
    }

    /// Store a byte array.
    fn bytes(&self, data: &[u8]) -> NativeArray<u8> {
        self.context.store_array_copy(data)
    }

    /// Store a utf16 unit array.
    fn utf16(&self, data: &[u16]) -> NativeArray<u16> {
        self.context.store_array_copy(data)
    }
}

/// String storage adapter for VM runtime calls.
#[derive(Debug)]
pub struct VmStringStore<'a, 'ctx> {
    /// Runtime context for the current VM call.
    context: &'a mut destack_vm::BindingContext<'ctx>,
}

impl<'a, 'ctx> VmStringStore<'a, 'ctx> {
    /// Create a VM string store for the runtime context.
    pub fn new(context: &'a mut destack_vm::BindingContext<'ctx>) -> Self {
        Self { context }
    }

    /// Store a required string.
    fn required(&mut self, value: &str) -> RuntimeResult<destack_vm::StringHandle> {
        self.context
            .string_handle(value)
            .map_err(Box::<RuntimeError>::from)
    }

    /// Store an optional string.
    fn optional(
        &mut self,
        value: Option<&String>,
    ) -> RuntimeResult<Option<destack_vm::StringHandle>> {
        value.map(|value| self.required(value)).transpose()
    }

    /// Store a byte array.
    fn bytes(&mut self, data: &[u8]) -> RuntimeResult<VmArray<u8>> {
        let mut context = self.context.write();

        VmArray::from_bytes(&mut context, data)
    }

    /// Store a utf16 unit array.
    fn utf16(&mut self, data: &[u16]) -> RuntimeResult<VmArray<u16>> {
        let mut context = self.context.write();

        VmArray::from_values(&mut context, data)
    }
}

/// Take a runtime error by id and normalize it as a platform error.
pub fn take_platform_error(
    diagnostics: &DiagnosticStore,
    error_id: DiagnosticId,
) -> DiagnosticPlatformError {
    let error = diagnostics.take_error(error_id).unwrap_or_else(|| {
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
) -> RuntimeResult<PlatformErrorVm> {
    let code = map_platform_error_code(error.code);
    let op = store.optional(error.op.as_ref())?;
    let source = platform_source_vm(store, error.source.as_ref())?;
    let context = platform_context_vm(store, error.context.as_ref())?;
    let message = store.optional(error.message.as_ref())?;

    Ok(PlatformErrorVm {
        code,
        op,
        source,
        context,
        message,
    })
}

/// Convert a diagnostic path payload into utf16 units.
fn payload_utf16_units(payload: &DiagnosticPlatformPathPayload) -> Vec<u16> {
    // utf16 payloads must always contain complete code units
    if !payload.data.len().is_multiple_of(2) {
        panic!(
            "internal platform error payload invariant violated: utf16 payload has odd byte length {}",
            payload.data.len()
        );
    }

    let mut units = Vec::with_capacity(payload.data.len() / 2 + 1);
    for chunk in payload.data.chunks(2) {
        let low = chunk[0];
        let high = chunk[1];
        units.push(u16::from_le_bytes([low, high]));
    }
    units
}

/// Convert an optional path payload into a native ABI payload.
fn platform_path_native(
    store: &NativeStringStore<'_>,
    payload: Option<&DiagnosticPlatformPathPayload>,
) -> Option<PlatformPathPayload> {
    let payload = payload?;

    match payload.encoding {
        DiagnosticPlatformPathEncoding::Bytes => Some(
            PlatformPathPayload::PlatformPathPayloadBytes(PlatformPathPayloadBytes {
                kind: store.required("bytes"),
                bytes: store.bytes(&payload.data),
            }),
        ),
        DiagnosticPlatformPathEncoding::Utf16 => {
            let utf16 = payload_utf16_units(payload);
            Some(PlatformPathPayload::PlatformPathPayloadUtf16(
                PlatformPathPayloadUtf16 {
                    kind: store.required("utf16"),
                    utf16: store.utf16(&utf16),
                },
            ))
        }
    }
}

/// Convert an optional path payload into a VM ABI payload.
fn platform_path_vm(
    store: &mut VmStringStore<'_, '_>,
    payload: Option<&DiagnosticPlatformPathPayload>,
) -> RuntimeResult<Option<PlatformPathPayloadVm>> {
    let Some(payload) = payload else {
        return Ok(None);
    };

    match payload.encoding {
        DiagnosticPlatformPathEncoding::Bytes => Ok(Some(
            PlatformPathPayloadVm::PlatformPathPayloadBytes(PlatformPathPayloadBytesVm {
                kind: store.required("bytes")?,
                bytes: store.bytes(&payload.data)?,
            }),
        )),
        DiagnosticPlatformPathEncoding::Utf16 => {
            let utf16 = payload_utf16_units(payload);
            Ok(Some(PlatformPathPayloadVm::PlatformPathPayloadUtf16(
                PlatformPathPayloadUtf16Vm {
                    kind: store.required("utf16")?,
                    utf16: store.utf16(&utf16)?,
                },
            )))
        }
    }
}

/// Convert an optional source object into a native ABI source.
fn platform_source_native(
    store: &NativeStringStore<'_>,
    source: Option<&DiagnosticPlatformSystemSource>,
) -> Option<PlatformSystemSource> {
    let source = source?;
    let kind = source.kind;
    let value = source.value;
    let name = source.name.as_ref();
    let name = store.optional(name);

    match kind {
        DiagnosticPlatformSystemSourceKind::Errno => Some(
            PlatformSystemSource::PlatformSystemSourceErrno(PlatformSystemSourceErrno {
                kind: store.required("errno"),
                value,
                name,
            }),
        ),
        DiagnosticPlatformSystemSourceKind::Winsock => Some(
            PlatformSystemSource::PlatformSystemSourceWinsock(PlatformSystemSourceWinsock {
                kind: store.required("winsock"),
                value,
                name,
            }),
        ),
        DiagnosticPlatformSystemSourceKind::HResult => Some(
            PlatformSystemSource::PlatformSystemSourceHResult(PlatformSystemSourceHResult {
                kind: store.required("hresult"),
                value,
                name,
            }),
        ),
        DiagnosticPlatformSystemSourceKind::Eai => Some(
            PlatformSystemSource::PlatformSystemSourceEai(PlatformSystemSourceEai {
                kind: store.required("eai"),
                value,
                name,
            }),
        ),
        DiagnosticPlatformSystemSourceKind::Signal => Some(
            PlatformSystemSource::PlatformSystemSourceSignal(PlatformSystemSourceSignal {
                kind: store.required("signal"),
                value,
                name,
            }),
        ),
        DiagnosticPlatformSystemSourceKind::Other => Some(
            PlatformSystemSource::PlatformSystemSourceOther(PlatformSystemSourceOther {
                kind: store.required("other"),
                value,
                name,
            }),
        ),
    }
}

/// Convert an optional source object into a VM ABI source.
fn platform_source_vm(
    store: &mut VmStringStore<'_, '_>,
    source: Option<&DiagnosticPlatformSystemSource>,
) -> RuntimeResult<Option<PlatformSystemSourceVm>> {
    let Some(source) = source else {
        return Ok(None);
    };

    let kind = source.kind;
    let value = source.value;
    let name = source.name.as_ref();
    let name = store.optional(name)?;

    let source = match kind {
        DiagnosticPlatformSystemSourceKind::Errno => {
            PlatformSystemSourceVm::PlatformSystemSourceErrno(PlatformSystemSourceErrnoVm {
                kind: store.required("errno")?,
                value,
                name,
            })
        }
        DiagnosticPlatformSystemSourceKind::Winsock => {
            PlatformSystemSourceVm::PlatformSystemSourceWinsock(PlatformSystemSourceWinsockVm {
                kind: store.required("winsock")?,
                value,
                name,
            })
        }
        DiagnosticPlatformSystemSourceKind::HResult => {
            PlatformSystemSourceVm::PlatformSystemSourceHResult(PlatformSystemSourceHResultVm {
                kind: store.required("hresult")?,
                value,
                name,
            })
        }
        DiagnosticPlatformSystemSourceKind::Eai => {
            PlatformSystemSourceVm::PlatformSystemSourceEai(PlatformSystemSourceEaiVm {
                kind: store.required("eai")?,
                value,
                name,
            })
        }
        DiagnosticPlatformSystemSourceKind::Signal => {
            PlatformSystemSourceVm::PlatformSystemSourceSignal(PlatformSystemSourceSignalVm {
                kind: store.required("signal")?,
                value,
                name,
            })
        }
        DiagnosticPlatformSystemSourceKind::Other => {
            PlatformSystemSourceVm::PlatformSystemSourceOther(PlatformSystemSourceOtherVm {
                kind: store.required("other")?,
                value,
                name,
            })
        }
    };

    Ok(Some(source))
}

/// Return one stable context kind label for invariant diagnostics.
fn platform_context_kind_label(kind: DiagnosticPlatformErrorContextKind) -> &'static str {
    match kind {
        DiagnosticPlatformErrorContextKind::Audio => "audio",
        DiagnosticPlatformErrorContextKind::Device => "device",
        DiagnosticPlatformErrorContextKind::Display => "display",
        DiagnosticPlatformErrorContextKind::Ffi => "ffi",
        DiagnosticPlatformErrorContextKind::Generic => "generic",
        DiagnosticPlatformErrorContextKind::Gpu => "gpu",
        DiagnosticPlatformErrorContextKind::Io => "io",
        DiagnosticPlatformErrorContextKind::IoDriver => "ioDriver",
        DiagnosticPlatformErrorContextKind::Ipc => "ipc",
        DiagnosticPlatformErrorContextKind::Net => "net",
        DiagnosticPlatformErrorContextKind::Process => "process",
        DiagnosticPlatformErrorContextKind::Resource => "resource",
        DiagnosticPlatformErrorContextKind::Security => "security",
        DiagnosticPlatformErrorContextKind::Thread => "thread",
        DiagnosticPlatformErrorContextKind::Timer => "timer",
    }
}

/// Require one numeric context field and fail loudly when missing.
fn required_context_numeric<T: Copy>(
    value: Option<T>,
    kind: DiagnosticPlatformErrorContextKind,
    field_name: &str,
) -> Option<T> {
    Some(value.unwrap_or_else(|| {
        let context_kind = platform_context_kind_label(kind);
        panic!(
            "internal platform error context invariant violated: missing {field_name} for {context_kind}"
        );
    }))
}

/// Convert an optional context object into a native ABI context.
fn platform_context_native(
    store: &NativeStringStore<'_>,
    context: Option<&DiagnosticPlatformErrorContext>,
) -> Option<PlatformErrorContext> {
    let context = context?;

    match context.kind {
        DiagnosticPlatformErrorContextKind::Audio => Some(
            PlatformErrorContext::PlatformErrorContextAudio(PlatformErrorContextAudio {
                kind: store.required("audio"),
                syscall: store.optional(context.syscall.as_ref()),
                feature: store.optional(context.feature.as_ref()),
            }),
        ),
        DiagnosticPlatformErrorContextKind::Device => Some(
            PlatformErrorContext::PlatformErrorContextDevice(PlatformErrorContextDevice {
                kind: store.required("device"),
                syscall: store.optional(context.syscall.as_ref()),
                path: platform_path_native(store, context.path.as_ref()),
                path_text: store.optional(context.path_text.as_ref()),
                feature: store.optional(context.feature.as_ref()),
            }),
        ),
        DiagnosticPlatformErrorContextKind::Display => Some(
            PlatformErrorContext::PlatformErrorContextDisplay(PlatformErrorContextDisplay {
                kind: store.required("display"),
                syscall: store.optional(context.syscall.as_ref()),
                feature: store.optional(context.feature.as_ref()),
            }),
        ),
        DiagnosticPlatformErrorContextKind::Ffi => Some(
            PlatformErrorContext::PlatformErrorContextFfi(PlatformErrorContextFfi {
                kind: store.required("ffi"),
                syscall: store.optional(context.syscall.as_ref()),
                library: store.optional(context.library.as_ref()),
                symbol: store.optional(context.symbol.as_ref()),
            }),
        ),
        DiagnosticPlatformErrorContextKind::Generic => Some(
            PlatformErrorContext::PlatformErrorContextGeneric(PlatformErrorContextGeneric {
                kind: store.required("generic"),
                syscall: store.optional(context.syscall.as_ref()),
                argument: store.optional(context.argument.as_ref()),
                pointer: store.optional(context.pointer.as_ref()),
                feature: store.optional(context.feature.as_ref()),
            }),
        ),
        DiagnosticPlatformErrorContextKind::Gpu => Some(
            PlatformErrorContext::PlatformErrorContextGpu(PlatformErrorContextGpu {
                kind: store.required("gpu"),
                syscall: store.optional(context.syscall.as_ref()),
                feature: store.optional(context.feature.as_ref()),
            }),
        ),
        DiagnosticPlatformErrorContextKind::Io => Some(
            PlatformErrorContext::PlatformErrorContextIo(PlatformErrorContextIo {
                kind: store.required("io"),
                syscall: store.optional(context.syscall.as_ref()),
                path: platform_path_native(store, context.path.as_ref()),
                dest: platform_path_native(store, context.dest.as_ref()),
                path_text: store.optional(context.path_text.as_ref()),
                dest_text: store.optional(context.dest_text.as_ref()),
                fd: required_context_numeric(context.fd, context.kind, "fd"),
            }),
        ),
        DiagnosticPlatformErrorContextKind::IoDriver => Some(
            PlatformErrorContext::PlatformErrorContextIoDriver(PlatformErrorContextIoDriver {
                kind: store.required("ioDriver"),
                syscall: store.optional(context.syscall.as_ref()),
                fd: required_context_numeric(context.fd, context.kind, "fd"),
                feature: store.optional(context.feature.as_ref()),
            }),
        ),
        DiagnosticPlatformErrorContextKind::Ipc => Some(
            PlatformErrorContext::PlatformErrorContextIpc(PlatformErrorContextIpc {
                kind: store.required("ipc"),
                syscall: store.optional(context.syscall.as_ref()),
                path: platform_path_native(store, context.path.as_ref()),
                path_text: store.optional(context.path_text.as_ref()),
                fd: required_context_numeric(context.fd, context.kind, "fd"),
            }),
        ),
        DiagnosticPlatformErrorContextKind::Net => Some(
            PlatformErrorContext::PlatformErrorContextNet(PlatformErrorContextNet {
                kind: store.required("net"),
                syscall: store.optional(context.syscall.as_ref()),
                address: store.optional(context.address.as_ref()),
                port: required_context_numeric(context.port, context.kind, "port"),
                hostname: store.optional(context.hostname.as_ref()),
            }),
        ),
        DiagnosticPlatformErrorContextKind::Process => Some(
            PlatformErrorContext::PlatformErrorContextProcess(PlatformErrorContextProcess {
                kind: store.required("process"),
                syscall: store.optional(context.syscall.as_ref()),
                pid: required_context_numeric(context.pid, context.kind, "pid"),
                signal: store.optional(context.signal.as_ref()),
                exit_code: required_context_numeric(context.exit_code, context.kind, "exitCode"),
            }),
        ),
        DiagnosticPlatformErrorContextKind::Resource => Some(
            PlatformErrorContext::PlatformErrorContextResource(PlatformErrorContextResource {
                kind: store.required("resource"),
                syscall: store.optional(context.syscall.as_ref()),
                resource_id: required_context_numeric(
                    context.resource_id,
                    context.kind,
                    "resourceId",
                ),
                resource_kind: store.optional(context.resource_kind.as_ref()),
            }),
        ),
        DiagnosticPlatformErrorContextKind::Security => Some(
            PlatformErrorContext::PlatformErrorContextSecurity(PlatformErrorContextSecurity {
                kind: store.required("security"),
                syscall: store.optional(context.syscall.as_ref()),
                capability: store.optional(context.capability.as_ref()),
                policy: store.optional(context.policy.as_ref()),
            }),
        ),
        DiagnosticPlatformErrorContextKind::Thread => Some(
            PlatformErrorContext::PlatformErrorContextThread(PlatformErrorContextThread {
                kind: store.required("thread"),
                syscall: store.optional(context.syscall.as_ref()),
                thread_id: required_context_numeric(context.thread_id, context.kind, "threadId"),
            }),
        ),
        DiagnosticPlatformErrorContextKind::Timer => Some(
            PlatformErrorContext::PlatformErrorContextTimer(PlatformErrorContextTimer {
                kind: store.required("timer"),
                syscall: store.optional(context.syscall.as_ref()),
                timer_id: required_context_numeric(context.timer_id, context.kind, "timerId"),
                deadline_ns: required_context_numeric(
                    context.deadline_ns,
                    context.kind,
                    "deadlineNs",
                ),
            }),
        ),
    }
}

/// Convert an optional context object into a VM ABI context.
fn platform_context_vm(
    store: &mut VmStringStore<'_, '_>,
    context: Option<&DiagnosticPlatformErrorContext>,
) -> RuntimeResult<Option<PlatformErrorContextVm>> {
    let Some(context) = context else {
        return Ok(None);
    };

    let value = match context.kind {
        DiagnosticPlatformErrorContextKind::Audio => {
            PlatformErrorContextVm::PlatformErrorContextAudio(PlatformErrorContextAudioVm {
                kind: store.required("audio")?,
                syscall: store.optional(context.syscall.as_ref())?,
                feature: store.optional(context.feature.as_ref())?,
            })
        }
        DiagnosticPlatformErrorContextKind::Device => {
            PlatformErrorContextVm::PlatformErrorContextDevice(PlatformErrorContextDeviceVm {
                kind: store.required("device")?,
                syscall: store.optional(context.syscall.as_ref())?,
                path: platform_path_vm(store, context.path.as_ref())?,
                path_text: store.optional(context.path_text.as_ref())?,
                feature: store.optional(context.feature.as_ref())?,
            })
        }
        DiagnosticPlatformErrorContextKind::Display => {
            PlatformErrorContextVm::PlatformErrorContextDisplay(PlatformErrorContextDisplayVm {
                kind: store.required("display")?,
                syscall: store.optional(context.syscall.as_ref())?,
                feature: store.optional(context.feature.as_ref())?,
            })
        }
        DiagnosticPlatformErrorContextKind::Ffi => {
            PlatformErrorContextVm::PlatformErrorContextFfi(PlatformErrorContextFfiVm {
                kind: store.required("ffi")?,
                syscall: store.optional(context.syscall.as_ref())?,
                library: store.optional(context.library.as_ref())?,
                symbol: store.optional(context.symbol.as_ref())?,
            })
        }
        DiagnosticPlatformErrorContextKind::Generic => {
            PlatformErrorContextVm::PlatformErrorContextGeneric(PlatformErrorContextGenericVm {
                kind: store.required("generic")?,
                syscall: store.optional(context.syscall.as_ref())?,
                argument: store.optional(context.argument.as_ref())?,
                pointer: store.optional(context.pointer.as_ref())?,
                feature: store.optional(context.feature.as_ref())?,
            })
        }
        DiagnosticPlatformErrorContextKind::Gpu => {
            PlatformErrorContextVm::PlatformErrorContextGpu(PlatformErrorContextGpuVm {
                kind: store.required("gpu")?,
                syscall: store.optional(context.syscall.as_ref())?,
                feature: store.optional(context.feature.as_ref())?,
            })
        }
        DiagnosticPlatformErrorContextKind::Io => {
            PlatformErrorContextVm::PlatformErrorContextIo(PlatformErrorContextIoVm {
                kind: store.required("io")?,
                syscall: store.optional(context.syscall.as_ref())?,
                path: platform_path_vm(store, context.path.as_ref())?,
                dest: platform_path_vm(store, context.dest.as_ref())?,
                path_text: store.optional(context.path_text.as_ref())?,
                dest_text: store.optional(context.dest_text.as_ref())?,
                fd: required_context_numeric(context.fd, context.kind, "fd"),
            })
        }
        DiagnosticPlatformErrorContextKind::IoDriver => {
            PlatformErrorContextVm::PlatformErrorContextIoDriver(PlatformErrorContextIoDriverVm {
                kind: store.required("ioDriver")?,
                syscall: store.optional(context.syscall.as_ref())?,
                fd: required_context_numeric(context.fd, context.kind, "fd"),
                feature: store.optional(context.feature.as_ref())?,
            })
        }
        DiagnosticPlatformErrorContextKind::Ipc => {
            PlatformErrorContextVm::PlatformErrorContextIpc(PlatformErrorContextIpcVm {
                kind: store.required("ipc")?,
                syscall: store.optional(context.syscall.as_ref())?,
                path: platform_path_vm(store, context.path.as_ref())?,
                path_text: store.optional(context.path_text.as_ref())?,
                fd: required_context_numeric(context.fd, context.kind, "fd"),
            })
        }
        DiagnosticPlatformErrorContextKind::Net => {
            PlatformErrorContextVm::PlatformErrorContextNet(PlatformErrorContextNetVm {
                kind: store.required("net")?,
                syscall: store.optional(context.syscall.as_ref())?,
                address: store.optional(context.address.as_ref())?,
                port: required_context_numeric(context.port, context.kind, "port"),
                hostname: store.optional(context.hostname.as_ref())?,
            })
        }
        DiagnosticPlatformErrorContextKind::Process => {
            PlatformErrorContextVm::PlatformErrorContextProcess(PlatformErrorContextProcessVm {
                kind: store.required("process")?,
                syscall: store.optional(context.syscall.as_ref())?,
                pid: required_context_numeric(context.pid, context.kind, "pid"),
                signal: store.optional(context.signal.as_ref())?,
                exit_code: required_context_numeric(context.exit_code, context.kind, "exitCode"),
            })
        }
        DiagnosticPlatformErrorContextKind::Resource => {
            PlatformErrorContextVm::PlatformErrorContextResource(PlatformErrorContextResourceVm {
                kind: store.required("resource")?,
                syscall: store.optional(context.syscall.as_ref())?,
                resource_id: required_context_numeric(
                    context.resource_id,
                    context.kind,
                    "resourceId",
                ),
                resource_kind: store.optional(context.resource_kind.as_ref())?,
            })
        }
        DiagnosticPlatformErrorContextKind::Security => {
            PlatformErrorContextVm::PlatformErrorContextSecurity(PlatformErrorContextSecurityVm {
                kind: store.required("security")?,
                syscall: store.optional(context.syscall.as_ref())?,
                capability: store.optional(context.capability.as_ref())?,
                policy: store.optional(context.policy.as_ref())?,
            })
        }
        DiagnosticPlatformErrorContextKind::Thread => {
            PlatformErrorContextVm::PlatformErrorContextThread(PlatformErrorContextThreadVm {
                kind: store.required("thread")?,
                syscall: store.optional(context.syscall.as_ref())?,
                thread_id: required_context_numeric(context.thread_id, context.kind, "threadId"),
            })
        }
        DiagnosticPlatformErrorContextKind::Timer => {
            PlatformErrorContextVm::PlatformErrorContextTimer(PlatformErrorContextTimerVm {
                kind: store.required("timer")?,
                syscall: store.optional(context.syscall.as_ref())?,
                timer_id: required_context_numeric(context.timer_id, context.kind, "timerId"),
                deadline_ns: required_context_numeric(
                    context.deadline_ns,
                    context.kind,
                    "deadlineNs",
                ),
            })
        }
    };

    Ok(Some(value))
}

/// Map diagnostic error codes into ABI platform error codes.
fn map_platform_error_code(code: DiagnosticPlatformErrorCode) -> PlatformErrorCode {
    // map numeric discriminants across parallel enums generated from one source
    //
    // safety: `code` comes from the parallel diagnostic enum derived from the same
    // source declaration, so its discriminant is valid for the generated ABI enum
    unsafe { std::mem::transmute::<i32, PlatformErrorCode>(code.number() as i32) }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_platform_context_kind_label_returns_expected_name() {
        let label =
            super::platform_context_kind_label(super::DiagnosticPlatformErrorContextKind::Io);

        assert_eq!(label, "io");
    }
}
