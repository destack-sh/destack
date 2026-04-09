use std::cell::RefCell;

use destack_artifact::{ArtifactDependency, ArtifactKey, ArtifactPinSet, ArtifactVersion};
use destack_workspace::{
    ArtifactRequirement as ProvideArtifactRequirement, ProvideError, RepositoryError,
    RepositorySnapshot, Requirement as ProvideRequirement, RequirementSet as ProvideRequirementSet,
    Revision, SourceRequirement,
};

#[cfg(test)]
use crate::tests::scenario::CompilerScenarioEvent;
use crate::{CompileError, Compiler, CompilerContext, CompilerObservationHandler, InternalError};

/// The internal compiler state for one active execution context.
#[derive(Debug)]
struct ActiveProvideContext {
    /// The running artifact key for this context.
    artifact_key: Option<ArtifactKey>,
    /// The pinned repository snapshot for this context.
    snapshot: RepositorySnapshot,
    /// The exact live artifact versions retained for this context.
    retained_artifacts: ArtifactPinSet,
    /// The artifact requirements satisfied during this context.
    requirements: Vec<crate::ArtifactRequirement>,
}

/// The current compiler execution scope for this thread.
#[derive(Default)]
struct ProvideScope {
    /// The active execution context for this thread.
    context: Option<ActiveProvideContext>,
    /// The active observation handler for this thread.
    observation_handler: Option<CompilerObservationHandler>,
}

impl ActiveProvideContext {
    /// Create one active execution context from one compiler context.
    fn new(context: &CompilerContext<'_>) -> Result<Self, RepositoryError> {
        Ok(Self {
            artifact_key: context.artifact_key(),
            snapshot: context.snapshot().clone(),
            retained_artifacts: ArtifactPinSet::new(
                context.compiler().repository.artifact_store().clone(),
            ),
            requirements: Vec::new(),
        })
    }
}

thread_local! {
    /// The current compiler execution scope for this thread.
    static CURRENT_PROVIDE_SCOPE: RefCell<ProvideScope> = const { RefCell::new(ProvideScope {
        context: None,
        observation_handler: None,
    }) };
}

