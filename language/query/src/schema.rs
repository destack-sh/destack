use destack_serde::SchemaRegistry;

use crate::{
    QueryModule, QueryPosition, QueryRange, QueryRequest, QueryResponse, QueryTarget, QueryText,
};

/// Include public query schema roots.
pub fn schema(registry: &mut SchemaRegistry) {
    registry.register::<QueryModule>();
    registry.register::<QueryPosition>();
    registry.register::<QueryRange>();
    registry.register::<QueryTarget>();
    registry.register::<QueryText>();

    registry.register::<QueryRequest>();
    registry.register::<QueryResponse>();
}
