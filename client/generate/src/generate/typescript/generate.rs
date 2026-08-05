use std::path::Path;

use anyhow::Result;

use crate::generate::core::write_text;
use crate::generate::schema::Schema;

use super::client::generate_protocol_workspace_client;
use super::defaults::generate_protocol_defaults;
use super::module::render_module;
use super::output::{module_path, prune_outputs};

/// Generate TypeScript workspace protocol declarations.
pub(in crate::generate) fn generate(root: &Path, schema: &Schema) -> Result<()> {
    prune_outputs(root)?;
    generate_protocol_defaults(root)?;

    for module in &schema.modules {
        let content = render_module(schema, module, &module.keys);
        let path = module_path(module);

        write_text(root, &path, content)?;
    }
    generate_protocol_workspace_client(root, schema)?;

    Ok(())
}
