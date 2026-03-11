use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use dashmap::DashMap;
use destack_source::{FileType, ModuleId, PackageId};

use crate::{OutputDependency, TargetId};

use super::{Output, OutputId, OutputKey, OutputScope};

/// Registry of generated outputs.
#[derive(Debug)]
pub struct OutputRegistry {
    /// Outputs by id.
    outputs_by_id: DashMap<OutputId, Arc<Output>>,
    /// Output ids by key and file type.
    outputs_by_key: DashMap<(OutputKey, FileType), OutputId>,
    /// Dependency stamps by output key.
    dependencies: DashMap<OutputKey, OutputDependency>,
    /// The next output id.
    next_id: AtomicU32,
}

impl Default for OutputRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputRegistry {
    /// Create a new output registry.
    pub fn new() -> Self {
        Self {
            outputs_by_id: DashMap::new(),
            outputs_by_key: DashMap::new(),
            dependencies: DashMap::new(),
            next_id: AtomicU32::new(0),
        }
    }

    /// Allocate the next output id.
    pub fn next_id(&self) -> OutputId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        OutputId::new(id)
    }

    /// Insert an output into the registry.
    pub fn insert(&self, output: Output) -> OutputId {
        let id = output.id;
        let key = OutputKey::new(output.scope, output.target.clone());
        let file_type = output.content.file_type();
        self.outputs_by_key.insert((key, file_type), id);
        self.outputs_by_id.insert(id, Arc::new(output));
        id
    }

    /// Get an output by id.
    pub fn get(&self, id: OutputId) -> Option<Arc<Output>> {
        self.outputs_by_id
            .get(&id)
            .map(|record| record.value().clone())
    }

    /// Get an output by scope, target, and file type.
    pub fn get_by_key(
        &self,
        scope: OutputScope,
        target: &TargetId,
        file_type: FileType,
    ) -> Option<Arc<Output>> {
        let key = OutputKey::new(scope, target.clone());
        let id = self.outputs_by_key.get(&(key, file_type))?;
        self.get(*id)
    }

    /// Get a module output.
    pub fn get_module(
        &self,
        module: ModuleId,
        target: &TargetId,
        file_type: FileType,
    ) -> Option<Arc<Output>> {
        self.get_by_key(OutputScope::Module(module), target, file_type)
    }

    /// Get a package output.
    pub fn get_package(
        &self,
        package: PackageId,
        target: &TargetId,
        file_type: FileType,
    ) -> Option<Arc<Output>> {
        self.get_by_key(OutputScope::Package(package), target, file_type)
    }

    /// Get the dependency stamp for one output key.
    pub fn dependency(&self, key: &OutputKey) -> Option<OutputDependency> {
        self.dependencies
            .get(key)
            .map(|dependency| *dependency.value())
    }

    /// Insert one dependency stamp for one output key.
    pub fn set_dependency(&self, key: OutputKey, dependency: OutputDependency) {
        self.dependencies.insert(key, dependency);
    }

    /// Return whether any output exists for one key.
    pub fn contains_key(&self, key: &OutputKey) -> bool {
        self.outputs_by_key
            .iter()
            .any(|entry| &entry.key().0 == key)
    }

    /// Check if an output exists by id.
    pub fn contains(&self, id: OutputId) -> bool {
        self.outputs_by_id.contains_key(&id)
    }

    /// Get all outputs for a target.
    pub fn get_by_target(&self, target: &TargetId) -> Vec<Arc<Output>> {
        self.outputs_by_id
            .iter()
            .filter(|record| &record.value().target == target)
            .map(|record| record.value().clone())
            .collect()
    }

    /// Get all module outputs for a target.
    pub fn get_modules_by_target(&self, target: &TargetId) -> Vec<Arc<Output>> {
        self.outputs_by_id
            .iter()
            .filter(|record| {
                &record.value().target == target && record.value().scope.module().is_some()
            })
            .map(|record| record.value().clone())
            .collect()
    }

    /// Get all outputs for a module.
    pub fn get_by_module(&self, module: ModuleId) -> Vec<Arc<Output>> {
        self.outputs_by_id
            .iter()
            .filter(|record| record.value().scope.module() == Some(module))
            .map(|record| record.value().clone())
            .collect()
    }

    /// Get all outputs for a module target.
    pub fn get_by_module_target(&self, module: ModuleId, target: &TargetId) -> Vec<Arc<Output>> {
        self.outputs_by_id
            .iter()
            .filter(|record| {
                record.value().scope == OutputScope::Module(module)
                    && &record.value().target == target
            })
            .map(|record| record.value().clone())
            .collect()
    }

    /// Get all outputs for a package.
    pub fn get_by_package(&self, package: PackageId) -> Vec<Arc<Output>> {
        self.outputs_by_id
            .iter()
            .filter(|record| record.value().scope.package() == Some(package))
            .map(|record| record.value().clone())
            .collect()
    }

    /// Get all outputs for a package target.
    pub fn get_by_package_target(&self, package: PackageId, target: &TargetId) -> Vec<Arc<Output>> {
        self.outputs_by_id
            .iter()
            .filter(|record| {
                let output = record.value();
                &output.target == target
                    && match output.scope {
                        OutputScope::Module(module_id) => module_id.package_id == package,
                        OutputScope::Package(package_id) => package_id == package,
                    }
            })
            .map(|record| record.value().clone())
            .collect()
    }

    /// Remove an output by id.
    pub fn remove(&self, id: OutputId) -> Option<Arc<Output>> {
        let output = self.outputs_by_id.remove(&id).map(|(_, value)| value)?;
        let key = OutputKey::new(output.scope, output.target.clone());
        let file_type = output.content.file_type();
        self.outputs_by_key.remove(&(key, file_type));
        Some(output)
    }

    /// Clear all outputs.
    pub fn clear(&self) {
        self.outputs_by_id.clear();
        self.outputs_by_key.clear();
        self.dependencies.clear();
    }

    /// Return the number of outputs.
    pub fn len(&self) -> usize {
        self.outputs_by_id.len()
    }

    /// Return whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.outputs_by_id.is_empty()
    }

    /// Iterate over all outputs.
    pub fn iter(&self) -> impl Iterator<Item = Arc<Output>> + '_ {
        self.outputs_by_id
            .iter()
            .map(|record| record.value().clone())
    }
}
