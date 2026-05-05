#![cfg_attr(windows, allow(dead_code, unused_imports))]

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::memory;
use crate::platform::memory::MemoryRange;
use crate::runtime::BindingCallContext;
pub(crate) use crate::tests::platform::{
    assert_ok_or_expected_error, assert_platform_error_codes, result_or_skip_not_supported,
};
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
    pub(crate) fn with_context<F, R>(&mut self, callback: F) -> R
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
                        let vm_context = vm_context as *mut vm::BindingContext<'_> as *mut ();
                        callback(MemoryHarnessContext {
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
    F: FnMut(&mut MemoryHarnessHandle),
{
    // execute callback against native harness
    let mut native = MemoryHarnessHandle::Native(NativeMemoryHarness::new());
    callback(&mut native);

    // execute callback against vm harness
    let mut vm = MemoryHarnessHandle::Vm(VmMemoryHarness::new());
    callback(&mut vm);
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
