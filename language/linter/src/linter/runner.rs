use std::sync::Arc;

use parking_lot::RwLock;

use destack_source::ModuleId;
use destack_workspace::{LintPreset, LinterOptions, Module, ProfileId, Program};

use crate::{
    BoxedLintRule, LintDiagnostic, LintLevel, LintModuleAstContext, LintModuleDirContext,
    LintProgramContext, all_rules, recommended_rules,
};

/// Runs lint rules against modules and programs.
pub struct LintRunner {
    rules: Vec<BoxedLintRule>,
    /// Whether to compute fixes for diagnostics.
    compute_fixes: bool,
}

impl std::fmt::Debug for LintRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LintRunner")
            .field("rules", &self.rules.len())
            .finish()
    }
}

impl LintRunner {
    /// Create a runner with the given rules.
    pub fn new(rules: Vec<BoxedLintRule>) -> Self {
        Self {
            rules,
            compute_fixes: true,
        }
    }

    /// Enable or disable fix computation (builder pattern).
    pub fn with_fixes(mut self, compute: bool) -> Self {
        self.compute_fixes = compute;
        self
    }

    /// Check if fix computation is enabled.
    pub fn compute_fixes(&self) -> bool {
        self.compute_fixes
    }

    /// Create a runner with rules based on preset.
    pub fn from_preset(preset: LintPreset) -> Self {
        let rules = match preset {
            LintPreset::None => Vec::new(),
            LintPreset::Recommended => recommended_rules(),
            LintPreset::All => all_rules(),
        };
        Self {
            rules,
            compute_fixes: true,
        }
    }

    /// Create a runner from linter options.
    pub fn from_options(options: &LinterOptions) -> Self {
        Self::from_preset(options.preset)
    }

    /// Create a runner with all rules.
    pub fn all() -> Self {
        Self::new(all_rules())
    }

    /// Create a runner with recommended rules.
    pub fn recommended() -> Self {
        Self::new(recommended_rules())
    }

    /// Create an empty runner.
    pub fn empty() -> Self {
        Self {
            rules: Vec::new(),
            compute_fixes: true,
        }
    }

    /// Get the rules.
    pub fn rules(&self) -> &[BoxedLintRule] {
        &self.rules
    }

    /// Lint a module at a specific IR level.
    pub fn lint_module(
        &self,
        program: Arc<Program>,
        module: Arc<RwLock<Module>>,
        profile: ProfileId,
        options: &LinterOptions,
        level: LintLevel,
    ) -> Vec<LintDiagnostic> {
        if !options.enabled {
            return Vec::new();
        }
        match level {
            LintLevel::Ast => self.lint_module_ast(program, module, options),
            LintLevel::Dir => self.lint_module_dir(program, module, profile, options),
            LintLevel::Mir => todo!(),
        }
    }

    /// Lint a module at AST level.
    fn lint_module_ast(
        &self,
        program: Arc<Program>,
        module: Arc<RwLock<Module>>,
        options: &LinterOptions,
    ) -> Vec<LintDiagnostic> {
        let module = module.read();
        let ast = &module.ast();
        let file = program.files.get(module.file_id);
        let mut ctx = LintModuleAstContext::new(
            program,
            &module,
            file,
            &ast.tree,
            &ast.parents,
            &ast.roots,
            &ast.strings,
            options,
            self.compute_fixes,
        );

        for rule in &self.rules {
            if rule.meta().level != LintLevel::Ast {
                continue;
            }
            if !ctx.is_rule_enabled(rule.meta()) {
                continue;
            }
            let severity = ctx.get_severity(rule.meta());
            rule.check_module_ast(severity, &mut ctx);
        }

        ctx.take_diagnostics()
    }

    /// Lint a module at DIR level.
    fn lint_module_dir(
        &self,
        program: Arc<Program>,
        module: Arc<RwLock<Module>>,
        profile: ProfileId,
        options: &LinterOptions,
    ) -> Vec<LintDiagnostic> {
        // context
        let module = module.read();
        let ast = &module.ast();
        let file = program.files.get(module.file_id);
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();
        let types = dir.types.read();
        let namespace_exports = dir.namespace_exports.read();
        let imported_modules = dir.imported_modules.read();
        let exported_symbols = dir.exported_symbols.read();
        let mut ctx = LintModuleDirContext::new(
            program,
            &module,
            file,
            &ast.tree,
            &tree,
            &symbols,
            &types,
            dir.roots.clone(),
            dir.namespace_symbol,
            dir.namespace_scope,
            dir.default_symbol,
            namespace_exports.clone(),
            imported_modules.clone(),
            exported_symbols.clone(),
            options,
            self.compute_fixes,
        );

        // check rules
        for rule in &self.rules {
            if rule.meta().level != LintLevel::Dir {
                continue;
            }
            if !ctx.is_rule_enabled(rule.meta()) {
                continue;
            }
            let severity = ctx.get_severity(rule.meta());
            rule.check_module_dir(severity, &mut ctx);
        }

        ctx.take_diagnostics()
    }

    /// Lint a module by id.
    pub fn lint_module_by_id(
        &self,
        program: Arc<Program>,
        module_id: ModuleId,
        profile: ProfileId,
        options: &LinterOptions,
        level: LintLevel,
    ) -> Vec<LintDiagnostic> {
        let module = program.modules.get(module_id);
        self.lint_module(program, module, profile, options, level)
    }

    /// Lint all modules at a specific level.
    pub fn lint_all_modules(
        &self,
        program: Arc<Program>,
        options: &LinterOptions,
        level: LintLevel,
    ) -> Vec<LintDiagnostic> {
        if !options.enabled {
            return Vec::new();
        }

        let mut diagnostics = Vec::new();
        for module in program.modules.iter() {
            let profile = program.default_profile_id_for_module(module.read().id);
            diagnostics.extend(self.lint_module(program.clone(), module, profile, options, level));
        }
        diagnostics
    }

    /// Lint the entire program (cross-module analysis).
    pub fn lint_program(
        &self,
        program: Arc<Program>,
        options: &LinterOptions,
    ) -> Vec<LintDiagnostic> {
        if !options.enabled {
            return Vec::new();
        }

        let mut context = LintProgramContext::new(program, options.clone());
        for rule in &self.rules {
            if !context.is_rule_enabled(rule.meta()) {
                continue;
            }
            rule.check_program(&mut context);
        }
        context.take_diagnostics()
    }
}

impl Default for LintRunner {
    fn default() -> Self {
        Self::recommended()
    }
}
