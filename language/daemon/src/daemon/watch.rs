use std::fmt;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use crossbeam_channel::{Sender, unbounded};
use tspp_source::{FileWatch, FileWatchError, FileWatchEvent};
use tspp_workspace::Workspace;

use super::{DaemonError, WorkspaceWatchError};

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
        let root = workspace.root().to_path_buf();
        let mut watch = FileWatch::new().map_err(|source| WorkspaceWatchError::FileWatch {
            root: root.clone(),
            source,
        })?;
        watch
            .watch(&root)
            .map_err(|source| WorkspaceWatchError::FileWatch {
                root: root.clone(),
                source,
            })?;

        // close the scan race after physical observation becomes active
        workspace
            .reload()
            .map_err(|source| WorkspaceWatchError::Reload {
                root,
                source: Box::new(source),
            })?;
        workspace.resume_watch()?;

        // serialize physical events and repository reconciliation
        let (stop, stopped) = unbounded();
        let thread = thread::spawn(move || -> Result<(), DaemonError> {
            loop {
                crossbeam_channel::select! {
                    recv(stopped) -> _ => return Ok(()),
                    recv(watch.changes) -> changed => {
                        let Ok(()) = changed else {
                            return Self::disconnect(&workspace);
                        };
                        Self::reconcile(&workspace, watch.take())?;
                    }
                    recv(watch.errors) -> error => {
                        let Ok(error) = error else {
                            return Self::disconnect(&workspace);
                        };
                        Self::recover(&workspace, error)?;

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
            workspace
                .reload()
                .map_err(|source| WorkspaceWatchError::Reload {
                    root: workspace.root().to_path_buf(),
                    source: Box::new(source),
                })?;

            return Ok(());
        }

        // publish the minimal semantic transition for exact changed paths
        if let Err(reconcile_error) = workspace.reconcile(changes.paths)
            && let Err(reload_error) = workspace.reload()
        {
            let error = WorkspaceWatchError::Reconcile {
                root: workspace.root().to_path_buf(),
                source: Box::new(reconcile_error),
                reload: Box::new(reload_error),
            };

            return Self::fail(workspace, error);
        }

        Ok(())
    }

    /// Restore authoritative workspace state after one host watcher failure.
    fn recover(workspace: &Workspace, source: FileWatchError) -> Result<(), DaemonError> {
        let root = workspace.root().to_path_buf();
        let message = format!("host file watch failed for {}: {source}", root.display());
        workspace.fail_watch(message)?;

        // reload complete physical state before accepting more events
        if let Err(reload) = workspace.reload() {
            return Err(WorkspaceWatchError::FileWatchReload {
                root,
                source,
                reload: Box::new(reload),
            }
            .into());
        }

        // reopen semantic observation after successful recovery
        workspace.resume_watch()?;

        Ok(())
    }

    /// Terminate semantic watches after the host watcher disconnects.
    fn disconnect(workspace: &Workspace) -> Result<(), DaemonError> {
        let error = WorkspaceWatchError::Disconnected {
            root: workspace.root().to_path_buf(),
        };

        Self::fail(workspace, error)
    }

    /// Terminate semantic watches after one physical observation failure.
    fn fail(workspace: &Workspace, error: WorkspaceWatchError) -> Result<(), DaemonError> {
        workspace.fail_watch(error.to_string())?;

        Err(error.into())
    }
}

impl Drop for WorkspaceWatch {
    /// Stop and join the physical watch worker.
    fn drop(&mut self) {
        if let Err(error) = self.stop() {
            eprintln!("TS++ workspace watch failed: {error}");
        }
    }
}
