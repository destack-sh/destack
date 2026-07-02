use std::sync::Arc;

use destack_artifact::{
    DirBound, DirCheckedModule, DirExpanded, DirExported, DirImported, DirParsed, GlobalEnvironment,
};
use destack_core::StringPool;
use destack_dir as dir;
use destack_repository::{
    ArtifactReader, LinterOptions, Module, Package, ProfileId, Repository, Revision,
};
use destack_source::{File, FileId, ModuleId, PackageId};

/// Shared state for one DIR lint pass.
#[derive(Debug, Clone)]
pub struct LintSession {
    /// The repository backing this lint pass.
    pub repository: Arc<Repository>,
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
        revision: Revision,
        profile_id: ProfileId,
        options: LinterOptions,
    ) -> Self {
        Self {
            repository,
            revision,
            profile_id,
            options,
        }
    }

    /// Return a revision-bound artifact reader.
    fn artifact_reader(&self) -> ArtifactReader<'_> {
        ArtifactReader::new(self.repository.as_ref(), self.revision)
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
        self.artifact_reader().dir_parsed(module_id).ok()
    }

    /// Return one bound DIR artifact for one revision-scoped module.
    pub fn dir_bound(&self, module_id: ModuleId) -> Option<Arc<DirBound>> {
        self.artifact_reader()
            .dir_bound(module_id, self.profile_id)
            .ok()
    }

    /// Return one imported DIR artifact for one revision-scoped module.
    pub fn dir_imported(&self, module_id: ModuleId) -> Option<Arc<DirImported>> {
        self.artifact_reader()
            .dir_imported(module_id, self.profile_id)
            .ok()
    }

    /// Return one expanded DIR artifact for one revision-scoped module.
    pub fn dir_expanded(&self, module_id: ModuleId) -> Option<Arc<DirExpanded>> {
        self.artifact_reader()
            .dir_expanded(module_id, self.profile_id)
            .ok()
    }

    /// Return one checked DIR artifact for one revision-scoped module.
    pub fn dir_checked(&self, module_id: ModuleId) -> Option<Arc<DirCheckedModule>> {
        self.artifact_reader()
            .dir_checked(module_id, self.profile_id)
            .ok()
    }

    /// Return one exported DIR artifact for one revision-scoped module.
    pub fn dir_exported(&self, module_id: ModuleId) -> Option<Arc<DirExported>> {
        self.artifact_reader()
            .dir_exported(module_id, self.profile_id)
            .ok()
    }

    /// Return the global environment for the active revision and profile.
    pub fn global_environment(&self) -> Option<Arc<GlobalEnvironment>> {
        self.artifact_reader()
            .global_environment(self.profile_id)
            .ok()
    }

    /// Return one checked semantic module view.
    pub fn checked_module(&self, module_id: ModuleId) -> Option<LintCheckedModule> {
        let artifacts = self.artifact_reader();
        let parsed = artifacts.dir_parsed(module_id).ok()?;
        let bound = artifacts.dir_bound(module_id, self.profile_id).ok()?;
        let expanded = artifacts.dir_expanded(module_id, self.profile_id).ok()?;
        let checked = artifacts.dir_checked(module_id, self.profile_id).ok()?;
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

        Some(read(&ty, &module))
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
