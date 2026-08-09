use destack_repository::{Commit, Revision};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// One semantic workspace change observed for an opened root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum WatchEvent {
    /// The exact root revision at subscription time.
    Ready {
        /// Current root revision.
        revision: Revision,
    },
    /// One committed root revision transition.
    Commit(Commit),
}
