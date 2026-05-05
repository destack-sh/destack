#![allow(dead_code)]

use std::sync::{Mutex, OnceLock};

use crate::diagnostic::RuntimeResult;
use crate::runtime::BindingCallContext;
use crate::tests::runtime::TestRuntime;
use destack_vm as vm;
use destack_workspace::RuntimeOptions;

#[path = "harness.generated.rs"]
mod generated;
pub(crate) use generated::HarnessValue;

/// Test harness context.
pub(crate) struct OsHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call BindingCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

/// The local alias used by handwritten tests.
pub(crate) type HarnessContext<'call> = OsHarnessContext<'call>;

impl OsHarnessContext<'_> {
    /// Return the host runtime id for this harness context.
    pub(crate) fn runtime_id(&self) -> u64 {
        self.call_context.host().host_session_id().0
    }
}

/// Shared mutex that serializes harness tests.
static OS_TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum Harness {
    /// Native OS harness.
    Native(TestRuntime),
    /// VM OS harness.
    Vm(TestRuntime),
}

impl Harness {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&mut self, callback: F) -> R
    where
        F: for<'call> FnOnce(HarnessContext<'call>) -> R,
    {
        match self {
            Harness::Native(runtime) => runtime.with_native_call_context(|call_context| {
                callback(OsHarnessContext {
                    call_context,
                    vm_context: None,
                })
            }),
            Harness::Vm(runtime) => runtime.with_vm_call_context(|call_context, vm_context| {
                let vm_context = vm_context as *mut vm::BindingContext<'_> as *mut ();

                callback(OsHarnessContext {
                    call_context,
                    vm_context: Some(vm_context),
                })
            }),
        }
    }
}

/// Build one deterministic OS test runtime.
fn os_test_runtime() -> TestRuntime {
    let runtime = TestRuntime::deterministic_random_without_host_ambient_ingress();
    runtime.drain_host_events();

    runtime
}

/// Build one deterministic OS test runtime with custom option seeding.
fn os_test_runtime_with_options(configure: fn(&mut RuntimeOptions)) -> TestRuntime {
    let runtime =
        TestRuntime::deterministic_random_without_host_ambient_ingress_with_options(configure);
    runtime.drain_host_events();

    runtime
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&mut Harness),
{
    // serialize harness runtimes because host callback registries are process-global
    let _guard = OS_TEST_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    let mut native = Harness::Native(os_test_runtime());
    callback(&mut native);
    let mut vm = Harness::Vm(os_test_runtime());
    callback(&mut vm);
}

/// Run one callback against both harnesses with custom runtime options.
pub(crate) fn with_configured_harnesses<F>(configure: fn(&mut RuntimeOptions), mut callback: F)
where
    F: FnMut(&mut Harness),
{
    // serialize harness runtimes because host callback registries are process-global
    let _guard = OS_TEST_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    let mut native = Harness::Native(os_test_runtime_with_options(configure));
    callback(&mut native);
    let mut vm = Harness::Vm(os_test_runtime_with_options(configure));
    callback(&mut vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(HarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness
            .with_context(&mut callback)
            .expect("os harness call should succeed");
    });
}

/// Run one callback against both harness contexts with custom runtime options.
pub(crate) fn with_configured_harness_context<F>(
    configure: fn(&mut RuntimeOptions),
    mut callback: F,
) where
    F: for<'call> FnMut(HarnessContext<'call>) -> RuntimeResult<()>,
{
    with_configured_harnesses(configure, |harness| {
        harness
            .with_context(&mut callback)
            .expect("os harness call should succeed");
    });
}
