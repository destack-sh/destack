use destack_serde::SchemaRegistry;

use crate::Program;

/// Include public program schema roots.
pub fn schema(registry: &mut SchemaRegistry) {
    registry.register::<Program>();
}
