use std::path::Path;
use std::sync::Arc;

use crate::{ProtocolError, Workspace};

/// Workspace registry used by a protocol server.
pub trait WorkspaceRegistry: Send + Sync + std::fmt::Debug {
    /// Open or return one workspace.
    fn open(&self, workspace: &Path) -> Result<Arc<dyn Workspace>, ProtocolError>;

    /// Return one opened workspace.
    fn workspace(&self, workspace: &Path) -> Option<Arc<dyn Workspace>>;

    /// Acquire one root lease.
    fn acquire(&self, workspace: &Path, root: &Path) -> Result<(), ProtocolError>;

    /// Release one root lease.
    fn release(&self, workspace: &Path, root: &Path) -> Result<(), ProtocolError>;
}
