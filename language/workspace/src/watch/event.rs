use serde::{Deserialize, Serialize};
use tspp_repository::{Commit, Revision};
use tspp_serde::Reflect;

/// One semantic workspace change observed from physical state or a branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum WatchEvent {
    /// The exact watched revision at subscription time.
    Ready {
        /// Current watched revision.
        revision: Revision,
    },
    /// One committed watched revision transition.
    Commit(Commit),
}
