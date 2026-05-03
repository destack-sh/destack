use destack_core::{StringId, StringPool};
use destack_dir::{Key, StaticKey, Tree};

use crate::common::dir as common_dir;

/// Resolve a static key from one DIR key when it is locally obvious.
pub(crate) fn static_key_from_key(
    tree: &Tree,
    strings: &StringPool,
    key: Key,
) -> Option<StaticKey> {
    common_dir::static_key_from_key(tree, key, |name| {
        let name = strings.get(name);
        StringId::for_text(&format!("#{}", name.as_ref()))
    })
}
