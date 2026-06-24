use destack_serde::SchemaRegistry;

use crate::{Program, ProgramHeader};

/// Include public executable program schema roots.
pub fn schema(registry: &mut SchemaRegistry) {
    registry.register::<ProgramHeader>();
    registry.register::<Program>();
}
