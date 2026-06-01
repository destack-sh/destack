use std::sync::Arc;

use destack_dir::{self as dir, LanguageItem, StringId};
use destack_source::{ModuleId, PackageId};
use destack_workspace::{LintSeverity, LinterOptions, Repository, Revision, Workspace};

use crate::linter::library::is_library_module;
use crate::{LintMeta, LintReport, LintRequirement, LintSession};

/// Context for workspace linting.
pub struct LintWorkspaceContext {
    /// Shared lint pass state.
    pub session: LintSession,
    /// The workspace being linted.
    pub workspace: Arc<Workspace>,
    /// Collected diagnostics.
    diagnostics: Vec<LintReport>,
}

impl std::fmt::Debug for LintWorkspaceContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LintWorkspaceContext")
            .field("root", &self.workspace.root)
            .field("profile_id", &self.session.profile_id)
            .finish()
    }
}

impl LintWorkspaceContext {
    /// Create a new workspace lint context.
    pub fn new(session: LintSession, workspace: Arc<Workspace>) -> Self {
        Self {
            session,
            workspace,
            diagnostics: Vec::new(),
        }
    }

    /// Get the linter options.
    pub fn options(&self) -> &LinterOptions {
        &self.session.options
    }

    /// Return the package ids in the active workspace.
    pub fn workspace_package_ids(&self) -> Vec<PackageId> {
        self.session
            .repository
            .package_ids(self.session.revision)
            .unwrap_or_else(|_| Vec::new())
    }

    /// Return all visible module ids in the active workspace.
    pub fn workspace_module_ids(&self) -> Vec<ModuleId> {
        collect_workspace_module_ids(&self.session.repository, self.session.revision)
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
fn is_lib_available(ctx: &LintWorkspaceContext, libs: &[&str]) -> bool {
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
