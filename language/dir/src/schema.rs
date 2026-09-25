use tspp_serde::Schema;

use crate::{Tree, Type};

/// Include public DIR schema roots.
pub fn schema(schema: &mut Schema) {
    schema.register::<Tree>();
    schema.register::<Type>();
}
