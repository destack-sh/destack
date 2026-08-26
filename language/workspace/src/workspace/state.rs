use std::collections::HashMap;
use std::sync::Arc;

use destack_artifact::ArtifactKey;
use destack_repository::{Commit, Revision, RevisionPin};
use destack_session::{ArtifactPriority, ArtifactRun, Session};
use parking_lot::MutexGuard;

use crate::{Error, Watch, Workspace};

/// Mutable state of one workspace.
#[derive(Debug)]
pub(crate) struct State {
    /// Current workspace lifecycle.
    pub(crate) lifecycle: Lifecycle,
    /// Current physical workspace revision.
    pub(crate) physical: RevisionPin,
    /// Current retained revision for every branch.
    pub(crate) branches: HashMap<String, RevisionPin>,
}

/// Lifecycle of one workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lifecycle {
    /// The workspace accepts operations.
    Open,
    /// The workspace rejects operations.
    Closed,
}

/// One proactive artifact run pinned to its exact workspace revision.
pub(crate) struct BackgroundRun {
    /// Scheduled proactive artifact work.
    artifacts: ArtifactRun,
    /// Workspace revision receiving completed artifacts.
    revision: super::WorkspacePin,
}

impl BackgroundRun {
    /// Create one proactive artifact run.
    fn new(artifacts: ArtifactRun, revision: super::WorkspacePin) -> Self {
        Self {
            artifacts,
            revision,
        }
    }

    /// Return this run's exact revision.
    pub(crate) fn revision(&self) -> Revision {
        self.revision.revision()
    }

    /// Cancel unfinished work and persist its completed artifacts.
    fn finish(self) {
        drop(self.artifacts);
        self.revision.persist_artifacts();
    }
}

impl State {
    /// Return one retained workspace branch.
    pub(crate) fn branch(&self, name: &str) -> Result<&RevisionPin, Error> {
        self.branches.get(name).ok_or_else(|| Error::MissingBranch {
            name: name.to_string(),
        })
    }
}

impl Workspace {
    /// Lock this workspace for one mutation while it remains open.
    pub(crate) fn lock(&self) -> Result<MutexGuard<'_, State>, Error> {
        let state = self.state.lock();
        if state.lifecycle == Lifecycle::Closed {
            return Err(Error::WorkspaceClosed);
        }

        Ok(state)
    }

    /// Close this workspace.
    pub fn close(&self) {
        // mark the workspace closed and detach its background run
        let background_run = {
            let mut state = self.state.lock();
            if state.lifecycle == Lifecycle::Closed {
                return;
            }
            state.lifecycle = Lifecycle::Closed;

            self.background_run.lock().take()
        };

        // finish background work after releasing workspace state
        if let Some(run) = background_run {
            run.finish();
        }

        // close workspace watches
        self.watch.lock().close();
    }

    /// Return the exact physical workspace revision.
    pub fn revision(&self) -> Result<Revision, Error> {
        let state = self.lock()?;

        Ok(state.physical.revision())
    }

    /// Watch physical workspace state.
    pub fn watch(&self) -> Result<Watch, Error> {
        let state = self.lock()?;
        let revision = state.physical.revision();

        Watch::new(
            self.root.clone(),
            None,
            revision,
            &self.repository,
            self.watch.clone(),
        )
    }

    /// Prime diagnostic and program index artifacts for one revision.
    pub fn prime(&self, revision: Revision) -> Result<(), Error> {
        let session = self.pin(revision)?;
        let repository = session.repository();
        let module_ids = repository.module_ids(revision)?;
        let modules = session.selected_modules(&module_ids)?;
        let mut artifacts = session.diagnostic_artifacts(&modules);
        artifacts.extend(session.program_indexes()?);
        artifacts.sort_unstable();
        artifacts.dedup();

        self.provide_background(revision, &artifacts)
    }

    /// Publish one committed transition to workspace watches.
    pub(crate) fn publish(&self, branch: Option<&str>, commit: &Commit, after: RevisionPin) {
        if commit.before != commit.after {
            self.watch.lock().publish(branch, commit, after);
        }
    }

    /// Provide proactive editor artifacts for one revision.
    pub(crate) fn provide_background(
        &self,
        revision: Revision,
        artifacts: &[ArtifactKey],
    ) -> Result<(), Error> {
        // install the new run while the workspace remains open
        let previous = {
            let _state = self.lock()?;
            let run = if artifacts.is_empty() {
                None
            } else {
                let session = self.pin(revision)?;
                let run = self
                    .session
                    .provide(revision, artifacts, ArtifactPriority::Background);

                Some(BackgroundRun::new(run, session))
            };

            std::mem::replace(&mut *self.background_run.lock(), run)
        };

        // finish obsolete work after releasing workspace state
        if let Some(run) = previous {
            run.finish();
        }

        Ok(())
    }

    /// Fail workspace watches after one host watch failure.
    pub fn fail_watch(&self, detail: String) -> Result<(), Error> {
        let _state = self.lock()?;
        self.watch.lock().fail(detail);

        Ok(())
    }

    /// Resume workspace watches after host observation becomes active.
    pub fn resume_watch(&self) -> Result<(), Error> {
        let _state = self.lock()?;
        self.watch.lock().recover();

        Ok(())
    }

    /// Return the artifact computation session.
    pub fn session(&self) -> Arc<Session> {
        self.session.clone()
    }
}

impl Drop for Workspace {
    /// Close this workspace before releasing its repository and session.
    fn drop(&mut self) {
        self.close();
    }
}
