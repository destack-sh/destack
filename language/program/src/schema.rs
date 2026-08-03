use destack_serde::SchemaRegistry;

use crate::{Object, Program};

/// Include public program schema roots.
pub fn schema(registry: &mut SchemaRegistry) {
    registry.register::<Object>();
    registry.register::<Program>();
}
