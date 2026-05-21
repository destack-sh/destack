use std::sync::Arc;

use destack_core::StringPool;
use destack_dir as dir;
use destack_formatter::format_file_source;
use destack_parser::{Parser, ParserOptions};
use destack_source::{
    DiagnosticCollection, DiffOptions, FileId, LanguageType, PrintOptions, print_diagnostics,
    print_diff,
};
use destack_workspace::{BuiltinFile, BuiltinPackage, FormatterOptions};

/// Parse every builtin source file.
#[test]
fn test_parse_builtin_library() {
    let package = BuiltinPackage::new();
    let mut diagnostics = DiagnosticCollection::new();

    // parse every builtin source
    for file in package.files().iter().copied() {
        let file_diagnostics = parse_builtin_file(&package, file);
        diagnostics.merge_from(&file_diagnostics);
    }

    assert!(
        diagnostics.is_empty(),
        "builtin library should parse cleanly:\n{}",
        render_builtin_diagnostics(&package, diagnostics)
    );
}

/// Format check every builtin source file and require stable second-pass output.
#[test]
fn test_format_builtin_library() {
    let package = BuiltinPackage::new();
    let failures = package
        .files()
        .iter()
        .copied()
        .filter_map(|file| format_builtin_file(file).err())
        .collect::<Vec<_>>();

    assert!(
        failures.is_empty(),
        "builtin library should format cleanly:\n{}",
        failures.join("\n\n")
    );
}

/// Parse one builtin file with the compiler module parser shape.
fn parse_builtin_file(package: &BuiltinPackage, file: BuiltinFile) -> DiagnosticCollection {
    let source_file = Arc::new(file.file());
    let language = LanguageType::try_from(source_file.ty).expect("builtin file should be code");
    let module = file.module_id(package.package_id());
    let tree = dir::Tree::new(module);

    // parse as one compiler module
    let mut parser = Parser::lex_module_tree_with_options(
        Arc::clone(&source_file),
        language,
        ParserOptions::default(),
        Arc::new(StringPool::new()),
        tree,
    );
    parser.parse();
    parser.diagnostics.collect()
}

/// Format check one builtin file and require stable second-pass output.
fn format_builtin_file(file: BuiltinFile) -> Result<(), String> {
    let source_file = file.file();
    let options = FormatterOptions::default();

    // format original source
    let first = match format_file_source(&source_file, file.content, options) {
        Ok(output) => output,
        Err(error) => {
            return Err(format!("{}:\n{}", file.path, error.message));
        }
    };

    // require checked-in formatting
    if first != file.content {
        let diff_options = DiffOptions::new()
            .with_path(file.path)
            .with_color(false)
            .with_lengths();
        print_diff(file.content, &first, &diff_options);

        return Err(format!(
            "{}:\nformatter output differs from source",
            file.path
        ));
    }

    // require formatter stability
    match format_file_source(&source_file, &first, options) {
        Ok(second) if second == first => Ok(()),
        Ok(second) => {
            let diff_options = DiffOptions::new()
                .with_path(file.path)
                .with_color(false)
                .with_lengths();
            print_diff(&first, &second, &diff_options);

            Err(format!(
                "{}:\nformatter output changed on second pass",
                file.path
            ))
        }
        Err(error) => Err(format!(
            "{}:\nsecond formatter pass failed\n{}",
            file.path, error.message
        )),
    }
}

/// Render builtin parse diagnostics with source snippets.
fn render_builtin_diagnostics(
    package: &BuiltinPackage,
    diagnostics: DiagnosticCollection,
) -> String {
    let lines = Arc::new(parking_lot::Mutex::new(Vec::new()));
    let writer_lines = Arc::clone(&lines);
    let writer = Arc::new(move |line: &str| {
        writer_lines.lock().push(line.to_string());
    });
    let options = PrintOptions::new()
        .with_color(false)
        .with_line_writer(writer);
    let files = package.files();
    let file_for_id =
        |file_id: FileId| builtin_file_for_id(files, file_id).map(|file| Arc::new(file.file()));
    print_diagnostics(&file_for_id, &diagnostics, options)
        .expect("builtin diagnostics should render");

    lines.lock().join("\n")
}

/// Return one builtin source file by file id.
fn builtin_file_for_id(files: &[BuiltinFile], file_id: FileId) -> Option<BuiltinFile> {
    files.iter().copied().find(|file| file.file_id() == file_id)
}
