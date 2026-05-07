use std::path::PathBuf;

use destack_source::PathExt;
use rustc_hash::FxHashMap;

use crate::Mount;

/// One prepared mount table.
#[derive(Debug, Clone, Default)]
pub(crate) struct MountTable {
    /// Mounted package roots by specifier.
    entries: FxHashMap<String, Mount>,
}

impl MountTable {
    /// Create one mount table from resolver options.
    pub(crate) fn new(mounts: &[Mount]) -> Self {
        let entries = mounts
            .iter()
            .map(|mount| (mount.specifier.clone(), mount.clone()))
            .collect();

        Self { entries }
    }

    /// Return the mounted path for one matching package specifier.
    pub(crate) fn path(&self, specifier: &str) -> Option<PathBuf> {
        let mut prefix = specifier;

        loop {
            if let Some(mount) = self.entries.get(prefix) {
                return Some(Self::mount_path(mount, specifier, prefix));
            }

            let Some((next_prefix, _)) = prefix.rsplit_once('/') else {
                return None;
            };
            prefix = next_prefix;
        }
    }

    /// Return the concrete path for one matched mount.
    fn mount_path(mount: &Mount, specifier: &str, prefix: &str) -> PathBuf {
        if specifier == prefix {
            return match &mount.entry {
                Some(entry) => mount.root.normalize_with(entry),
                None => mount.root.clone(),
            };
        }

        let tail = specifier
            .strip_prefix(prefix)
            .and_then(|tail| tail.strip_prefix('/'))
            .expect("mount prefix should be split on a specifier boundary");

        mount.root.normalize_with(tail)
    }
}
