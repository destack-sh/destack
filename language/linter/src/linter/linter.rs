use std::fmt;
use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactFailure, DiagnosticDefinition, DiagnosticLike,
};
use destack_repository::{
    LinterOptions, Module, ProviderContext, ProviderError, Repository, Revision,
};
use destack_source::{ModuleId, PackageId};

use super::LintSet;
use crate::{LINTS, Lint};

/// Linter for one repository.
#[derive(Clone)]
pub struct Linter {
    /// The repository.
    pub(super) repository: Arc<Repository>,
    /// The check warnings.
    pub(super) check_warnings: Arc<[&'static DiagnosticDefinition]>,
    /// The lints.
    pub(super) lints: Arc<[Lint]>,
}

impl fmt::Debug for Linter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Linter")
            .field("repository", &"...")
            .field("check_warnings", &self.check_warnings.len())
            .field("lints", &self.lints.len())
            .finish()
    }
}

impl Linter {
    /// Create a linter.
    pub fn new(
        repository: Arc<Repository>,
        check_warnings: impl IntoIterator<Item = &'static DiagnosticDefinition>,
    ) -> Self {
        let check_warnings = check_warnings.into_iter().collect::<Vec<_>>();
        let lints = LINTS.iter().map(|lint| (*lint).clone()).collect::<Vec<_>>();

        Self {
            repository,
            check_warnings: check_warnings.into(),
            lints: lints.into(),
        }
    }

    /// Resolve one package's linter options.
    pub(super) fn resolve_lints(
        &self,
        context: &dyn ProviderContext,
        package: PackageId,
    ) -> Result<LintSet, ProviderError> {
        let revision = context.revision();
        let config = self
            .repository
            .destack_for_package_id(revision, package)
            .map_err(|error| ProviderError::internal(error.to_string()))?;
        let defaults = LinterOptions::default();
        let options = config.as_ref().map_or(&defaults, |config| &config.linter);
        let lints = LintSet::resolve(package, options, self.lints.clone());

        match lints {
            Ok(lints) => Ok(lints),
            Err(error) => self.reject(context, error),
        }
    }

    /// Emit one source error and reject the current lint artifact.
    pub(super) fn reject<T>(
        &self,
        context: &dyn ProviderContext,
        diagnostic: impl DiagnosticLike,
    ) -> Result<T, ProviderError> {
        context
            .emit(&diagnostic)
            .map_err(|error| ProviderError::internal(error.to_string()))?;

        Err(ProviderError::failed(ArtifactFailure::diagnostics()))
    }

    /// Return one repository module.
    pub(super) fn module(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> Result<Arc<Module>, ProviderError> {
        let module = self
            .repository
            .module(revision, module_id)
            .map_err(|error| ProviderError::internal(error.to_string()))?;

        module.ok_or_else(|| ProviderError::internal(format!("missing lint module {module_id:?}")))
    }

    /// Emit lint diagnostics.
    pub(super) fn emit<D>(
        &self,
        context: &dyn ProviderContext,
        diagnostics: impl IntoIterator<Item = D>,
    ) -> Result<(), ProviderError>
    where
        D: DiagnosticLike,
    {
        for diagnostic in diagnostics {
            context
                .emit(&diagnostic)
                .map_err(|error| ProviderError::internal(error.to_string()))?;
        }

        Ok(())
    }

    /// Observe the linter configuration files.
    pub(super) fn observe_configuration(
        &self,
        context: &dyn ProviderContext,
        package: PackageId,
        dependencies: &mut ArtifactDependencySet,
    ) -> Result<(), ProviderError> {
        let revision = context.revision();
        let config = self
            .repository
            .destack_for_package_id(revision, package)
            .map_err(|error| ProviderError::internal(error.to_string()))?;
        let Some(config) = config else {
            return Ok(());
        };

        // observe configuration contents
        for file in &config.file_ids {
            let content = self
                .repository
                .file_content_id(revision, *file)
                .map_err(|error| ProviderError::internal(error.to_string()))?
                .ok_or_else(|| {
                    ProviderError::internal(format!(
                        "missing linter configuration content for file {file:?}"
                    ))
                })?;
            dependencies.observe_file(*file, content);
        }

        Ok(())
    }
}
