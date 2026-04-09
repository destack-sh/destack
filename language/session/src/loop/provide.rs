use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

use destack_artifact::{ArtifactKey, ArtifactProvider};
use destack_compiler::{CompilerObservation, CompilerObservationHandler};
use destack_workspace::{ProvideError, Revision};

use crate::{Session, SessionError, SessionEvent, SessionObservation, SessionStats};

/// Pending artifact queue for one provide run.
#[derive(Debug, Default)]
struct PendingArtifacts {
    /// Pending artifact keys in stack order.
    artifact_keys: Vec<ArtifactKey>,
    /// Pending artifact membership for deduplication.
    artifact_key_set: HashSet<ArtifactKey>,
}

impl PendingArtifacts {
    /// Push one artifact key when it is not already queued.
    fn push(&mut self, artifact_key: ArtifactKey) {
        if self.artifact_key_set.insert(artifact_key) {
            self.artifact_keys.push(artifact_key);
        }
    }

    /// Push many artifact keys in reverse order.
    fn extend_reversed(&mut self, artifact_keys: impl IntoIterator<Item = ArtifactKey>) {
        for artifact_key in artifact_keys {
            self.push(artifact_key);
        }
    }

    /// Pop one pending artifact key.
    fn pop(&mut self) -> Option<ArtifactKey> {
        let artifact_key = self.artifact_keys.pop()?;
        self.artifact_key_set.remove(&artifact_key);

        Some(artifact_key)
    }
}

impl Session {
    /// Provide one root artifact slice until every key is ready.
    pub fn provide(&self, artifact_keys: &[ArtifactKey]) -> Result<SessionStats, SessionError> {
        let _mutation_guard = self.enter_mutation();
        let revision = self.freeze_revision()?;

        self.emit_event(SessionEvent::RunStarted);

        let mut run_stats = SessionStats::default();
        let result = self.provide_revision(revision, artifact_keys, &mut run_stats);
        let result = match result {
            Ok(revision) => self.publish_revision(revision).map(|_revision| run_stats),
            Err(error) => Err(error),
        };

        self.emit_event(SessionEvent::RunFinished { stats: run_stats });

        result
    }

    /// Provide one root artifact slice until every key is ready.
    fn provide_revision(
        &self,
        mut revision: Revision,
        artifact_keys: &[ArtifactKey],
        run_stats: &mut SessionStats,
    ) -> Result<Revision, SessionError> {
        let repository = self.repository();
        let mut pending_artifacts = PendingArtifacts::default();

        // root artifact queue
        pending_artifacts.extend_reversed(artifact_keys.iter().rev().copied());

        // each outer artifact run owns compiler diagnostic/session state
        self.compiler().clear_artifact_run();

        // drive one root stack until nothing remains pending
        while let Some(artifact_key) = pending_artifacts.pop() {
            let provide_id = self.next_provide_id();
            let started_at = Instant::now();
            run_stats.started += 1;

            self.emit_event(SessionEvent::ArtifactStarted {
                provide_id,
                artifact_key,
            });

            let was_completed = match artifact_key.provider() {
                // compiler owned keys can yield both source and artifact requirements
                ArtifactProvider::Compiler => {
                    match self.compiler().provide_with_observation_handler(
                        revision,
                        artifact_key,
                        self.compiler_observation_handler(provide_id, artifact_key),
                    ) {
                        Ok(()) => true,
                        Err(error) => self.handle_provide_error(
                            repository.as_ref(),
                            &mut revision,
                            &mut pending_artifacts,
                            run_stats,
                            provide_id,
                            artifact_key,
                            started_at,
                            error,
                        )?,
                    }
                }

                ArtifactProvider::Linter => match self.linter().provide(revision, artifact_key) {
                    Ok(()) => true,
                    Err(error) => self.handle_provide_error(
                        repository.as_ref(),
                        &mut revision,
                        &mut pending_artifacts,
                        run_stats,
                        provide_id,
                        artifact_key,
                        started_at,
                        error,
                    )?,
                },
            };

            if !was_completed {
                continue;
            }

            let elapsed = started_at.elapsed();
            self.emit_slow_artifact(run_stats, provide_id, artifact_key, elapsed);
            run_stats.completed += 1;
            self.emit_event(SessionEvent::ArtifactCompleted {
                provide_id,
                artifact_key,
                elapsed,
            });
        }

        Ok(revision)
    }

    /// Handle one yielded or failed provider result.
    fn handle_provide_error<E: std::fmt::Debug>(
        &self,
        repository: &destack_workspace::Repository,
        revision: &mut Revision,
        pending_artifacts: &mut PendingArtifacts,
        run_stats: &mut SessionStats,
        provide_id: crate::ProvideId,
        artifact_key: ArtifactKey,
        started_at: Instant,
        error: ProvideError<E>,
    ) -> Result<bool, SessionError> {
        let elapsed = started_at.elapsed();
        self.emit_slow_artifact(run_stats, provide_id, artifact_key, elapsed);

        match error {
            ProvideError::Requirements(requirements) => {
                if requirements.has_source_requirements() {
                    *revision =
                        self.apply_file_requirements(repository, *revision, &requirements)?;

                    self.compiler().clear_artifact_run();
                }

                // retry the root after any yielded changes
                pending_artifacts.push(artifact_key);

                // exact artifact requirements do not survive a revision change
                if !requirements.has_source_requirements() {
                    let mut required_artifact_keys = Vec::new();
                    requirements.for_each_artifact(|requirement| {
                        required_artifact_keys.push(requirement.version.key);
                    });

                    pending_artifacts.extend_reversed(required_artifact_keys.into_iter().rev());
                }

                self.emit_event(SessionEvent::ArtifactYielded {
                    provide_id,
                    artifact_key,
                });
                run_stats.yielded += 1;

                Ok(false)
            }
            ProvideError::Failed(error) => {
                self.emit_event(SessionEvent::ArtifactFailed {
                    provide_id,
                    artifact_key,
                });
                run_stats.failed += 1;

                Err(SessionError::Internal {
                    detail: format!("failed to provide artifact {artifact_key:?}: {error:?}"),
                })
            }
        }
    }

    /// Build one session observation bridge for one compiler provide attempt.
    fn compiler_observation_handler(
        &self,
        provide_id: crate::ProvideId,
        artifact_key: ArtifactKey,
    ) -> Option<CompilerObservationHandler> {
        let handler = self.observation_handler()?;

        Some(Arc::new(move |observation| match observation {
            CompilerObservation::TimingTag {
                name,
                duration,
                sample_count,
            } => handler(SessionObservation::CompilerTimingTag {
                provide_id,
                artifact_key,
                name,
                duration,
                sample_count,
            }),
            CompilerObservation::ParserTimingTag {
                name,
                duration,
                self_duration,
                sample_count,
            } => handler(SessionObservation::ParserTimingTag {
                provide_id,
                artifact_key,
                name,
                duration,
                self_duration,
                sample_count,
            }),
        }))
    }

    /// Emit one slow artifact event when the attempt crosses the threshold.
    fn emit_slow_artifact(
        &self,
        run_stats: &mut SessionStats,
        provide_id: crate::ProvideId,
        artifact_key: ArtifactKey,
        elapsed: Duration,
    ) {
        let Some(threshold) = self.slow_artifact_threshold() else {
            return;
        };

        if elapsed < threshold {
            return;
        }

        run_stats.slow += 1;
        self.emit_event(SessionEvent::ArtifactSlow {
            provide_id,
            artifact_key,
            elapsed,
        });
    }
}
