use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::security::{PlatformCapabilityVm, SecurityFilterVm, SecurityPolicyRuleVm};
use crate::platform::{PlatformError, VmSlice, resource};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Stub for destack.security.capability.has.
pub(super) fn destack_security_capability_has(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    capability: PlatformCapabilityVm,
) -> RuntimeResult<bool> {
    let _ = capability;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.capability.has is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.security.capability.list.
pub(super) fn destack_security_capability_list(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<VmSlice<PlatformCapabilityVm>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.capability.list is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.security.enforce.sandboxInstallFilter.
pub(super) fn destack_security_sandbox_install_filter(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SandboxHandle,
    filter: SecurityFilterVm,
) -> RuntimeResult<()> {
    let _ = (handle, filter);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.enforce.sandboxInstallFilter is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.security.enforce.sandboxSeal.
pub(super) fn destack_security_sandbox_seal(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SandboxHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.enforce.sandboxSeal is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.security.enforce.sandboxSetCapabilities.
pub(super) fn destack_security_sandbox_set_capabilities(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SandboxHandle,
    capabilities: VmSlice<PlatformCapabilityVm>,
) -> RuntimeResult<()> {
    let _ = (handle, capabilities);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.enforce.sandboxSetCapabilities is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.security.policy.get.
pub(super) fn destack_security_policy_get(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    scope: vm::StringHandle,
) -> RuntimeResult<VmSlice<PlatformCapabilityVm>> {
    let _ = scope;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.policy.get is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.security.policy.getRules.
pub(super) fn destack_security_policy_get_rules(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    scope: vm::StringHandle,
) -> RuntimeResult<VmSlice<SecurityPolicyRuleVm>> {
    let _ = scope;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.policy.getRules is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.security.policy.set.
pub(super) fn destack_security_policy_set(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    scope: vm::StringHandle,
    capabilities: VmSlice<PlatformCapabilityVm>,
) -> RuntimeResult<()> {
    let _ = (scope, capabilities);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.policy.set is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.security.policy.setRules.
pub(super) fn destack_security_policy_set_rules(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    scope: vm::StringHandle,
    rules: VmSlice<SecurityPolicyRuleVm>,
) -> RuntimeResult<()> {
    let _ = (scope, rules);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.policy.setRules is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.security.sandbox.enter.
pub(super) fn destack_security_sandbox_enter(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    name: vm::StringHandle,
) -> RuntimeResult<resource::SandboxHandle> {
    let _ = name;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.sandbox.enter is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.security.sandbox.exit.
pub(super) fn destack_security_sandbox_exit(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SandboxHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.sandbox.exit is not available in the VM yet",
    ))
    .boxed())
}
