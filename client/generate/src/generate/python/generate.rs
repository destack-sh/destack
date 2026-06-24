use std::fs;
use std::path::Path;

use anyhow::Result;

use crate::generate::core::write_text;
use crate::generate::schema::Schema;

use super::client::generate_protocol_workspace_client;
use super::defaults::generate_protocol_defaults;
use super::facade::{
    render_generated_root_facade, render_generated_root_stub, render_native_stub,
    render_root_facade, render_root_stub, render_workspace_facade, render_workspace_module_stub,
    render_workspace_package_facade, render_workspace_package_stub,
};
use super::module::{render_module, render_module_stub};
use super::output::{
    generated_package_facade_path, generated_package_stub_path, module_facade_path,
    module_stub_path, package_facade_path, package_stub_path, protocol_module_facade_path,
    prune_outputs, prune_protocol_outputs,
};
use super::package::PythonPackage;

/// Generate Python client bindings.
pub(in crate::generate) fn generate(root: &Path, schema: &Schema) -> Result<()> {
    prune_outputs(root, schema)?;

    for module in &schema.modules {
        let facade = render_module(schema, module);
        write_text(root, &module_facade_path(schema, module), facade)?;

        let stub = render_module_stub(schema, module);
        write_text(root, &module_stub_path(schema, module), stub)?;
    }

    for package in PythonPackage::all(schema).values() {
        let facade = package.render_generated_facade();
        write_text(root, &generated_package_facade_path(package), facade)?;

        let stub = package.render_generated_stub();
        write_text(root, &generated_package_stub_path(package), stub)?;
    }

    for package in PythonPackage::public(schema).values() {
        let facade = package.render_public_facade();
        write_text(root, &package_facade_path(package), facade)?;

        let stub = package.render_public_stub();
        write_text(root, &package_stub_path(package), stub)?;
    }

    let facade = render_root_facade(schema);
    write_text(root, "client/python/src/destack/__init__.py", facade)?;

    let stub = render_root_stub(schema);
    write_text(root, "client/python/src/destack/__init__.pyi", stub)?;

    let facade = render_generated_root_facade();
    write_text(
        root,
        "client/python/src/destack/_generated/__init__.py",
        facade,
    )?;

    let stub = render_generated_root_stub();
    write_text(
        root,
        "client/python/src/destack/_generated/__init__.pyi",
        stub,
    )?;

    let stub = render_native_stub(schema);
    write_text(root, "client/python/src/destack/_native.pyi", stub)?;

    let facade = render_workspace_package_facade();
    write_text(
        root,
        "client/python/src/destack/workspace/__init__.py",
        facade,
    )?;

    let stub = render_workspace_package_stub();
    write_text(
        root,
        "client/python/src/destack/workspace/__init__.pyi",
        stub,
    )?;

    let facade = render_workspace_facade();
    write_text(
        root,
        "client/python/src/destack/workspace/workspace.py",
        facade,
    )?;

    let stub = render_workspace_module_stub();
    write_text(
        root,
        "client/python/src/destack/workspace/workspace.pyi",
        stub,
    )?;

    Ok(())
}

/// Generate Python workspace protocol declarations.
pub(in crate::generate) fn generate_protocol(root: &Path, schema: &Schema) -> Result<()> {
    let protocol_root = root.join("client/python/src/destack/protocol");
    if protocol_root.exists() {
        prune_protocol_outputs(&protocol_root)?;
    }

    let generated_protocol_root = root.join("client/python/src/destack/_generated/protocol");
    if generated_protocol_root.exists() {
        fs::remove_dir_all(generated_protocol_root)?;
    }

    generate_protocol_defaults(root)?;
    generate_protocol_workspace_client(root, schema)?;

    for module in &schema.modules {
        let facade = render_module(schema, module);
        write_text(root, &protocol_module_facade_path(schema, module), facade)?;
    }

    for package in PythonPackage::protocol(schema).values() {
        let facade = package.render_generated_facade();
        write_text(root, &generated_package_facade_path(package), facade)?;
    }

    Ok(())
}
