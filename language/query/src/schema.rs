use destack_serde::SchemaRegistry;

use crate::{Module, QueryPosition, QueryRange, QueryRequest, QueryResponse, Target};

/// Register public query schema roots.
pub fn schema(registry: &mut SchemaRegistry) {
    // register target shapes
    registry.register::<Module>();
    registry.register::<QueryPosition>();
    registry.register::<QueryRange>();
    registry.register::<Target>();

    // register request envelopes
    registry.register::<QueryRequest>();
    registry.register::<QueryResponse>();
}
