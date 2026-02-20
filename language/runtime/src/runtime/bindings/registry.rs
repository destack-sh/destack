use std::collections::HashMap;
use std::sync::Arc;

use destack_vm as vm;
use destack_vm::Isolate;

use crate::platform;
use crate::runtime::bindings::{
    BindingDescriptor, BindingEngine, BindingId, BindingPolicy, NativeBinding, NativeBindingSet,
    VmBindingSet,
};
use crate::runtime::scheduler::EventLoop;
use crate::runtime::{BindingCallContext, RuntimeState, enter_binding_call_context};
use destack_workspace::RuntimeOptions;

/// Raw pointers captured for binding calls.
#[derive(Debug, Clone, Copy)]
struct BindingRuntimeHandle {
    /// Pointer to the shared runtime state.
    runtime: *const RuntimeState,
    /// Pointer to the event loop instance.
    event_loop: *const EventLoop,
}

impl BindingRuntimeHandle {
    /// Return the runtime state pointer.
    pub(crate) const fn runtime_ptr(self) -> *const RuntimeState {
        self.runtime
    }

    /// Return the event loop pointer.
    pub(crate) const fn event_loop_ptr(self) -> *const EventLoop {
        self.event_loop
    }
}

// safety: pointers are immutable and outlive the registered handlers
unsafe impl Send for BindingRuntimeHandle {}
// safety: pointers are immutable and outlive the registered handlers
unsafe impl Sync for BindingRuntimeHandle {}

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
    /// Runtime handles for binding calls.
    binding_runtime_handles: Option<BindingRuntimeHandle>,
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

        // compile policy lookups for currently registered descriptors
        self.policy.compile_descriptors(&self.descriptors);
    }

    /// Get the binding policy for this registry.
    pub fn policy_snapshot(&self) -> BindingPolicy {
        self.policy.clone()
    }

    /// Apply runtime options to binding policy.
    pub fn apply_runtime_options(&mut self, options: &RuntimeOptions) {
        self.policy.apply_runtime_options(options);
        self.policy.compile_descriptors(&self.descriptors);
    }

    /// Set runtime handles for binding calls.
    pub fn set_runtime_handles(&mut self, runtime: &Arc<RuntimeState>, event_loop: &EventLoop) {
        // record runtime state pointers for call contexts
        self.binding_runtime_handles = Some(BindingRuntimeHandle {
            runtime: Arc::as_ptr(runtime),
            event_loop: event_loop as *const EventLoop,
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
        self.policy.compile_descriptor(binding.spec);
    }

    /// Register a VM binding handler with metadata.
    pub fn register_vm_binding(
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

        // snapshot policy for the installed handler
        let policy = Arc::new(self.policy.clone());
        let handles = self.binding_runtime_handles.unwrap_or_else(|| {
            panic!(
                "binding registry missing runtime handles for {}",
                descriptor.name
            )
        });

        // NOTE #Incomplete: serialize args/results for replay payloads
        // register the external handler with policy enforcement
        isolate.register_vm_binding(descriptor.name, move |context, args| {
            policy.check(descriptor)?;
            let call_context = BindingCallContext::from_raw(
                handles.runtime_ptr(),
                handles.event_loop_ptr(),
                policy.clone(),
                BindingEngine::Vm,
            );
            let _guard = enter_binding_call_context(&call_context);
            handler(context, args)
        });

        // track the binding metadata for diagnostics
        if !has_descriptor {
            self.descriptor_by_id.insert(descriptor.id, descriptor);
            self.descriptors.push(descriptor);
            self.policy.compile_descriptor(descriptor);
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
