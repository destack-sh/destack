use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_core::StringPool;
use destack_source::{File, FileId, FileType, LanguageType, PrintOptions, Uri, print_diagnostics};

use crate::{Parser, ParserOptions, ParserTriviaMode, source_colorizer};

/// Return the checked-in library corpus root.
fn library_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../library")
}

/// Collect every checked-in library `.ds` source file.
fn library_sources(root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_library_sources(root, &mut paths);

    paths
}

/// Collect every checked-in library `.ds` source file.
fn collect_library_sources(root: &Path, paths: &mut Vec<PathBuf>) {
    let mut entries = fs::read_dir(root)
        .expect("expected library directory")
        .map(|entry| entry.expect("expected library entry").path())
        .collect::<Vec<_>>();
    entries.sort();

    for path in entries {
        // descend into library modules
        if path.is_dir() {
            collect_library_sources(&path, paths);
        }
        // collect source files
        else if path.extension().is_some_and(|extension| extension == "ds") {
            paths.push(path);
        }
    }
}

/// Build a parser file for one library path.
fn library_file(path: &Path, source: String) -> Arc<File> {
    let file_name = path
        .file_name()
        .expect("expected library file name")
        .to_string_lossy()
        .to_string();
    let path_text = path.to_string_lossy();

    Arc::new(File::from_text(
        FileId::from_logical_path(path),
        file_name,
        Uri::from_string(path_text.as_ref()),
        Some(path.to_path_buf()),
        FileType::Destack,
        source,
    ))
}

/// Parse one source file as a checked-in library module.
fn parse_library_source(path: &Path, strings: Arc<StringPool>) -> (Arc<File>, Parser) {
    let source = fs::read_to_string(path).expect("expected library source");
    let file = library_file(path, source);
    let parser = Parser::lex_file_with_options(
        file.clone(),
        LanguageType::Destack,
        ParserOptions {
            trivia_mode: ParserTriviaMode::Documentation,
            ..ParserOptions::default()
        },
        strings,
    );

    (file, parser)
}

/// Print parser diagnostics for one library file.
fn print_library_diagnostics(file: Arc<File>, parser: &Parser) {
    let file_id = file.id;
    let file_for_id = |current_file_id| {
        if current_file_id == file_id {
            Some(file.clone())
        } else {
            None
        }
    };

    let diagnostics = parser.diagnostics();
    let options = PrintOptions::new().with_colorizer(source_colorizer());
    let _ = print_diagnostics(&file_for_id, &diagnostics, options);
}

/// Parse every checked-in library source without parser diagnostics.
#[test]
fn test_parse_library() {
    let root = library_root();
    let paths = library_sources(&root);
    let strings = Arc::new(StringPool::new());
    let mut failures = Vec::new();

    for path in paths {
        let (file, mut parser) = parse_library_source(&path, strings.clone());
        parser.parse();

        // record every failing path
        if !parser.errors.is_empty() {
            let relative_path = path
                .strip_prefix(&root)
                .expect("expected source below library root");
            print_library_diagnostics(file, &parser);
            failures.push(relative_path.display().to_string());
        }
    }

    assert!(
        failures.is_empty(),
        "library parser corpus failures: {} files\n{}",
        failures.len(),
        failures.join("\n")
    );
}
