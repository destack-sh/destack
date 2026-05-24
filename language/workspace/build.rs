use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::{env, fs, io};

use serde::Deserialize;

const BUILTIN_SCHEME: &str = "destack://";
const LIBRARY_DIRECTORY: &str = "../library";
const LIBRARY_SOURCE_DIRECTORY: &str = "../library/src";
const MANIFEST_FILE: &str = "destack.json";
const OUTPUT_FILE: &str = "builtin.rs";

/// Builtin package manifest fields needed during code generation.
#[derive(Debug, Deserialize)]
struct PackageManifest {
    /// Package name.
    name: Option<String>,
    /// Public package exports.
    exports: BTreeMap<String, PackageExport>,
}

/// Builtin package export fields needed during code generation.
#[derive(Debug, Deserialize)]
#[serde(default)]
struct PackageExport {
    /// Exported material kind.
    kind: ExportKind,
    /// Package relative material path.
    path: String,
}

impl Default for PackageExport {
    fn default() -> Self {
        Self {
            kind: ExportKind::Module,
            path: String::new(),
        }
    }
}

/// Builtin package export material kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ExportKind {
    /// Source module.
    Module,
    /// Static or generated asset.
    Asset,
    /// Template material.
    Template,
    /// Reflect-derived schema material.
    Schema,
    /// Simulation scenario or model material.
    Simulation,
    /// Service definition material.
    Service,
    /// Application definition material.
    App,
}

/// Build embedded builtin source file lists.
fn main() -> Result<(), Box<dyn Error>> {
    let manifest_directory = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let library_directory = manifest_directory.join(LIBRARY_DIRECTORY);
    let library_source_directory = manifest_directory.join(LIBRARY_SOURCE_DIRECTORY);
    let manifest_path = library_directory.join(MANIFEST_FILE);
    let output_directory = PathBuf::from(env::var("OUT_DIR")?);
    let output_path = output_directory.join(OUTPUT_FILE);

    println!("cargo:rerun-if-changed={}", manifest_path.display());

    let manifest = load_manifest(&manifest_path)?;
    let files = collect_builtin_files(&library_source_directory)?;
    let output = render_builtin_table(&files, &manifest);
    fs::write(output_path, output)?;

    Ok(())
}

/// Load the builtin package manifest.
fn load_manifest(manifest_path: &Path) -> Result<PackageManifest, Box<dyn Error>> {
    let manifest = fs::read_to_string(manifest_path)?;
    let manifest = serde_json::from_str(&manifest)?;

    Ok(manifest)
}

/// Collect every builtin file.
fn collect_builtin_files(library_source_directory: &Path) -> io::Result<Vec<String>> {
    let mut files = Vec::new();
    collect_builtin_files_from_directory(
        library_source_directory,
        library_source_directory,
        &mut files,
    )?;
    files.sort();

    Ok(files)
}

/// Collect builtin files recursively from one directory.
fn collect_builtin_files_from_directory(
    library_source_directory: &Path,
    directory: &Path,
    files: &mut Vec<String>,
) -> io::Result<()> {
    println!("cargo:rerun-if-changed={}", directory.display());

    let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_builtin_files_from_directory(library_source_directory, &path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "ds") {
            println!("cargo:rerun-if-changed={}", path.display());

            let relative = path
                .strip_prefix(library_source_directory)
                .map_err(|error| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("builtin source escaped library root: {error}"),
                    )
                })?;
            let file = relative.to_string_lossy().replace('\\', "/");
            files.push(file);
        }
    }

    Ok(())
}

/// Render the embedded builtin file table.
fn render_builtin_table(files: &[String], manifest: &PackageManifest) -> String {
    let mut output = String::new();
    output.push_str("pub(crate) const BUILTINS: &[BuiltinFile] = &[\n");

    for file in files {
        render_builtin_file(&mut output, file);
    }

    output.push_str("];\n");
    output.push_str("pub(crate) const BUILTIN_EXPORTS: &[BuiltinExport] = &[\n");

    for (specifier, export) in &manifest.exports {
        render_builtin_export(&mut output, specifier, export);
    }

    output.push_str("];\n");
    output.push_str("pub(crate) const BUILTIN_PACKAGE_NAME: &str = ");
    output.push_str(&rust_string(manifest.name.as_deref().unwrap_or("destack")));
    output.push_str(";\n");

    output
}

/// Render one builtin file entry.
fn render_builtin_file(output: &mut String, file: &str) {
    let include_path = format!("/{LIBRARY_SOURCE_DIRECTORY}/{file}");
    let uri = builtin_uri(file);

    output.push_str("    BuiltinFile {\n");
    output.push_str("        uri: ");
    output.push_str(&rust_string(&uri));
    output.push_str(",\n");
    output.push_str("        path: ");
    output.push_str(&rust_string(file));
    output.push_str(",\n");
    output.push_str("        content: include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), ");
    output.push_str(&rust_string(&include_path));
    output.push_str(")),\n");
    output.push_str("    },\n");
}

/// Render one builtin package export.
fn render_builtin_export(output: &mut String, specifier: &str, export: &PackageExport) {
    output.push_str("    BuiltinExport {\n");
    output.push_str("        specifier: ");
    output.push_str(&rust_string(specifier));
    output.push_str(",\n");
    output.push_str("        path: ");
    output.push_str(&rust_string(&export.path));
    output.push_str(",\n");
    output.push_str("        kind: ");
    output.push_str(export_kind_name(export.kind));
    output.push_str(",\n");
    output.push_str("    },\n");
}

/// Return the canonical builtin module URI for one library path.
fn builtin_uri(source: &str) -> String {
    let source = source.strip_suffix(".ds").unwrap_or(source);
    let source = source.strip_suffix("/index").unwrap_or(source);

    format!("{BUILTIN_SCHEME}{source}")
}

/// Return the generated Rust variant for one export kind.
fn export_kind_name(kind: ExportKind) -> &'static str {
    match kind {
        ExportKind::Module => "ExportKind::Module",
        ExportKind::Asset => "ExportKind::Asset",
        ExportKind::Template => "ExportKind::Template",
        ExportKind::Schema => "ExportKind::Schema",
        ExportKind::Simulation => "ExportKind::Simulation",
        ExportKind::Service => "ExportKind::Service",
        ExportKind::App => "ExportKind::App",
    }
}

/// Escape a string as a Rust literal.
fn rust_string(value: &str) -> String {
    format!("{value:?}")
}
