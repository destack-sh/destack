use std::collections::HashMap;

use parking_lot::RwLock;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::binding::{BindingAccess, BindingDescriptor, BindingId};
use crate::world::policy::ActionSet;
use destack_repository::{ExecutionMode, RuntimeOptions};

/// Registry for runtime bindings and shims.
#[derive(Debug)]
pub struct BindingRegistry {
    /// Registered binding descriptors for policy enforcement.
    descriptors: Vec<BindingDescriptor>,
    /// Binding metadata indexed by binding id.
    descriptor_by_id: HashMap<BindingId, BindingDescriptor>,
    /// Access policy for runtime bindings.
    access: RwLock<BindingAccess>,
}

impl BindingRegistry {
    /// Create a new binding registry.
    pub fn new() -> Self {
        Self {
            descriptors: Vec::new(),
            descriptor_by_id: HashMap::new(),
            access: RwLock::new(BindingAccess::new(ExecutionMode::Strict)),
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

    /// Register binding metadata.
    pub fn register(&mut self, descriptor: BindingDescriptor) -> RuntimeResult<()> {
        if let Some(existing) = self.descriptor_by_id.get(&descriptor.id)
            && *existing != descriptor
        {
            return Err(binding_collision_error(descriptor.name));
        }

        self.descriptor_by_id.insert(descriptor.id, descriptor);
        self.descriptors.push(descriptor);

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
}

impl Default for BindingRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Return one binding id collision error.
fn binding_collision_error(name: &str) -> Box<RuntimeError> {
    RuntimeError::Internal {
        message: format!("binding id collision for {name}"),
    }
    .boxed()
}
