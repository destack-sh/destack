use std::collections::HashMap;
use std::sync::Arc;

use destack_artifact::ArtifactKey;
use destack_repository::{Commit, Revision, RevisionPin};
use destack_session::{ArtifactPriority, Session};
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

    /// Close this workspace and its dependent state.
    pub fn close(&self) {
        let mut state = self.state.lock();
        if state.lifecycle == Lifecycle::Closed {
            return;
        }
        state.lifecycle = Lifecycle::Closed;

        // cancel background work and terminate semantic subscriptions
        let background_run = self.background_run.lock().take();
        drop(background_run);
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

    /// Publish one committed transition to semantic watches.
    pub(crate) fn publish(&self, branch: Option<&str>, commit: &Commit, after: RevisionPin) {
        if commit.before != commit.after {
            self.watch.lock().publish(branch, commit, after);
        }
    }

    /// Schedule proactive editor artifacts for one revision.
    pub(crate) fn schedule_background(
        &self,
        revision: Revision,
        artifacts: &[ArtifactKey],
    ) -> Result<(), Error> {
        let _state = self.lock()?;
        let run = if artifacts.is_empty() {
            None
        } else {
            Some(
                self.session
                    .provide(revision, artifacts, ArtifactPriority::Background),
            )
        };
        let previous = std::mem::replace(&mut *self.background_run.lock(), run);

        // cancel obsolete work after publishing its replacement
        drop(previous);

        Ok(())
    }

    /// Terminate semantic subscriptions after one host watch failure.
    pub fn fail_watch(&self, detail: String) -> Result<(), Error> {
        let _state = self.lock()?;
        self.watch.lock().fail(detail);

        Ok(())
    }

    /// Resume semantic watches after host observation becomes active.
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
