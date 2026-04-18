#![cfg_attr(windows, allow(dead_code, unused_imports))]

use destack_vm as vm;

#[cfg(windows)]
use core::ffi::c_void;

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, VmAbi};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::core as core_fs;
use crate::platform::io::{
    CompletionEvent, CompletionEventVm, CompletionOperation, CompletionOperationKind,
    DescriptorControlCommand, DescriptorControlFlags, DescriptorRequest, DescriptorRequestVm,
    EventToken, PollBackend, PollEvent, PollEventVm, PollInterest, UringParameters,
    UringParametersVm,
};
#[cfg(target_os = "linux")]
use crate::platform::resource::UringHandle;
use crate::platform::resource::{
    CompletionHandle, DeviceHandle, PollHandle, ResourceEntry, ResourceId, ResourceKind,
};
use crate::platform::{PlatformError, VmArray, VmSlice, fs};
use crate::runtime::BindingCallContext;
use crate::tests::platform::error_code_from_runtime_error;
pub(crate) use crate::tests::platform::{
    assert_not_supported_result, assert_platform_error_code, is_not_supported_code,
};
use crate::tests::runtime::TestRuntime;

#[path = "harness.generated.rs"]
mod harness;

/// Monotonic counter used for unique temporary path names.
static UNIQUE_TEMP_PATH_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Test harness context used by tests.
pub(crate) struct IoHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call BindingCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

impl<'call> IoHarnessContext<'call> {
    /// Return one mutable vm context when this harness runs in vm mode.
    #[allow(clippy::mut_from_ref)]
    fn vm_context_mut(&self) -> Option<&mut vm::ExternalCallContext<'_>> {
        self.vm_context
            .map(|context| unsafe { &mut *(context as *mut vm::ExternalCallContext<'_>) })
    }

    /// Wait for poll events and normalize native and vm arrays into one vector.
    pub(crate) fn poll_wait_events(
        &mut self,
        handle: PollHandle,
        timeoutns: u64,
        maxevents: u32,
    ) -> RuntimeResult<Vec<PollEvent>> {
        let value = self.destack_io_poll_wait(handle, timeoutns, maxevents)?;
        match value {
            harness::HarnessValue::Native(value) => {
                let events = unsafe { value.as_slice()? };
                Ok(events.to_vec())
            }
            harness::HarnessValue::Vm(value) => {
                let vm_context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "missing vm context for vm poll events",
                    ))
                    .boxed()
                })?;
                decode_vm_poll_events(vm_context, value)
            }
        }
    }

    /// Wait for completion events and normalize native and vm arrays into one vector.
    pub(crate) fn completion_wait_events(
        &mut self,
        handle: CompletionHandle,
        timeoutns: u64,
        maxevents: u32,
    ) -> RuntimeResult<Vec<CompletionEvent>> {
        let value = self.destack_io_completion_wait(handle, timeoutns, maxevents)?;
        match value {
            harness::HarnessValue::Native(value) => {
                let events = unsafe { value.as_slice()? };
                Ok(events.to_vec())
            }
            harness::HarnessValue::Vm(value) => {
                let vm_context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "missing vm context for vm completion events",
                    ))
                    .boxed()
                })?;
                decode_vm_completion_events(vm_context, value)
            }
        }
    }

    /// Build one byte slice harness value for the active engine mode.
    pub(crate) fn byte_slice_value(
        &mut self,
        values: &[u8],
    ) -> RuntimeResult<harness::HarnessValue<NativeSlice<u8>, VmSlice<u8>>> {
        if let Some(vm_context) = self.vm_context_mut() {
            let value = VmSlice::from_bytes(&mut vm_context.write(), values)?;
            return Ok(self.harness_value_vm(value));
        }

        let value = self.call_context.store_slice(values.to_vec());
        Ok(self.harness_value(value))
    }

    /// Build one completion operation harness value for the active engine mode.
    pub(crate) fn completion_operation_value(
        &self,
        operation: CompletionOperation,
    ) -> harness::HarnessValue<CompletionOperation, CompletionOperation> {
        if self.vm_context.is_some() {
            return self.harness_value_vm(operation);
        }

        self.harness_value(operation)
    }

    /// Build one u64 slice harness value for the active engine mode.
    pub(crate) fn u64_slice_value(
        &mut self,
        values: &[u64],
    ) -> RuntimeResult<harness::HarnessValue<NativeSlice<u64>, VmSlice<u64>>> {
        if let Some(vm_context) = self.vm_context_mut() {
            let value = VmSlice::from_values(&mut vm_context.write(), values)?;
            return Ok(self.harness_value_vm(value));
        }

        let value = self.call_context.store_slice(values.to_vec());
        Ok(self.harness_value(value))
    }

    /// Build one resource id slice harness value for the active engine mode.
    #[cfg(target_os = "linux")]
    pub(crate) fn resource_id_slice_value(
        &mut self,
        values: &[ResourceId],
    ) -> RuntimeResult<harness::HarnessValue<NativeSlice<ResourceId>, VmSlice<ResourceId>>> {
        if let Some(vm_context) = self.vm_context_mut() {
            let value = VmSlice::from_values(&mut vm_context.write(), values)?;
            return Ok(self.harness_value_vm(value));
        }

        let value = self.call_context.store_slice(values.to_vec());
        Ok(self.harness_value(value))
    }

    /// Build one u32 slice harness value for the active engine mode.
    #[cfg(target_os = "linux")]
    pub(crate) fn u32_slice_value(
        &mut self,
        values: &[u32],
    ) -> RuntimeResult<harness::HarnessValue<NativeSlice<u32>, VmSlice<u32>>> {
        if let Some(vm_context) = self.vm_context_mut() {
            let value = VmSlice::from_values(&mut vm_context.write(), values)?;
            return Ok(self.harness_value_vm(value));
        }

        let value = self.call_context.store_slice(values.to_vec());
        Ok(self.harness_value(value))
    }

    /// Build one descriptor request harness value for the active engine mode.
    pub(crate) fn descriptor_request_value(
        &mut self,
        code: u64,
        input: &[u8],
        output_size: u32,
        flags: u32,
    ) -> RuntimeResult<harness::HarnessValue<DescriptorRequest, DescriptorRequestVm>> {
        if let Some(vm_context) = self.vm_context_mut() {
            let input = VmSlice::from_bytes(&mut vm_context.write(), input)?;
            let request = DescriptorRequestVm {
                code,
                input,
                output_size,
                flags,
            };
            return Ok(self.harness_value_vm(request));
        }

        let input = self.call_context.store_slice(input.to_vec());
        let request = DescriptorRequest {
            code,
            input,
            output_size,
            flags,
        };
        Ok(self.harness_value(request))
    }

    /// Build one path harness value for the active engine mode.
    pub(crate) fn os_path_value(
        &mut self,
        value: &str,
    ) -> RuntimeResult<harness::HarnessValue<fs::OsPath, fs::OsPathVm>> {
        if let Some(vm_context) = self.vm_context_mut() {
            let value = vm_path_from_utf8(vm_context, value)?;
            return Ok(self.harness_value_vm(value));
        }

        let value = core_fs::os_path_from_utf8_string(self.call_context, value.to_string());
        Ok(self.harness_value(value))
    }

    /// Open one completion queue or return None when the backend is unsupported.
    pub(crate) fn completion_open_or_skip(
        &mut self,
        entries: u32,
    ) -> RuntimeResult<Option<CompletionHandle>> {
        match self.destack_io_completion_open(entries) {
            Ok(handle) => Ok(Some(handle)),
            Err(error) => {
                if is_not_supported_code(error_code_from_runtime_error(&error)) {
                    Ok(None)
                } else {
                    Err(error)
                }
            }
        }
    }

    /// Build one uring parameter harness value for the active engine mode.
    pub(crate) fn uring_parameters_value(
        &self,
        parameters: UringParameters,
    ) -> harness::HarnessValue<UringParameters, UringParametersVm> {
        if self.vm_context.is_some() {
            return self.harness_value_vm(parameters);
        }

        self.harness_value(parameters)
    }

    /// Open one io_uring instance or return None when unsupported on this host.
    #[cfg(target_os = "linux")]
    pub(crate) fn uring_open_or_skip(
        &mut self,
        parameters: UringParameters,
    ) -> RuntimeResult<Option<UringHandle>> {
        match self.destack_io_uring_open(self.uring_parameters_value(parameters)) {
            Ok(handle) => Ok(Some(handle)),
            Err(error) => {
                if is_not_supported_code(error_code_from_runtime_error(&error)) {
                    Ok(None)
                } else {
                    Err(error)
                }
            }
        }
    }
}

