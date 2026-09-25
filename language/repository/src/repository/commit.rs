use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::repository::{Change, Revision};

/// One immutable repository revision transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Commit {
    /// The previous repository revision.
    pub before: Revision,
    /// The updated repository revision.
    pub after: Revision,
    /// Canonical file differences ordered by path.
    pub changes: Vec<Change>,
}
