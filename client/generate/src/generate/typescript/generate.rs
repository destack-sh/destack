use std::fs;
use std::path::Path;

use anyhow::Result;

use crate::generate::core::write_text;
use crate::generate::schema::Schema;

use super::client::generate_protocol_workspace_client;
use super::defaults::generate_protocol_defaults;
use super::index::{render_generated_indexes, render_index, render_public_indexes};
use super::module::render_module;
use super::output::{module_path, prune_generated_file, prune_outputs};

/// Generate TypeScript client declarations.
pub(in crate::generate) fn generate(root: &Path, schema: &Schema) -> Result<()> {
    prune_outputs(root, schema)?;

    for module in &schema.modules {
        let content = render_module(schema, module, &module.keys);
        let path = module_path(module);

        write_text(root, &path, content)?;
    }
    for output in render_generated_indexes(schema) {
        write_text(root, &output.path, output.content)?;
    }
    for output in render_public_indexes(schema) {
        prune_generated_file(root, &replaced_public_index_path(&output.path))?;
        write_text(root, &output.path, output.content)?;
    }

    Ok(())
}

/// Return the generated public facade path replaced by one output.
fn replaced_public_index_path(path: &str) -> String {
    path.replace("index.generated.ts", "index.ts")
}

/// Generate TypeScript workspace protocol declarations.
pub(in crate::generate) fn generate_protocol(root: &Path, schema: &Schema) -> Result<()> {
    let generated_protocol_root = root.join("client/typescript/src/_generated/protocol");
    if generated_protocol_root.exists() {
        fs::remove_dir_all(generated_protocol_root)?;
    }

    generate_protocol_defaults(root)?;
    for module in &schema.modules {
        let content = render_module(schema, module, &module.keys);
        let path = module_path(module);

        write_text(root, &path, content)?;
    }

    generate_protocol_workspace_client(root, schema)?;

    Ok(())
}

/// Generate the canonical TypeScript public entrypoint.
pub(in crate::generate) fn generate_index(root: &Path, schema: &Schema) -> Result<()> {
    let content = render_index(schema);

    write_text(root, "client/typescript/src/index.generated.ts", content)
}
