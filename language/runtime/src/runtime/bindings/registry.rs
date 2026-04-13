use std::collections::HashMap;

use destack_vm as vm;
use destack_vm::Isolate;
use parking_lot::RwLock;

use crate::platform;
use crate::runtime::bindings::{
    BindingDescriptor, BindingId, BindingPolicy, NativeBinding, NativeBindingSet,
};
use crate::runtime::capability::PlatformCapabilitySet;
use crate::runtime::{BindingCallContext, enter_binding_call_context};
use destack_workspace::RuntimeOptions;

use crate::runtime::bindings::VmBindingSet;
use destack_workspace::ExecutionMode;

/// Registry for external bindings and shims.
#[derive(Debug)]
pub struct BindingRegistry {
    /// Registered binding descriptors for policy enforcement.
    descriptors: Vec<BindingDescriptor>,
    /// Binding metadata indexed by binding id.
    descriptor_by_id: HashMap<BindingId, BindingDescriptor>,
    /// Native binding metadata indexed by binding id.
    native_by_id: HashMap<BindingId, NativeBinding>,
    /// Native binding metadata for linking.
    native_bindings: Vec<NativeBinding>,
    /// Policy configuration for external bindings.
    policy: RwLock<BindingPolicy>,
}

impl BindingRegistry {
    /// Create a new binding registry.
    pub fn new() -> Self {
        Self {
            descriptors: Vec::new(),
            descriptor_by_id: HashMap::new(),
            native_by_id: HashMap::new(),
            native_bindings: Vec::new(),
            policy: RwLock::new(BindingPolicy::new(ExecutionMode::Fast)),
        }
    }

    /// Borrow the live binding policy state.
    pub fn policy(&self) -> &RwLock<BindingPolicy> {
        &self.policy
    }

    /// Set the binding policy for this registry.
    pub fn set_policy(&mut self, policy: BindingPolicy) {
        // store the binding policy
        *self.policy.write() = policy;
    }

    /// Apply runtime defaults to binding policy without loading control rules.
    pub fn apply_runtime_defaults(&mut self, options: &RuntimeOptions) {
        let mut policy = self.policy.write();
        policy.apply_runtime_defaults(options);
    }

    /// Apply a capability set to policy checks and recompile descriptor decisions.
    pub fn set_capabilities(&mut self, capabilities: PlatformCapabilitySet) {
        let mut policy = self.policy.write();
        policy.set_capabilities(capabilities);
    }

    /// Set capability requirement enforcement mode and recompile descriptor decisions.
    pub fn set_capability_requirements_enforced(&mut self, is_enforced: bool) {
        let mut policy = self.policy.write();
        policy.set_capability_requirements_enforced(is_enforced);
    }

    /// Install default VM bindings into a VM isolate.
    #[allow(dead_code)]
    pub(crate) fn install_vm_defaults(&mut self, isolate: &mut Isolate) -> vm::Result<()> {
        for set in platform::PLATFORM_VM_BINDINGS {
            self.install_vm_binding_set(isolate, set)?;
        }

        Ok(())
    }

    /// Install default native bindings for the runtime.
    pub fn install_native_defaults(&mut self) {
        for set in platform::PLATFORM_NATIVE_BINDINGS {
            self.install_native_binding_set(set);
        }
    }

    /// Install a binding set into a VM isolate.
    #[allow(dead_code)]
    pub(crate) fn install_vm_binding_set(
        &mut self,
        isolate: &mut Isolate,
        set: &VmBindingSet,
    ) -> vm::Result<()> {
        // dispatch to the binding set install hook
        (set.install)(self, isolate)
    }

    /// Install a native binding set into the registry.
    pub(crate) fn install_native_binding_set(&mut self, set: &NativeBindingSet) {
        for binding in set.bindings {
            self.register_native_binding(*binding);
        }
    }

    /// Register native binding metadata.
    pub(crate) fn register_native_binding(&mut self, binding: NativeBinding) {
        if let Some(existing) = self.descriptor_by_id.get(&binding.spec.id)
            && *existing != binding.spec
        {
            panic!("binding id collision for {}", binding.spec.name);
        }

        if let Some(existing) = self.native_by_id.get(&binding.spec.id) {
            if existing.spec != binding.spec
                || existing.symbol != binding.symbol
                || existing.function != binding.function
            {
                panic!("native binding id collision for {}", binding.spec.name);
            }
            return;
        }

        self.descriptor_by_id.insert(binding.spec.id, binding.spec);
        self.descriptors.push(binding.spec);
        self.native_by_id.insert(binding.spec.id, binding);
        self.native_bindings.push(binding);
    }

    /// Register a VM binding handler with metadata.
    pub(crate) fn register_vm_binding(
        &mut self,
        isolate: &mut Isolate,
        descriptor: BindingDescriptor,
        handler: impl vm::ExternalHandler + 'static,
    ) {
        // hard error on descriptor mismatches
        let mut has_descriptor = false;
        if let Some(existing) = self.descriptor_by_id.get(&descriptor.id) {
            if *existing != descriptor {
                panic!("binding id collision for {}", descriptor.name);
            }
            has_descriptor = true;
        }

        // NOTE #Incomplete: serialize args/results for replay payloads
        // register the external handler through the live binding call context
        isolate.register_vm_binding(descriptor.name, move |context, args| {
            let call_context = BindingCallContext::from_current_agent_for_vm()?;
            let _guard = enter_binding_call_context(&call_context);
            handler(context, args)
        });

        // track the binding metadata for diagnostics
        if !has_descriptor {
            self.descriptor_by_id.insert(descriptor.id, descriptor);
            self.descriptors.push(descriptor);
        }
    }

    /// Return registered binding descriptors.
    pub fn descriptors(&self) -> &[BindingDescriptor] {
        self.descriptors.as_slice()
    }

    /// Return binding descriptors by id.
    pub fn descriptors_by_id(&self) -> &HashMap<BindingId, BindingDescriptor> {
        &self.descriptor_by_id
    }

    /// Return native binding metadata.
    pub fn native_bindings(&self) -> &[NativeBinding] {
        &self.native_bindings
    }
}
