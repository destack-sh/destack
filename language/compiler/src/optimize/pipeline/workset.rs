use std::sync::Arc;

use destack_artifact::MirLowered;
use destack_core::StringPool;
use destack_mir as mir;
use destack_source::{ModuleId, PackageId, ProfileId, TargetId};
use parking_lot::RwLock;

use crate::optimize::{OptimizationLevel, PipelineOptions};

/// A module entry tracked by package and program pipelines.
#[derive(Debug, Clone)]
pub struct ModuleWorkItem {
    /// The module id for this work item.
    module_id: ModuleId,
    /// The semantic profile for this work item.
    profile_id: ProfileId,
    /// The target id for this work item.
    target_id: TargetId,
    /// The local mutable MIR state for this work item.
    mir: Arc<RwLock<MirLowered>>,
    /// Shared strings referenced by this MIR.
    strings: Arc<StringPool>,
    /// The pipeline options for this module.
    options: PipelineOptions,
}

impl ModuleWorkItem {
    /// Create a new module work item.
    pub fn new(
        module_id: ModuleId,
        profile_id: ProfileId,
        target_id: TargetId,
        mir: MirLowered,
        strings: Arc<StringPool>,
        options: PipelineOptions,
    ) -> Self {
        Self {
            module_id,
            profile_id,
            target_id,
            mir: Arc::new(RwLock::new(mir)),
            strings,
            options,
        }
    }

    /// Get the module id.
    pub fn module_id(&self) -> ModuleId {
        self.module_id
    }

    /// Get the profile id.
    pub fn profile_id(&self) -> ProfileId {
        self.profile_id
    }

    /// Get the target id.
    pub fn target_id(&self) -> &TargetId {
        &self.target_id
    }

    /// Get the pipeline options for this module.
    pub fn options(&self) -> &PipelineOptions {
        &self.options
    }

    /// Read the MIR tree for this module.
    pub fn with_tree<T>(&self, f: impl FnOnce(&mir::Tree) -> T) -> T {
        let mir = self.mir.read();
        f(&mir.tree)
    }

    /// Mutate the MIR tree for this module.
    pub fn with_tree_mut<T>(&self, f: impl FnOnce(&mut mir::Tree) -> T) -> T {
        let mut mir = self.mir.write();
        f(&mut mir.tree)
    }

    /// Access the module string pool.
    pub fn with_strings<T>(&self, f: impl FnOnce(&StringPool) -> T) -> T {
        f(self.strings.as_ref())
    }

    /// Access the profile data for this module.
    pub fn with_profile<T>(&self, f: impl FnOnce(Option<&mir::ProfileTable>) -> T) -> T {
        f(None)
    }

    /// Clone the profile data for this module.
    pub fn clone_profile(&self) -> Option<Arc<mir::ProfileTable>> {
        None
    }

    /// Access the module MIR data.
    pub fn with_mir<T>(&self, f: impl FnOnce(&MirLowered) -> T) -> T {
        let mir = self.mir.read();
        f(&mir)
    }
}

/// A set of modules optimized together within a package.
#[derive(Debug, Clone)]
pub struct PackageWorkset {
    /// The package id for this workset.
    package_id: PackageId,
    /// The target id used for all modules.
    target_id: TargetId,
    /// The optimization level for this package.
    optimization_level: OptimizationLevel,
    /// The modules included in this workset.
    modules: Vec<ModuleWorkItem>,
}

impl PackageWorkset {
    /// Create a new package workset.
    pub fn new(
        package_id: PackageId,
        target_id: TargetId,
        optimization_level: OptimizationLevel,
    ) -> Self {
        Self {
            package_id,
            target_id,
            optimization_level,
            modules: Vec::new(),
        }
    }

    /// Get the package id.
    pub fn package_id(&self) -> PackageId {
        self.package_id
    }

    /// Get the target id.
    pub fn target_id(&self) -> &TargetId {
        &self.target_id
    }

    /// Get the optimization level.
    pub fn optimization_level(&self) -> OptimizationLevel {
        self.optimization_level
    }

    /// Add a module to the workset.
    pub fn add_module(&mut self, module: ModuleWorkItem) {
        self.modules.push(module);
    }

    /// Return the number of modules in the workset.
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }

    /// Return all modules in the workset.
    pub fn modules(&self) -> &[ModuleWorkItem] {
        &self.modules
    }

    /// Return all modules in the workset mutably.
    pub fn modules_mut(&mut self) -> &mut [ModuleWorkItem] {
        &mut self.modules
    }
}

/// A set of packages optimized together within a program.
#[derive(Debug, Clone)]
pub struct ProgramWorkset {
    /// The packages included in this workset.
    packages: Vec<PackageWorkset>,
}

impl ProgramWorkset {
    /// Create a new program workset.
    pub fn new() -> Self {
        Self {
            packages: Vec::new(),
        }
    }

    /// Add a package workset.
    pub fn add_package(&mut self, package: PackageWorkset) {
        self.packages.push(package);
    }

    /// Return the number of packages in the workset.
    pub fn package_count(&self) -> usize {
        self.packages.len()
    }

    /// Return all package worksets.
    pub fn packages(&self) -> &[PackageWorkset] {
        &self.packages
    }

    /// Return all package worksets mutably.
    pub fn packages_mut(&mut self) -> &mut [PackageWorkset] {
        &mut self.packages
    }
}

impl Default for ProgramWorkset {
    fn default() -> Self {
        Self::new()
    }
}
