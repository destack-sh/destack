use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{PlatformError, VmSlice, resource};
use crate::runtime::BindingCallContext;
use destack_vm;

use crate::platform::security::SecurityPolicyRuleVm;

/// Check one capability.
///
/// Evaluate whether one capability is active in the current runtime context.
/// Capability checks use runtime policy rules and sandbox scope inheritance.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime security policy state.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `security.policy.read`.
///
/// # Replay
/// Deterministic.
pub(crate) fn destack_security_capability_has(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    capability: destack_vm::StringHandle,
) -> RuntimeResult<bool> {
    let _ = capability;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.capability.has is not available in the VM yet",
    ))
    .boxed())
}

/// List active capabilities.
///
/// Return the currently active runtime capability set for the executing context.
/// Capabilities are normalized to the canonical intrinsic capability string space.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime security policy state.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `security.policy.read`.
///
/// # Replay
/// Deterministic.
pub(crate) fn destack_security_capability_list(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<destack_vm::StringHandle>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.capability.list is not available in the VM yet",
    ))
    .boxed())
}

/// Seal one sandbox policy.
///
/// Transition one sandbox scope into sealed mode.
/// Sealed policy rejects further capability broadening.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime sandbox policy controls.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `security.restrict`.
///
/// # Replay
/// Deterministic.
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

/// Apply an explicit capability set to one sandbox scope.
///
/// Replace one sandbox capability set with explicit allow-list semantics.
/// Capability inheritance and scope visibility follow runtime policy rules.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime sandbox policy controls.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `security.restrict`.
///
/// # Replay
/// Deterministic.
pub(crate) fn destack_security_sandbox_set_capabilities(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::SandboxHandle,
    capabilities: VmSlice<destack_vm::StringHandle>,
) -> RuntimeResult<()> {
    let _ = (handle, capabilities);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.enforce.sandboxSetCapabilities is not available in the VM yet",
    ))
    .boxed())
}

/// Set runtime W^X policy.
///
/// Enable or disable runtime write-xor-execute policy enforcement.
/// Policy update affects subsequent executable-memory transitions.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime memory policy controls layered over host page protections.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `security.restrict`.
///
/// # Replay
/// Deterministic.
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

/// Read policy capabilities for one named scope.
///
/// Read one policy scope and return its effective capability set.
/// Scope resolution and inheritance follow runtime security policy rules.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime security policy state.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `security.policy.read`.
///
/// # Replay
/// Deterministic.
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
///
/// Read one policy scope and return explicit capability rules with allow, deny, and audit decisions.
/// Rule ordering and precedence follow runtime policy resolution semantics.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime security policy state.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `security.policy.read`.
///
/// # Replay
/// Deterministic.
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

/// Replace policy capabilities for one named scope.
///
/// Replace one policy scope with an explicit capability set.
/// Policy writes are validated against runtime security admin policy.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime security policy state.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `security.policy.write`.
///
/// # Replay
/// Deterministic.
pub(crate) fn destack_security_policy_set(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    scope: destack_vm::StringHandle,
    capabilities: VmSlice<destack_vm::StringHandle>,
) -> RuntimeResult<()> {
    let _ = (scope, capabilities);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.security.policy.set is not available in the VM yet",
    ))
    .boxed())
}

/// Replace structured policy rules for one named scope.
///
/// Replace one policy scope with explicit capability rules and decision modes.
/// Rule conflicts and precedence are validated by runtime policy authoring rules.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime security policy state.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `security.policy.write`.
///
/// # Replay
/// Deterministic.
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
///
/// Create and enter one new runtime sandbox scope.
/// Scope inherits baseline policy then applies runtime sandbox restrictions.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime sandbox scope state.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `security.sandbox`.
///
/// # Replay
/// External, recordable.
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
///
/// Exit one previously entered sandbox scope.
/// Scope unwinding follows strict runtime stack discipline.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime sandbox scope state.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `security.sandbox`.
///
/// # Replay
/// External, recordable.
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
