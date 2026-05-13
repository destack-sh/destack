use std::sync::Arc;

use destack_artifact::{Ast, DirExported, DirImported, GlobalEnvironment};
use destack_ast::StringId;
use destack_dir::{self as dir, LanguageItem};
use destack_source::{File, FileId, ModuleId, PackageId};
use destack_workspace::{
    ArtifactCache, LintSeverity, LinterOptions, Module, Package, ProfileId, Repository, Revision,
};

use crate::linter::library::is_library_module;
use crate::{LintMeta, LintReport, LintRequirement};

/// Context for AST-level package linting.
pub struct LintPackageAstContext {
    /// The repository backing this lint pass.
    pub repository: Arc<Repository>,
    /// Revision artifact cache for this lint pass.
    pub artifacts: Arc<ArtifactCache>,
    /// The package being linted.
    pub package: Arc<Package>,
    /// The source revision for this lint pass.
    pub revision: Revision,
    /// Linter configuration.
    options: LinterOptions,
    /// Collected diagnostics.
    diagnostics: Vec<LintReport>,
}

impl std::fmt::Debug for LintPackageAstContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LintPackageAstContext")
            .field("package_id", &self.package.id)
            .finish()
    }
}

impl LintPackageAstContext {
    /// Create a new AST package lint context.
    pub fn new(
        repository: Arc<Repository>,
        artifacts: Arc<ArtifactCache>,
        package: Arc<Package>,
        revision: Revision,
        options: LinterOptions,
    ) -> Self {
        Self {
            repository,
            artifacts,
            package,
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
        self.artifacts.ast(module_id)
    }

    /// Return all visible module ids in the active package.
    pub fn package_module_ids(&self) -> Vec<ModuleId> {
        collect_package_module_ids(&self.repository, self.revision, self.package.id)
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

/// Context for DIR-level package linting.
pub struct LintPackageDirContext {
    /// The repository backing this lint pass.
    pub repository: Arc<Repository>,
    /// Revision artifact cache for this lint pass.
    pub artifacts: Arc<ArtifactCache>,
    /// The package being linted.
    pub package: Arc<Package>,
    /// The source revision for this lint pass.
    pub revision: Revision,
    /// The active profile for this package pass.
    pub profile_id: ProfileId,
    /// Linter configuration.
    options: LinterOptions,
    /// Collected diagnostics.
    diagnostics: Vec<LintReport>,
}

impl std::fmt::Debug for LintPackageDirContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LintPackageDirContext")
            .field("package_id", &self.package.id)
            .field("profile_id", &self.profile_id)
            .finish()
    }
}

impl LintPackageDirContext {
    /// Create a new DIR package lint context.
    pub fn new(
        repository: Arc<Repository>,
        artifacts: Arc<ArtifactCache>,
        package: Arc<Package>,
        revision: Revision,
        profile_id: ProfileId,
        options: LinterOptions,
    ) -> Self {
        Self {
            repository,
            artifacts,
            package,
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

    /// Return one imported DIR artifact for one revision-scoped module.
    pub fn imported_dir(&self, module_id: ModuleId) -> Option<Arc<DirImported>> {
        self.artifacts.dir_imported(module_id, self.profile_id)
    }

    /// Return one exported DIR artifact for one revision-scoped module.
    pub fn exported_dir(&self, module_id: ModuleId) -> Option<Arc<DirExported>> {
        self.artifacts.dir_exported(module_id, self.profile_id)
    }

    /// Return the global environment for the active revision and profile.
    pub fn global_environment(&self) -> Option<Arc<GlobalEnvironment>> {
        self.artifacts.global_environment(self.profile_id)
    }

    /// Return all visible module ids in the active package.
    pub fn package_module_ids(&self) -> Vec<ModuleId> {
        collect_package_module_ids(&self.repository, self.revision, self.package.id)
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
        let environment = self.global_environment()?;

        environment.language.symbols.get(&name).copied()
    }

    /// Get a specific language item for the active profile.
    pub fn get_language_item(&self, symbol: LanguageItem) -> Option<dir::GlobalSymbolId> {
        let environment = self.global_environment()?;

        environment.language.item(symbol)
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
            LintRequirement::RequireLanguageItem(symbol) => {
                self.get_language_item(*symbol).is_some()
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

/// Return the visible module ids for one package and revision.
fn collect_package_module_ids(
    repository: &Repository,
    revision: Revision,
    package_id: PackageId,
) -> Vec<ModuleId> {
    let Ok(mut module_ids) = repository.package_module_ids(revision, package_id) else {
        return Vec::new();
    };

    module_ids.sort_unstable();
    module_ids.dedup();
    module_ids
}

/// Return true when at least one of the required libs is available.
fn is_lib_available(ctx: &LintPackageDirContext, libs: &[&str]) -> bool {
    if libs.is_empty() {
        return true;
    }

    let Some(environment) = ctx.global_environment() else {
        return false;
    };

    environment
        .modules
        .iter()
        .filter_map(|module_id| ctx.repository_module(*module_id))
        .any(|module| is_library_module(module.as_ref(), libs))
}
