use std::collections::HashSet;
use std::fmt;
use std::sync::{Arc, LazyLock};

use destack_artifact::{
    ArtifactKey, ArtifactPayload, ModuleLinted, PackageLinted, ProviderContext, ProviderError,
    ProviderResult, WorkspaceLinted,
};
use destack_source::{FileId, ModuleId, PackageId};
use destack_workspace::{
    LintPreset, LinterOptions, Module, Profile, ProfileId, Repository, Revision,
};

use crate::{LintLevel, LintReport, LintRunner};

/// Key used to deduplicate lint diagnostics.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct LintReportKey {
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
    /// The artifact key does not belong to the linter.
    UnsupportedArtifact { artifact_key: ArtifactKey },
    /// The repository could not resolve revision-scoped lint inputs.
    Repository { message: String },
}

impl fmt::Display for LinterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedArtifact { artifact_key } => {
                write!(f, "unsupported lint artifact key: {artifact_key:?}")
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
    fn cached_runner(options: &LinterOptions) -> &'static LintRunner {
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

    /// Return one module for one revision when present.
    fn repository_module(&self, revision: Revision, module_id: ModuleId) -> Option<Arc<Module>> {
        self.repository.module(revision, module_id).ok().flatten()
    }

    /// Return the effective profile id for one module.
    fn module_profile_id(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> Result<ProfileId, LinterError> {
        let profile = self
            .repository
            .module_profile(revision, module_id)
            .map_err(|error| LinterError::Repository {
                message: error.to_string(),
            })?;

        Ok(profile.id())
    }

    /// Convert one lint diagnostic to a dedupe key.
    fn lint_key_from_lint_report(diagnostic: &LintReport) -> LintReportKey {
        let primary_span = diagnostic.primary;

        LintReportKey {
            code: diagnostic.code().to_string(),
            file_id: primary_span.file,
            start: primary_span.start,
            end: primary_span.end,
            message: diagnostic.message().to_string(),
        }
    }

    /// Record lint diagnostics on one provider context.
    fn record_lint_diagnostics(
        &self,
        context: &dyn ProviderContext<Revision = Revision>,
        diagnostics: impl IntoIterator<Item = LintReport>,
    ) -> Result<(), LinterError> {
        let mut seen = HashSet::new();

        // keep only the first copy of each lint diagnostic
        for diagnostic in diagnostics {
            let key = Self::lint_key_from_lint_report(&diagnostic);
            if !seen.insert(key) || !diagnostic.is_enabled() {
                continue;
            }

            context
                .emit(&diagnostic)
                .map_err(|error| LinterError::Repository {
                    message: format!("failed to emit lint diagnostic: {error}"),
                })?;
        }

        Ok(())
    }

    /// Return workspace scoped lint options for one revision.
    fn workspace_linter_options(&self, revision: Revision) -> Result<LinterOptions, LinterError> {
        let workspace_options = self
            .repository
            .workspace_options(revision)
            .map_err(|error| LinterError::Repository {
                message: error.to_string(),
            })?;

        Ok(workspace_options
            .map(|options| options.package.linter)
            .unwrap_or_default())
    }

    /// Return package scoped lint options for one revision.
    fn package_linter_options(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<LinterOptions, LinterError> {
        let package_options = self
            .repository
            .package_options(revision, package_id)
            .map_err(|error| LinterError::Repository {
                message: error.to_string(),
            })?;

        if let Some(package_options) = package_options {
            return Ok(package_options.linter);
        }

        self.workspace_linter_options(revision)
    }

    /// Return module scoped lint options for one revision.
    fn module_linter_options(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> Result<LinterOptions, LinterError> {
        let Some(module) = self.repository_module(revision, module_id) else {
            return Ok(LinterOptions::default());
        };

        self.package_linter_options(revision, module.package_id)
    }

    /// Build the compiler artifact keys required for one module lint.
    fn module_lint_dependency_keys(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Result<Vec<ArtifactKey>, LinterError> {
        let Some(module) = self.repository_module(revision, module_id) else {
            return Ok(Vec::new());
        };
        let module = module.as_ref();

        // only code modules carry AST and DIR products
        if !module.is_code() {
            return Ok(Vec::new());
        }

        let artifact_keys = vec![
            ArtifactKey::ast(module_id),
            ArtifactKey::dir_checked(module_id, profile_id),
        ];

        Ok(artifact_keys)
    }

    /// Lint one module and return fresh diagnostics.
    pub fn lint_module(
        &self,
        context: &dyn ProviderContext<Revision = Revision>,
        revision: Revision,
        module_id: ModuleId,
        profile: Profile,
    ) -> Result<(), LinterError> {
        let profile_id = profile.id();

        // validate the required compiler products up front
        let dependency_keys = self.module_lint_dependency_keys(revision, module_id, profile_id)?;
        let artifact_keys = self.missing_artifact_keys(revision, dependency_keys.iter().copied());
        if !artifact_keys.is_empty() {
            return Err(LinterError::Repository {
                message: format!("lint module dependencies are not ready: {artifact_keys:?}"),
            });
        }

        // skip non code modules after validation
        let Some(module) = self.repository_module(revision, module_id) else {
            return Ok(());
        };
        if !module.is_code() {
            return Ok(());
        }

        // skip disabled linter configurations
        let options = self.module_linter_options(revision, module_id)?;
        if !options.enabled {
            return Ok(());
        }

        // run module scoped AST and DIR lint rules
        let runner = Self::cached_runner(&options);
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

        self.record_lint_diagnostics(context, ast_diagnostics.into_iter().chain(dir_diagnostics))
    }

    /// Lint one package and record package scoped diagnostics.
    pub fn lint_package(
        &self,
        context: &dyn ProviderContext<Revision = Revision>,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<(), LinterError> {
        // collect package modules in a stable order
        let mut module_ids: Vec<_> = self
            .repository
            .package_module_ids(revision, package_id)
            .map_err(|error| LinterError::Repository {
                message: error.to_string(),
            })?;
        module_ids.sort_unstable();

        // skip package scoped rules when no code modules remain
        let Some(_options_module_id) = module_ids.iter().copied().find(|module_id| {
            self.repository
                .module(revision, *module_id)
                .ok()
                .flatten()
                .is_some_and(|module| module.is_code())
        }) else {
            return Ok(());
        };

        // skip disabled linter configurations
        let options = self.package_linter_options(revision, package_id)?;
        if !options.enabled {
            return Ok(());
        }

        // run package scoped AST rules once
        let runner = Self::cached_runner(&options);
        let ast_diagnostics =
            runner.lint_package_ast(self.repository.clone(), revision, package_id, &options);
        self.record_lint_diagnostics(context, ast_diagnostics)?;

        // run package scoped DIR rules once per active profile
        let mut profiles = HashSet::new();
        for module_id in &module_ids {
            let profile_id = self.module_profile_id(revision, *module_id)?;
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
            self.record_lint_diagnostics(context, dir_diagnostics)?;
        }

        Ok(())
    }

    /// Lint one workspace and record workspace scoped diagnostics.
    pub fn lint_workspace(
        &self,
        context: &dyn ProviderContext<Revision = Revision>,
        revision: Revision,
    ) -> Result<(), LinterError> {
        // skip disabled linter configurations
        let options = self.workspace_linter_options(revision)?;
        if !options.enabled {
            return Ok(());
        }

        // run workspace scoped AST rules once
        let runner = Self::cached_runner(&options);
        let ast_diagnostics =
            runner.lint_workspace_ast(self.repository.clone(), revision, &options);
        self.record_lint_diagnostics(context, ast_diagnostics)?;

        // run workspace scoped DIR rules once per active profile
        let mut profiles = HashSet::new();
        let module_ids =
            self.repository
                .module_ids(revision)
                .map_err(|error| LinterError::Repository {
                    message: error.to_string(),
                })?;
        for module_id in &module_ids {
            let profile_id = self.module_profile_id(revision, *module_id)?;
            profiles.insert(profile_id);
        }

        for profile_id in profiles {
            let dir_diagnostics =
                runner.lint_workspace_dir(self.repository.clone(), revision, profile_id, &options);
            self.record_lint_diagnostics(context, dir_diagnostics)?;
        }

        Ok(())
    }

    /// Provide one lint artifact key.
    pub fn provide(
        &self,
        context: &dyn ProviderContext<Revision = Revision>,
    ) -> ProviderResult<ArtifactPayload> {
        match context.artifact_key() {
            ArtifactKey::ModuleLinted { module, profile } => {
                self.provide_module(context, module, profile)
            }
            ArtifactKey::PackageLinted { package } => self.provide_package(context, package),
            ArtifactKey::WorkspaceLinted => self.provide_workspace(context),
            artifact_key => Err(ProviderError::internal(
                LinterError::UnsupportedArtifact { artifact_key }.to_string(),
            )
            .into()),
        }
    }

    /// Provide one module lint artifact.
    fn provide_module(
        &self,
        context: &dyn ProviderContext<Revision = Revision>,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ProviderResult<ArtifactPayload> {
        let revision = context.revision();
        let dependency_keys = self
            .module_lint_dependency_keys(revision, module_id, profile_id)
            .map_err(|error| ProviderError::internal(error.to_string()))?;

        for dependency_key in dependency_keys {
            context
                .require(dependency_key)
                .map_err(ProviderError::from)?;
        }

        let profile = self
            .repository
            .module_profile_by_id(revision, module_id, profile_id)
            .map_err(|error| LinterError::Repository {
                message: error.to_string(),
            })
            .map_err(|error| ProviderError::internal(error.to_string()))?
            .ok_or_else(|| LinterError::Repository {
                message: format!(
                    "missing profile {profile_id:?} for module {module_id:?} at revision {revision}"
                ),
            })
            .map_err(|error| ProviderError::internal(error.to_string()))?;
        self.lint_module(context, revision, module_id, profile.as_ref().clone())
            .map_err(|error| ProviderError::internal(error.to_string()))?;

        Ok(ArtifactPayload::ModuleLinted(ModuleLinted))
    }

    /// Provide one package lint artifact.
    fn provide_package(
        &self,
        context: &dyn ProviderContext<Revision = Revision>,
        package_id: PackageId,
    ) -> ProviderResult<ArtifactPayload> {
        let revision = context.revision();
        let dependency_keys = self
            .package_lint_dependency_keys(revision, package_id)
            .map_err(|error| ProviderError::internal(error.to_string()))?;

        for dependency_key in dependency_keys {
            context
                .require(dependency_key)
                .map_err(ProviderError::from)?;
        }

        self.lint_package(context, revision, package_id)
            .map_err(|error| ProviderError::internal(error.to_string()))?;

        Ok(ArtifactPayload::PackageLinted(PackageLinted))
    }

    /// Provide one workspace lint artifact.
    fn provide_workspace(
        &self,
        context: &dyn ProviderContext<Revision = Revision>,
    ) -> ProviderResult<ArtifactPayload> {
        let revision = context.revision();
        let dependency_keys = self
            .workspace_lint_dependency_keys(revision)
            .map_err(|error| ProviderError::internal(error.to_string()))?;

        for dependency_key in dependency_keys {
            context
                .require(dependency_key)
                .map_err(ProviderError::from)?;
        }

        self.lint_workspace(context, revision)
            .map_err(|error| ProviderError::internal(error.to_string()))?;

        Ok(ArtifactPayload::WorkspaceLinted(WorkspaceLinted))
    }

    /// Return the missing artifact keys from one required key set.
    fn missing_artifact_keys(
        &self,
        revision: Revision,
        keys: impl IntoIterator<Item = ArtifactKey>,
    ) -> Vec<ArtifactKey> {
        let mut artifact_keys = Vec::new();

        for artifact_key in keys {
            let Ok(Some(version)) = self.repository.artifact_version(revision, &artifact_key)
            else {
                artifact_keys.push(artifact_key);
                continue;
            };

            if !self.repository.artifact_store().has(&version) {
                artifact_keys.push(artifact_key);
            }
        }

        artifact_keys
    }

    /// Return the dependency keys for one package lint artifact.
    fn package_lint_dependency_keys(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<Vec<ArtifactKey>, LinterError> {
        let mut module_ids = self
            .repository
            .package_module_ids(revision, package_id)
            .map_err(|error| LinterError::Repository {
                message: error.to_string(),
            })?;
        module_ids.sort_unstable();

        let mut artifact_keys = Vec::new();

        for module_id in module_ids {
            let Some(module) = self.repository_module(revision, module_id) else {
                continue;
            };
            if !module.is_code() {
                continue;
            }

            let profile_id = self.module_profile_id(revision, module_id)?;
            artifact_keys.push(ArtifactKey::module_linted(module_id, profile_id));
        }

        Ok(artifact_keys)
    }

    /// Return the dependency keys for one workspace lint artifact.
    fn workspace_lint_dependency_keys(
        &self,
        revision: Revision,
    ) -> Result<Vec<ArtifactKey>, LinterError> {
        let mut package_ids =
            self.repository
                .package_ids(revision)
                .map_err(|error| LinterError::Repository {
                    message: error.to_string(),
                })?;
        package_ids.sort_unstable();

        Ok(package_ids
            .into_iter()
            .map(ArtifactKey::package_linted)
            .collect())
    }
}
