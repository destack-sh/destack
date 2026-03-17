#![cfg_attr(feature = "execution", allow(dead_code))]

use std::sync::{Mutex, OnceLock};

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::host::core::HostRuntimeRegistry;
use crate::host::{
    HostEvent, HostIntentEvent, HostIntentPayload, HostLifecycleEvent, HostLifecycleState,
    HostMemoryPressureEvent, HostMemoryPressureLevel, HostPermissionEvent, HostPowerMode,
    HostPowerModeEvent,
};
use crate::runtime::BindingCallContext;
use crate::tests::runtime::TestRuntime;

#[path = "harness.generated.rs"]
mod harness;
pub(crate) use harness::HarnessValue;

/// Test harness context used by tests.
pub(crate) struct OsHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call BindingCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

impl OsHarnessContext<'_> {
    /// Return the host runtime id for this harness context.
    pub(crate) fn runtime_id(&self) -> u64 {
        self.call_context.host().runtime_id().0
    }

    /// Dispatch one host event into the runtime-owned observer path.
    pub(crate) fn dispatch_host_event(&self, event: HostEvent) -> RuntimeResult<()> {
        HostRuntimeRegistry::dispatch_host_event(self.call_context.host().runtime_id(), &event)
    }

    /// Enqueue one host lifecycle transition for this harness runtime.
    pub(crate) fn enqueue_lifecycle_event(&self, state: HostLifecycleState) -> RuntimeResult<()> {
        self.dispatch_host_event(HostEvent::Lifecycle(HostLifecycleEvent { state }))
    }

    /// Enqueue one host memory-pressure event for this harness runtime.
    pub(crate) fn enqueue_memory_pressure_event(
        &self,
        level: HostMemoryPressureLevel,
    ) -> RuntimeResult<()> {
        self.dispatch_host_event(HostEvent::MemoryPressure(HostMemoryPressureEvent { level }))
    }

    /// Enqueue one host power-mode event for this harness runtime.
    pub(crate) fn enqueue_power_mode_event(&self, mode: HostPowerMode) -> RuntimeResult<()> {
        self.dispatch_host_event(HostEvent::PowerMode(HostPowerModeEvent { mode }))
    }

    /// Enqueue one host permission result for this harness runtime.
    pub(crate) fn enqueue_permission_event(
        &self,
        permission: &str,
        granted: bool,
    ) -> RuntimeResult<()> {
        self.dispatch_host_event(HostEvent::Permission(HostPermissionEvent {
            permission: permission.to_string(),
            granted,
        }))
    }

    /// Enqueue one open-url intent event for this harness runtime.
    pub(crate) fn enqueue_intent_open_url_event(
        &self,
        source: Option<&str>,
        url: &str,
    ) -> RuntimeResult<()> {
        self.dispatch_host_event(HostEvent::Intent(HostIntentEvent {
            source: source.map(str::to_string),
            payload: HostIntentPayload::OpenUrl {
                url: url.to_string(),
            },
        }))
    }

    /// Enqueue one open-file intent event for this harness runtime.
    pub(crate) fn enqueue_intent_open_file_event(
        &self,
        source: Option<&str>,
        path: &str,
        mime_type: Option<&str>,
    ) -> RuntimeResult<()> {
        self.dispatch_host_event(HostEvent::Intent(HostIntentEvent {
            source: source.map(str::to_string),
            payload: HostIntentPayload::OpenFile {
                path: path.to_string(),
                mime_type: mime_type.map(str::to_string),
            },
        }))
    }
}

/// Native os harness.
pub(crate) struct NativeOsHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeOsHarness {
    /// Create a new native os harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: os_test_runtime(),
        }
    }
}

/// VM os harness.
pub(crate) struct VmOsHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmOsHarness {
    /// Create a new VM os harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: os_test_runtime(),
        }
    }
}

/// Build one deterministic os test runtime.
fn os_test_runtime() -> TestRuntime {
    // keep os harness expectations focused on injected events
    let runtime = TestRuntime::deterministic_random();
    runtime.drain_host_events();

    runtime
}

/// Shared mutex that serializes os harness tests.
static OS_TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum OsHarnessHandle {
    /// Native os harness.
    Native(NativeOsHarness),
    /// VM os harness.
    Vm(VmOsHarness),
}

impl OsHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&self, callback: F) -> R
    where
        F: for<'call> FnOnce(OsHarnessContext<'call>) -> R,
    {
        match self {
            OsHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(OsHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            OsHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();
                        callback(OsHarnessContext {
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
        F: for<'call> FnOnce(OsHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("os harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&OsHarnessHandle),
{
    // serialize os harness runtimes because host callback registries are process-global
    let _guard = OS_TEST_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    let native = OsHarnessHandle::Native(NativeOsHarness::new());
    callback(&native);
    let vm = OsHarnessHandle::Vm(VmOsHarness::new());
    callback(&vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(OsHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}