/// Decode one VM poll event array payload.
fn decode_vm_poll_events(
    context: &mut vm::ExternalCallContext<'_>,
    value: VmArray<PollEventVm>,
) -> RuntimeResult<Vec<PollEvent>> {
    // read vm array payload
    let values = value.raw_values(&context.read())?;
    let mut events = Vec::with_capacity(values.len());

    for value in values {
        // decode aggregate fields
        let slots = context
            .decode_component_values(value)
            .map_err(|error| RuntimeError::from(error).boxed())?;
        if slots.len() != 3 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "event",
                "expected 3 fields",
            ))
            .boxed());
        }

        let (key, key_width) = slots[0].as_uint_with_width().ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_type("event.key", "uint64")).boxed()
        })?;
        if key_width != 64 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_type(
                "event.key",
                "uint64",
            ))
            .boxed());
        }

        let (ready, ready_width) = slots[1].as_uint_with_width().ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_type(
                "event.ready",
                "uint32",
            ))
            .boxed()
        })?;
        if ready_width != 32 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_type(
                "event.ready",
                "uint32",
            ))
            .boxed());
        }

        let (data, data_width) = slots[2].as_int_with_width().ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_type("event.data", "int32")).boxed()
        })?;
        if data_width != 32 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_type(
                "event.data",
                "int32",
            ))
            .boxed());
        }

        // normalize decoded event
        events.push(PollEvent {
            key,
            ready: PollInterest(ready as u32),
            data: data as i32,
        });
    }

    Ok(events)
}

/// Decode one VM completion event array payload.
fn decode_vm_completion_events(
    context: &mut vm::ExternalCallContext<'_>,
    value: VmArray<CompletionEventVm>,
) -> RuntimeResult<Vec<CompletionEvent>> {
    // read vm array payload
    let values = value.raw_values(&context.read())?;
    let mut events = Vec::with_capacity(values.len());

    for value in values {
        // decode aggregate fields
        let slots = context
            .decode_component_values(value)
            .map_err(|error| RuntimeError::from(error).boxed())?;
        if slots.len() != 3 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "event",
                "expected 3 fields",
            ))
            .boxed());
        }

        let (key, key_width) = slots[0].as_uint_with_width().ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_type("event.key", "uint64")).boxed()
        })?;
        if key_width != 64 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_type(
                "event.key",
                "uint64",
            ))
            .boxed());
        }

        let (result, result_width) = slots[1].as_int_with_width().ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_type(
                "event.result",
                "int64",
            ))
            .boxed()
        })?;
        if result_width != 64 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_type(
                "event.result",
                "int64",
            ))
            .boxed());
        }

        let (flags, flags_width) = slots[2].as_uint_with_width().ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_type(
                "event.flags",
                "uint32",
            ))
            .boxed()
        })?;
        if flags_width != 32 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_type(
                "event.flags",
                "uint32",
            ))
            .boxed());
        }

        // normalize decoded event
        events.push(CompletionEvent {
            key,
            result,
            flags: flags as u32,
        });
    }

    Ok(events)
}

