#![cfg_attr(windows, allow(dead_code, unused_imports))]

#[path = "harness.rs"]
pub(crate) mod harness;

use destack_vm as vm;
use std::sync::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::resource::ResourceId;
use crate::platform::{crypto as crypto_platform, resource};
use crate::runtime::BindingCallContext;
pub(crate) use crate::tests::platform::{
    assert_not_not_supported_platform_code, assert_not_supported_platform_code,
    error_code_from_runtime_error, is_not_supported_code,
};
use crate::tests::runtime::TestRuntime;

/// Global lock to serialize host-sensitive crypto tests.
static CRYPTO_TEST_LOCK: Mutex<()> = Mutex::new(());

/// Key usage bit: sign.
pub(crate) const KEY_USAGE_SIGN: u32 = crypto_platform::CRYPTO_KEY_USAGE_SIGN.0;
/// Key usage bit: verify.
pub(crate) const KEY_USAGE_VERIFY: u32 = crypto_platform::CRYPTO_KEY_USAGE_VERIFY.0;
/// Key usage bit: encrypt.
pub(crate) const KEY_USAGE_ENCRYPT: u32 = crypto_platform::CRYPTO_KEY_USAGE_ENCRYPT.0;
/// Key usage bit: decrypt.
pub(crate) const KEY_USAGE_DECRYPT: u32 = crypto_platform::CRYPTO_KEY_USAGE_DECRYPT.0;
/// Key usage bit: wrap.
pub(crate) const KEY_USAGE_WRAP: u32 = crypto_platform::CRYPTO_KEY_USAGE_WRAP.0;
/// Key usage bit: unwrap.
pub(crate) const KEY_USAGE_UNWRAP: u32 = crypto_platform::CRYPTO_KEY_USAGE_UNWRAP.0;
/// Key usage bit: derive bits.
pub(crate) const KEY_USAGE_DERIVE_BITS: u32 = crypto_platform::CRYPTO_KEY_USAGE_DERIVE_BITS.0;
/// Key usage bit: derive keys.
pub(crate) const KEY_USAGE_DERIVE_KEYS: u32 = crypto_platform::CRYPTO_KEY_USAGE_DERIVE_KEYS.0;
/// Key usage bit: export.
pub(crate) const KEY_USAGE_EXPORT: u32 = crypto_platform::CRYPTO_KEY_USAGE_EXPORT.0;

/// Test harness context used by tests.
pub(crate) struct CryptoHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call BindingCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

/// Native crypto harness.
pub(crate) struct NativeCryptoHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeCryptoHarness {
    /// Create a new native crypto harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM crypto harness.
pub(crate) struct VmCryptoHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmCryptoHarness {
    /// Create a new VM crypto harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum CryptoHarnessHandle {
    /// Native crypto harness.
    Native(NativeCryptoHarness),
    /// VM crypto harness.
    Vm(VmCryptoHarness),
}

impl CryptoHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&mut self, callback: F) -> R
    where
        F: for<'call> FnOnce(CryptoHarnessContext<'call>) -> R,
    {
        // dispatch callback through the selected harness runtime
        match self {
            CryptoHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(CryptoHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            CryptoHarnessHandle::Vm(harness) => {
                // pass one raw vm context pointer through the shared harness context
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::BindingContext<'_> as *mut ();
                        callback(CryptoHarnessContext {
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
        F: for<'call> FnOnce(CryptoHarnessContext<'call>) -> RuntimeResult<()>,
    {
        // execute one harness callback and fail loud on unexpected runtime errors
        self.with_context(callback)
            .expect("crypto harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&mut CryptoHarnessHandle),
{
    // serialize crypto test harness runs to avoid host keychain contention
    let _lock_guard = CRYPTO_TEST_LOCK
        .lock()
        .expect("crypto test lock should not be poisoned");

    // execute callback against native bindings
    let mut native = CryptoHarnessHandle::Native(NativeCryptoHarness::new());
    callback(&mut native);

    // execute callback against vm bindings
    let mut vm = CryptoHarnessHandle::Vm(VmCryptoHarness::new());
    callback(&mut vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(CryptoHarnessContext<'call>) -> RuntimeResult<()>,
{
    // bridge harness handle dispatch into one shared callback signature
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}

/// Return one placeholder store handle for invalid-handle tests.
pub(crate) fn placeholder_store_handle() -> resource::CryptoStoreHandle {
    resource::CryptoStoreHandle(ResourceId::local(1))
}
