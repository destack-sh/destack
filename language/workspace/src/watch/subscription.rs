use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_repository::{Commit, Repository, Revision, RevisionPin};
use futures::StreamExt;
use futures::channel::mpsc::{Receiver, Sender, channel};
use parking_lot::Mutex;

use super::WatchEvent;
use crate::Error;

/// One semantic watch over a workspace.
pub struct Watch {
    /// Watched workspace root.
    root: PathBuf,
    /// Initial revision not yet returned to the caller.
    ready: Option<Revision>,
    /// Pending commits and their result revision pins.
    receiver: Receiver<(Commit, RevisionPin)>,
    /// Input revision retained for the latest delivered commit.
    before: Option<RevisionPin>,
    /// Ready or result revision retained for the latest delivered event.
    after: RevisionPin,
    /// Terminal reason set by the workspace.
    end: Arc<Mutex<Option<WatchEnd>>>,
    /// Mutable state shared with the workspace.
    state: Arc<Mutex<WatchState>>,
    /// Identifier inside the workspace watch state.
    identifier: u64,
}

impl std::fmt::Debug for Watch {
    /// Format the visible watch state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Watch")
            .field("root", &self.root)
            .field("ready", &self.ready)
            .field("end", &self.end)
            .finish()
    }
}

impl Watch {
    /// Register one watch beginning at an exact workspace revision.
    pub(crate) fn new(
        root: PathBuf,
        revision: Revision,
        repository: &Arc<Repository>,
        state: Arc<Mutex<WatchState>>,
    ) -> Result<Self, Error> {
        let after = repository.pin(revision)?;
        let (identifier, receiver, end) = state.lock().subscribe(&root)?;

        Ok(Self {
            root,
            ready: Some(revision),
            receiver,
            before: None,
            after,
            end,
            state,
            identifier,
        })
    }

    /// Receive the next semantic workspace event.
    pub async fn next(&mut self) -> Result<WatchEvent, Error> {
        if let Some(revision) = self.ready.take() {
            return Ok(WatchEvent::Ready { revision });
        }

        let commit = self.receiver.next().await;
        if let Some(end) = self.end.lock().clone() {
            return Err(end.error(self.root.clone()));
        }
        let Some((commit, after)) = commit else {
            return Err(Error::Internal {
                detail: format!(
                    "workspace watch ended without a reason: {}",
                    self.root.display()
                ),
            });
        };

        if self.after.revision() != commit.before {
            return Err(Error::Internal {
                detail: format!(
                    "workspace watch expected revision {} before {}",
                    self.after.revision(),
                    commit.before,
                ),
            });
        }

        let before = std::mem::replace(&mut self.after, after);
        self.before = Some(before);

        Ok(WatchEvent::Commit(commit))
    }
}

impl Drop for Watch {
    /// Remove this watch from its workspace.
    fn drop(&mut self) {
        self.state.lock().remove(self.identifier);
    }
}

/// Semantic watch state for one workspace.
#[derive(Debug)]
pub(crate) struct WatchState {
    /// Active subscriptions keyed by identifier.
    subscriptions: HashMap<u64, Subscription>,
    /// Next subscription identifier.
    next_identifier: u64,
    /// Latest host watch failure blocking new subscriptions.
    failure: Option<String>,
    /// Whether the workspace has closed.
    is_closed: bool,
}

impl Default for WatchState {
    /// Create empty watch state for an open workspace.
    fn default() -> Self {
        Self {
            subscriptions: HashMap::new(),
            next_identifier: 1,
            failure: None,
            is_closed: false,
        }
    }
}

impl WatchState {
    /// Register one bounded subscription.
    fn subscribe(
        &mut self,
        root: &Path,
    ) -> Result<
        (
            u64,
            Receiver<(Commit, RevisionPin)>,
            Arc<Mutex<Option<WatchEnd>>>,
        ),
        Error,
    > {
        // reject subscriptions while the workspace cannot sustain a watch
        if self.is_closed {
            return Err(Error::WatchClosed {
                root: root.to_path_buf(),
            });
        } else if let Some(detail) = &self.failure {
            return Err(Error::WatchFailed {
                root: root.to_path_buf(),
                detail: detail.clone(),
            });
        }

        // use the sender's implicit slot as the single pending commit
        let (sender, receiver) = channel(0);
        let end = Arc::new(Mutex::new(None));
        let identifier = self.next_identifier;
        self.next_identifier += 1;
        let subscription = Subscription {
            sender,
            end: end.clone(),
        };
        self.subscriptions.insert(identifier, subscription);

        Ok((identifier, receiver, end))
    }

    /// Publish one committed workspace transition to every active watch.
    pub(crate) fn publish(
        &mut self,
        commit: &Commit,
        repository: &Arc<Repository>,
    ) -> Result<(), Error> {
        if self.subscriptions.is_empty() {
            return Ok(());
        }

        let after = repository.pin(commit.after)?;
        let commit = (commit.clone(), after);

        // retain subscriptions that accepted this exact commit
        self.subscriptions.retain(|_, subscription| {
            match subscription.sender.try_send(commit.clone()) {
                Ok(()) => true,
                Err(error) if error.is_full() => {
                    *subscription.end.lock() = Some(WatchEnd::Lagged);

                    false
                }
                Err(_) => false,
            }
        });

        Ok(())
    }

    /// Close every active watch because the workspace closed.
    pub(crate) fn close(&mut self) {
        self.is_closed = true;

        // terminate every subscription with the exact workspace lifecycle reason
        for subscription in self.subscriptions.values() {
            *subscription.end.lock() = Some(WatchEnd::Closed);
        }
        self.subscriptions.clear();
    }

    /// Fail every active watch because its host watcher failed.
    pub(crate) fn fail(&mut self, detail: String) {
        self.failure = Some(detail.clone());

        // terminate active subscriptions without exposing host events
        for subscription in self.subscriptions.values() {
            *subscription.end.lock() = Some(WatchEnd::Failed(detail.clone()));
        }
        self.subscriptions.clear();
    }

    /// Accept new watches after one successful host reload.
    pub(crate) fn recover(&mut self) {
        self.failure = None;
    }

    /// Remove one exact subscription.
    fn remove(&mut self, identifier: u64) {
        self.subscriptions.remove(&identifier);
    }
}

/// One semantic commit subscription.
#[derive(Debug)]
struct Subscription {
    /// Pending semantic event sender.
    sender: Sender<(Commit, RevisionPin)>,
    /// Terminal reason shared with the receiving watch.
    end: Arc<Mutex<Option<WatchEnd>>>,
}

/// Terminal reason for one semantic watch.
#[derive(Debug, Clone)]
enum WatchEnd {
    /// The consumer exceeded its bounded commit backlog.
    Lagged,
    /// The workspace closed.
    Closed,
    /// The workspace host watcher failed.
    Failed(String),
}

impl WatchEnd {
    /// Convert this terminal reason into a public workspace failure.
    fn error(self, root: PathBuf) -> Error {
        match self {
            Self::Lagged => Error::WatchLagged { root },
            Self::Closed => Error::WatchClosed { root },
            Self::Failed(detail) => Error::WatchFailed { root, detail },
        }
    }
}
