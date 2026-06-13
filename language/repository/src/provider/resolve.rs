use destack_artifact::{
    ArtifactDependency, ArtifactDependencySet, ArtifactFailure, ArtifactKey, ArtifactOutcome,
    ArtifactPayload, ArtifactSidecar, ArtifactVersion,
};
use destack_source::DiagnosticCollection;

use crate::provider::{ProviderError, ProviderResult};
use crate::repository::{Repository, Revision};

/// The terminal product of one provider attempt.
#[derive(Debug)]
pub enum ArtifactProduct {
    /// The attempt produced a ready payload.
    Ready(ArtifactPayload),
    /// The attempt failed with a terminal failure.
    Failed(ArtifactFailure),
}

/// One artifact a provider produced, with the outputs it emitted.
#[derive(Debug)]
pub struct ProviderOutput {
    /// The produced artifact or its terminal failure.
    pub product: ArtifactProduct,
    /// The diagnostics the attempt emitted.
    pub diagnostics: DiagnosticCollection,
    /// The sidecars the attempt emitted.
    pub sidecars: Vec<ArtifactSidecar>,
}

/// A system that declares one artifact's dependency closure.
pub trait Collector {
    /// Declare the dependencies one artifact reads given what is built.
    fn collect(
        &self,
        revision: Revision,
        key: ArtifactKey,
    ) -> ProviderResult<ArtifactDependencySet>;
}

/// A system that declares an artifact's dependencies, then builds it.
pub trait Provider: Collector {
    /// Build one artifact over its frozen dependency closure.
    fn provide(&self, revision: Revision, key: ArtifactKey) -> ProviderResult<ProviderOutput>;
}

/// One artifact's collected dependency closure.
#[derive(Debug)]
pub enum Collected {
    /// The closure names dependencies that are not yet terminal.
    Waiting {
        /// The declared dependency keys that are not yet terminal.
        frontier: Vec<ArtifactKey>,
        /// The fully named closure, present when the collect pass was complete.
        ///
        /// A complete closure cannot change once its frontier is built, so a
        /// caller may freeze it directly instead of collecting a second time.
        /// A partial closure is absent, since building the frontier may reveal
        /// further dependencies on the next pass.
        closure: Option<ArtifactDependencySet>,
    },
    /// The closure is frozen, with one poisoned dependency when present.
    Frozen {
        /// The exact dependencies feeding this artifact's version.
        dependencies: Vec<ArtifactDependency>,
        /// The first terminally failed dependency, when one poisons the build.
        failed: Option<ArtifactKey>,
    },
}

impl Repository {
    /// Collect and classify one artifact's dependency closure.
    pub fn collect_closure<P: Collector + ?Sized>(
        &self,
        provider: &P,
        revision: Revision,
        key: ArtifactKey,
    ) -> ProviderResult<Collected> {
        loop {
            let set = provider.collect(revision, key)?;

            // classify each declared artifact dependency
            let mut pending = Vec::new();
            let mut failed = None;
            for dependency in &set.artifacts {
                match self.artifact_outcome(revision, *dependency)? {
                    None => pending.push(*dependency),
                    Some(ArtifactOutcome::Failed(_)) => failed = Some(*dependency),
                    Some(ArtifactOutcome::Ok) => {}
                }
            }

            // a poisoned dependency freezes the closure immediately
            if let Some(failed) = failed {
                let dependencies = self.resolve_dependencies(revision, &set)?;

                return Ok(Collected::Frozen {
                    dependencies,
                    failed: Some(failed),
                });
            }

            // park until the unready frontier becomes terminal, carrying a
            // complete closure so the caller can freeze it without collecting again
            if !pending.is_empty() {
                let closure = (!set.is_partial).then_some(set);

                return Ok(Collected::Waiting {
                    frontier: pending,
                    closure,
                });
            }

            // a partial pass with everything built widens on the next pass
            if set.is_partial {
                continue;
            }

            // a complete pass with every dependency terminal freezes the closure
            let dependencies = self.resolve_dependencies(revision, &set)?;

            return Ok(Collected::Frozen {
                dependencies,
                failed: None,
            });
        }
    }

    /// Build or reuse one artifact from its frozen closure, recording its outcome.
    pub fn commit<P: Provider + ?Sized>(
        &self,
        provider: &P,
        revision: Revision,
        key: ArtifactKey,
        dependencies: Vec<ArtifactDependency>,
        failed: Option<ArtifactKey>,
    ) -> ProviderResult<ArtifactVersion> {
        let version = ArtifactVersion::new(key, dependencies.iter().cloned());

        // a poisoned dependency fails this artifact without running the provider
        if let Some(failed) = failed {
            self.publish_failure(
                revision,
                version,
                dependencies,
                DiagnosticCollection::new(),
                Vec::new(),
                ArtifactFailure::requirement(failed),
            )?;

            return Ok(version);
        }

        // reuse a committed payload when the closure already produced this version
        if self.artifact_store().outcome(&version).is_some() {
            self.bind_artifact(revision, version)
                .map_err(|error| ProviderError::internal(error.to_string()))?;

            return Ok(version);
        }

        // run the provider exactly once over the frozen closure
        let output = provider.provide(revision, key)?;
        match output.product {
            ArtifactProduct::Ready(payload) => {
                self.complete_artifact(
                    revision,
                    version,
                    payload,
                    dependencies,
                    output.diagnostics,
                    output.sidecars,
                )
                .map_err(|error| ProviderError::internal(error.to_string()))?;
            }
            ArtifactProduct::Failed(failure) => {
                self.publish_failure(
                    revision,
                    version,
                    dependencies,
                    output.diagnostics,
                    output.sidecars,
                    failure,
                )?;
            }
        }

        Ok(version)
    }

