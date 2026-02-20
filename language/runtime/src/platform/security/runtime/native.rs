use crate::diagnostic::RuntimeResult;
use crate::platform::security::{
    PlatformCapability, SecurityPolicyRule, native as security_native,
};
use crate::platform::{NativeSlice, NativeStringRef};
use crate::runtime::BindingCallContext;

use crate::platform::resource;

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
pub(crate) unsafe fn destack_security_capability_has(
    context: &BindingCallContext,
    out: *mut bool,
    capability: PlatformCapability,
) -> RuntimeResult<()> {
    unsafe { security_native::destack_security_capability_has(context, out, capability) }
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
pub(crate) unsafe fn destack_security_capability_list(
    context: &BindingCallContext,
    out: *mut NativeSlice<PlatformCapability>,
) -> RuntimeResult<()> {
    unsafe { security_native::destack_security_capability_list(context, out) }
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
pub(crate) unsafe fn destack_security_policy_get(
    context: &BindingCallContext,
    out: *mut NativeSlice<PlatformCapability>,
    scope: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe { security_native::destack_security_policy_get(context, out, scope) }
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
pub(crate) unsafe fn destack_security_policy_get_rules(
    context: &BindingCallContext,
    out: *mut NativeSlice<SecurityPolicyRule>,
    scope: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe { security_native::destack_security_policy_get_rules(context, out, scope) }
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
pub(crate) unsafe fn destack_security_policy_set(
    context: &BindingCallContext,
    scope: NativeStringRef,
    capabilities: NativeSlice<PlatformCapability>,
) -> RuntimeResult<()> {
    unsafe { security_native::destack_security_policy_set(context, scope, capabilities) }
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
pub(crate) unsafe fn destack_security_policy_set_rules(
    context: &BindingCallContext,
    scope: NativeStringRef,
    rules: NativeSlice<SecurityPolicyRule>,
) -> RuntimeResult<()> {
    unsafe { security_native::destack_security_policy_set_rules(context, scope, rules) }
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
pub(crate) unsafe fn destack_security_sandbox_enter(
    context: &BindingCallContext,
    out: *mut resource::SandboxHandle,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe { security_native::destack_security_sandbox_enter(context, out, name) }
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
pub(crate) unsafe fn destack_security_sandbox_exit(
    context: &BindingCallContext,
    handle: resource::SandboxHandle,
) -> RuntimeResult<()> {
    unsafe { security_native::destack_security_sandbox_exit(context, handle) }
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
pub(crate) unsafe fn destack_security_sandbox_seal(
    context: &BindingCallContext,
    handle: resource::SandboxHandle,
) -> RuntimeResult<()> {
    unsafe { security_native::destack_security_sandbox_seal(context, handle) }
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
pub(crate) unsafe fn destack_security_sandbox_set_capabilities(
    context: &BindingCallContext,
    handle: resource::SandboxHandle,
    capabilities: NativeSlice<PlatformCapability>,
) -> RuntimeResult<()> {
    unsafe {
        security_native::destack_security_sandbox_set_capabilities(context, handle, capabilities)
    }
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
pub(crate) unsafe fn destack_security_set_write_xor_execute(
    context: &BindingCallContext,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { security_native::destack_security_set_write_xor_execute(context, enabled) }
}
