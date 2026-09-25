use std::fmt;
use std::sync::Arc;

use tspp_artifact::{ArtifactDependencySet, ArtifactFailure, DiagnosticLike};
use tspp_repository::{
    ArtifactReader, LinterOptions, Module, ProviderContext, ProviderError, Repository, Revision,
};
use tspp_source::{ModuleId, PackageId};

use super::LintSet;

#[cfg(test)]
use crate::Lint;

/// Linter for one repository.
#[derive(Clone)]
pub struct Linter {
    /// The repository.
    pub(super) repository: Arc<Repository>,
    /// The single lint selected by an isolated test run.
    #[cfg(test)]
    lint: Option<&'static Lint>,
}

impl fmt::Debug for Linter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Linter")
            .field("repository", &"...")
            .finish()
    }
}

impl Linter {
    /// Create a linter.
    pub fn new(repository: Arc<Repository>) -> Self {
        Self {
            repository,
            #[cfg(test)]
            lint: None,
        }
    }

    /// Create a linter that runs one implementation in an isolated test.
    #[cfg(test)]
    pub(crate) fn with_lint(repository: Arc<Repository>, lint: &'static Lint) -> Self {
        Self {
            repository,
            lint: Some(lint),
        }
    }

    /// Return the artifact reader for one provider attempt.
    pub(super) fn artifact_reader<'a>(
        &'a self,
        context: &'a dyn ProviderContext,
    ) -> ArtifactReader<'a> {
        let artifacts = self
            .repository
            .artifact_reader(context.revision())
            .with_context(context);
        let Some(dependencies) = context.artifact_dependencies() else {
            return artifacts;
        };

        artifacts.restrict(dependencies)
    }

    /// Resolve the lints selected by one package.
    pub(super) fn resolve_lints(
        &self,
        context: &dyn ProviderContext,
        package: PackageId,
    ) -> Result<LintSet, ProviderError> {
        #[cfg(test)]
        if let Some(lint) = self.lint {
            return Ok(LintSet::single(lint));
        }

        let revision = context.revision();
        let config = self
            .repository
            .destack_for_package_id(revision, package)
            .map_err(|error| ProviderError::internal(error.to_string()))?;
        let defaults = LinterOptions::default();
        let options = config.as_ref().map_or(&defaults, |config| &config.linter);
        let lints = LintSet::resolve(package, options);

        Ok(lints)
    }

    /// Reject one lint set containing invalid configuration.
    pub(super) fn reject_invalid_lints(
        &self,
        context: &dyn ProviderContext,
        lints: &LintSet,
    ) -> Result<(), ProviderError> {
        if lints.errors().is_empty() {
            return Ok(());
        }

        // emit every independent configuration error
        self.emit(context, lints.errors().iter().cloned())?;

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

        // observe exact configuration Blobs
        for file in &config.file_ids {
            let blob = self
                .repository
                .file_blob(revision, *file)
                .map_err(|error| ProviderError::internal(error.to_string()))?
                .ok_or_else(|| {
                    ProviderError::internal(format!(
                        "missing linter configuration Blob for File {file:?}"
                    ))
                })?;
            dependencies.observe_file(*file, blob.id);
        }

        Ok(())
    }
}
