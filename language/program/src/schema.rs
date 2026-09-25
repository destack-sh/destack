use tspp_serde::Schema;

use crate::{Object, Program};

/// Include public program schema roots.
pub fn schema(schema: &mut Schema) {
    schema.register::<Object>();
    schema.register::<Program>();
}
