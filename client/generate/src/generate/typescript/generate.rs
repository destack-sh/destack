use std::fs;
use std::path::Path;

use anyhow::Result;

use crate::generate::core::write_text;
use crate::generate::schema::Schema;

use super::client::Client;
use super::defaults::generate_rpc_defaults;
use super::module::render_module;
use super::path::{GENERATED_ROOT, output_path};

/// Generate TypeScript protocol declarations and service clients.
pub(in crate::generate) fn generate(root: &Path, schema: &Schema) -> Result<()> {
    clear(root)?;
    generate_rpc_defaults(root)?;

    for module in &schema.modules {
        let content = render_module(schema, module, &module.keys);
        let path = output_path(&module.path);

        write_text(root, &path, content)?;
    }
    Client::generate(root, schema)?;

    Ok(())
}

/// Remove all previous generated TypeScript files.
fn clear(root: &Path) -> Result<()> {
    let path = root.join(GENERATED_ROOT);
    if path.exists() {
        fs::remove_dir_all(path)?;
    }

    Ok(())
}
