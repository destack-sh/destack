use std::fmt;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use crossbeam_channel::{Sender, unbounded};
use destack_source::{FileWatch, FileWatchEvent};
use destack_workspace::{Error, Workspace};

use super::DaemonError;

/// Physical file observation for one daemon workspace.
pub(crate) struct WorkspaceWatch {
    /// Worker stop sender.
    stop: Option<Sender<()>>,
    /// Physical observation worker.
    thread: Option<JoinHandle<Result<(), DaemonError>>>,
}

impl fmt::Debug for WorkspaceWatch {
    /// Format the visible physical watch state.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WorkspaceWatch")
            .field(
                "is_finished",
                &self.thread.as_ref().is_none_or(JoinHandle::is_finished),
            )
            .finish()
    }
}

impl WorkspaceWatch {
    /// Start physical observation for one workspace.
    pub(crate) fn start(workspace: Arc<Workspace>) -> Result<Self, DaemonError> {
        let root = workspace.root();
        let mut watch = FileWatch::new()?;
        watch.watch(root)?;

        // close the scan race after physical observation becomes active
        workspace.reload()?;
        workspace.resume_watch()?;

        // serialize physical events and repository reconciliation
        let (stop, stopped) = unbounded();
        let thread = thread::spawn(move || -> Result<(), DaemonError> {
            loop {
                crossbeam_channel::select! {
                    recv(stopped) -> _ => return Ok(()),
                    recv(watch.changes) -> changed => {
                        let Ok(()) = changed else {
                            return Self::fail(
                                &workspace,
                                "host file watch stopped unexpectedly".into(),
                            );
                        };
                        Self::reconcile(&workspace, watch.take())?;
                    }
                    recv(watch.errors) -> error => {
                        let Ok(error) = error else {
                            return Self::fail(
                                &workspace,
                                "host file watch stopped unexpectedly".into(),
                            );
                        };
                        let detail = error.to_string();
                        workspace.fail_watch(detail)?;
                        workspace.reload()?;
                        workspace.resume_watch()?;

                        continue;
                    }
                }
            }
        });

        Ok(Self {
            stop: Some(stop),
            thread: Some(thread),
        })
    }

    /// Stop and join this physical watch.
    pub(crate) fn stop(&mut self) -> Result<(), DaemonError> {
        let Some(thread) = self.thread.take() else {
            return Ok(());
        };

        // disconnect the stop receiver to request normal worker termination
        drop(self.stop.take());

        thread.join().map_err(|_| DaemonError::Thread)??;

        Ok(())
    }

    /// Reconcile one physical event batch.
    fn reconcile(workspace: &Workspace, changes: FileWatchEvent) -> Result<(), DaemonError> {
        // restore complete source truth when exact physical changes were lost
        if changes.is_rescan {
            workspace.reload()?;

            return Ok(());
        }

        // publish the minimal semantic transition for exact changed paths
        if let Err(reconcile_error) = workspace.reconcile(changes.paths)
            && let Err(reload_error) = workspace.reload()
        {
            let detail = format!(
                "workspace reconciliation failed: {reconcile_error}; \
                 workspace reload failed: {reload_error}"
            );
            return Self::fail(workspace, detail);
        }

        Ok(())
    }

    /// Terminate semantic watches after one physical observation failure.
    fn fail(workspace: &Workspace, detail: String) -> Result<(), DaemonError> {
        workspace.fail_watch(detail.clone())?;

        Err(Error::WatchFailed {
            root: workspace.root().to_path_buf(),
            detail,
        }
        .into())
    }
}

impl Drop for WorkspaceWatch {
    /// Stop and join the physical watch worker.
    fn drop(&mut self) {
        if let Err(error) = self.stop() {
            eprintln!("Destack workspace watch failed: {error}");
        }
    }
}
