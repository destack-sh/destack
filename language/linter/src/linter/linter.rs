use std::collections::HashSet;
use std::fmt;
use std::sync::{Arc, LazyLock};

use destack_source::{Diagnostic, FileId, ModuleId, PackageId};
use destack_workspace::{LintPreset, LinterOptions, ProfileId, Program};

use crate::{LintDiagnostic, LintLevel, LintRunner};

/// Key used to deduplicate lint diagnostics.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct LintDiagnosticKey {
    /// The diagnostic code.
    code: String,
    /// The file id.
    file_id: FileId,
    /// Primary span start offset.
    start: u32,
    /// Primary span end offset.
    end: u32,
    /// The diagnostic message.
    message: String,
}

/// Error while linting already-built compiler products.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinterError {
    /// The module has no AST yet.
    MissingAst { module_id: ModuleId },
    /// The module has no analyzed DIR for the requested profile yet.
    MissingDir {
        module_id: ModuleId,
        profile_id: ProfileId,
    },
}

impl fmt::Display for LinterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingAst { module_id } => {
                write!(f, "module {module_id:?} has no AST available for linting")
            }
            Self::MissingDir {
                module_id,
                profile_id,
            } => {
                write!(
                    f,
                    "module {module_id:?} has no analyzed DIR available for profile {profile_id:?}",
                )
            }
        }
    }
}

impl std::error::Error for LinterError {}

/// Consumer that runs lint rules over already-built compiler products.
#[derive(Debug, Clone)]
pub struct Linter {
    /// The program being linted.
    program: Arc<Program>,
}

impl Linter {
    /// Create a linter for one program.
    pub fn new(program: Arc<Program>) -> Self {
        Self { program }
    }

    /// Reuse immutable lint runners by preset to avoid per-module rule allocation.
    fn cached_runner_for_options(options: &LinterOptions) -> &'static LintRunner {
        static NONE: LazyLock<LintRunner> =
            LazyLock::new(|| LintRunner::from_preset(LintPreset::None).with_fixes(false));
        static RECOMMENDED: LazyLock<LintRunner> =
            LazyLock::new(|| LintRunner::from_preset(LintPreset::Recommended).with_fixes(false));
        static STRICT: LazyLock<LintRunner> =
            LazyLock::new(|| LintRunner::from_preset(LintPreset::Strict).with_fixes(false));
        static ALL: LazyLock<LintRunner> =
            LazyLock::new(|| LintRunner::from_preset(LintPreset::All).with_fixes(false));

        // explicit category or rule overrides can enable any rule
        if !options.categories.is_empty() || !options.overrides.is_empty() {
            return &ALL;
        }

