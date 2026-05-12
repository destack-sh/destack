use std::collections::HashMap;

use parking_lot::RwLock;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform;
use crate::runtime::action::ActionSet;
use crate::runtime::binding::{
    BindingAccess, BindingDescriptor, BindingId, NativeBinding, NativeBindingSet,
};
use destack_workspace::{ExecutionMode, RuntimeOptions};

/// Registry for runtime bindings and shims.
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
    /// Access policy for runtime bindings.
    access: RwLock<BindingAccess>,
}

impl BindingRegistry {
    /// Create a new binding registry.
    pub fn new() -> Self {
        Self {
            descriptors: Vec::new(),
            descriptor_by_id: HashMap::new(),
            native_by_id: HashMap::new(),
            native_bindings: Vec::new(),
            access: RwLock::new(BindingAccess::new(ExecutionMode::Fast)),
        }
    }

    /// Borrow the live binding access.
    pub fn access(&self) -> &RwLock<BindingAccess> {
        &self.access
    }

    /// Set the binding access for this registry.
    pub fn set_access(&mut self, access: BindingAccess) {
        *self.access.write() = access;
    }

    /// Apply runtime defaults to binding access without loading control rules.
    pub fn apply_runtime_defaults(&mut self, options: &RuntimeOptions) {
        let mut access = self.access.write();
        access.apply_runtime_defaults(options);
    }

    /// Apply one allowed action set to binding access checks.
    pub fn set_allowed_actions(&mut self, actions: ActionSet) {
        let mut access = self.access.write();
        access.set_allowed_actions(actions);
    }

    /// Install default native bindings for the runtime.
    pub fn install_native_defaults(&mut self) -> RuntimeResult<()> {
        for set in platform::PLATFORM_NATIVE_BINDINGS {
            self.install_native_binding_set(set)?;
        }

        Ok(())
    }

    /// Install a native binding set into the registry.
    pub(crate) fn install_native_binding_set(
        &mut self,
        set: &NativeBindingSet,
    ) -> RuntimeResult<()> {
        for binding in set.bindings {
            self.register_native_binding(*binding)?;
        }

        Ok(())
    }

    /// Register native binding metadata.
    pub(crate) fn register_native_binding(&mut self, binding: NativeBinding) -> RuntimeResult<()> {
        if let Some(existing) = self.descriptor_by_id.get(&binding.spec.id)
            && *existing != binding.spec
        {
            return Err(binding_collision_error(binding.spec.name));
        }

        if let Some(existing) = self.native_by_id.get(&binding.spec.id) {
            if existing.spec != binding.spec
                || existing.symbol != binding.symbol
                || existing.function != binding.function
            {
                return Err(binding_collision_error(binding.spec.name));
            }

            return Ok(());
        }

        self.descriptor_by_id.insert(binding.spec.id, binding.spec);
        self.descriptors.push(binding.spec);
        self.native_by_id.insert(binding.spec.id, binding);
        self.native_bindings.push(binding);

        Ok(())
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

/// Return one binding id collision error.
fn binding_collision_error(name: &str) -> Box<RuntimeError> {
    RuntimeError::Internal {
        message: format!("binding id collision for {name}"),
    }
    .boxed()
}
