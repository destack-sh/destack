use destack_dir::{Key, NodeTree, StaticKey};

use crate::Compiler;
use crate::common::dir as common_dir;

impl Compiler {
    /// Resolve a static key from one DIR key when it is locally obvious.
    pub(crate) fn static_key_from_key(&self, tree: &NodeTree, key: Key) -> Option<StaticKey> {
        common_dir::static_key_from_key(tree, key, |name| {
            let name = self.repository.strings.get(name);
            self.repository
                .strings
                .intern(&format!("#{}", name.as_ref()))
        })
    }
}
