use std::collections::HashMap;
use std::path::{Path, PathBuf};

use parking_lot::Mutex;

use crate::Workspace;
use crate::diagnostic::Error;

/// Reference counter for opened protocol roots.
#[derive(Debug, Default)]
pub struct RootLease {
    /// Active handle count by root path.
    counts: Mutex<HashMap<PathBuf, usize>>,
}

impl RootLease {
    /// Acquire one protocol root handle.
    pub fn acquire(&self, workspace: &dyn Workspace, root: &Path) -> Result<(), Error> {
        let mut counts = self.counts.lock();

        // reuse already opened roots
        if let Some(count) = counts.get_mut(root) {
            *count += 1;
            return Ok(());
        }

        // open and register the first handle
        workspace.open(root.to_path_buf())?;
        counts.insert(root.to_path_buf(), 1);

        Ok(())
    }

    /// Release one protocol root handle.
    pub fn release(&self, workspace: &dyn Workspace, root: &Path) -> Result<(), Error> {
        let mut counts = self.counts.lock();
        let Some(count) = counts.get_mut(root) else {
            return Err(Error::Internal {
                detail: format!("root lease missing for {}", root.display()),
            });
        };

        // keep roots open while other handles exist
        if *count > 1 {
            *count -= 1;
            return Ok(());
        }

        // close and remove the final lease
        workspace.close(root)?;
        counts.remove(root);

        Ok(())
    }
}