        match options.preset {
            LintPreset::None => &NONE,
            LintPreset::Recommended => &RECOMMENDED,
            LintPreset::Strict => &STRICT,
            LintPreset::All => &ALL,
        }
    }

    /// Convert one source diagnostic to a lint dedupe key.
    fn lint_key_from_source_diagnostic(diagnostic: &Diagnostic) -> LintDiagnosticKey {
        LintDiagnosticKey {
            code: diagnostic.code.clone(),
            file_id: diagnostic.file_id,
            start: diagnostic.primary_span.span.start,
            end: diagnostic.primary_span.span.end,
            message: diagnostic.message.clone(),
        }
    }

    /// Convert one lint diagnostic to a dedupe key.
    fn lint_key_from_lint_diagnostic(diagnostic: &LintDiagnostic) -> LintDiagnosticKey {
        LintDiagnosticKey {
            code: diagnostic.code.to_string(),
            file_id: diagnostic.file_id,
            start: diagnostic.span.start,
            end: diagnostic.span.end,
            message: diagnostic.message.clone(),
        }
    }

    /// Insert lint diagnostics while deduplicating against existing diagnostics.
    fn insert_lint_diagnostics(&self, diagnostics: impl IntoIterator<Item = LintDiagnostic>) {
        // seed dedupe state from already emitted diagnostics
        let mut seen = HashSet::new();
        for diagnostic in self.program.diagnostics.iter() {
            if diagnostic.code.starts_with('L') {
                seen.insert(Self::lint_key_from_source_diagnostic(&diagnostic));
            }
        }

        // insert only fresh lint diagnostics
        for diagnostic in diagnostics {
            let key = Self::lint_key_from_lint_diagnostic(&diagnostic);
            if seen.insert(key) {
                self.program
                    .diagnostics
                    .insert(diagnostic.into_diagnostic());
            }
        }
    }

    /// Ensure the module has the products required for linting.
    fn validate_module_inputs(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Result<(), LinterError> {
        let module = self.program.modules.get(module_id);
        let module = module.read();

        // only code modules carry AST and DIR products
        if !module.is_code() {
            return Ok(());
        }

        // require the AST product
        if module.ast_maybe().is_none() {
            return Err(LinterError::MissingAst { module_id });
        }

        // require the analyzed DIR product
        if module.dir_maybe(profile_id).is_none() {
            return Err(LinterError::MissingDir {
                module_id,
                profile_id,
            });
        }

        Ok(())
    }

    /// Lint one module and insert fresh diagnostics into the program collection.
    pub fn lint_module(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Result<(), LinterError> {
        // validate the required compiler products up front
        self.validate_module_inputs(module_id, profile_id)?;

        // skip non code modules after validation
        let module = self.program.modules.get(module_id);
        if !module.read().is_code() {
            return Ok(());
        }

        // skip disabled linter configurations
        let options = self.program.get_linter_options(module_id);
        if !options.enabled {
            return Ok(());
        }

        // run module scoped AST and DIR lint rules
        let runner = Self::cached_runner_for_options(&options);
        let ast_diagnostics = runner.lint_module_by_id(
            self.program.clone(),
            module_id,
            profile_id,
            &options,
            LintLevel::Ast,
        );
        let dir_diagnostics = runner.lint_module_by_id(
            self.program.clone(),
            module_id,
            profile_id,
            &options,
            LintLevel::Dir,
        );

        // merge fresh diagnostics into the program collection
        self.insert_lint_diagnostics(ast_diagnostics.into_iter().chain(dir_diagnostics));

        Ok(())
    }

    /// Lint one package and insert fresh diagnostics into the program collection.
    pub fn lint_package(&self, package_id: PackageId) -> Result<(), LinterError> {
        // collect package modules in a stable order
        let mut module_ids: Vec<_> = self
            .program
            .modules
            .iter()
            .filter(|module| module.read().package_id == package_id)
            .map(|module| module.read().id)
            .collect();
        module_ids.sort_unstable();

        // lint each module with its default profile
        for module_id in &module_ids {
            let profile_id = self.program.default_profile_id_for_module(*module_id);
            self.lint_module(*module_id, profile_id)?;
        }

        // skip package scoped rules when no code modules remain
        let Some(options_module_id) = module_ids.iter().copied().find(|module_id| {
            let module = self.program.modules.get(*module_id);
            module.read().is_code()
        }) else {
            return Ok(());
        };

        // skip disabled linter configurations
        let options = self.program.get_linter_options(options_module_id);
        if !options.enabled {
            return Ok(());
        }

        // run program scoped AST rules once
        let runner = Self::cached_runner_for_options(&options);
        let ast_diagnostics = runner.lint_program_ast(self.program.clone(), &options);
        self.insert_lint_diagnostics(ast_diagnostics);

        // run program scoped DIR rules once per active profile
        let mut profiles = HashSet::new();
        for module_id in &module_ids {
            let profile_id = self.program.default_profile_id_for_module(*module_id);
            profiles.insert(profile_id);
        }
        for profile_id in profiles {
            let dir_diagnostics =
                runner.lint_program_dir(self.program.clone(), profile_id, &options);
            self.insert_lint_diagnostics(dir_diagnostics);
        }

        Ok(())
    }
}
