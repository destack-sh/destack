#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::security::bindings_generated as bindings;
use crate::platform::{NativeSlice, NativeStringRef, PlatformError};

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::resource;
use crate::platform::security::{PlatformCapability, SecurityFilter, SecurityPolicyRule};

/// Stub for destack.security.capability.has.
pub unsafe fn destack_security_capability_has(
    context: &RuntimeCallContext,
    out: *mut bool,
    capability: PlatformCapability,
) -> RuntimeResult<()> {
    context.check_policy(SECURITY_CAPABILITY_HAS)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, capability);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.capability.has",
    ))
    .boxed())
}

/// Stub for destack.security.capability.list.
pub unsafe fn destack_security_capability_list(
    context: &RuntimeCallContext,
    out: *mut NativeSlice<PlatformCapability>,
) -> RuntimeResult<()> {
    context.check_policy(SECURITY_CAPABILITY_LIST)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.capability.list",
    ))
    .boxed())
}

/// Stub for destack.security.enforce.sandboxInstallFilter.
pub unsafe fn destack_security_sandbox_install_filter(
    context: &RuntimeCallContext,
    handle: resource::SandboxHandle,
    filter: SecurityFilter,
) -> RuntimeResult<()> {
    context.check_policy(SECURITY_ENFORCE_SANDBOX_INSTALL_FILTER)?;
    let _ = (handle, filter);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.enforce.sandboxInstallFilter",
    ))
    .boxed())
}

/// Stub for destack.security.enforce.sandboxSeal.
pub unsafe fn destack_security_sandbox_seal(
    context: &RuntimeCallContext,
    handle: resource::SandboxHandle,
) -> RuntimeResult<()> {
    context.check_policy(SECURITY_ENFORCE_SANDBOX_SEAL)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.enforce.sandboxSeal",
    ))
    .boxed())
}

/// Stub for destack.security.enforce.sandboxSetCapabilities.
pub unsafe fn destack_security_sandbox_set_capabilities(
    context: &RuntimeCallContext,
    handle: resource::SandboxHandle,
    capabilities: NativeSlice<PlatformCapability>,
) -> RuntimeResult<()> {
    context.check_policy(SECURITY_ENFORCE_SANDBOX_SET_CAPABILITIES)?;
    let _ = (handle, capabilities);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.enforce.sandboxSetCapabilities",
    ))
    .boxed())
}

/// Stub for destack.security.policy.get.
pub unsafe fn destack_security_policy_get(
    context: &RuntimeCallContext,
    out: *mut NativeSlice<PlatformCapability>,
    scope: NativeStringRef,
) -> RuntimeResult<()> {
    context.check_policy(SECURITY_POLICY_GET)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, scope);

    Err(RuntimeError::from(PlatformError::not_supported("destack.security.policy.get")).boxed())
}

/// Stub for destack.security.policy.getRules.
pub unsafe fn destack_security_policy_get_rules(
    context: &RuntimeCallContext,
    out: *mut NativeSlice<SecurityPolicyRule>,
    scope: NativeStringRef,
) -> RuntimeResult<()> {
    context.check_policy(SECURITY_POLICY_GET_RULES)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, scope);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.policy.getRules",
    ))
    .boxed())
}

/// Stub for destack.security.policy.set.
pub unsafe fn destack_security_policy_set(
    context: &RuntimeCallContext,
    scope: NativeStringRef,
    capabilities: NativeSlice<PlatformCapability>,
) -> RuntimeResult<()> {
    context.check_policy(SECURITY_POLICY_SET)?;
    let _ = (scope, capabilities);

    Err(RuntimeError::from(PlatformError::not_supported("destack.security.policy.set")).boxed())
}

/// Stub for destack.security.policy.setRules.
pub unsafe fn destack_security_policy_set_rules(
    context: &RuntimeCallContext,
    scope: NativeStringRef,
    rules: NativeSlice<SecurityPolicyRule>,
) -> RuntimeResult<()> {
    context.check_policy(SECURITY_POLICY_SET_RULES)?;
    let _ = (scope, rules);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.policy.setRules",
    ))
    .boxed())
}

/// Stub for destack.security.sandbox.enter.
pub unsafe fn destack_security_sandbox_enter(
    context: &RuntimeCallContext,
    out: *mut resource::SandboxHandle,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    context.check_policy(SECURITY_SANDBOX_ENTER)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, name);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.sandbox.enter",
    ))
    .boxed())
}

/// Stub for destack.security.sandbox.exit.
pub unsafe fn destack_security_sandbox_exit(
    context: &RuntimeCallContext,
    handle: resource::SandboxHandle,
) -> RuntimeResult<()> {
    context.check_policy(SECURITY_SANDBOX_EXIT)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.sandbox.exit",
    ))
    .boxed())
}