/// Native io harness.
pub(crate) struct NativeIoHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeIoHarness {
    /// Create a new native io harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM io harness.
pub(crate) struct VmIoHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmIoHarness {
    /// Create a new VM io harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum IoHarnessHandle {
    /// Native io harness.
    Native(NativeIoHarness),
    /// VM io harness.
    Vm(VmIoHarness),
}

/// Return whether the VM exposes the io binding surface yet.
fn vm_io_bindings_are_available() -> bool {
    let mut harness = IoHarnessHandle::Vm(VmIoHarness::new());

    harness
        .with_context(|mut context| -> RuntimeResult<bool> {
            match context.destack_io_event_open(0) {
                Ok(token) => {
                    context.destack_io_event_close(token)?;
                    Ok(true)
                }
                Err(error) => {
                    if is_not_supported_code(error_code_from_runtime_error(&error)) {
                        Ok(false)
                    } else {
                        Err(error)
                    }
                }
            }
        })
        .expect("io vm capability probe should not fail")
}

impl IoHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&mut self, callback: F) -> R
    where
        F: for<'call> FnOnce(IoHarnessContext<'call>) -> R,
    {
        match self {
            IoHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(IoHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            IoHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();
                        callback(IoHarnessContext {
                            call_context,
                            vm_context: Some(vm_context),
                        })
                    })
            }
        }
    }

    /// Run one callback that returns a runtime result.
    pub(crate) fn run<F>(&mut self, callback: F)
    where
        F: for<'call> FnOnce(IoHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("io harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&mut IoHarnessHandle),
{
    let mut native = IoHarnessHandle::Native(NativeIoHarness::new());
    callback(&mut native);

    // skip the vm lane until platform.io is actually implemented there
    if vm_io_bindings_are_available() {
        let mut vm = IoHarnessHandle::Vm(VmIoHarness::new());
        callback(&mut vm);
    }
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(IoHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}

/// Build one unique temporary path for one test-specific prefix.
fn unique_temp_path(prefix: &str) -> PathBuf {
    let sequence = UNIQUE_TEMP_PATH_COUNTER.fetch_add(1, Ordering::Relaxed);
    let process_id = std::process::id();
    std::env::temp_dir().join(format!("destack-io-{prefix}-{process_id}-{sequence}"))
}

/// Return one well-known null device path for the active host.
fn host_null_device_path() -> &'static str {
    #[cfg(unix)]
    {
        "/dev/null"
    }

    #[cfg(windows)]
    {
        r"\\.\NUL"
    }

    #[cfg(not(any(unix, windows)))]
    {
        "/dev/null"
    }
}

/// Encode one UTF-8 path string into VM `OsPath`.
fn vm_path_from_utf8(
    context: &mut vm::ExternalCallContext<'_>,
    value: &str,
) -> RuntimeResult<fs::OsPathVm> {
    #[cfg(unix)]
    {
        let bytes =
            fs::PathBytesAbi::<VmAbi>(VmArray::from_bytes(&mut context.write(), value.as_bytes())?);
        let kind = vm::StringHandle::new(context.intern_string("bytes")?);
        Ok(fs::OsPathVm::OsPathBytes(fs::OsPathBytesVm { kind, bytes }))
    }

    #[cfg(windows)]
    {
        let utf16_values = value.encode_utf16().collect::<Vec<_>>();
        let utf16 =
            fs::PathUtf16Abi::<VmAbi>(VmArray::from_values(&mut context.write(), &utf16_values)?);
        let kind = vm::StringHandle::new(context.intern_string("utf16")?);
        Ok(fs::OsPathVm::OsPathUtf16(fs::OsPathUtf16Vm { kind, utf16 }))
    }

    #[cfg(not(any(unix, windows)))]
    {
        let bytes =
            fs::PathBytesAbi::<VmAbi>(VmArray::from_bytes(&mut context.write(), value.as_bytes()));
        let kind = vm::StringHandle::new(context.intern_string("bytes"));
        Ok(fs::OsPathVm::OsPathBytes(fs::OsPathBytesVm { kind, bytes }))
    }
}

/// Insert one runtime resource that resolves to one host completion handle.
fn insert_completion_target_with_host_handle(binding: &BindingCallContext) -> ResourceId {
    #[cfg(unix)]
    {
        binding.worker().resources.insert(
            &binding.world(),
            ResourceEntry::new(ResourceKind::File).with_fd(0),
            Some(binding.engine()),
        )
    }

    #[cfg(windows)]
    {
        binding.worker().resources.insert(
            &binding.world(),
            ResourceEntry::new(ResourceKind::File).with_handle(std::ptr::dangling_mut::<c_void>()),
            Some(binding.engine()),
        )
    }
}

