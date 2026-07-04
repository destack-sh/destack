use destack_serde::SchemaRegistry;

use crate::{Module, Position, QueryRequest, QueryResponse, Range, Target, Text};

/// Register public query schema roots.
pub fn schema(registry: &mut SchemaRegistry) {
    // register target shapes
    registry.register::<Module>();
    registry.register::<Position>();
    registry.register::<Range>();
    registry.register::<Target>();
    registry.register::<Text>();

    // register request envelopes
    registry.register::<QueryRequest>();
    registry.register::<QueryResponse>();
}
