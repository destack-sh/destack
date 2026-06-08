use std::sync::Arc;

use destack_artifact::{
    DirBound, DirCheckedModule, DirExpanded, DirExported, DirImported, DirParsed, GlobalEnvironment,
};
use destack_core::StringPool;
use destack_dir as dir;
use destack_repository::{
    ArtifactCache, LinterOptions, Module, Package, ProfileId, Repository, Revision,
};
use destack_source::{File, FileId, ModuleId, PackageId};

/// Shared state for one DIR lint pass.
#[derive(Debug, Clone)]
pub struct LintSession {
    /// The repository backing this lint pass.
    pub repository: Arc<Repository>,
    /// Revision artifact cache for this lint pass.
    pub artifacts: Arc<ArtifactCache>,
    /// The source revision for this lint pass.
    pub revision: Revision,
    /// The active profile for this lint pass.
    pub profile_id: ProfileId,
    /// Linter configuration.
    pub options: LinterOptions,
}

impl LintSession {
    /// Create a new lint session.
    pub fn new(
        repository: Arc<Repository>,
        artifacts: Arc<ArtifactCache>,
        revision: Revision,
        profile_id: ProfileId,
        options: LinterOptions,
    ) -> Self {
        Self {
            repository,
            artifacts,
            revision,
            profile_id,
            options,
        }
    }

    /// Return one module for the active revision when present.
    pub fn repository_module(&self, module_id: ModuleId) -> Option<Arc<Module>> {
        self.repository
            .module(self.revision, module_id)
            .ok()
            .flatten()
    }

    /// Return one package for the active revision when present.
    pub fn repository_package(&self, package_id: PackageId) -> Option<Arc<Package>> {
        self.repository
            .package(self.revision, package_id)
            .ok()
            .flatten()
    }

    /// Return one source file for the active revision when present.
    pub fn repository_file(&self, file_id: FileId) -> Option<Arc<File>> {
        self.repository.file(self.revision, file_id).ok().flatten()
    }

    /// Return one parsed DIR artifact for one revision-scoped module.
    pub fn dir_parsed(&self, module_id: ModuleId) -> Option<Arc<DirParsed>> {
        self.artifacts.dir_parsed(module_id)
    }

    /// Return one bound DIR artifact for one revision-scoped module.
    pub fn dir_bound(&self, module_id: ModuleId) -> Option<Arc<DirBound>> {
        self.artifacts.dir_bound(module_id, self.profile_id)
    }

    /// Return one imported DIR artifact for one revision-scoped module.
    pub fn dir_imported(&self, module_id: ModuleId) -> Option<Arc<DirImported>> {
        self.artifacts.dir_imported(module_id, self.profile_id)
    }

    /// Return one expanded DIR artifact for one revision-scoped module.
    pub fn dir_expanded(&self, module_id: ModuleId) -> Option<Arc<DirExpanded>> {
        self.artifacts.dir_expanded(module_id, self.profile_id)
    }

    /// Return one checked DIR artifact for one revision-scoped module.
    pub fn dir_checked(&self, module_id: ModuleId) -> Option<Arc<DirCheckedModule>> {
        self.artifacts.dir_checked(module_id, self.profile_id)
    }

    /// Return one exported DIR artifact for one revision-scoped module.
    pub fn dir_exported(&self, module_id: ModuleId) -> Option<Arc<DirExported>> {
        self.artifacts.dir_exported(module_id, self.profile_id)
    }

    /// Return the global environment for the active revision and profile.
    pub fn global_environment(&self) -> Option<Arc<GlobalEnvironment>> {
        self.artifacts.global_environment(self.profile_id)
    }

    /// Return one checked semantic module view.
    pub fn checked_module(&self, module_id: ModuleId) -> Option<LintCheckedModule> {
        let parsed = self.dir_parsed(module_id)?;
        let bound = self.dir_bound(module_id)?;
        let expanded = self.dir_expanded(module_id)?;
        let checked = self.dir_checked(module_id)?;
        let strings = self.repository.string_pool().clone();

        Some(LintCheckedModule::new(
            parsed, bound, expanded, checked, strings,
        ))
    }

    /// Read one global type through its owning checked module.
    pub fn with_type<R>(
        &self,
        type_id: dir::GlobalTypeId,
        read: impl FnOnce(&dir::Type, &LintCheckedModule) -> R,
    ) -> Option<R> {
        let module = self.checked_module(type_id.module_id)?;
        let ty = module.types.get_type_maybe(type_id.local_id)?;

        Some(read(ty, &module))
    }

    /// Read one global static value through its owning checked module.
    pub fn with_static<R>(
        &self,
        static_id: dir::GlobalStaticId,
        read: impl FnOnce(&dir::StaticTerm, &LintCheckedModule) -> R,
    ) -> Option<R> {
        let module = self.checked_module(static_id.module_id)?;
        let term = module.statics.get_static_maybe(static_id.local_id)?;

        Some(read(term, &module))
    }

    /// Resolve the checked type id for one global symbol.
    pub fn symbol_type_id(&self, symbol_id: dir::GlobalSymbolId) -> Option<dir::GlobalTypeId> {
        let module = self.checked_module(symbol_id.module_id)?;

        module.types.get_symbol_type_id(symbol_id)
    }
}

/// Checked semantic tables for one module.
#[derive(Debug, Clone)]
pub struct LintCheckedModule {
    /// The parsed DIR artifact.
    pub parsed: Arc<DirParsed>,
    /// The DIR string pool.
    pub strings: Arc<StringPool>,
    /// The symbol table.
    pub symbols: dir::BindingTable<'static>,
    /// The type table.
    pub types: dir::TypeTable<'static>,
    /// The static table.
    pub statics: dir::StaticTable<'static>,
    /// The resolution table.
    pub resolutions: dir::ResolutionTable<'static>,
}

impl LintCheckedModule {
    /// Create a checked semantic module view.
    pub fn new(
        parsed: Arc<DirParsed>,
        bound: Arc<DirBound>,
        expanded: Arc<DirExpanded>,
        checked: Arc<DirCheckedModule>,
        strings: Arc<StringPool>,
    ) -> Self {
        let symbols = expanded.binding_table(&bound);
        let types = checked.type_table(&bound, &expanded);
        let statics = checked.static_table(&bound, &expanded);
        let resolutions = checked.resolution_table();

        Self {
            parsed,
            strings,
            symbols,
            types,
            statics,
            resolutions,
        }
    }
}
