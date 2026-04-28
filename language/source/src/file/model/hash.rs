use std::path::Path;

use destack_core::StableHasher;

/// Hash one source id domain and framed components into a stable 128-bit id.
pub(super) fn stable_source_id(domain: &[u8], parts: &[&[u8]]) -> u128 {
    let mut hasher = StableHasher::new();
    hasher.update_len_prefixed(domain);

    for part in parts {
        hasher.update_len_prefixed(part);
    }

    hasher.finish_u128()
}

/// Return one normalized source path string for stable source ids.
pub(super) fn stable_source_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
