#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::abi::{NativeSlice, NativeStringRef, NativeStringSlice};

use crate::runtime::BindingCallContext;

use crate::platform::resource;
use crate::platform::security::SecurityPolicyRule;

/// Check one action.
pub(crate) unsafe fn destack_security_action_has(
    _binding: &BindingCallContext,
    out: *mut bool,
    action: NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, action);

    Err(RuntimeError::from(PlatformError::not_supported("destack.security.action.has")).boxed())
}

/// List active actions.
pub(crate) unsafe fn destack_security_action_list(
    _binding: &BindingCallContext,
    out: *mut NativeStringSlice,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.security.action.list")).boxed())
}

/// Seal one sandbox policy.
pub(crate) unsafe fn destack_security_sandbox_seal(
    _binding: &BindingCallContext,
    handle: resource::SandboxHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.enforce.sandboxSeal",
    ))
    .boxed())
}

/// Apply an explicit action set to one sandbox scope.
pub(crate) unsafe fn destack_security_sandbox_set_actions(
    _binding: &BindingCallContext,
    handle: resource::SandboxHandle,
    actions: NativeStringSlice,
) -> RuntimeResult<()> {
    let _ = (handle, actions);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.enforce.sandboxSetActions",
    ))
    .boxed())
}

/// Set runtime W^X policy.
pub(crate) unsafe fn destack_security_set_write_xor_execute(
    _binding: &BindingCallContext,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = enabled;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.enforce.setWriteXorExecute",
    ))
    .boxed())
}

/// Read policy actions for one named scope.
pub(crate) unsafe fn destack_security_policy_get(
    _binding: &BindingCallContext,
    out: *mut NativeStringSlice,
    scope: NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, scope);

    Err(RuntimeError::from(PlatformError::not_supported("destack.security.policy.get")).boxed())
}

/// Read structured policy rules for one named scope.
pub(crate) unsafe fn destack_security_policy_get_rules(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<SecurityPolicyRule>,
    scope: NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, scope);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.policy.getRules",
    ))
    .boxed())
}

/// Replace policy actions for one named scope.
pub(crate) unsafe fn destack_security_policy_set(
    _binding: &BindingCallContext,
    scope: NativeStringRef,
    actions: NativeStringSlice,
) -> RuntimeResult<()> {
    let _ = (scope, actions);

    Err(RuntimeError::from(PlatformError::not_supported("destack.security.policy.set")).boxed())
}

/// Replace structured policy rules for one named scope.
pub(crate) unsafe fn destack_security_policy_set_rules(
    _binding: &BindingCallContext,
    scope: NativeStringRef,
    rules: NativeSlice<SecurityPolicyRule>,
) -> RuntimeResult<()> {
    let _ = (scope, rules);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.policy.setRules",
    ))
    .boxed())
}

/// Enter a sandbox scope.
pub(crate) unsafe fn destack_security_sandbox_enter(
    _binding: &BindingCallContext,
    out: *mut resource::SandboxHandle,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, name);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.sandbox.enter",
    ))
    .boxed())
}

/// Leave a sandbox scope.
pub(crate) unsafe fn destack_security_sandbox_exit(
    _binding: &BindingCallContext,
    handle: resource::SandboxHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.sandbox.exit",
    ))
    .boxed())
}
