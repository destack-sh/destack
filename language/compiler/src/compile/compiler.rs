use std::sync::Arc;

use dashmap::DashMap;
use destack_artifact::{ArtifactKey, ArtifactStore, ArtifactVersion, ProfileKey};
use destack_resolver::Resolver;
use destack_source::{
    DiagnosticCollection, DiagnosticSeverity, ModuleId, ProfileId, TargetId, Uri,
};
use destack_workspace::{Change, Edit, Profile, Repository, Revision, Target};
use parking_lot::Mutex;

use super::{CompilerContext, CompilerIndex};
#[cfg(test)]
use crate::TaskHandle;
#[cfg(test)]
use crate::tests::scenario::CompilerScenarioEvent;
use crate::{
    CompileDiagnostic, CompilerEvent, CompilerOptions, CompilerStats, Requirement,
    RequirementCollector, RequirementSet, TaskError, TaskQueue, TaskWarning,
};

/// One pending compiler diagnostic paired with the snapshot that produced it.
#[derive(Debug, Clone)]
struct PendingDiagnostic<T> {
    /// The artifact key that produced the diagnostic when known.
    artifact_key: Option<ArtifactKey>,
    /// The snapshot that produced the diagnostic.
    revision: Revision,
    /// The underlying compiler diagnostic value.
    value: T,
}

/// One pending direct diagnostic collection paired with the artifact that produced it.
#[derive(Debug, Clone)]
struct PendingDiagnosticCollection {
    /// The artifact key that produced the diagnostics when known.
    artifact_key: Option<ArtifactKey>,
    /// The snapshot that produced the diagnostics.
    revision: Revision,
    /// The underlying diagnostic collection.
    diagnostics: DiagnosticCollection,
}

/// Compile files and sources into something (via DIR).
/// #Architecture: should Compiler be per-target? what about comptime though?
#[allow(clippy::type_complexity)]
pub struct Compiler {
    /// The repository being compiled.
    pub repository: Arc<Repository>,
    /// The live artifact store for the repository.
    pub artifacts: Arc<ArtifactStore>,
    /// The options for compiling.
    pub options: CompilerOptions,
    /// Base resolver reused for import resolution option variants.
    pub(crate) base_resolver: Resolver,

    /// Seen errors for deduplication.
    seen_errors: Mutex<Vec<PendingDiagnostic<TaskError>>>,
    /// Seen warnings for deduplication.
    seen_warnings: Mutex<Vec<PendingDiagnostic<TaskWarning>>>,
    /// Pending direct diagnostics for exact artifact attempts.
    pending_direct_diagnostics: Mutex<Vec<PendingDiagnosticCollection>>,
    /// Satisfied exact artifact versions for one logical compile run.
    pub(super) satisfied_artifacts: DashMap<(Revision, ArtifactVersion), ()>,
    /// The shared comptime target configuration.
    pub comptime_target: Target,

    /// The queue of compiler tasks.
    pub(super) queue: TaskQueue,
    /// Compilation statistics.
    pub stats: Arc<CompilerStats>,
    /// Locks for serializing module creation per (URI, loader) pair.
    /// The loader salt distinguishes imports with non-default loaders.
    import_locks: DashMap<(Uri, Option<String>), Arc<Mutex<Option<ModuleId>>>>,
    /// Ephemeral compiler indices derived from artifacts.
    pub(crate) index: CompilerIndex,
}

