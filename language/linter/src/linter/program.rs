use std::sync::Arc;

use destack_ast::StringId;
use destack_dir::{self as dir, WellKnownSymbol};
use destack_workspace::{LintSeverity, LinterOptions, ProfileId, Program, WellKnownSymbols};

use crate::{LintDiagnostic, LintMeta, LintRequirement};

/// Context for AST-level program linting.
pub struct LintProgramAstContext {
    /// The program being linted.
    pub program: Arc<Program>,
    /// Linter configuration.
    options: LinterOptions,
    /// Collected diagnostics.
    diagnostics: Vec<LintDiagnostic>,
}

impl std::fmt::Debug for LintProgramAstContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LintProgramAstContext").finish()
    }
}

impl LintProgramAstContext {
    /// Create a new AST program lint context.
    pub fn new(program: Arc<Program>, options: LinterOptions) -> Self {
        Self {
            program,
            options,
            diagnostics: Vec::new(),
        }
    }

    /// Get the linter options.
    pub fn options(&self) -> &LinterOptions {
        &self.options
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
        false // AST program context has no symbol table
    }

    /// Check if a rule is supported.
    pub fn is_rule_supported(&self, meta: &LintMeta) -> bool {
        // requires all
        if !meta.requires_all.is_empty() {
            for requirement in meta.requires_all {
                if !self.is_requirement_met(requirement) {
                    return false;
                }
            }
        }

        // requires any
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
    pub fn report(&mut self, diagnostic: LintDiagnostic) {
        if diagnostic.is_enabled() {
            self.diagnostics.push(diagnostic);
        }
    }

    /// Take the collected diagnostics.
    pub fn take_diagnostics(&mut self) -> Vec<LintDiagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Get reference to collected diagnostics.
    pub fn diagnostics(&self) -> &[LintDiagnostic] {
        &self.diagnostics
    }
}

/// Context for DIR-level program linting.
pub struct LintProgramDirContext {
    /// The program being linted.
    pub program: Arc<Program>,
    /// The active profile for this program pass.
    pub profile_id: ProfileId,
    /// Linter configuration.
    options: LinterOptions,
    /// Collected diagnostics.
    diagnostics: Vec<LintDiagnostic>,
}

impl std::fmt::Debug for LintProgramDirContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LintProgramDirContext")
            .field("profile_id", &self.profile_id)
            .finish()
    }
}

impl LintProgramDirContext {
    /// Create a new DIR program lint context.
    pub fn new(program: Arc<Program>, profile_id: ProfileId, options: LinterOptions) -> Self {
        Self {
            program,
            profile_id,
            options,
            diagnostics: Vec::new(),
        }
    }

    /// Get the linter options.
    pub fn options(&self) -> &LinterOptions {
        &self.options
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

    /// Get a cached declared lib symbol for the active profile and name.
    pub fn get_declared_lib_symbol(&self, name: StringId) -> Option<dir::GlobalSymbolId> {
        let builtins = self.program.builtins.as_ref()?;
        let profile = self.program.profile(self.profile_id);
        builtins.get_declared_lib_symbol_from(
            &profile.key,
            name,
            dir::SymbolSpaceOrder::ValueThenType,
        )
    }

    /// Get well-known symbols for the active profile.
    pub fn get_well_known_symbols(&self) -> Option<WellKnownSymbols> {
        let builtins = self.program.builtins.as_ref()?;
        let profile = self.program.profile(self.profile_id);
        builtins.well_known_symbols(&profile.key)
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
                if !self.is_lib_available(libs) {
                    return false;
                }
                let name = self.program.strings.intern(name);
                self.get_declared_lib_symbol(name).is_some()
            }
            LintRequirement::RequireWellKnownSymbol(symbol) => {
                self.get_well_known_symbol(*symbol).is_some()
            }
        }
    }

    /// Return true when at least one of the required libs is available.
    fn is_lib_available(&self, libs: &[&str]) -> bool {
        if libs.is_empty() {
            return true;
        }
        let Some(builtins) = self.program.builtins.as_ref() else {
            return false;
        };
        let profile = self.program.profile(self.profile_id);
        let Some(ambient_modules) = builtins.ambient_libs(&profile.key) else {
            return false;
        };

        for module_id in ambient_modules {
            let Some(lib_name) = builtins.lib_name_for_module(module_id) else {
                continue;
            };
            if libs.contains(&lib_name) {
                return true;
            }
        }

        false
    }

    /// Check if a rule is supported.
    pub fn is_rule_supported(&self, meta: &LintMeta) -> bool {
        // requires all
        if !meta.requires_all.is_empty() {
            for requirement in meta.requires_all {
                if !self.is_requirement_met(requirement) {
                    return false;
                }
            }
        }

        // requires any
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
    pub fn report(&mut self, diagnostic: LintDiagnostic) {
        if diagnostic.is_enabled() {
            self.diagnostics.push(diagnostic);
        }
    }

    /// Take the collected diagnostics.
    pub fn take_diagnostics(&mut self) -> Vec<LintDiagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Get reference to collected diagnostics.
    pub fn diagnostics(&self) -> &[LintDiagnostic] {
        &self.diagnostics
    }
}