    /// Resolve one artifact synchronously, building its closure depth first.
    pub fn resolve<P: Provider + ?Sized>(
        &self,
        provider: &P,
        revision: Revision,
        key: ArtifactKey,
    ) -> ProviderResult<ArtifactVersion> {
        // return an already terminal artifact
        if let Some(version) = self.terminal_version(revision, key)? {
            return Ok(version);
        }

        // build dependencies depth first, then commit once the closure converges
        loop {
            match self.collect_closure(provider, revision, key)? {
                Collected::Waiting { frontier, closure } => {
                    for dependency in frontier {
                        self.resolve(provider, revision, dependency)?;
                    }

                    // a complete closure is final once its frontier is terminal, so
                    // freeze it directly rather than collecting the same set again
                    if let Some(closure) = closure {
                        let (dependencies, failed) = self.freeze_closure(revision, &closure)?;

                        return self.commit(provider, revision, key, dependencies, failed);
                    }
                }
                Collected::Frozen {
                    dependencies,
                    failed,
                } => {
                    return self.commit(provider, revision, key, dependencies, failed);
                }
            }
        }
    }

    /// Freeze one fully built closure into its exact dependencies and first failure.
    ///
    /// Every declared artifact is terminal by the time this runs, so it only
    /// reads recorded outcomes; it never builds anything.
    fn freeze_closure(
        &self,
        revision: Revision,
        set: &ArtifactDependencySet,
    ) -> ProviderResult<(Vec<ArtifactDependency>, Option<ArtifactKey>)> {
        // the first poisoned dependency fails the artifact without running it
        let mut failed = None;
        for dependency in &set.artifacts {
            if let Some(ArtifactOutcome::Failed(_)) = self.artifact_outcome(revision, *dependency)? {
                failed = Some(*dependency);

                break;
            }
        }

        Ok((self.resolve_dependencies(revision, set)?, failed))
    }

    /// Return the version of one artifact once it carries a terminal outcome.
    fn terminal_version(
        &self,
        revision: Revision,
        key: ArtifactKey,
    ) -> ProviderResult<Option<ArtifactVersion>> {
        let Some(version) = self.bound_version(revision, key)? else {
            return Ok(None);
        };

        Ok(self.artifact_store().outcome(&version).map(|_| version))
    }

    /// Return the terminal outcome of one artifact bound in this revision.
    fn artifact_outcome(
        &self,
        revision: Revision,
        key: ArtifactKey,
    ) -> ProviderResult<Option<ArtifactOutcome>> {
        let Some(version) = self.bound_version(revision, key)? else {
            return Ok(None);
        };

        Ok(self.artifact_store().outcome(&version))
    }

    /// Return the version bound to one artifact key in this revision.
    fn bound_version(
        &self,
        revision: Revision,
        key: ArtifactKey,
    ) -> ProviderResult<Option<ArtifactVersion>> {
        self.artifact_version(revision, &key)
            .map_err(|error| ProviderError::internal(error.to_string()).into())
    }

    /// Resolve one declared dependency set into exact recorded dependencies.
    fn resolve_dependencies(
        &self,
        revision: Revision,
        set: &ArtifactDependencySet,
    ) -> ProviderResult<Vec<ArtifactDependency>> {
        let mut dependencies = Vec::with_capacity(set.artifacts.len() + set.sources.len());

        // bind each declared artifact to its exact version
        for key in &set.artifacts {
            if let Some(version) = self.bound_version(revision, *key)? {
                dependencies.push(ArtifactDependency::artifact(version));
            }
        }

        // fold in primitive source observations
        for source in &set.sources {
            dependencies.push(ArtifactDependency::Source(source.clone()));
        }

        Ok(dependencies)
    }

    /// Record one failed artifact outcome for this revision.
    #[allow(clippy::too_many_arguments)]
    fn publish_failure(
        &self,
        revision: Revision,
        version: ArtifactVersion,
        dependencies: Vec<ArtifactDependency>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
        failure: ArtifactFailure,
    ) -> ProviderResult<()> {
        self.fail_artifact(revision, version, dependencies, diagnostics, sidecars, failure)
            .map_err(|error| ProviderError::internal(error.to_string()).into())
    }
}
