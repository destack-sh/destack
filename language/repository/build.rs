use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::{env, fs, io};

use serde::Deserialize;
use tspp_core::Blob;
use tspp_source::{File, FileId, ModuleId, PackageId, Uri};

const BUILTIN_SCHEME: &str = "tspp://";
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
    Reflect,
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
    let output =
        render_builtin_table(&files, &manifest, &library_source_directory, &manifest_path)?;
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
        } else if path
            .extension()
            .is_some_and(|extension| extension == "tspp")
        {
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
fn render_builtin_table(
    files: &[String],
    manifest: &PackageManifest,
    source_directory: &Path,
    manifest_path: &Path,
) -> Result<String, Box<dyn Error>> {
    let package_name = manifest.name.as_deref().unwrap_or("tspp");
    let package_id = PackageId::from_uri(&Uri::from_string(package_name));
    let mut output = String::new();
    output.push_str("pub(crate) const BUILTINS: &[BuiltinFile] = &[\n");

    for file in files {
        let content = fs::read_to_string(source_directory.join(file))?;
        ensure_canonical_line_endings(file, &content)?;
        let builtin = render_builtin_file(file, &content, package_id, LIBRARY_SOURCE_DIRECTORY);
        output.push_str(&builtin);
        output.push_str(",\n");
    }

    output.push_str("];\n");
    render_builtin_file_ids(&mut output, files);
    output.push_str("pub(crate) const BUILTIN_EXPORTS: &[BuiltinExport] = &[\n");

    for (specifier, export) in &manifest.exports {
        render_builtin_export(&mut output, specifier, export);
    }

    output.push_str("];\n");
    let manifest_content = fs::read_to_string(manifest_path)?;
    ensure_canonical_line_endings(MANIFEST_FILE, &manifest_content)?;
    let manifest = render_builtin_file(
        MANIFEST_FILE,
        &manifest_content,
        package_id,
        LIBRARY_DIRECTORY,
    );
    output.push_str("pub(crate) const BUILTIN_MANIFEST_FILE: BuiltinFile = ");
    output.push_str(&manifest);
    output.push_str(";\n");
    output.push_str("pub(crate) const BUILTIN_PACKAGE_NAME: &str = ");
    output.push_str(&rust_string(package_name));
    output.push_str(";\n");
    output.push_str("pub(crate) const BUILTIN_PACKAGE_ID: PackageId = PackageId::new(");
    output.push_str(&package_id.0.to_string());
    output.push_str(");\n");

    Ok(output)
}

/// Render one builtin file entry.
fn render_builtin_file(
    file: &str,
    content: &str,
    package_id: PackageId,
    include_directory: &str,
) -> String {
    let include_path = format!("/{include_directory}/{file}");
    let uri = builtin_uri(file);
    let file_id = FileId::from_logical_str(&uri);
    let module_id = ModuleId::from_path(package_id, Path::new(file), None);
    let blob = Blob::for_bytes(content.as_bytes());
    let line_starts = File::line_starts(content);
    let file_id = file_id.0;
    let package_id = module_id.package_id.0;
    let module_key = module_id.module_key.raw();
    let blob_id = blob.id.bytes();
    let blob_len = blob.byte_len;
    let uri = rust_string(&uri);
    let path = rust_string(file);
    let include_path = rust_string(&include_path);

    format!(
        concat!(
            "BuiltinFile {{\n",
            "    file_id: FileId::new({file_id}),\n",
            "    module_id: ModuleId::new(PackageId::new({package_id}), {module_key}),\n",
            "    blob: Blob::new(BlobId::new({blob_id:?}), {blob_len}),\n",
            "    uri: {uri},\n",
            "    path: {path},\n",
            "    content: include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), ",
            "{include_path})),\n",
            "    line_starts: &{line_starts:?},\n",
            "}}",
        ),
        file_id = file_id,
        package_id = package_id,
        module_key = module_key,
        blob_id = blob_id,
        blob_len = blob_len,
        uri = uri,
        path = path,
        include_path = include_path,
        line_starts = line_starts,
    )
}

/// Render source-table positions ordered by FileId.
fn render_builtin_file_ids(output: &mut String, files: &[String]) {
    let mut ids = files
        .iter()
        .enumerate()
        .map(|(index, file)| (FileId::from_logical_str(&builtin_uri(file)), index))
        .collect::<Vec<_>>();
    ids.sort_by_key(|(file_id, _)| *file_id);

    output.push_str("pub(crate) const BUILTIN_FILE_IDS: &[(FileId, usize)] = &[\n");
    for (file_id, index) in ids {
        output.push_str("    (FileId::new(");
        output.push_str(&file_id.0.to_string());
        output.push_str("), ");
        output.push_str(&index.to_string());
        output.push_str("),\n");
    }
    output.push_str("];\n");
}

/// Require embedded sources to retain their exact build-time bytes.
fn ensure_canonical_line_endings(file: &str, content: &str) -> io::Result<()> {
    if content.contains('\r') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("builtin source uses non-LF line endings: {file}"),
        ));
    }

    Ok(())
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
    let source = source.strip_suffix(".tspp").unwrap_or(source);
    let source = source.strip_suffix("/index").unwrap_or(source);

    format!("{BUILTIN_SCHEME}{source}")
}

/// Return the generated Rust variant for one export kind.
fn export_kind_name(kind: ExportKind) -> &'static str {
    match kind {
        ExportKind::Module => "ExportKind::Module",
        ExportKind::Asset => "ExportKind::Asset",
        ExportKind::Template => "ExportKind::Template",
        ExportKind::Reflect => "ExportKind::Reflect",
        ExportKind::Simulation => "ExportKind::Simulation",
        ExportKind::Service => "ExportKind::Service",
        ExportKind::App => "ExportKind::App",
    }
}

/// Escape a string as a Rust literal.
fn rust_string(value: &str) -> String {
    format!("{value:?}")
}