impl std::fmt::Debug for Compiler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Compiler")
            .field("repository", &"...")
            .field("options", &self.options)
            .field("queue", &self.queue)
            .finish()
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Create a new Compiler.
    pub fn new(repository: Arc<Repository>, options: CompilerOptions) -> Self {
        let comptime_target = Target::comptime("comptime");
        let timings = options.timings;
        let base_resolver =
            Resolver::from_repository(repository.as_ref(), options.import_resolve.clone());
        let artifacts = repository.artifact_store().clone();

        Self {
            repository,
            artifacts,
            options,
            base_resolver,
            seen_errors: Mutex::new(Vec::new()),
            seen_warnings: Mutex::new(Vec::new()),
            pending_direct_diagnostics: Mutex::new(Vec::new()),
            satisfied_artifacts: DashMap::new(),
            comptime_target,
            queue: TaskQueue::new(),
            import_locks: DashMap::new(),
            index: CompilerIndex::default(),
            stats: Arc::new(CompilerStats::new_with_timings(timings)),
        }
    }

    /// Remember one derived semantic profile in the compiler cache.
    pub fn remember_profile(&self, profile: Profile) -> ProfileId {
        let profile_id = profile.id();
        self.index.profiles.entry(profile_id).or_insert(profile);
        profile_id
    }

    /// Return one derived semantic profile by id when present.
    pub(crate) fn profile_maybe(&self, profile_id: ProfileId) -> Option<Profile> {
        self.index
            .profiles
            .get(&profile_id)
            .map(|entry| entry.value().clone())
    }

    /// Return one derived semantic profile by id.
    pub(crate) fn profile(&self, profile_id: ProfileId) -> Profile {
        self.profile_maybe(profile_id)
            .unwrap_or_else(|| panic!("missing compiler profile for {profile_id:?}"))
    }

    /// Remember one canonical profile key in the compiler cache.
    pub fn remember_profile_key(&self, key: ProfileKey) -> ProfileId {
        let profile = Profile::from_key(key);

        self.remember_profile(profile)
    }

    /// Return the display name for one target id.
    pub(crate) fn target_name(&self, target_id: &TargetId) -> String {
        self.repository
            .target_name_by_target_id(*target_id)
            .map(|name| name.to_string())
            .unwrap_or_else(|| target_id.to_string())
    }

    /// Emit a compiler event to the event handler (if configured).
    #[inline]
    pub fn emit_event(&self, event: CompilerEvent) {
        if let Some(handler) = &self.options.event_handler {
            handler(event);
        }
    }

    /// Emit one internal compiler scenario event.
    #[cfg(test)]
    #[inline]
    pub(crate) fn emit_scenario_event(&self, event: CompilerScenarioEvent) {
        if let Some(handler) = &self.options.scenario_event_handler {
            handler(event);
        }
    }

    /// Snapshot all tracked task handles.
    #[cfg(test)]
    pub(crate) fn task_handles(&self) -> Vec<TaskHandle> {
        self.queue.task_handles()
    }

    /// Clone the base resolver with one request specific option set.
    pub(crate) fn resolver_with_options(
        &self,
        options: destack_resolver::ResolveOptions,
    ) -> Resolver {
        self.base_resolver.with_options(options)
    }

    /// Get the import lock for a (URI, loader) pair.
    /// Used to serialize module creation and prevent race conditions when multiple import tasks
    /// resolve to the same file with the same loader.
    ///
    /// The `loader_salt` parameter distinguishes imports with non-default loaders
    /// (e.g., `with { type: "text" }`). Default loaders use `None`.
    pub(crate) fn get_import_lock(
        &self,
        uri: &Uri,
        loader_salt: Option<&str>,
    ) -> Arc<Mutex<Option<ModuleId>>> {
        let key = (uri.clone(), loader_salt.map(String::from));
        self.import_locks
            .entry(key)
            .or_insert_with(|| Arc::new(Mutex::new(None)))
            .clone()
    }

    /// Add an error to the compiler (deduplicated).
    pub fn error<T: Into<TaskError>>(&self, error: T) {
        let context = self.current_context();
        self.error_for_artifact(context.revision(), context.artifact_key(), error);
    }

    /// Add an error to the compiler for one explicit revision.
    pub(crate) fn error_for_revision<T: Into<TaskError>>(&self, revision: Revision, error: T) {
        self.error_for_artifact(revision, self.current_artifact_key(), error);
    }

    /// Add an error to the compiler for one explicit artifact attempt.
    pub(crate) fn error_for_artifact<T: Into<TaskError>>(
        &self,
        revision: Revision,
        artifact_key: Option<ArtifactKey>,
        error: T,
    ) {
        let error: TaskError = error.into();
        if !self.should_emit_error(revision, &error) {
            return;
        }
        let mut seen = self.seen_errors.lock();
        if seen.iter().any(|pending| pending.value == error) {
            return;
        }
        seen.push(PendingDiagnostic {
            artifact_key,
            revision,
            value: error,
        });
    }

    /// Add a warning to the compiler (deduplicated).
    pub fn warning<T: Into<TaskWarning>>(&self, warning: T) {
        let context = self.current_context();
        self.warning_for_artifact(context.revision(), context.artifact_key(), warning);
    }

    /// Add a warning to the compiler for one explicit artifact attempt.
    pub(crate) fn warning_for_artifact<T: Into<TaskWarning>>(
        &self,
        revision: Revision,
        artifact_key: Option<ArtifactKey>,
        warning: T,
    ) {
        let warning: TaskWarning = warning.into();
        if !self.should_emit_warning(revision, &warning) {
            return;
        }
        let mut seen = self.seen_warnings.lock();
        if seen.iter().any(|pending| pending.value == warning) {
            return;
        }
        seen.push(PendingDiagnostic {
            artifact_key,
            revision,
            value: warning,
        });
    }

    /// Record direct diagnostics for one explicit artifact attempt.
    pub(crate) fn diagnostics_for_artifact(
        &self,
        revision: Revision,
        artifact_key: Option<ArtifactKey>,
        diagnostics: DiagnosticCollection,
    ) {
        if diagnostics.is_empty() {
            return;
        }

        self.pending_direct_diagnostics
            .lock()
            .push(PendingDiagnosticCollection {
                artifact_key,
                revision,
                diagnostics,
            });
    }

    /// Collect a result into a RequirementCollector, reporting non-yield errors.
    ///
    /// Returns `Some(value)` on success, `None` on error (yield or hard error).
    /// Yields are collected into the collector, hard errors are reported via `self.error()`.
    pub fn collect<T, E>(
        &self,
        collector: &mut RequirementCollector,
        result: Result<T, E>,
    ) -> Option<T>
    where
        E: TryInto<RequirementSet, Error = E> + Into<TaskError>,
    {
        match &result {
            Ok(_) => {}
            Err(_) => {
                if let Some(error) = collector.try_collect(result) {
                    self.error(error);
                }
                return None;
            }
        }
        result.ok()
    }

    /// Enqueue all artifact keys needed by a requirement set.
    pub fn enqueue_requirements(&self, revision: Revision, requirement: &RequirementSet) {
        requirement.for_each_artifact(|requirement| {
            self.enqueue(revision, requirement.key);
        });
    }

    /// Run one requirement-producing operation to completion.
    pub fn run_to_completion<T, E, F>(
        &self,
        initial_revision: Revision,
        mut action: F,
    ) -> Result<T, E>
    where
        F: FnMut(&Self, &CompilerContext<'_>) -> Result<T, E>,
        E: TryInto<RequirementSet, Error = E>,
    {
        // start one fresh diagnostic session for this logical operation
        self.clear_diagnostics_session();
        let mut revision = initial_revision;

        loop {
            let result = self.with_revision_scope(revision, |context| action(self, context));
            match result {
                Ok(value) => return Ok(value),
                Err(error) => match error.try_into() {
                    // unmet requirements: enqueue and keep building
                    Ok(requirement) => {
                        if requirement.has_file_requirements() {
                            let next_revision =
                                self.materialize_required_files(revision, &requirement);

                            self.queue.supersede_revision(revision);
                            self.clear_diagnostics_session();
                            revision = next_revision;
                            continue;
                        }

                        self.enqueue_requirements(revision, &requirement);

                        let is_blocked_on_files = self.compile_queued(Some(revision));
                        if is_blocked_on_files {
                            continue;
                        }
                    }

                    // hard failure
                    Err(error) => return Err(error),
                },
            }
        }
    }

    /// Clear the local diagnostics session state.
    fn clear_diagnostics_session(&self) {
        self.seen_errors.lock().clear();
        self.seen_warnings.lock().clear();
        self.pending_direct_diagnostics.lock().clear();
        self.satisfied_artifacts.clear();
    }

    /// Apply all required file edits for one fixed revision.
    fn materialize_required_files(
        &self,
        revision: Revision,
        requirement: &RequirementSet,
    ) -> Revision {
        use std::collections::BTreeMap;

        let mut edits_by_path = BTreeMap::new();

        requirement.for_each(|requirement| {
            let Requirement::File(requirement) = requirement else {
                return;
            };

            let logical_path = match &requirement.edit {
                Edit::AddFile { logical_path, .. }
                | Edit::SetFile { logical_path, .. }
                | Edit::RemoveFile { logical_path }
                | Edit::MoveFile {
                    to: logical_path, ..
                } => logical_path.clone(),
            };

            edits_by_path.insert(logical_path, requirement.edit.clone());
        });

        let edits = edits_by_path.into_values().collect::<Vec<_>>();
        let change = Change::from(edits);

        self.repository
            .apply_to_revision(revision, change)
            .unwrap_or_else(|error| panic!("failed to materialize required files: {error}"))
    }

    /// Publish pending diagnostics onto their realized artifact versions.
    pub fn flush_diagnostics(&self) {
        let mut diagnostics_by_version = std::collections::HashMap::new();

        // convert errors to diagnostics
        let errors = {
            let mut seen = self.seen_errors.lock();
            std::mem::take(&mut *seen)
        };
        for pending in errors {
            let artifact_key = pending.artifact_key;
            let revision = pending.revision;
            let error = pending.value;

            // skip yielded dependency failures
            // (this is internal; they are already reported by other user-facing diagnostics)
            if error.is_yield_failed() {
                continue;
            }

            // apply diagnostic directive overrides
            let Some(severity) = self.error_effective_severity(&error) else {
                continue;
            };

            // build the diagnostic for the error
            let mut diagnostic = CompileDiagnostic::Error(error).to_diagnostic(
                revision,
                &self.repository,
                &self.artifacts,
            );
            if severity != DiagnosticSeverity::Error {
                diagnostic.original_severity = Some(DiagnosticSeverity::Error);
                diagnostic.severity = severity;
            }

            let Some(artifact_key) = artifact_key else {
                continue;
            };
            let version = self.repository.artifact_version(revision, &artifact_key);
            diagnostics_by_version
                .entry(version)
                .or_insert_with(DiagnosticCollection::new)
                .insert(diagnostic);
        }

        // convert warnings to diagnostics
        let warnings = {
            let mut seen = self.seen_warnings.lock();
            std::mem::take(&mut *seen)
        };
        for pending in warnings {
            let artifact_key = pending.artifact_key;
            let revision = pending.revision;
            let warning = pending.value;

            // apply warning directive overrides
            // NOTE #Architecture: is Compiler.flush_diagnostics the right place for directive overrides?
            let Some(severity) = self.warning_effective_severity(&warning) else {
                continue;
            };

            // build the diagnostic for the warning
            let mut diagnostic = CompileDiagnostic::Warning(warning).to_diagnostic(
                revision,
                &self.repository,
                &self.artifacts,
            );
            if severity != DiagnosticSeverity::Warning {
                diagnostic.original_severity = Some(DiagnosticSeverity::Warning);
                diagnostic.severity = severity;
            }

            let Some(artifact_key) = artifact_key else {
                continue;
            };
            let version = self.repository.artifact_version(revision, &artifact_key);
            diagnostics_by_version
                .entry(version)
                .or_insert_with(DiagnosticCollection::new)
                .insert(diagnostic);
        }

        let direct_diagnostics = {
            let mut pending = self.pending_direct_diagnostics.lock();
            std::mem::take(&mut *pending)
        };
        for pending in direct_diagnostics {
            let Some(artifact_key) = pending.artifact_key else {
                continue;
            };
            let version = self
                .repository
                .artifact_version(pending.revision, &artifact_key);
            diagnostics_by_version
                .entry(version)
                .or_insert_with(DiagnosticCollection::new)
                .merge_from(&pending.diagnostics);
        }

        for (version, diagnostics) in diagnostics_by_version {
            self.artifacts.publish_diagnostics(version, diagnostics);
        }
    }
}
