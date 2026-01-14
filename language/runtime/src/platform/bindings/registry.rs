use destack_vm::{ExternalHandler, Isolate};

use super::{BindingDescriptor, BindingPolicy, BindingSet};
use crate::platform::host::{self, HostContext};

/// Registry for external bindings and shims.
#[derive(Debug, Default)]
pub struct BindingRegistry {
    /// Registered binding specs for policy enforcement.
    specs: Vec<BindingDescriptor>,
    /// Policy configuration for external bindings.
    policy: BindingPolicy,
}

impl BindingRegistry {
    /// Create a new binding registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the binding policy for this registry.
    pub fn set_policy(&mut self, policy: BindingPolicy) {
        // store the binding policy
        self.policy = policy;
    }

    /// Get the binding policy for this registry.
    pub fn policy(&self) -> BindingPolicy {
        self.policy
    }

    /// Install default VM bindings into a VM isolate.
    pub fn install_defaults(&mut self, isolate: &mut Isolate, host: &HostContext) {
        // install built in host bindings
        self.install_set(isolate, host, &host::console::CONSOLE_VM_BINDINGS);
        self.install_set(isolate, host, &host::process::PROCESS_VM_BINDINGS);
    }

    /// Install a binding set into a VM isolate.
    pub fn install_set(&mut self, isolate: &mut Isolate, host: &HostContext, set: &BindingSet) {
        // dispatch to the binding set install hook
        (set.install)(self, isolate, host);
    }

    /// Register a binding handler with metadata.
    pub fn register_external(
        &mut self,
        isolate: &mut Isolate,
        spec: BindingDescriptor,
        handler: impl ExternalHandler + 'static,
    ) {
        // snapshot policy for the installed handler
        let policy = self.policy;

        // register the external handler with policy enforcement
        isolate.register_external(spec.name, move |context, args| {
            policy.check(spec)?;
            handler(context, args)
        });

        // track the binding metadata for diagnostics
        self.specs.push(spec);
    }

    /// Return registered binding specs.
    pub fn specs(&self) -> &[BindingDescriptor] {
        self.specs.as_slice()
    }
}
