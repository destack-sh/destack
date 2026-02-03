use std::collections::HashMap;

use destack_vm as vm;
use destack_vm::Isolate;

use crate::platform;
use crate::platform::bindings::{
    BindingDescriptor, BindingId, BindingPolicy, NativeBinding, NativeBindingSet, VmBindingSet,
};
use crate::runtime::{
    RuntimeCallContext, RuntimeContext, RuntimeState, enter_runtime_call_context,
};
use crate::scheduler::Scheduler;

/// Raw pointers captured for binding calls.
#[derive(Debug, Clone, Copy)]
struct RuntimeHandle {
    /// Pointer to the shared runtime state.
    runtime: *const RuntimeState,
    /// Pointer to the scheduler instance.
    scheduler: *const Scheduler,
}

impl RuntimeHandle {
    /// Return the runtime state pointer.
    pub(crate) const fn runtime_ptr(self) -> *const RuntimeState {
        self.runtime
    }

    /// Return the scheduler pointer.
    pub(crate) const fn scheduler_ptr(self) -> *const Scheduler {
        self.scheduler
    }
}

// safety: pointers are immutable and outlive the registered handlers
unsafe impl Send for RuntimeHandle {}
// safety: pointers are immutable and outlive the registered handlers
unsafe impl Sync for RuntimeHandle {}

/// Registry for external bindings and shims.
#[derive(Debug, Default)]
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
    policy: BindingPolicy,
    /// Runtime handle for binding calls.
    runtime_handle: Option<RuntimeHandle>,
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

    /// Set runtime handles for binding calls.
    pub fn set_runtime_handle(&mut self, runtime: &RuntimeContext, scheduler: &Scheduler) {
        // record runtime state pointers for call contexts
        self.runtime_handle = Some(RuntimeHandle {
            runtime: runtime.state_ptr(),
            scheduler: scheduler as *const Scheduler,
        });
    }

    /// Install default VM bindings into a VM isolate.
    pub fn install_vm_defaults(&mut self, isolate: &mut Isolate) {
        // install built in platform bindings
        for set in platform::PLATFORM_VM_BINDINGS {
            self.install_vm_binding_set(isolate, set);
        }
    }

    /// Install default native bindings for the runtime.
    pub fn install_native_defaults(&mut self) {
        for set in platform::PLATFORM_NATIVE_BINDINGS {
            self.install_native_binding_set(set);
        }
    }

    /// Install a binding set into a VM isolate.
    pub fn install_vm_binding_set(&mut self, isolate: &mut Isolate, set: &VmBindingSet) {
        // dispatch to the binding set install hook
        (set.install)(self, isolate);
    }

    /// Install a native binding set into the registry.
    pub fn install_native_binding_set(&mut self, set: &NativeBindingSet) {
        for binding in set.bindings {
            self.register_native_binding(*binding);
        }
    }

    /// Register native binding metadata.
    pub fn register_native_binding(&mut self, binding: NativeBinding) {
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
    pub fn register_vm_binding(
        &mut self,
        isolate: &mut Isolate,
        descriptor: BindingDescriptor,
        handler: impl vm::ExternalHandler + 'static,
    ) {
        // hard error on duplicate registrations
        if let Some(existing) = self.descriptor_by_id.get(&descriptor.id) {
            if *existing != descriptor {
                panic!("binding id collision for {}", descriptor.name);
            }
            return;
        }

        // snapshot policy for the installed handler
        let policy = self.policy;
        let handles = self.runtime_handle.unwrap_or_else(|| {
            panic!(
                "binding registry missing runtime handles for {}",
                descriptor.name
            )
        });

        // NOTE #Incomplete: serialize args/results for replay payloads
        // register the external handler with policy enforcement
        isolate.register_vm_binding(descriptor.name, move |context, args| {
            policy.check(descriptor)?;
            let call_context = RuntimeCallContext::from_raw(
                handles.runtime_ptr(),
                handles.scheduler_ptr(),
                policy,
            );
            let _guard = enter_runtime_call_context(&call_context);
            handler(context, args)
        });

        // track the binding metadata for diagnostics
        self.descriptor_by_id.insert(descriptor.id, descriptor);
        self.descriptors.push(descriptor);
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
