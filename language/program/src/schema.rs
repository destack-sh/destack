use destack_serde::SchemaRegistry;

use crate::ProgramHeader;

/// Include public program schema roots.
pub fn schema(registry: &mut SchemaRegistry) {
    registry.register::<ProgramHeader>();
}
