use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_repository::Revision;
use futures::StreamExt;
use futures::channel::mpsc::{Receiver, Sender, channel};
use parking_lot::Mutex;

use super::WatchEvent;
use crate::{Commit, Error};

/// Maximum pending semantic changes retained by one workspace watch.
const WATCH_CAPACITY: usize = 64;

/// One semantic watch over an opened workspace root.
pub struct Watch {
    /// Watched workspace root.
    root: PathBuf,
    /// Initial revision not yet returned to the caller.
    ready: Option<Revision>,
    /// Pending committed changes.
    receiver: Receiver<WatchEvent>,
    /// Terminal reason set by the owning root.
    end: Arc<Mutex<Option<WatchEnd>>>,
    /// Mutable state shared with the owning root.
    state: Arc<Mutex<WatchState>>,
    /// Identifier inside the root watch state.
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
    /// Register one watch beginning at an exact root revision.
    pub(crate) fn new(
        root: PathBuf,
        revision: Revision,
        state: Arc<Mutex<WatchState>>,
    ) -> Result<Self, Error> {
        let (identifier, receiver, end) = state.lock().subscribe(&root)?;

        Ok(Self {
            root,
            ready: Some(revision),
            receiver,
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

        let event = self.receiver.next().await;
        if let Some(end) = self.end.lock().clone() {
            return Err(end.error(self.root.clone()));
        }
        let Some(event) = event else {
            return Err(Error::Internal {
                detail: format!(
                    "workspace watch ended without a reason: {}",
                    self.root.display()
                ),
            });
        };

        Ok(event)
    }
}

impl Drop for Watch {
    /// Remove this watch from its root.
    fn drop(&mut self) {
        self.state.lock().remove(self.identifier);
    }
}

/// Semantic watch state for one opened workspace root.
#[derive(Debug)]
pub(crate) struct WatchState {
    /// Active subscriptions keyed by identifier.
    subscriptions: HashMap<u64, Subscription>,
    /// Next subscription identifier.
    next_identifier: u64,
    /// Latest host watch failure blocking new subscriptions.
    failure: Option<String>,
    /// Whether the owning root has closed.
    is_closed: bool,
}

impl Default for WatchState {
    /// Create empty watch state for an opened root.
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
    ) -> Result<(u64, Receiver<WatchEvent>, Arc<Mutex<Option<WatchEnd>>>), Error> {
        // reject subscriptions while the root cannot sustain a watch
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

        // allocate and publish one live subscription
        let (sender, receiver) = channel(WATCH_CAPACITY);
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

    /// Publish one committed root transition to every active watch.
    pub(crate) fn publish(&mut self, commit: &Commit) {
        // retain subscriptions that accepted this exact commit
        self.subscriptions.retain(|_, subscription| {
            let event = WatchEvent::Commit(commit.clone());
            match subscription.sender.try_send(event) {
                Ok(()) => true,
                Err(error) if error.is_full() => {
                    *subscription.end.lock() = Some(WatchEnd::Lagged);

                    false
                }
                Err(_) => false,
            }
        });
    }

    /// Close every active watch because the owning root closed.
    pub(crate) fn close(&mut self) {
        self.is_closed = true;

        // terminate every subscription with the exact root lifecycle reason
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

/// One sender and its overflow state.
#[derive(Debug)]
struct Subscription {
    /// Pending semantic event sender.
    sender: Sender<WatchEvent>,
    /// Terminal reason shared with the receiving watch.
    end: Arc<Mutex<Option<WatchEnd>>>,
}

/// Terminal reason for one semantic watch.
#[derive(Debug, Clone)]
enum WatchEnd {
    /// The consumer exceeded its bounded commit backlog.
    Lagged,
    /// The owning root closed.
    Closed,
    /// The root host watcher failed.
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