/// Open and close one completion queue across native and VM engines.
#[test]
fn test_io_completion_open_close_roundtrip() {
    with_harness_context(|mut context| {
        let Some(handle) = context.completion_open_or_skip(32)? else {
            return Ok(());
        };
        context.destack_io_completion_close(handle)?;
        assert_platform_error_code(
            context.destack_io_completion_close(handle),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Reject raw device opens for missing paths.
#[test]
fn test_io_device_open_rejects_missing_path() {
    with_harness_context(|mut context| {
        #[cfg(unix)]
        let path = unique_temp_path("missing-device")
            .to_string_lossy()
            .to_string();
        #[cfg(windows)]
        let path = String::from(r"\\.\DestackMissingDevice");
        #[cfg(not(any(unix, windows)))]
        let path = unique_temp_path("missing-device")
            .to_string_lossy()
            .to_string();
        let path = context.os_path_value(&path)?;

        assert_platform_error_code(
            context.destack_io_device_open(path, 0, 0),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Open one well-known null device and preserve raw read or write semantics.
#[test]
fn test_io_device_open_read_write_null_device() {
    with_harness_context(|mut context| {
        let path = context.os_path_value(host_null_device_path())?;
        let handle = context.destack_io_device_open(path, libc::O_RDWR as u32, 0)?;

        let bytes = context.byte_slice_value(b"ping")?;
        let written = context.destack_io_device_write(handle, bytes)?;
        assert_eq!(written, 4);

        let buffer = context.byte_slice_value(&[0u8; 4])?;
        let read = context.destack_io_device_read(handle, buffer)?;
        assert_eq!(read, 0);

        context.destack_io_device_close(handle)?;

        Ok(())
    });
}

/// Reject raw device opens for ordinary file paths.
#[test]
fn test_io_device_open_rejects_regular_files() {
    with_harness_context(|mut context| {
        let path_buf = unique_temp_path("regular-file");
        std::fs::write(&path_buf, b"io device regular file probe")
            .map_err(|error| RuntimeError::from(PlatformError::io(error.to_string())).boxed())?;
        let path_string = path_buf.to_string_lossy().to_string();
        let path = context.os_path_value(&path_string)?;

        let result = context.destack_io_device_open(path, 0, 0);
        std::fs::remove_file(&path_buf)
            .map_err(|error| RuntimeError::from(PlatformError::io(error.to_string())).boxed())?;

        assert_platform_error_code(result, PlatformErrorCode::InvalidArgumentValue)?;

        Ok(())
    });
}

/// Reject device operations for unknown handles.
#[test]
fn test_io_device_operations_reject_unknown_handle() {
    with_harness_context(|mut context| {
        let handle = DeviceHandle(ResourceId(999_999));
        let buffer = context.byte_slice_value(&[0u8; 4])?;
        let bytes = context.byte_slice_value(b"ping")?;
        let request = context.descriptor_request_value(1, &[], 0, 0)?;

        assert_platform_error_code(
            context.destack_io_device_close(handle),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        assert_platform_error_code(
            context.destack_io_device_read(handle, buffer),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        assert_platform_error_code(
            context.destack_io_device_write(handle, bytes),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        assert_platform_error_code(
            context.destack_io_device_control(handle, request),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        Ok(())
    });
}

/// Reject completion queue opens that request zero entries.
#[test]
fn test_io_completion_open_rejects_zero_entries() {
    with_harness_context(|mut context| {
        assert_platform_error_code(
            context.destack_io_completion_open(0),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        Ok(())
    });
}

/// Reject forged completion handles that point to non-completion resources.
#[test]
fn test_io_completion_close_rejects_non_completion_handle() {
    with_harness_context(|mut context| {
        let foreign = context.call_context.worker().resources.insert(
            context.call_context.world(),
            ResourceEntry::new(ResourceKind::File),
            Some(context.call_context.engine()),
        );
        let forged = CompletionHandle(foreign);
        assert_platform_error_code(
            context.destack_io_completion_close(forged),
            PlatformErrorCode::IoNotFound,
        )?;
        assert!(context.call_context.worker().resources.contains(foreign));
        context.call_context.worker().resources.remove_and_finalize(
            context.call_context.world(),
            foreign,
            Some(context.call_context.engine()),
        );

        Ok(())
    });
}

/// Reject completion enter calls for unknown completion handles.
#[test]
fn test_io_completion_enter_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let unknown = CompletionHandle(ResourceId(999_999));
        assert_platform_error_code(
            context.destack_io_completion_enter(unknown, 0, 0, 0),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Reject completion wait calls for unknown completion handles.
#[test]
fn test_io_completion_wait_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let unknown = CompletionHandle(ResourceId(999_999));
        assert_platform_error_code(
            context.destack_io_completion_wait(unknown, 0, 8),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Reject completion cancel calls for unknown completion handles.
#[test]
fn test_io_completion_cancel_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let unknown = CompletionHandle(ResourceId(999_999));
        assert_platform_error_code(
            context.destack_io_completion_cancel(unknown, ResourceId(1)),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Reject completion waits that request zero max events.
#[test]
fn test_io_completion_wait_rejects_zero_maxevents() {
    with_harness_context(|mut context| {
        let Some(handle) = context.completion_open_or_skip(16)? else {
            return Ok(());
        };

        assert_platform_error_code(
            context.destack_io_completion_wait(handle, 0, 0),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        context.destack_io_completion_close(handle)?;

        Ok(())
    });
}

/// Return an empty completion list when no operations have completed.
#[test]
fn test_io_completion_wait_empty_returns_no_events() {
    with_harness_context(|mut context| {
        let Some(handle) = context.completion_open_or_skip(16)? else {
            return Ok(());
        };

        let events = context.completion_wait_events(handle, 0, 8)?;
        assert!(events.is_empty());
        context.destack_io_completion_close(handle)?;

        Ok(())
    });
}

/// Reject completion submits that target unknown resources.
#[test]
fn test_io_completion_submit_rejects_unknown_target() {
    with_harness_context(|mut context| {
        let Some(handle) = context.completion_open_or_skip(16)? else {
            return Ok(());
        };

        let operation = CompletionOperation {
            kind: CompletionOperationKind::Read,
            target: ResourceId(999_999),
            key: 1,
            offset: 0,
            length: 16,
            flags: 0,
            argument0: 0,
            argument1: 0,
        };
        let operation = context.completion_operation_value(operation);
        assert_platform_error_code(
            context.destack_io_completion_submit(handle, operation),
            PlatformErrorCode::IoNotFound,
        )?;
        context.destack_io_completion_close(handle)?;

        Ok(())
    });
}

/// Reject completion submits that provide null pointers for pointer-backed operations.
#[test]
fn test_io_completion_submit_rejects_null_pointer_argument() {
    with_harness_context(|mut context| {
        let Some(handle) = context.completion_open_or_skip(16)? else {
            return Ok(());
        };

        let target = insert_completion_target_with_host_handle(context.call_context);
        let mut key = 1u64;
        for kind in [
            CompletionOperationKind::Read,
            CompletionOperationKind::Write,
            CompletionOperationKind::Connect,
            CompletionOperationKind::Send,
            CompletionOperationKind::Receive,
        ] {
            let operation = CompletionOperation {
                kind,
                target,
                key,
                offset: 0,
                length: 16,
                flags: 0,
                argument0: 0,
                argument1: 0,
            };
            let operation = context.completion_operation_value(operation);
            assert_platform_error_code(
                context.destack_io_completion_submit(handle, operation),
                PlatformErrorCode::NullPointer,
            )?;
            key = key.saturating_add(1);
        }

        context.destack_io_completion_close(handle)?;
        context.call_context.worker().resources.remove_and_finalize(
            context.call_context.world(),
            target,
            Some(context.call_context.engine()),
        );

        Ok(())
    });
}

/// Reject completion submits that use one reserved internal wake token.
#[test]
fn test_io_completion_submit_rejects_reserved_token() {
    with_harness_context(|mut context| {
        let Some(handle) = context.completion_open_or_skip(16)? else {
            return Ok(());
        };

        let operation = CompletionOperation {
            kind: CompletionOperationKind::Timeout,
            target: ResourceId(0),
            key: u64::MAX,
            offset: 1_000_000,
            length: 0,
            flags: 0,
            argument0: 0,
            argument1: 0,
        };
        let operation = context.completion_operation_value(operation);
        assert_platform_error_code(
            context.destack_io_completion_submit(handle, operation),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        context.destack_io_completion_close(handle)?;

        Ok(())
    });
}

/// Reject completion batches that include duplicate tokens.
#[test]
fn test_io_completion_submit_batch_rejects_duplicate_tokens() {
    with_harness_context(|mut context| {
        let Some(handle) = context.completion_open_or_skip(16)? else {
            return Ok(());
        };

        let words = [
            5u64, 0, 55, 1_000_000, 0, 0, 0, 0, 5, 0, 55, 1_000_000, 0, 0, 0, 0,
        ];
        let words = context.u64_slice_value(&words)?;
        assert_platform_error_code(
            context.destack_io_completion_submit_batch(handle, words, 2, 8),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        context.destack_io_completion_close(handle)?;

        Ok(())
    });
}

/// Reject completion batches that reuse one token already pending in the queue.
#[test]
fn test_io_completion_submit_batch_rejects_pending_token() {
    with_harness_context(|mut context| {
        let Some(handle) = context.completion_open_or_skip(16)? else {
            return Ok(());
        };

        let operation = CompletionOperation {
            kind: CompletionOperationKind::Timeout,
            target: ResourceId(0),
            key: 91,
            offset: 1_000_000_000,
            length: 0,
            flags: 0,
            argument0: 0,
            argument1: 0,
        };
        let operation = context.completion_operation_value(operation);
        context.destack_io_completion_submit(handle, operation)?;

        let words = [5u64, 0, 91, 1_000_000, 0, 0, 0, 0];
        let words = context.u64_slice_value(&words)?;
        assert_platform_error_code(
            context.destack_io_completion_submit_batch(handle, words, 1, 8),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        context.destack_io_completion_close(handle)?;

        Ok(())
    });
}

/// Return one timeout completion for submitted timeout operations.
#[test]
fn test_io_completion_submit_timeout_wait_roundtrip() {
    with_harness_context(|mut context| {
        let Some(handle) = context.completion_open_or_skip(16)? else {
            return Ok(());
        };

        let operation = CompletionOperation {
            kind: CompletionOperationKind::Timeout,
            target: ResourceId(0),
            key: 77,
            offset: 1_000_000,
            length: 0,
            flags: 0,
            argument0: 0,
            argument1: 0,
        };
        let operation = context.completion_operation_value(operation);
        context.destack_io_completion_submit(handle, operation)?;

        let events = context.completion_wait_events(handle, 1_000_000_000, 8)?;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, 77);

        context.destack_io_completion_close(handle)?;

        Ok(())
    });
}

/// Reject batch submissions that provide one short operation stride.
#[test]
fn test_io_completion_submit_batch_rejects_short_stride() {
    with_harness_context(|mut context| {
        let Some(handle) = context.completion_open_or_skip(16)? else {
            return Ok(());
        };

        let words = [1u64, 1, 1, 0, 0, 0, 0, 0];
        let words = context.u64_slice_value(&words)?;
        assert_platform_error_code(
            context.destack_io_completion_submit_batch(handle, words, 1, 7),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        context.destack_io_completion_close(handle)?;

        Ok(())
    });
}

/// Reject completion batches that encode unknown operation kinds.
#[test]
fn test_io_completion_submit_batch_rejects_unknown_operation_kind() {
    with_harness_context(|mut context| {
        let Some(handle) = context.completion_open_or_skip(16)? else {
            return Ok(());
        };

        let words = [99u64, 0, 1, 0, 0, 0, 0, 0];
        let words = context.u64_slice_value(&words)?;
        assert_platform_error_code(
            context.destack_io_completion_submit_batch(handle, words, 1, 8),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        context.destack_io_completion_close(handle)?;

        Ok(())
    });
}

/// Reject completion enter requests that set unsupported flags.
#[test]
fn test_io_completion_enter_rejects_unknown_flags() {
    with_harness_context(|mut context| {
        let Some(handle) = context.completion_open_or_skip(16)? else {
            return Ok(());
        };

        assert_platform_error_code(
            context.destack_io_completion_enter(handle, 0, 0, 1),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        context.destack_io_completion_close(handle)?;

        Ok(())
    });
}

/// Return zero canceled operations when a known target has no pending requests.
#[test]
fn test_io_completion_cancel_returns_zero_without_pending_requests() {
    with_harness_context(|mut context| {
        let Some(handle) = context.completion_open_or_skip(16)? else {
            return Ok(());
        };

        let target = context.call_context.worker().resources.insert(
            context.call_context.world(),
            ResourceEntry::new(ResourceKind::File),
            Some(context.call_context.engine()),
        );
        let canceled = context.destack_io_completion_cancel(handle, target)?;
        assert_eq!(canceled, 0);

        context.destack_io_completion_close(handle)?;
        context.call_context.worker().resources.remove_and_finalize(
            context.call_context.world(),
            target,
            Some(context.call_context.engine()),
        );

        Ok(())
    });
}

/// Reject completion cancels for unknown target resources.
#[test]
fn test_io_completion_cancel_rejects_unknown_target() {
    with_harness_context(|mut context| {
        let Some(handle) = context.completion_open_or_skip(16)? else {
            return Ok(());
        };

        assert_platform_error_code(
            context.destack_io_completion_cancel(handle, ResourceId(999_999)),
            PlatformErrorCode::IoNotFound,
        )?;
        context.destack_io_completion_close(handle)?;

        Ok(())
    });
}

/// Open and close one event token across native and VM engines.
#[test]
fn test_io_event_open_close_roundtrip() {
    with_harness_context(|mut context| {
        let token = context.destack_io_event_open(0)?;
        context.destack_io_event_close(token)?;
        assert_platform_error_code(
            context.destack_io_event_close(token),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Reject event opens that use one reserved initial value.
#[test]
fn test_io_event_open_rejects_reserved_initial_value() {
    with_harness_context(|mut context| {
        assert_platform_error_code(
            context.destack_io_event_open(u64::MAX),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        Ok(())
    });
}

/// Reject forged event tokens that point to non-event resources.
#[test]
fn test_io_event_close_rejects_non_event_token() {
    with_harness_context(|mut context| {
        let foreign = context.call_context.worker().resources.insert(
            context.call_context.world(),
            ResourceEntry::new(ResourceKind::File),
            Some(context.call_context.engine()),
        );
        let forged = EventToken(foreign.0);
        assert_platform_error_code(
            context.destack_io_event_close(forged),
            PlatformErrorCode::IoNotFound,
        )?;
        assert!(context.call_context.worker().resources.contains(foreign));
        context.call_context.worker().resources.remove_and_finalize(
            context.call_context.world(),
            foreign,
            Some(context.call_context.engine()),
        );

        Ok(())
    });
}

/// Reject event signals that use one reserved maximum counter value.
#[test]
fn test_io_event_signal_rejects_reserved_value() {
    with_harness_context(|mut context| {
        let token = context.destack_io_event_open(0)?;
        assert_platform_error_code(
            context.destack_io_event_signal(token, u64::MAX),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        context.destack_io_event_close(token)?;

        Ok(())
    });
}

/// Accept nonzero event signals for valid event tokens.
#[test]
fn test_io_event_signal_accepts_nonzero_value() {
    with_harness_context(|mut context| {
        let token = context.destack_io_event_open(0)?;
        context.destack_io_event_signal(token, 1)?;
        context.destack_io_event_close(token)?;

        Ok(())
    });
}

/// Reject zero event signal values.
#[test]
fn test_io_event_signal_rejects_zero_value() {
    with_harness_context(|mut context| {
        let token = context.destack_io_event_open(0)?;
        assert_platform_error_code(
            context.destack_io_event_signal(token, 0),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        context.destack_io_event_close(token)?;

        Ok(())
    });
}

/// Reject zero signals for unknown event tokens as io-not-found.
#[test]
fn test_io_event_signal_zero_rejects_unknown_token() {
    with_harness_context(|mut context| {
        assert_platform_error_code(
            context.destack_io_event_signal(EventToken(999_999), 0),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Reject nonzero signals for unknown event tokens as io-not-found.
#[test]
fn test_io_event_signal_rejects_unknown_token_nonzero() {
    with_harness_context(|mut context| {
        assert_platform_error_code(
            context.destack_io_event_signal(EventToken(999_999), 1),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Reject event attaches that target unknown runtime resources.
#[test]
fn test_io_event_attach_rejects_unknown_target() {
    with_harness_context(|mut context| {
        let token = context.destack_io_event_open(0)?;
        assert_platform_error_code(
            context.destack_io_event_attach(token, ResourceId(999_999), 7),
            PlatformErrorCode::IoNotFound,
        )?;
        context.destack_io_event_close(token)?;

        Ok(())
    });
}

/// Reject event attaches for unknown event tokens.
#[test]
fn test_io_event_attach_rejects_unknown_token() {
    with_harness_context(|mut context| {
        let poll = context.destack_io_poll_open(PollBackend::Auto)?;
        assert_platform_error_code(
            context.destack_io_event_attach(EventToken(999_999), poll.0, 7),
            PlatformErrorCode::IoNotFound,
        )?;
        context.destack_io_poll_close(poll)?;

        Ok(())
    });
}

/// Accept event attaches when token and target both exist.
#[test]
fn test_io_event_attach_accepts_known_target() {
    with_harness_context(|mut context| {
        let token = context.destack_io_event_open(0)?;
        let poll = context.destack_io_poll_open(PollBackend::Auto)?;

        context.destack_io_event_attach(token, poll.0, 7)?;
        context.destack_io_poll_close(poll)?;
        context.destack_io_event_close(token)?;

        Ok(())
    });
}

/// Keep event attachment state scoped to the current runtime.
#[test]
fn test_io_event_attachment_state_is_runtime_scoped() {
    // native runtime pair
    let mut native_first = IoHarnessHandle::Native(NativeIoHarness::new());
    native_first
        .with_context(|mut context| -> RuntimeResult<()> {
            let token = context.destack_io_event_open(0)?;
            let poll = context.destack_io_poll_open(PollBackend::Auto)?;
            context.destack_io_event_attach(token, poll.0, 91)?;

            Ok(())
        })
        .expect("first native runtime setup should succeed");

    let mut native_second = IoHarnessHandle::Native(NativeIoHarness::new());
    native_second
        .with_context(|mut context| -> RuntimeResult<()> {
            let token = context.destack_io_event_open(0)?;
            let poll = context.destack_io_poll_open(PollBackend::Auto)?;

            context.destack_io_event_signal(token, 1)?;
            let events = context.poll_wait_events(poll, 0, 8)?;
            assert!(events.is_empty());

            context.destack_io_poll_close(poll)?;
            context.destack_io_event_close(token)?;

            Ok(())
        })
        .expect("second native runtime verification should succeed");

    // vm runtime pair
    if vm_io_bindings_are_available() {
        let mut vm_first = IoHarnessHandle::Vm(VmIoHarness::new());
        vm_first
            .with_context(|mut context| -> RuntimeResult<()> {
                let token = context.destack_io_event_open(0)?;
                let poll = context.destack_io_poll_open(PollBackend::Auto)?;
                context.destack_io_event_attach(token, poll.0, 91)?;

                Ok(())
            })
            .expect("first vm runtime setup should succeed");

        let mut vm_second = IoHarnessHandle::Vm(VmIoHarness::new());
        vm_second
            .with_context(|mut context| -> RuntimeResult<()> {
                let token = context.destack_io_event_open(0)?;
                let poll = context.destack_io_poll_open(PollBackend::Auto)?;

                context.destack_io_event_signal(token, 1)?;
                let events = context.poll_wait_events(poll, 0, 8)?;
                assert!(events.is_empty());

                context.destack_io_poll_close(poll)?;
                context.destack_io_event_close(token)?;

                Ok(())
            })
            .expect("second vm runtime verification should succeed");
    }
}

/// Reject event attaches when the target exists but is not a poll handle.
#[test]
fn test_io_event_attach_rejects_non_poll_target() {
    with_harness_context(|mut context| {
        let token = context.destack_io_event_open(0)?;
        let target = context.call_context.worker().resources.insert(
            context.call_context.world(),
            ResourceEntry::new(ResourceKind::File),
            Some(context.call_context.engine()),
        );

        assert_platform_error_code(
            context.destack_io_event_attach(token, target, 7),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        context.destack_io_event_close(token)?;
        context.call_context.worker().resources.remove_and_finalize(
            context.call_context.world(),
            target,
            Some(context.call_context.engine()),
        );

        Ok(())
    });
}

/// Dispatch synthetic poll events when signaling attached event tokens.
#[test]
fn test_io_event_signal_dispatches_attached_poll_event() {
    with_harness_context(|mut context| {
        let token = context.destack_io_event_open(0)?;
        let poll = context.destack_io_poll_open(PollBackend::Auto)?;

        context.destack_io_event_attach(token, poll.0, 41)?;
        context.destack_io_event_signal(token, 99)?;

        let events = context.poll_wait_events(poll, 0, 8)?;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, 41);
        assert_eq!(events[0].data, 99);
        assert_eq!(events[0].ready.0 & (1 << 0), 1);

        context.destack_io_poll_close(poll)?;
        context.destack_io_event_close(token)?;

        Ok(())
    });
}

/// Stage overflow poll events for future waits when maxevents is smaller than ready events.
#[test]
fn test_io_event_signal_stages_poll_overflow_for_next_wait() {
    with_harness_context(|mut context| {
        let first_token = context.destack_io_event_open(0)?;
        let second_token = context.destack_io_event_open(0)?;
        let third_token = context.destack_io_event_open(0)?;
        let poll = context.destack_io_poll_open(PollBackend::Auto)?;

        context.destack_io_event_attach(first_token, poll.0, 51)?;
        context.destack_io_event_attach(second_token, poll.0, 51)?;
        context.destack_io_event_attach(third_token, poll.0, 51)?;
        context.destack_io_event_signal(first_token, 1)?;
        context.destack_io_event_signal(second_token, 2)?;
        context.destack_io_event_signal(third_token, 3)?;

        let first_events = context.poll_wait_events(poll, 0, 2)?;
        assert_eq!(first_events.len(), 2);

        let second_events = context.poll_wait_events(poll, 0, 2)?;
        assert_eq!(second_events.len(), 1);
        let observed_values: Vec<i32> = first_events
            .iter()
            .map(|event| event.data)
            .chain(second_events.iter().map(|event| event.data))
            .collect();
        assert!(observed_values.contains(&1));
        assert!(observed_values.contains(&2));
        assert!(observed_values.contains(&3));

        context.destack_io_poll_close(poll)?;
        context.destack_io_event_close(first_token)?;
        context.destack_io_event_close(second_token)?;
        context.destack_io_event_close(third_token)?;

        Ok(())
    });
}

/// Prune stale event attachments after poll handles are closed.
#[test]
fn test_io_event_signal_after_poll_close_prunes_stale_attachment() {
    with_harness_context(|mut context| {
        let token = context.destack_io_event_open(0)?;
        let poll = context.destack_io_poll_open(PollBackend::Auto)?;

        context.destack_io_event_attach(token, poll.0, 44)?;
        context.destack_io_poll_close(poll)?;
        context.destack_io_event_signal(token, 5)?;

        let replacement_poll = context.destack_io_poll_open(PollBackend::Auto)?;
        let events = context.poll_wait_events(replacement_poll, 0, 8)?;
        assert!(events.is_empty());

        context.destack_io_poll_close(replacement_poll)?;
        context.destack_io_event_close(token)?;

        Ok(())
    });
}

/// Reject descriptor control fcntl operations with unknown flags.
#[test]
fn test_io_control_fcntl_rejects_unknown_flags() {
    with_harness_context(|mut context| {
        assert_platform_error_code(
            context.destack_io_control_fcntl(
                ResourceId(999_999),
                DescriptorControlCommand(0),
                0,
                DescriptorControlFlags(1),
            ),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        Ok(())
    });
}

/// Reject descriptor control ioctl requests with unknown flags.
#[test]
fn test_io_control_ioctl_rejects_unknown_flags() {
    with_harness_context(|mut context| {
        let request = context.descriptor_request_value(0, &[], 0, 1)?;
        assert_platform_error_code(
            context.destack_io_control_ioctl(ResourceId(999_999), request),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        Ok(())
    });
}

/// Reject descriptor control ioctl requests that exceed the output safety limit.
#[test]
fn test_io_control_ioctl_rejects_oversized_output() {
    with_harness_context(|mut context| {
        let request = context.descriptor_request_value(0, &[], 16 * 1024 * 1024 + 1, 0)?;
        assert_platform_error_code(
            context.destack_io_control_ioctl(ResourceId(999_999), request),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        Ok(())
    });
}

/// Reject descriptor control ioctl requests that exceed the input safety limit.
#[test]
fn test_io_control_ioctl_rejects_oversized_input() {
    with_harness_context(|mut context| {
        let oversized_input = vec![0u8; 16 * 1024 * 1024 + 1];
        let request = context.descriptor_request_value(0, &oversized_input, 0, 0)?;
        assert_platform_error_code(
            context.destack_io_control_ioctl(ResourceId(999_999), request),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        Ok(())
    });
}

/// Return io-not-found for ioctl when the target resource does not exist.
#[test]
fn test_io_control_ioctl_rejects_unknown_target() {
    with_harness_context(|mut context| {
        let request = context.descriptor_request_value(0, &[], 0, 0)?;
        assert_platform_error_code(
            context.destack_io_control_ioctl(ResourceId(999_999), request),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Return io-not-found for fcntl when the target resource does not exist.
#[cfg(not(windows))]
#[test]
fn test_io_control_fcntl_rejects_unknown_target() {
    with_harness_context(|mut context| {
        assert_platform_error_code(
            context.destack_io_control_fcntl(
                ResourceId(999_999),
                DescriptorControlCommand(0),
                0,
                DescriptorControlFlags(0),
            ),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Reject descriptor control fcntl commands that cannot fit one host command lane.
#[cfg(not(windows))]
#[test]
fn test_io_control_fcntl_rejects_command_width_overflow() {
    with_harness_context(|mut context| {
        let target = insert_completion_target_with_host_handle(context.call_context);

        assert_platform_error_code(
            context.destack_io_control_fcntl(
                target,
                DescriptorControlCommand(u32::MAX),
                0,
                DescriptorControlFlags(0),
            ),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        context.call_context.worker().resources.remove_and_finalize(
            context.call_context.world(),
            target,
            Some(context.call_context.engine()),
        );

        Ok(())
    });
}

/// Validate io_uring open and close behavior for supported and unsupported hosts.
#[test]
fn test_io_uring_open_contract() {
    with_harness_context(|mut context| {
        let parameters = UringParameters {
            entries: 8,
            flags: 0,
            sq_thread_idle_ms: 0,
        };
        let result = context.destack_io_uring_open(context.uring_parameters_value(parameters));
        match result {
            Ok(handle) => context.destack_io_uring_close(handle)?,
            Err(error) => {
                if !is_not_supported_code(error_code_from_runtime_error(&error)) {
                    return Err(error);
                }
            }
        }

        Ok(())
    });
}

/// Reject io_uring opens that request zero entries.
#[cfg(target_os = "linux")]
#[test]
fn test_io_uring_open_rejects_zero_entries() {
    with_harness_context(|mut context| {
        let parameters = UringParameters {
            entries: 0,
            flags: 0,
            sq_thread_idle_ms: 0,
        };
        let result = context.destack_io_uring_open(context.uring_parameters_value(parameters));

        assert_platform_error_code(result, PlatformErrorCode::InvalidArgumentValue)?;

        Ok(())
    });
}

/// Reject io_uring opens with unknown setup flags.
#[cfg(target_os = "linux")]
#[test]
fn test_io_uring_open_rejects_unknown_flags() {
    with_harness_context(|mut context| {
        let parameters = UringParameters {
            entries: 8,
            flags: 1 << 31,
            sq_thread_idle_ms: 0,
        };
        let result = context.destack_io_uring_open(context.uring_parameters_value(parameters));

        assert_platform_error_code(result, PlatformErrorCode::InvalidArgumentValue)?;

        Ok(())
    });
}

/// Reject io_uring registration calls with mismatched address and length lanes.
#[cfg(target_os = "linux")]
#[test]
fn test_io_uring_register_buffers_rejects_lane_mismatch() {
    with_harness_context(|mut context| {
        let Some(handle) = context.uring_open_or_skip(UringParameters {
            entries: 8,
            flags: 0,
            sq_thread_idle_ms: 0,
        })?
        else {
            return Ok(());
        };

        let addresses = context.u64_slice_value(&[0x1000, 0x2000])?;
        let lengths = context.u32_slice_value(&[128])?;
        assert_platform_error_code(
            context.destack_io_uring_register_buffers(handle, addresses, lengths),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        context.destack_io_uring_close(handle)?;

        Ok(())
    });
}

/// Reject io_uring register-files calls that include unknown resource ids.
#[cfg(target_os = "linux")]
#[test]
fn test_io_uring_register_files_rejects_unknown_target() {
    with_harness_context(|mut context| {
        let Some(handle) = context.uring_open_or_skip(UringParameters {
            entries: 8,
            flags: 0,
            sq_thread_idle_ms: 0,
        })?
        else {
            return Ok(());
        };

        let files = context.resource_id_slice_value(&[ResourceId(999_999)])?;
        assert_platform_error_code(
            context.destack_io_uring_register_files(handle, files),
            PlatformErrorCode::IoNotFound,
        )?;
        context.destack_io_uring_close(handle)?;

        Ok(())
    });
}

/// Query io_uring feature metadata for supported hosts.
#[cfg(target_os = "linux")]
#[test]
fn test_io_uring_features_contract() {
    with_harness_context(|mut context| {
        let Some(handle) = context.uring_open_or_skip(UringParameters {
            entries: 8,
            flags: 0,
            sq_thread_idle_ms: 0,
        })?
        else {
            return Ok(());
        };

        let features = context.destack_io_uring_features(handle)?;
        match features {
            harness::HarnessValue::Native(features) => {
                assert!(features.max_entries > 0);
            }
            harness::HarnessValue::Vm(features) => {
                assert!(features.max_entries > 0);
            }
        }

        context.destack_io_uring_close(handle)?;

        Ok(())
    });
}

/// Allow idempotent io_uring unregister calls when nothing is registered.
#[cfg(target_os = "linux")]
#[test]
fn test_io_uring_unregister_without_registration_is_idempotent() {
    with_harness_context(|mut context| {
        let Some(handle) = context.uring_open_or_skip(UringParameters {
            entries: 8,
            flags: 0,
            sq_thread_idle_ms: 0,
        })?
        else {
            return Ok(());
        };

        context.destack_io_uring_unregister_files(handle)?;
        context.destack_io_uring_unregister_buffers(handle)?;
        context.destack_io_uring_unregister_files(handle)?;
        context.destack_io_uring_unregister_buffers(handle)?;

        context.destack_io_uring_close(handle)?;

        Ok(())
    });
}

/// Reject io_uring features calls for unknown handles.
#[cfg(target_os = "linux")]
#[test]
fn test_io_uring_features_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let unknown = UringHandle(ResourceId(999_999));
        assert_platform_error_code(
            context.destack_io_uring_features(unknown),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Reject io_uring close calls for unknown handles.
#[cfg(target_os = "linux")]
#[test]
fn test_io_uring_close_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let unknown = UringHandle(ResourceId(999_999));
        assert_platform_error_code(
            context.destack_io_uring_close(unknown),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Reject io_uring unregister-files calls for unknown handles.
#[cfg(target_os = "linux")]
#[test]
fn test_io_uring_unregister_files_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let unknown = UringHandle(ResourceId(999_999));
        assert_platform_error_code(
            context.destack_io_uring_unregister_files(unknown),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Reject io_uring unregister-buffers calls for unknown handles.
#[cfg(target_os = "linux")]
#[test]
fn test_io_uring_unregister_buffers_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let unknown = UringHandle(ResourceId(999_999));
        assert_platform_error_code(
            context.destack_io_uring_unregister_buffers(unknown),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}
