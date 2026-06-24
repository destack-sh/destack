use destack_serde::SchemaRegistry;

use crate::{Tree, Type};

/// Include public DIR schema roots.
pub fn schema(registry: &mut SchemaRegistry) {
    registry.register::<Tree>();
    registry.register::<Type>();
}
