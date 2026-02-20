use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::security::{PlatformCapabilityVm, SecurityPolicyRuleVm, vm as security_vm};
use crate::platform::{VmSlice, resource};
use crate::runtime::BindingCallContext;

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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    capability: PlatformCapabilityVm,
) -> RuntimeResult<bool> {
    security_vm::destack_security_capability_has(runtime, context, capability)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<PlatformCapabilityVm>> {
    security_vm::destack_security_capability_list(runtime, context)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    scope: vm::StringHandle,
) -> RuntimeResult<VmSlice<PlatformCapabilityVm>> {
    security_vm::destack_security_policy_get(runtime, context, scope)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    scope: vm::StringHandle,
) -> RuntimeResult<VmSlice<SecurityPolicyRuleVm>> {
    security_vm::destack_security_policy_get_rules(runtime, context, scope)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    scope: vm::StringHandle,
    capabilities: VmSlice<PlatformCapabilityVm>,
) -> RuntimeResult<()> {
    security_vm::destack_security_policy_set(runtime, context, scope, capabilities)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    scope: vm::StringHandle,
    rules: VmSlice<SecurityPolicyRuleVm>,
) -> RuntimeResult<()> {
    security_vm::destack_security_policy_set_rules(runtime, context, scope, rules)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    name: vm::StringHandle,
) -> RuntimeResult<resource::SandboxHandle> {
    security_vm::destack_security_sandbox_enter(runtime, context, name)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SandboxHandle,
) -> RuntimeResult<()> {
    security_vm::destack_security_sandbox_exit(runtime, context, handle)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SandboxHandle,
) -> RuntimeResult<()> {
    security_vm::destack_security_sandbox_seal(runtime, context, handle)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SandboxHandle,
    capabilities: VmSlice<PlatformCapabilityVm>,
) -> RuntimeResult<()> {
    security_vm::destack_security_sandbox_set_capabilities(runtime, context, handle, capabilities)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    enabled: bool,
) -> RuntimeResult<()> {
    security_vm::destack_security_set_write_xor_execute(runtime, context, enabled)
}
