use std::collections::HashMap;
use std::path::{Path, PathBuf};

use parking_lot::Mutex;

/// Reference counts for protocol root handles.
#[derive(Debug, Default)]
pub(crate) struct RootLeaseTable {
    /// Active handle count by root path.
    counts: Mutex<HashMap<PathBuf, usize>>,
}

impl RootLeaseTable {
    /// Record one acquired root handle.
    pub(crate) fn acquire(&self, root: &Path) {
        let mut counts = self.counts.lock();
        let count = counts.entry(root.to_path_buf()).or_default();
        *count += 1;
    }

    /// Release one root handle and return true when the root should close.
    pub(crate) fn release(&self, root: &Path) -> bool {
        let mut counts = self.counts.lock();
        let Some(count) = counts.get_mut(root) else {
            return false;
        };

        if *count > 1 {
            *count -= 1;
            return false;
        }

        counts.remove(root);

        true
    }
}
