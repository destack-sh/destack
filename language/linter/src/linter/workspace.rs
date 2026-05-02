use std::sync::Arc;

use destack_artifact::{AmbientEnvironment, Ast, DirExported, WellKnownSymbols};
use destack_ast::StringId;
use destack_dir::{self as dir, WellKnownSymbol};
use destack_source::{File, FileId, ModuleId, PackageId};
use destack_workspace::{
    LintSeverity, LinterOptions, Module, Package, ProfileId, Repository, Revision, Workspace,
};

use crate::linter::artifact::{read_ambient_environment, read_ast, read_dir_exported};
use crate::linter::library::is_builtin_library_module;
use crate::{LintMeta, LintReport, LintRequirement};

/// Context for AST-level workspace linting.
pub struct LintWorkspaceAstContext {
    /// The repository backing this lint pass.
    pub repository: Arc<Repository>,
    /// The workspace being linted.
    pub workspace: Arc<Workspace>,
    /// The source revision for this lint pass.
    pub revision: Revision,
    /// Linter configuration.
    options: LinterOptions,
    /// Collected diagnostics.
    diagnostics: Vec<LintReport>,
}

impl std::fmt::Debug for LintWorkspaceAstContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LintWorkspaceAstContext")
            .field("root", &self.workspace.root)
            .finish()
    }
}

impl LintWorkspaceAstContext {
    /// Create a new AST workspace lint context.
    pub fn new(
        repository: Arc<Repository>,
        workspace: Arc<Workspace>,
        revision: Revision,
        options: LinterOptions,
    ) -> Self {
        Self {
            repository,
            workspace,
            revision,
            options,
            diagnostics: Vec::new(),
        }
    }

    /// Get the linter options.
    pub fn options(&self) -> &LinterOptions {
        &self.options
    }

    /// Return one module for the active revision when present.
    pub fn repository_module(&self, module_id: ModuleId) -> Option<Arc<Module>> {
        self.repository
            .module(self.revision, module_id)
            .ok()
            .flatten()
    }

    /// Return one source file for the active revision when present.
    pub fn repository_file(&self, file_id: FileId) -> Option<Arc<File>> {
        self.repository.file(self.revision, file_id).ok().flatten()
    }

    /// Return one AST artifact for one revision-scoped module.
    pub fn module_ast(&self, module_id: ModuleId) -> Option<Arc<Ast>> {
        read_ast(&self.repository, self.revision, module_id)
    }

    /// Return the package ids in the active workspace.
    pub fn workspace_package_ids(&self) -> Vec<PackageId> {
        self.repository
            .package_ids(self.revision)
            .unwrap_or_else(|_| Vec::new())
    }

    /// Return all visible module ids in the active workspace.
    pub fn workspace_module_ids(&self) -> Vec<ModuleId> {
        collect_workspace_module_ids(&self.repository, self.revision)
    }

    /// Resolve severity for a rule.
    pub fn get_severity(&self, meta: &LintMeta) -> LintSeverity {
        self.options.resolve_severity(
            meta.id,
            meta.category,
            meta.category.default_severity(),
            meta.is_recommended(),
            meta.is_strict(),
        )
    }

    /// Check if a requirement is met.
    pub fn is_requirement_met(&self, _requirement: &LintRequirement) -> bool {
        false
    }

    /// Check if a rule is supported.
    pub fn is_rule_supported(&self, meta: &LintMeta) -> bool {
        if !meta.requires_all.is_empty() {
            for requirement in meta.requires_all {
                if !self.is_requirement_met(requirement) {
                    return false;
                }
            }
        }

        if !meta.requires_any.is_empty() {
            for requirement in meta.requires_any {
                if self.is_requirement_met(requirement) {
                    return true;
                }
            }

            return false;
        }

        true
    }

    /// Check if a rule is enabled.
    pub fn is_rule_enabled(&self, meta: &LintMeta) -> bool {
        self.get_severity(meta).is_enabled()
    }

    /// Report a lint diagnostic.
    pub fn report(&mut self, diagnostic: LintReport) {
        if diagnostic.is_enabled() {
            self.diagnostics.push(diagnostic);
        }
    }

    /// Take the collected diagnostics.
    pub fn take_diagnostics(&mut self) -> Vec<LintReport> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Get reference to collected diagnostics.
    pub fn diagnostics(&self) -> &[LintReport] {
        &self.diagnostics
    }
}

/// Context for DIR-level workspace linting.
pub struct LintWorkspaceDirContext {
    /// The repository backing this lint pass.
    pub repository: Arc<Repository>,
    /// The workspace being linted.
    pub workspace: Arc<Workspace>,
    /// The source revision for this lint pass.
    pub revision: Revision,
    /// The active profile for this workspace pass.
    pub profile_id: ProfileId,
    /// Linter configuration.
    options: LinterOptions,
    /// Collected diagnostics.
    diagnostics: Vec<LintReport>,
}

impl std::fmt::Debug for LintWorkspaceDirContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LintWorkspaceDirContext")
            .field("root", &self.workspace.root)
            .field("profile_id", &self.profile_id)
            .finish()
    }
}

