#![cfg_attr(windows, allow(dead_code, unused_imports))]

use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::memory::MemoryRange;
use crate::platform::{PlatformError, memory};
use crate::runtime::BindingCallContext;
use crate::tests::runtime::TestRuntime;

#[path = "harness.generated.rs"]
mod harness;
use harness::HarnessValue;

/// Test harness context used by tests.
pub(crate) struct MemoryHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call BindingCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

/// Native memory harness.
pub(crate) struct NativeMemoryHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeMemoryHarness {
    /// Create a new native memory harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM memory harness.
pub(crate) struct VmMemoryHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmMemoryHarness {
    /// Create a new VM memory harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum MemoryHarnessHandle {
    /// Native memory harness.
    Native(NativeMemoryHarness),
    /// VM memory harness.
    Vm(VmMemoryHarness),
}

impl MemoryHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&self, callback: F) -> R
    where
        F: for<'call> FnOnce(MemoryHarnessContext<'call>) -> R,
    {
        // dispatch callback through matching runtime harness context
        match self {
            MemoryHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(MemoryHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            MemoryHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();
                        callback(MemoryHarnessContext {
                            call_context,
                            vm_context: Some(vm_context),
                        })
                    })
            }
        }
    }

    /// Run one callback that returns a runtime result.
    pub(crate) fn run<F>(&self, callback: F)
    where
        F: for<'call> FnOnce(MemoryHarnessContext<'call>) -> RuntimeResult<()>,
    {
        // run callback and fail test if runtime call returned an error
        self.with_context(callback)
            .expect("memory harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&MemoryHarnessHandle),
{
    // execute callback against native harness
    let native = MemoryHarnessHandle::Native(NativeMemoryHarness::new());
    callback(&native);

    // execute callback against vm harness
    let vm = MemoryHarnessHandle::Vm(VmMemoryHarness::new());
    callback(&vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(MemoryHarnessContext<'call>) -> RuntimeResult<()>,
{
    // run callback through both harness handles
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}

/// Extract one platform error code from one failed result.
pub(crate) fn error_code<T>(result: RuntimeResult<T>) -> RuntimeResult<PlatformErrorCode> {
    // normalize successful result into one expected test failure
    match result {
        Ok(_) => Err(
            RuntimeError::from(PlatformError::invalid_argument("operation should fail")).boxed(),
        ),
        Err(error) => {
            // extract platform error code payload from runtime error
            let platform = error.platform_error().ok_or_else(|| {
                RuntimeError::from(PlatformError::invalid_argument(
                    "missing platform error payload",
                ))
                .boxed()
            })?;
            Ok(platform.code)
        }
    }
}

/// Assert one failed result with one code from the allowed set.
pub(crate) fn assert_platform_error_codes<T>(
    result: RuntimeResult<T>,
    expected: &[PlatformErrorCode],
) -> RuntimeResult<()> {
    // resolve platform error code and assert one of the expected values
    let actual = error_code(result)?;
    assert!(
        expected.contains(&actual),
        "unexpected platform error code {actual:?}, expected one of {expected:?}"
    );

    Ok(())
}

/// Assert one result is ok or fails with one expected platform error code.
pub(crate) fn assert_ok_or_expected_error<T>(
    result: RuntimeResult<T>,
    expected: &[PlatformErrorCode],
) -> RuntimeResult<Option<T>> {
    // accept either success or one allowed platform error code
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error) => {
            let Some(code) = error.platform_error().map(|platform| platform.code) else {
                return Err(error);
            };

            if expected.contains(&code) {
                return Ok(None);
            }

            Err(error)
        }
    }
}

/// Decode one harness value where native and vm payloads are the same ABI type.
pub(crate) fn decode_harness_value<T>(value: HarnessValue<T, T>) -> T {
    // unwrap the value regardless of native or vm lane
    match value {
        HarnessValue::Native(value) => value,
        HarnessValue::Vm(value) => value,
    }
}

/// Return one reserve flag payload with no bits set.
pub(crate) fn reserve_flags_none() -> memory::MemoryReserveFlags {
    memory::MemoryReserveFlags(0)
}

/// Return one remap flag payload that allows movement.
pub(crate) fn remap_flags_may_move() -> memory::MemoryRemapFlags {
    memory::MemoryRemapFlags(0x1)
}

/// Return one read-write memory protection mask.
pub(crate) fn protection_read_write() -> memory::MemoryProtection {
    memory::MemoryProtection(0x3)
}

/// Return one reserve length from one queried page size.
pub(crate) fn page_aligned_length(page_size: u64) -> u64 {
    page_size.max(4096)
}

/// Decode one reserve payload from one harness result.
pub(crate) fn decode_memory_range(
    value: HarnessValue<MemoryRange, memory::MemoryRangeVm>,
) -> MemoryRange {
    decode_harness_value(value)
}

/// Convert one `u64` mapping address into one writable test pointer.
pub(crate) fn writable_byte_pointer(address: u64) -> *mut u8 {
    // ensure test mapping addresses fit host pointer width
    let address = usize::try_from(address).expect("mapping address should fit host pointer width");

    // project to mutable byte pointer for direct memory probes
    address as *mut u8
}
