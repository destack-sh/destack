use std::collections::HashSet;
use std::fmt;
use std::sync::{Arc, LazyLock};

use destack_source::{DiagnosticCollection, FileId, ModuleId, PackageId};
use destack_workspace::{LintPreset, LinterOptions, Profile, ProfileId, Repository, Revision};

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
    /// The repository could not resolve revision-scoped lint inputs.
    Repository { message: String },
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
            Self::Repository { message } => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for LinterError {}

/// Consumer that runs lint rules over already-built compiler products.
#[derive(Debug, Clone)]
pub struct Linter {
    /// The repository being linted.
    repository: Arc<Repository>,
}

impl Linter {
    /// Create a linter for one repository.
    pub fn new(repository: Arc<Repository>) -> Self {
        Self { repository }
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

    /// Return one module snapshot for one revision when present.
    fn repository_module(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> Option<Arc<destack_workspace::Module>> {
        self.repository.module(revision, module_id).ok().flatten()
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

    /// Build one owned diagnostics collection from lint diagnostics.
    fn collect_lint_diagnostics(
        &self,
        diagnostics: impl IntoIterator<Item = LintDiagnostic>,
    ) -> DiagnosticCollection {
        let mut seen = HashSet::new();
        let mut collection = DiagnosticCollection::new();

        // keep only the first copy of each lint diagnostic
        for diagnostic in diagnostics {
            let key = Self::lint_key_from_lint_diagnostic(&diagnostic);
            if seen.insert(key) {
                collection.insert(diagnostic.into_diagnostic());
            }
        }

        collection
    }

    /// Ensure the module has the products required for linting.
    fn validate_module_inputs(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Result<(), LinterError> {
        let Some(module) = self.repository_module(revision, module_id) else {
            return Ok(());
        };
        let module = module.as_ref();

        // only code modules carry AST and DIR products
        if !module.is_code() {
            return Ok(());
        }

        // require the AST product
        if self.repository.ast(revision, module_id).is_none() {
            return Err(LinterError::MissingAst { module_id });
        }

        // require the analyzed DIR product
        if self
            .repository
            .dir_analyzed(revision, module_id, profile_id)
            .is_none()
        {
            return Err(LinterError::MissingDir {
                module_id,
                profile_id,
            });
        }

        Ok(())
    }

    /// Lint one module and return fresh diagnostics.
    pub fn lint_module(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile: Profile,
    ) -> Result<DiagnosticCollection, LinterError> {
        let profile_id = profile.id();

        // validate the required compiler products up front
        self.validate_module_inputs(revision, module_id, profile_id)?;

        // skip non code modules after validation
        let Some(module) = self.repository_module(revision, module_id) else {
            return Ok(DiagnosticCollection::new());
        };
        if !module.is_code() {
            return Ok(DiagnosticCollection::new());
        }

        // skip disabled linter configurations
        let options = self.repository.linter.clone();
        if !options.enabled {
            return Ok(DiagnosticCollection::new());
        }

        // run module scoped AST and DIR lint rules
        let runner = Self::cached_runner_for_options(&options);
        let ast_diagnostics = runner.lint_module_by_id(
            self.repository.clone(),
            revision,
            module_id,
            profile.clone(),
            &options,
            LintLevel::Ast,
        );
        let dir_diagnostics = runner.lint_module_by_id(
            self.repository.clone(),
            revision,
            module_id,
            profile,
            &options,
            LintLevel::Dir,
        );

        Ok(self.collect_lint_diagnostics(ast_diagnostics.into_iter().chain(dir_diagnostics)))
    }

    /// Lint one package and return fresh diagnostics.
    pub fn lint_package(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<DiagnosticCollection, LinterError> {
        let mut diagnostics = DiagnosticCollection::new();

        // collect package modules in a stable order
        let mut module_ids: Vec<_> = self
            .repository
            .package_module_ids(revision, package_id)
            .expect("package module ids should load");
        module_ids.sort_unstable();

        // lint each module with its default profile
        for module_id in &module_ids {
            let profile_id = self
                .repository
                .default_profile_id_for_module(revision, *module_id)
                .map_err(|error| LinterError::Repository {
                    message: error.to_string(),
                })?;
            let profile = self
                .repository
                .default_profile_for_module(revision, *module_id)
                .map_err(|error| LinterError::Repository {
                    message: error.to_string(),
                })?;
            debug_assert_eq!(profile.id(), profile_id);
            diagnostics.merge_from(&self.lint_module(revision, *module_id, profile)?);
        }

        // skip package scoped rules when no code modules remain
        let Some(_options_module_id) = module_ids.iter().copied().find(|module_id| {
            self.repository
                .module(revision, *module_id)
                .ok()
                .flatten()
                .is_some_and(|module| module.is_code())
        }) else {
            return Ok(diagnostics);
        };

        // skip disabled linter configurations
        let options = self.repository.linter.clone();
        if !options.enabled {
            return Ok(diagnostics);
        }

        // run package scoped AST rules once
        let runner = Self::cached_runner_for_options(&options);
        let ast_diagnostics =
            runner.lint_package_ast(self.repository.clone(), revision, package_id, &options);
        diagnostics.merge_from(&self.collect_lint_diagnostics(ast_diagnostics));

        // run package scoped DIR rules once per active profile
        let mut profiles = HashSet::new();
        for module_id in &module_ids {
            let profile_id = self
                .repository
                .default_profile_id_for_module(revision, *module_id)
                .map_err(|error| LinterError::Repository {
                    message: error.to_string(),
                })?;
            profiles.insert(profile_id);
        }
        for profile_id in profiles {
            let dir_diagnostics = runner.lint_package_dir(
                self.repository.clone(),
                revision,
                package_id,
                profile_id,
                &options,
            );
            diagnostics.merge_from(&self.collect_lint_diagnostics(dir_diagnostics));
        }

        Ok(diagnostics)
    }

    /// Lint one workspace and return fresh diagnostics.
    pub fn lint_workspace(&self, revision: Revision) -> Result<DiagnosticCollection, LinterError> {
        let mut diagnostics = DiagnosticCollection::new();
        let package_ids = self
            .repository
            .workspace_package_ids(revision)
            .map_err(|error| LinterError::Repository {
                message: error.to_string(),
            })?;

        // lint package scoped rules and module scoped rules first
        for package_id in &package_ids {
            diagnostics.merge_from(&self.lint_package(revision, *package_id)?);
        }

        // skip disabled linter configurations
        let options = self.repository.linter.clone();
        if !options.enabled {
            return Ok(diagnostics);
        }

        // run workspace scoped AST rules once
        let runner = Self::cached_runner_for_options(&options);
        let ast_diagnostics =
            runner.lint_workspace_ast(self.repository.clone(), revision, &options);
        diagnostics.merge_from(&self.collect_lint_diagnostics(ast_diagnostics));

        // run workspace scoped DIR rules once per active profile
        let mut profiles = HashSet::new();
        let module_ids = self
            .repository
            .workspace_module_ids(revision)
            .map_err(|error| LinterError::Repository {
                message: error.to_string(),
            })?;
        for module_id in &module_ids {
            let profile_id = self
                .repository
                .default_profile_id_for_module(revision, *module_id)
                .map_err(|error| LinterError::Repository {
                    message: error.to_string(),
                })?;
            profiles.insert(profile_id);
        }

        for profile_id in profiles {
            let dir_diagnostics =
                runner.lint_workspace_dir(self.repository.clone(), revision, profile_id, &options);
            diagnostics.merge_from(&self.collect_lint_diagnostics(dir_diagnostics));
        }

        Ok(diagnostics)
    }
}