impl LintWorkspaceDirContext {
    /// Create a new DIR workspace lint context.
    pub fn new(
        repository: Arc<Repository>,
        workspace: Arc<Workspace>,
        revision: Revision,
        profile_id: ProfileId,
        options: LinterOptions,
    ) -> Self {
        Self {
            repository,
            workspace,
            revision,
            profile_id,
            options,
            diagnostics: Vec::new(),
        }
    }

    /// Get the linter options.
    pub fn options(&self) -> &LinterOptions {
        &self.options
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

    /// Return one resolved DIR artifact for one revision-scoped module.
    pub fn resolved_dir(&self, module_id: ModuleId) -> Option<Arc<DirExported>> {
        read_dir_exported(&self.repository, self.revision, module_id, self.profile_id)
    }

    /// Return the library environment for the active revision and profile.
    pub fn ambient_environment(&self) -> Option<Arc<AmbientEnvironment>> {
        read_ambient_environment(&self.repository, self.revision, self.profile_id)
    }

    /// Return the package ids in the active workspace.
    pub fn workspace_package_ids(&self) -> Vec<PackageId> {
        self.repository
            .package_ids(self.revision)
            .unwrap_or_else(|_| Vec::new())
    }

    /// Return all visible module ids in the active workspace.
    pub fn workspace_module_ids(&self) -> Vec<ModuleId> {
        collect_workspace_module_ids(&self.repository, self.revision)
    }

    /// Resolve severity for a rule.
    pub fn get_severity(&self, meta: &LintMeta) -> LintSeverity {
        self.options.resolve_severity(
            meta.id,
            meta.category,
            meta.category.default_severity(),
            meta.is_recommended(),
            meta.is_strict(),
        )
    }

    /// Get a cached declared library symbol for the active profile and name.
    pub fn get_declared_library_symbol(&self, name: StringId) -> Option<dir::GlobalSymbolId> {
        let environment = self.ambient_environment()?;
        let key = dir::StaticKey::Name(name);

        environment.declared_symbol_from_key(&key, dir::SymbolSpaceOrder::ValueThenType)
    }

    /// Get well-known symbols for the active profile.
    pub fn get_well_known_symbols(&self) -> Option<WellKnownSymbols> {
        let environment = self.ambient_environment()?;
        Some(environment.well_known_symbols())
    }

    /// Get a specific well-known symbol for the active profile.
    pub fn get_well_known_symbol(&self, symbol: WellKnownSymbol) -> Option<dir::GlobalSymbolId> {
        let well_known_symbols = self.get_well_known_symbols()?;
        well_known_symbols.get_symbol(symbol)
    }

    /// Check if a requirement is met.
    pub fn is_requirement_met(&self, requirement: &LintRequirement) -> bool {
        match requirement {
            LintRequirement::RequireLibSymbol(name, libs) => {
                if !is_lib_available(self, libs) {
                    return false;
                }

                let name = StringId::for_text(name);
                self.get_declared_library_symbol(name).is_some()
            }
            LintRequirement::RequireWellKnownSymbol(symbol) => {
                self.get_well_known_symbol(*symbol).is_some()
            }
        }
    }

    /// Check if a rule is supported.
    pub fn is_rule_supported(&self, meta: &LintMeta) -> bool {
        if !meta.requires_all.is_empty() {
            for requirement in meta.requires_all {
                if !self.is_requirement_met(requirement) {
                    return false;
                }
            }
        }

        if !meta.requires_any.is_empty() {
            for requirement in meta.requires_any {
                if self.is_requirement_met(requirement) {
                    return true;
                }
            }

            return false;
        }

        true
    }

    /// Check if a rule is enabled.
    pub fn is_rule_enabled(&self, meta: &LintMeta) -> bool {
        self.get_severity(meta).is_enabled()
    }

    /// Report a lint diagnostic.
    pub fn report(&mut self, diagnostic: LintReport) {
        if diagnostic.is_enabled() {
            self.diagnostics.push(diagnostic);
        }
    }

    /// Take the collected diagnostics.
    pub fn take_diagnostics(&mut self) -> Vec<LintReport> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Get reference to collected diagnostics.
    pub fn diagnostics(&self) -> &[LintReport] {
        &self.diagnostics
    }
}

/// Return the visible module ids for one workspace and revision.
fn collect_workspace_module_ids(repository: &Repository, revision: Revision) -> Vec<ModuleId> {
    let mut module_ids = Vec::new();

    let Ok(package_ids) = repository.package_ids(revision) else {
        return module_ids;
    };

    for package_id in package_ids {
        let Ok(package_module_ids) = repository.package_module_ids(revision, package_id) else {
            continue;
        };
        module_ids.extend(package_module_ids);
    }

    module_ids.sort_unstable();
    module_ids.dedup();
    module_ids
}

/// Return true when at least one of the required libs is available.
fn is_lib_available(ctx: &LintWorkspaceDirContext, libs: &[&str]) -> bool {
    if libs.is_empty() {
        return true;
    }

    let Some(environment) = ctx.ambient_environment() else {
        return false;
    };

    environment
        .ambient_modules
        .iter()
        .filter_map(|module_id| ctx.repository_module(*module_id))
        .any(|module| is_builtin_library_module(module.as_ref(), libs))
}
