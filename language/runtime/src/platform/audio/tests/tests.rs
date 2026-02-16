#![cfg_attr(windows, allow(dead_code, unused_imports))]

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::runtime::RuntimeCallContext;
use crate::tests::runtime::TestRuntime;

#[path = "harness.generated.rs"]
mod harness;

/// Test harness context used by tests.
pub(crate) struct AudioHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call RuntimeCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

/// Native audio harness.
pub(crate) struct NativeAudioHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeAudioHarness {
    /// Create a new native audio harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM audio harness.
pub(crate) struct VmAudioHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmAudioHarness {
    /// Create a new VM audio harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum AudioHarnessHandle {
    /// Native audio harness.
    Native(NativeAudioHarness),
    /// VM audio harness.
    Vm(VmAudioHarness),
}

impl AudioHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&self, callback: F) -> R
    where
        F: for<'call> FnOnce(AudioHarnessContext<'call>) -> R,
    {
        match self {
            AudioHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(AudioHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            AudioHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();
                        callback(AudioHarnessContext {
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
        F: for<'call> FnOnce(AudioHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("audio harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&AudioHarnessHandle),
{
    let native = AudioHarnessHandle::Native(NativeAudioHarness::new());
    callback(&native);
    let vm = AudioHarnessHandle::Vm(VmAudioHarness::new());
    callback(&vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(AudioHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}
