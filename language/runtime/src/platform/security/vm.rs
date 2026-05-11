use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{PlatformError, VmSlice, resource};
use crate::runtime::BindingCallContext;
use destack_vm;

use crate::platform::security::SecurityPolicyRuleVm;

/// Check one action.
pub(crate) fn destack_security_action_has(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    action: destack_vm::StringHandle,
) -> RuntimeResult<bool> {
    let _ = action;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.action.has is not available in the VM yet",
    ))
    .boxed())
}

/// List active actions.
pub(crate) fn destack_security_action_list(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<destack_vm::StringHandle>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.action.list is not available in the VM yet",
    ))
    .boxed())
}

/// Seal one sandbox policy.
pub(crate) fn destack_security_sandbox_seal(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::SandboxHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.enforce.sandboxSeal is not available in the VM yet",
    ))
    .boxed())
}

/// Apply an explicit action set to one sandbox scope.
pub(crate) fn destack_security_sandbox_set_actions(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::SandboxHandle,
    actions: VmSlice<destack_vm::StringHandle>,
) -> RuntimeResult<()> {
    let _ = (handle, actions);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.enforce.sandboxSetActions is not available in the VM yet",
    ))
    .boxed())
}

/// Set runtime W^X policy.
pub(crate) fn destack_security_set_write_xor_execute(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = enabled;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.enforce.setWriteXorExecute is not available in the VM yet",
    ))
    .boxed())
}

/// Read policy actions for one named scope.
pub(crate) fn destack_security_policy_get(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    scope: destack_vm::StringHandle,
) -> RuntimeResult<VmSlice<destack_vm::StringHandle>> {
    let _ = scope;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.policy.get is not available in the VM yet",
    ))
    .boxed())
}

/// Read structured policy rules for one named scope.
pub(crate) fn destack_security_policy_get_rules(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    scope: destack_vm::StringHandle,
) -> RuntimeResult<VmSlice<SecurityPolicyRuleVm>> {
    let _ = scope;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.policy.getRules is not available in the VM yet",
    ))
    .boxed())
}

/// Replace policy actions for one named scope.
pub(crate) fn destack_security_policy_set(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    scope: destack_vm::StringHandle,
    actions: VmSlice<destack_vm::StringHandle>,
) -> RuntimeResult<()> {
    let _ = (scope, actions);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.policy.set is not available in the VM yet",
    ))
    .boxed())
}

/// Replace structured policy rules for one named scope.
pub(crate) fn destack_security_policy_set_rules(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    scope: destack_vm::StringHandle,
    rules: VmSlice<SecurityPolicyRuleVm>,
) -> RuntimeResult<()> {
    let _ = (scope, rules);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.policy.setRules is not available in the VM yet",
    ))
    .boxed())
}

/// Enter a sandbox scope.
pub(crate) fn destack_security_sandbox_enter(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    name: destack_vm::StringHandle,
) -> RuntimeResult<resource::SandboxHandle> {
    let _ = name;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.sandbox.enter is not available in the VM yet",
    ))
    .boxed())
}

/// Leave a sandbox scope.
pub(crate) fn destack_security_sandbox_exit(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::SandboxHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.sandbox.exit is not available in the VM yet",
    ))
    .boxed())
}