impl Compiler {
    /// Return the active compiler context for this execution scope when it exists.
    pub(crate) fn current_context_maybe(&self) -> Option<CompilerContext<'_>> {
        CURRENT_PROVIDE_SCOPE.with(|current| {
            current.borrow().context.as_ref().map(|context| {
                CompilerContext::new(self, context.snapshot.clone(), context.artifact_key)
            })
        })
    }

    /// Return the active compiler context for this execution scope.
    #[track_caller]
    pub(crate) fn current_context(&self) -> CompilerContext<'_> {
        let caller = std::panic::Location::caller();

        self.current_context_maybe().unwrap_or_else(|| {
            panic!(
                "missing compiler execution context at {}:{}:{}",
                caller.file(),
                caller.line(),
                caller.column()
            )
        })
    }

    /// Return one compiler context pinned to one explicit revision.
    pub fn context(&self, revision: Revision) -> Result<CompilerContext<'_>, RepositoryError> {
        let snapshot = self.repository.snapshot(revision)?;

        Ok(CompilerContext::new(self, snapshot, None))
    }

    /// Return one compiler context pinned to one explicit artifact attempt.
    pub fn artifact_context(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
    ) -> Result<CompilerContext<'_>, RepositoryError> {
        let snapshot = self.repository.snapshot(revision)?;

        Ok(CompilerContext::new(self, snapshot, Some(artifact_key)))
    }

    /// Return the artifact key currently executing on this thread.
    pub(crate) fn current_artifact_key(&self) -> Option<ArtifactKey> {
        CURRENT_PROVIDE_SCOPE.with(|current| {
            current
                .borrow()
                .context
                .as_ref()
                .and_then(|context| context.artifact_key)
        })
    }

    /// Run one action with one temporary compiler observation handler.
    pub fn with_observation_handler<T>(
        &self,
        handler: Option<CompilerObservationHandler>,
        action: impl FnOnce() -> T,
    ) -> T {
        let previous = CURRENT_PROVIDE_SCOPE.with(|current| {
            let mut current = current.borrow_mut();
            std::mem::replace(&mut current.observation_handler, handler)
        });
        let _guard = ObservationHandlerGuard { previous };

        action()
    }

    /// Return the active compiler observation handler for this thread.
    pub(crate) fn current_observation_handler(&self) -> Option<CompilerObservationHandler> {
        CURRENT_PROVIDE_SCOPE.with(|current| current.borrow().observation_handler.clone())
    }

    /// Return the revision currently active for this execution scope.
    #[cfg(test)]
    pub(crate) fn current_execution_revision(&self) -> Option<Revision> {
        self.current_context_maybe()
            .map(|context| context.revision())
    }

    /// Run one action inside one explicit compiler context.
    pub fn with_context<T>(
        &self,
        compiler_context: CompilerContext<'_>,
        action: impl FnOnce(&CompilerContext<'_>) -> T,
    ) -> T {
        let context = ActiveProvideContext::new(&compiler_context)
            .unwrap_or_else(|error| panic!("failed to activate compiler context: {error}"));

        CURRENT_PROVIDE_SCOPE.with(|current| {
            let mut current = current.borrow_mut();

            assert!(
                current.context.is_none(),
                "nested compiler execution scopes are not allowed",
            );
            assert!(
                context.retained_artifacts.is_empty(),
                "compiler retained artifact set must be empty before entering a provide scope",
            );
            assert!(
                context.requirements.is_empty(),
                "compiler requirement log must be empty before entering a provide scope",
            );

            current.context = Some(context);
        });

        let result = action(&compiler_context);

        CURRENT_PROVIDE_SCOPE.with(|current| {
            let context = current
                .borrow_mut()
                .context
                .take()
                .expect("compiler execution scope should be active on exit");

            assert!(
                context.requirements.is_empty(),
                "compiler requirement log must be empty after leaving a provide scope",
            );

            drop(context);
        });

        result
    }

    /// Record one satisfied artifact requirement for the current provide attempt.
    pub(crate) fn record_current_requirement(&self, requirement: crate::ArtifactRequirement) {
        if !self.should_record_current_requirement(&requirement) {
            return;
        }

        CURRENT_PROVIDE_SCOPE.with(|current| {
            let mut current = current.borrow_mut();
            let context = current
                .context
                .as_mut()
                .expect("compiler requirement log requires one active provide context");

            if context.requirements.iter().any(|existing| {
                existing.key == requirement.key && existing.stamp == requirement.stamp
            }) {
                return;
            }

            context.requirements.push(requirement);
        });
    }

    /// Return whether one exact requirement was already satisfied in the current provide attempt.
    pub(crate) fn current_requirement_is_recorded(
        &self,
        requirement: &crate::ArtifactRequirement,
    ) -> bool {
        CURRENT_PROVIDE_SCOPE.with(|current| {
            current.borrow().context.as_ref().is_some_and(|context| {
                context.requirements.iter().any(|existing| {
                    existing.key == requirement.key && existing.stamp == requirement.stamp
                })
            })
        })
    }

    /// Return whether one satisfied requirement should persist past provide completion.
    fn should_record_current_requirement(&self, requirement: &crate::ArtifactRequirement) -> bool {
        let _ = requirement;

        CURRENT_PROVIDE_SCOPE.with(|current| {
            current
                .borrow()
                .context
                .as_ref()
                .is_some_and(|context| context.artifact_key.is_some())
        })
    }

    /// Clear the current provide requirement log.
    fn clear_current_requirements(&self) {
        CURRENT_PROVIDE_SCOPE.with(|current| {
            if let Some(context) = current.borrow_mut().context.as_mut() {
                context.requirements.clear();
            }
        });
    }

    /// Take the current provide requirement log.
    fn take_current_requirements(&self) -> Vec<crate::ArtifactRequirement> {
        CURRENT_PROVIDE_SCOPE.with(|current| {
            let mut current = current.borrow_mut();
            let Some(context) = current.context.as_mut() else {
                return Vec::new();
            };

            std::mem::take(&mut context.requirements)
        })
    }

    /// Clone the current provide requirement log.
    pub(crate) fn current_requirements(&self) -> Vec<crate::ArtifactRequirement> {
        CURRENT_PROVIDE_SCOPE.with(|current| {
            current
                .borrow()
                .context
                .as_ref()
                .map(|context| context.requirements.clone())
                .unwrap_or_default()
        })
    }

    /// Retain one exact live artifact version for the current execution scope.
    pub(crate) fn retain_current_artifact_version(&self, version: &ArtifactVersion) {
        CURRENT_PROVIDE_SCOPE.with(|current| {
            let mut current = current.borrow_mut();
            let Some(context) = current.context.as_mut() else {
                return;
            };

            context.retained_artifacts.pin(*version);
        });
    }

    /// Provide one compiler owned artifact key for one revision.
    pub fn provide(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
    ) -> Result<(), ProvideError<CompileError>> {
        let context = self
            .artifact_context(revision, artifact_key)
            .map_err(|error| {
                ProvideError::Failed(CompileError::Internal(InternalError::InvalidRevision {
                    revision,
                    message: error.to_string(),
                }))
            })?;
        let result = self.with_context(context, |context| {
            self.clear_current_requirements();
            self.execute_artifact(context, artifact_key)
        });

        self.finish_provide_result(revision, artifact_key, result)
    }

    /// Execute one compiler owned artifact key.
    fn execute_artifact(
        &self,
        context: &CompilerContext<'_>,
        artifact_key: ArtifactKey,
    ) -> Result<(), CompileError> {
        match artifact_key {
            ArtifactKey::ModuleGraph { profile } => self
                .process_module_graph(profile, context)
                .map_err(Into::into),
            ArtifactKey::Ast { module } => self.process_ast(*module, &context).into(),
            ArtifactKey::Data { module } => self.process_ast(*module, &context).into(),
            ArtifactKey::DirBase { module } => self.process_dir_base(*module, &context).into(),
            ArtifactKey::DirPrepared { module, profile } => self
                .process_dir_prepared(module, profile, context)
                .map_err(Into::into),
            ArtifactKey::LanguageEnvironment { profile } => self
                .process_language_environment(profile, context)
                .map_err(Into::into),
            ArtifactKey::IntrinsicEnvironment { profile } => self
                .process_intrinsic_environment(profile, context)
                .map_err(Into::into),
            ArtifactKey::LibraryEnvironment { profile } => self
                .process_library_environment(profile, context)
                .map_err(Into::into),
            ArtifactKey::DirResolved { module, profile } => self
                .process_dir_resolved(module, profile, context)
                .map_err(Into::into),
            ArtifactKey::DirDeclared { module, profile } => self
                .process_dir_declared(module, profile, context)
                .map_err(Into::into),
            ArtifactKey::DirInterface { module, profile } => self
                .process_dir_interface(module, profile, context)
                .map_err(Into::into),
            ArtifactKey::DirAnalyzed { module, profile } => self
                .process_dir_analyzed(module, profile, context)
                .map_err(Into::into),
            ArtifactKey::DirElaborated { module, profile } => self
                .process_dir_elaborated(module, profile, context)
                .map_err(Into::into),
            ArtifactKey::DirPatched { module, profile } => self
                .process_dir_patched(module, profile, context)
                .map_err(Into::into),
            ArtifactKey::MirBase {
                module,
                profile,
                target,
            } => self
                .process_mir(module, profile, target, context)
                .map_err(Into::into),
            ArtifactKey::MirOptimized {
                module,
                profile,
                target,
            } => self
                .process_mir_optimized(module, profile, target, context)
                .map_err(Into::into),
            ArtifactKey::ModuleOutput { module, target } => {
                let profile = context
                    .profile_id_for_target(module, &target)
                    .unwrap_or_else(|| {
                        panic!("missing profile for module {module:?} target {target:?}")
                    });

                self.process_module_output(module, profile, target, context)
                    .map_err(Into::into)
            }
            ArtifactKey::PackageOutput { package, target } => self
                .process_package_output(package, target, context)
                .map_err(Into::into),
            ArtifactKey::ModuleLinted { .. }
            | ArtifactKey::PackageLinted { .. }
            | ArtifactKey::WorkspaceLinted => {
                panic!("non compiler artifact key reached compiler provider: {artifact_key:?}")
            }
        }
    }

    /// Finish one direct provide attempt.
    fn finish_provide_result(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
        result: Result<(), CompileError>,
    ) -> Result<(), ProvideError<CompileError>> {
        // successful provides commit their diagnostics and exact dependency log
        if result.is_ok() {
            #[cfg(test)]
            self.emit_scenario_event(CompilerScenarioEvent::BeforeTaskCommit { artifact_key });

            let _requirements = self.take_current_requirements();
            self.flush_diagnostics();

            return Ok(());
        }

        let error = result.unwrap_err();

        // yielded requirements belong to the outer driver, not the provider
        if let Some(requirements) = error.yielded_to() {
            let requirements = strip_compiler_requirements(requirements);
            self.clear_current_requirements();

            return Err(ProvideError::Requirements(requirements));
        }

        // hard failures publish an exact failed artifact version immediately
        let mut dependencies = self.live_dependencies_for_current_artifact(&artifact_key);
        if let Some(requirement) = error.blocking_requirement() {
            dependencies
                .extend(self.live_dependencies_for_requirement_set(&artifact_key, requirement));
        }

        self.clear_current_requirements();
        self.publish_failed_artifact_version(revision, artifact_key, dependencies);
        self.error_for_artifact(revision, Some(artifact_key), error.clone());
        self.flush_diagnostics();

        Err(ProvideError::Failed(error))
    }
    /// Publish one failed exact artifact version with its blocking dependencies.
    fn publish_failed_artifact_version(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
        dependencies: Vec<ArtifactDependency>,
    ) {
        let version = self.repository.artifact_version(revision, &artifact_key);

        self.artifacts.publish_failure(version, dependencies);
    }
}

