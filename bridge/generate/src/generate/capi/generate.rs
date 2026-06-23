use std::path::Path;

use anyhow::Result;

use crate::generate::core::{write_rust, write_text};
use crate::generate::schema::Schema;

use super::header::Header;
use super::output::prune_outputs;
use super::projection::Projection;
use super::rust::Rust;

/// Generate C ABI bridge bindings.
pub(in crate::generate) fn generate(root: &Path, schema: &Schema) -> Result<()> {
    prune_outputs(root)?;

    let projection = Projection::new(schema)?;

    let header = Header::new(schema, &projection);
    for output in header.render() {
        write_text(root, &output.path, output.content)?;
    }

    let rust = Rust::new(schema, &projection);
    write_rust(root, "bridge/capi/src/generated.rs", rust.render())?;

    Ok(())
}
