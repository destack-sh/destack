use std::error::Error;
use std::path::{Path, PathBuf};
use std::{env, fs, io};

const BUILTIN_SCHEME: &str = "destack://";
const OUTPUT_FILE: &str = "builtin.rs";

/// Build embedded builtin source file lists.
fn main() -> Result<(), Box<dyn Error>> {
    let manifest_directory = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let library_directory = manifest_directory.join("../library");
    let output_directory = PathBuf::from(env::var("OUT_DIR")?);
    let output_path = output_directory.join(OUTPUT_FILE);

    let files = collect_builtin_files(&library_directory)?;
    let output = render_builtin_table(&files);
    fs::write(output_path, output)?;

    Ok(())
}

/// Collect every builtin file.
fn collect_builtin_files(library_directory: &Path) -> io::Result<Vec<String>> {
    let mut files = Vec::new();
    collect_builtin_files_from_directory(library_directory, library_directory, &mut files)?;
    files.sort();

    Ok(files)
}

/// Collect builtin files recursively from one directory.
fn collect_builtin_files_from_directory(
    library_directory: &Path,
    directory: &Path,
    files: &mut Vec<String>,
) -> io::Result<()> {
    println!("cargo:rerun-if-changed={}", directory.display());

    let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_builtin_files_from_directory(library_directory, &path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "ds") {
            println!("cargo:rerun-if-changed={}", path.display());

            let relative = path.strip_prefix(library_directory).map_err(|error| {
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
fn render_builtin_table(files: &[String]) -> String {
    let mut output = String::new();
    output.push_str("pub(crate) const BUILTINS: &[BuiltinFile] = &[\n");

    for file in files {
        render_builtin_file(&mut output, file);
    }

    output.push_str("];\n");

    output
}

/// Render one builtin file entry.
fn render_builtin_file(output: &mut String, file: &str) {
    let include_path = format!("/../library/{file}");
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

/// Return the canonical builtin module URI for one library path.
fn builtin_uri(source: &str) -> String {
    let source = source.strip_suffix(".ds").unwrap_or(source);
    let source = source.strip_suffix("/index").unwrap_or(source);

    format!("{BUILTIN_SCHEME}{source}")
}

/// Escape a string as a Rust literal.
fn rust_string(value: &str) -> String {
    format!("{value:?}")
}