/// Convert one compiler requirement set into one provider requirement set.
fn strip_compiler_requirements(requirements: &crate::RequirementSet) -> ProvideRequirementSet {
    let mut converted_requirements = Vec::new();

    requirements.for_each(|requirement| match requirement {
        crate::Requirement::Artifact(requirement) => {
            converted_requirements.push(ProvideRequirement::Artifact(ProvideArtifactRequirement {
                key: requirement.key,
                stamp: requirement.stamp,
            }));
        }
        crate::Requirement::File(requirement) => {
            converted_requirements.push(ProvideRequirement::Source(SourceRequirement {
                change: requirement.change.clone(),
            }));
        }
    });

    match converted_requirements.len() {
        0 => ProvideRequirementSet::All(Vec::new()),
        1 => {
            let requirement = converted_requirements
                .into_iter()
                .next()
                .expect("single converted requirement should exist");
            ProvideRequirementSet::One(requirement)
        }
        _ => ProvideRequirementSet::All(converted_requirements),
    }
}

/// Guard that restores the previous observation handler on drop.
struct ObservationHandlerGuard {
    /// The previous handler for this thread.
    previous: Option<CompilerObservationHandler>,
}

impl Drop for ObservationHandlerGuard {
    fn drop(&mut self) {
        let previous = self.previous.take();

        CURRENT_PROVIDE_SCOPE.with(|current| {
            current.borrow_mut().observation_handler = previous;
        });
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    use destack_workspace::{Change, Ref, Repository};

    use crate::{Compiler, CompilerOptions};

    /// Keep anonymous revisions alive while one compiler revision scope is active.
    #[test]
    fn test_with_context_roots_anonymous_revision() {
        let root = unique_test_root("compiler-revision-scope");
        fs::create_dir_all(&root).expect("compiler revision scope test root should exist");

        let repository = Arc::new(Repository::open_root(root.clone()));
        let compiler = Compiler::new(Arc::clone(&repository), CompilerOptions::default());
        let reference = Ref::for_workspace_root(&root);
        let base_revision = repository
            .current(&reference)
            .expect("workspace root ref should exist");
        let anonymous_revision = repository
            .apply_to_revision(
                base_revision,
                Change::add_text("src/example.ts", "export const value = 1"),
            )
            .expect("anonymous revision should publish");

        let context = compiler
            .context(anonymous_revision)
            .expect("anonymous revision should pin one compiler context");
        compiler.with_context(context, |context| {
            let file_id = context
                .compiler()
                .repository
                .file_id_for_workspace_path(std::path::Path::new("src/example.ts"));
            let file = context
                .compiler()
                .repository
                .file(context.revision(), file_id)
                .expect("anonymous revision should stay rooted inside the scope")
                .expect("anonymous revision file should exist");

            assert_eq!(file.text(), "export const value = 1");
        });
    }

    /// Build a unique temporary root path for one compiler test.
    fn unique_test_root(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();

        std::env::temp_dir().join(format!("destack-{label}-{nanos}"))
    }
}
