use tspp_serde::Schema;

use crate::{Module, QueryPosition, QueryRange, QueryRequest, QueryResponse, Target};

/// Register public query schema roots.
pub fn schema(schema: &mut Schema) {
    // register target shapes
    schema.register::<Module>();
    schema.register::<QueryPosition>();
    schema.register::<QueryRange>();
    schema.register::<Target>();

    // register request envelopes
    schema.register::<QueryRequest>();
    schema.register::<QueryResponse>();
}
