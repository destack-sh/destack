use std::fs;
use std::path::Path;

use anyhow::Result;

use crate::generate::core::write_text;
use crate::generate::schema::Schema;

use super::client::generate_protocol_workspace_client;
use super::defaults::generate_protocol_defaults;
use super::index::render_index;
use super::module::{render_module, render_protocol_module};
use super::output::{module_path, prune_outputs};

/// Generate TypeScript bridge declarations.
pub(in crate::generate) fn generate(root: &Path, schema: &Schema) -> Result<()> {
    prune_outputs(root, schema)?;

    for module in &schema.modules {
        let content = render_module(schema, module, &module.names);
        let path = module_path(module);

        write_text(root, &path, content)?;
    }

    Ok(())
}

/// Generate TypeScript workspace protocol declarations.
pub(in crate::generate) fn generate_protocol(root: &Path, schema: &Schema) -> Result<()> {
    let generated_protocol_root = root.join("bridge/typescript/src/generated/protocol");
    if generated_protocol_root.exists() {
        fs::remove_dir_all(generated_protocol_root)?;
    }

    generate_protocol_defaults(root)?;
    for module in &schema.modules {
        let content = render_protocol_module(schema, module, &module.names);
        let path = module_path(module);

        write_text(root, &path, content)?;
    }

    generate_protocol_workspace_client(root, schema)?;

    Ok(())
}

/// Generate the canonical TypeScript public entrypoint.
pub(in crate::generate) fn generate_index(root: &Path, schema: &Schema) -> Result<()> {
    let content = render_index(schema);

    write_text(root, "bridge/typescript/src/index.ts", content)
}
