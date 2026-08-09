use std::sync::Arc;

use destack_artifact::ArtifactKey;
use destack_repository::{Commit, Revision};
use destack_session::{ArtifactPriority, Session};
use parking_lot::MutexGuard;

use crate::{Error, Watch, Workspace};

/// Lifecycle of one workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    /// The workspace accepts operations.
    Open,
    /// The workspace rejects operations.
    Closed,
}

impl Workspace {
    /// Lock this workspace for one mutation while it remains open.
    pub(crate) fn write(&self) -> Result<MutexGuard<'_, State>, Error> {
        let state = self.state.lock();
        if *state == State::Closed {
            return Err(Error::WorkspaceClosed);
        }

        Ok(state)
    }

    /// Close this workspace and its dependent state.
    pub fn close(&self) {
        let mut state = self.state.lock();
        if *state == State::Closed {
            return;
        }
        *state = State::Closed;

        // remove editor state before terminating workspace operations
        self.remove_open_files();

        // cancel background work and terminate semantic subscriptions
        let background_run = self.background_run.lock().take();
        drop(background_run);
        self.watch.lock().close();
    }

    /// Return the revision currently published by this workspace.
    pub fn revision(&self) -> Result<Revision, Error> {
        self.repository.current(&self.head).map_err(Error::from)
    }

    /// Open one semantic watch at the current workspace revision.
    pub fn watch(&self) -> Result<Watch, Error> {
        let _write = self.write()?;
        let revision = self.revision()?;

        Watch::new(
            self.root.clone(),
            revision,
            &self.repository,
            self.watch.clone(),
        )
    }

    /// Publish one committed transition to semantic watches.
    pub(crate) fn publish(&self, commit: Commit) -> Result<Commit, Error> {
        if commit.before != commit.after {
            self.watch.lock().publish(&commit, &self.repository)?;
        }

        Ok(commit)
    }

    /// Schedule proactive editor artifacts for one revision.
    pub(crate) fn schedule_background(
        &self,
        revision: Revision,
        artifacts: &[ArtifactKey],
    ) -> Result<(), Error> {
        let _write = self.write()?;
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
        let _write = self.write()?;
        self.watch.lock().fail(detail);

        Ok(())
    }

    /// Resume semantic watches after host observation becomes active.
    pub fn resume_watch(&self) -> Result<(), Error> {
        let _write = self.write()?;
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
