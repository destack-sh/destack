use std::error::Error;
use std::path::{Path, PathBuf};
use std::{env, fs, io};

const CORE_ROOTS: &[&str] = &["collections", "core", "math", "memory", "reflect", "string"];
const OUTPUT_FILE: &str = "library_sources.rs";

/// Build embedded library source lists.
fn main() -> Result<(), Box<dyn Error>> {
    let manifest_directory = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let library_directory = manifest_directory.join("../library");
    let output_directory = PathBuf::from(env::var("OUT_DIR")?);
    let output_path = output_directory.join(OUTPUT_FILE);

    let sources = collect_sources(&library_directory)?;
    let output = render_sources(&sources);
    fs::write(output_path, output)?;

    Ok(())
}

/// Collect every library source file.
fn collect_sources(library_directory: &Path) -> io::Result<Vec<String>> {
    let mut sources = Vec::new();
    collect_sources_from_directory(library_directory, library_directory, &mut sources)?;
    sources.sort();

    Ok(sources)
}

/// Collect sources recursively from one directory.
fn collect_sources_from_directory(
    library_directory: &Path,
    directory: &Path,
    sources: &mut Vec<String>,
) -> io::Result<()> {
    println!("cargo:rerun-if-changed={}", directory.display());

    let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_sources_from_directory(library_directory, &path, sources)?;
        } else if path.extension().is_some_and(|extension| extension == "ds") {
            println!("cargo:rerun-if-changed={}", path.display());

            let relative = path.strip_prefix(library_directory).map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("library source escaped library root: {error}"),
                )
            })?;
            let source = relative.to_string_lossy().replace('\\', "/");
            sources.push(source);
        }
    }

    Ok(())
}

/// Render embedded source arrays.
fn render_sources(sources: &[String]) -> String {
    let core_sources = sources.iter().filter(|source| is_core_source(source));
    let platform_sources = sources.iter().filter(|source| is_platform_source(source));
    let standard_sources = sources.iter().filter(|source| is_standard_source(source));

    let mut output = String::new();
    output
        .push_str("use crate::library::source::{LibraryOutput, LibraryRuntime, LibrarySource};\n");
    output.push('\n');
    output.push_str("const NATIVE_RUNTIMES: &[LibraryRuntime] = &[LibraryRuntime::Destack];\n\n");
    output.push_str("const NATIVE_OUTPUTS: &[LibraryOutput] = &[LibraryOutput::Native];\n\n");
    output.push_str("pub(crate) const CORE_DECLARED_SYMBOLS: &[&str] = &[\n");
    for symbol in core_declared_symbols() {
        output.push_str("    ");
        output.push_str(&rust_string(symbol));
        output.push_str(",\n");
    }
    output.push_str("];\n\n");

    render_source_array(&mut output, "CORE_SOURCES", core_sources, false);
    render_source_array(&mut output, "PLATFORM_SOURCES", platform_sources, true);
    render_source_array(&mut output, "STANDARD_SOURCES", standard_sources, true);

    output
}

/// Render one source array.
fn render_source_array<'a>(
    output: &mut String,
    name: &str,
    sources: impl Iterator<Item = &'a String>,
    is_native_only: bool,
) {
    output.push_str("pub(crate) const ");
    output.push_str(name);
    output.push_str(": &[LibrarySource] = &[\n");

    for source in sources {
        render_source(output, source, is_native_only);
    }

    output.push_str("];\n\n");
}

/// Render one source entry.
fn render_source(output: &mut String, source: &str, is_native_only: bool) {
    let (path, name) = source.rsplit_once('/').unwrap_or(("", source));
    let include_path = format!("/../library/{source}");

    if is_native_only {
        output.push_str("    LibrarySource::new_with_targets(\n");
    } else {
        output.push_str("    LibrarySource::new(\n");
    }
    output.push_str("        \"library\",\n");
    output.push_str("        ");
    output.push_str(&rust_string(path));
    output.push_str(",\n");
    output.push_str("        ");
    output.push_str(&rust_string(name));
    output.push_str(",\n");
    output.push_str("        include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), ");
    output.push_str(&rust_string(&include_path));
    output.push_str(")),\n");

    if is_native_only {
        output.push_str("        NATIVE_RUNTIMES,\n");
        output.push_str("        NATIVE_OUTPUTS,\n");
        output.push_str("        &[],\n");
    }

    output.push_str("    ),\n");
}

/// Return whether a source belongs to the core language package.
fn is_core_source(source: &str) -> bool {
    source == "prelude.ds"
        || CORE_ROOTS
            .iter()
            .any(|root| source.starts_with(&format!("{root}/")))
}

/// Return whether a source belongs to the platform package.
fn is_platform_source(source: &str) -> bool {
    source.starts_with("platform/")
}

/// Return whether a source belongs to the standard package.
fn is_standard_source(source: &str) -> bool {
    !is_core_source(source) && !is_platform_source(source)
}

/// Return core symbols known directly to the compiler.
fn core_declared_symbols() -> &'static [&'static str] {
    &[
        "String",
        "Array",
        "ReadonlyArray",
        "FixedArray",
        "arrayOf",
        "arrayFill",
        "Slice",
        "Vector",
        "Map",
        "Record",
        "Set",
    ]
}

/// Escape a string as a Rust literal.
fn rust_string(value: &str) -> String {
    format!("{value:?}")
}
