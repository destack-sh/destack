use std::sync::Arc;

use destack_dir::{self as dir, LanguageItem, StringId};
use destack_repository::{LintSeverity, LinterOptions, Package, Repository, Revision};
use destack_source::{ModuleId, PackageId};

use crate::linter::library::is_library_module;
use crate::{LintMeta, LintReport, LintRequirement, LintSession};

/// Context for package linting.
pub struct LintPackageContext {
    /// Shared lint pass state.
    pub session: LintSession,
    /// The package being linted.
    pub package: Arc<Package>,
    /// Collected diagnostics.
    diagnostics: Vec<LintReport>,
}

impl std::fmt::Debug for LintPackageContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LintPackageContext")
            .field("package_id", &self.package.id)
            .field("profile_id", &self.session.profile_id)
            .finish()
    }
}

impl LintPackageContext {
    /// Create a new package lint context.
    pub fn new(session: LintSession, package: Arc<Package>) -> Self {
        Self {
            session,
            package,
            diagnostics: Vec::new(),
        }
    }

    /// Get the linter options.
    pub fn options(&self) -> &LinterOptions {
        &self.session.options
    }

    /// Return all visible module ids in the active package.
    pub fn package_module_ids(&self) -> Vec<ModuleId> {
        collect_package_module_ids(
            &self.session.repository,
            self.session.revision,
            self.package.id,
        )
    }

    /// Resolve severity for a rule.
    pub fn get_severity(&self, meta: &LintMeta) -> LintSeverity {
        self.session.options.resolve_severity(
            meta.id,
            meta.category,
            meta.category.default_severity(),
            meta.is_recommended(),
            meta.is_strict(),
        )
    }

    /// Get a cached declared library symbol for the active profile and name.
    pub fn get_declared_library_symbol(&self, name: StringId) -> Option<dir::GlobalSymbolId> {
        let environment = self.session.global_environment()?;

        environment.language.symbols.get(&name).copied()
    }

    /// Get a specific language item for the active profile.
    pub fn get_language_item(&self, symbol: LanguageItem) -> Option<dir::GlobalSymbolId> {
        let environment = self.session.global_environment()?;

        environment.language.symbol(symbol)
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
fn is_lib_available(ctx: &LintPackageContext, libs: &[&str]) -> bool {
    if libs.is_empty() {
        return true;
    }

    let Some(environment) = ctx.session.global_environment() else {
        return false;
    };

    environment
        .globals
        .iter()
        .filter_map(|module_id| ctx.session.repository_module(*module_id))
        .any(|module| is_library_module(module.as_ref(), libs))
}
