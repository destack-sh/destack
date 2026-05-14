use destack_core::{StringId, StringPool};
use destack_dir as dir;

use crate::common::dir as common_dir;

/// Resolve a static key from one DIR key when it is locally obvious.
pub(crate) fn static_key_from_key(
    tree: &dir::Tree,
    strings: &StringPool,
    key: dir::Key,
) -> Option<dir::StaticKey> {
    common_dir::static_key_from_key(tree, key, |name| {
        let name = strings.get(name);
        StringId::for_text(&format!("#{name}"))
    })
}
