use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::format_file_source;
use destack_core::StringPool;
use destack_parser::{Parser, ParserOptions, source_colorizer};
use destack_source::{
    DiffOptions, File, FileId, FileType, LanguageType, PrintOptions, Uri, print_diagnostics,
    print_diff,
};
use destack_workspace::FormatterOptions;

const CORPUS_SECTION_SEPARATOR: &str =
    "================================================================================";
const CORPUS_FIRST_PASS: &str = "first pass";
const CORPUS_SECOND_PASS: &str = "second pass";
const CORPUS_IDEMPOTENCE_PASS: &str = "idempotence";

/// One library corpus failure summary.
#[derive(Debug)]
struct CorpusFailure {
    /// The library-relative path.
    path: String,
    /// The failed formatter pass.
    pass: &'static str,
    /// The failure detail.
    detail: String,
}

impl CorpusFailure {
    /// Create one failure summary.
    fn new(path: &Path, pass: &'static str, detail: impl Into<String>) -> Self {
        let detail = detail.into().replace('\n', "; ");

        Self {
            path: path.display().to_string(),
            pass,
            detail,
        }
    }

    /// Format the failure as one summary line.
    fn summary(&self) -> String {
        format!("- {}: {}: {}", self.path, self.pass, self.detail)
    }
}

/// Return whether one source file belongs to the library formatter corpus.
fn is_library_formatter_source(file_type: FileType) -> bool {
    matches!(
        file_type,
        FileType::Destack
            | FileType::DestackDeclaration
            | FileType::JavaScript
            | FileType::JavaScriptXml
            | FileType::TypeScript
            | FileType::TypeScriptDeclaration
            | FileType::TypeScriptXml
    )
}

/// Collect library source files accepted by the formatter.
fn collect_library_formatter_sources(root: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root).unwrap();

    // recurse in lexical order for stable failure lists
    let mut entries = entries
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    entries.sort();

    for path in entries {
        // nested directories
        if path.is_dir() {
            collect_library_formatter_sources(&path, files);
            continue;
        }

        let Some(file_type) = FileType::from_path(&path) else {
            continue;
        };

        if is_library_formatter_source(file_type) {
            files.push(path);
        }
    }
}

/// Get the library corpus root path.
fn library_corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../library")
}

/// Return one path relative to the library corpus root.
fn relative_library_path<'a>(root: &Path, path: &'a Path) -> &'a Path {
    path.strip_prefix(root).unwrap()
}

/// Format one path once.
fn format_library_source(path: &Path, source: &str) -> Result<String, String> {
    let file = build_library_file(path, source);

    format_file_source(&file, source, FormatterOptions::default()).map_err(|error| error.message)
}

/// Print one corpus diagnostics section header.
fn print_corpus_section(title: &str, path: &Path) {
    eprintln!();
    eprintln!("{CORPUS_SECTION_SEPARATOR}");
    eprintln!("=== {title}");
    eprintln!("=== {}", path.display());
    eprintln!("{CORPUS_SECTION_SEPARATOR}");
    eprintln!();
}

/// Count failures for one formatter pass.
fn count_corpus_failures(failures: &[CorpusFailure], pass: &str) -> usize {
    failures
        .iter()
        .filter(|failure| failure.pass == pass)
        .count()
}

/// Print one corpus failure summary.
fn print_corpus_summary(checked_file_count: usize, failures: &[CorpusFailure]) {
    print_corpus_section("library formatter corpus: summary", Path::new("."));

    let first_pass_failures = count_corpus_failures(failures, CORPUS_FIRST_PASS);
    let second_pass_failures = count_corpus_failures(failures, CORPUS_SECOND_PASS);
    let idempotence_failures = count_corpus_failures(failures, CORPUS_IDEMPOTENCE_PASS);

    eprintln!(
        "checked {checked_file_count} files, found {} failures",
        failures.len()
    );
    eprintln!("first pass parse failures: {first_pass_failures}");
    eprintln!("second pass parse failures: {second_pass_failures}");
    eprintln!("idempotence failures: {idempotence_failures}");
    eprintln!();

    for failure in failures {
        eprintln!("{}", failure.summary());
    }
}

/// Build a source file for one library path.
fn build_library_file(path: &Path, source: &str) -> File {
    let file_name = path.file_name().unwrap().to_string_lossy().to_string();
    let path_text = path.to_string_lossy();
    File::from_text(
        FileId::from_logical_path(path),
        file_name,
        Uri::from_string(path_text.as_ref()),
        Some(path.to_path_buf()),
        FileType::from_path(path).unwrap(),
        source.to_string(),
    )
}

/// Print parser diagnostics for one library source.
fn print_library_parse_diagnostics(path: &Path, source: &str) {
    let file = Arc::new(build_library_file(path, source));
    let file_id = file.id;
    let file_for_id = |current_file_id| {
        if current_file_id == file_id {
            Some(file.clone())
        } else {
            None
        }
    };
    let language_type = LanguageType::try_from(file.ty).expect("file type has no parser language");
    let mut parser = Parser::lex_file_with_options(
        file.clone(),
        language_type,
        ParserOptions {
            preserve_parenthesized_wrappers: false,
            ..ParserOptions::default()
        },
        Arc::new(StringPool::new()),
    );
    parser.parse();

    let diagnostics = parser.diagnostics();

    let options = PrintOptions::new().with_colorizer(source_colorizer());
    let _ = print_diagnostics(&file_for_id, &diagnostics, options);
}

/// Assert parser and formatter idempotence over the checked-in library corpus.
#[test]
#[ignore]
fn test_format_library_corpus_is_idempotent() -> Result<(), String> {
    let root = library_corpus_root();
    let mut paths = Vec::new();
    collect_library_formatter_sources(&root, &mut paths);

    let checked_file_count = paths.len();
    let mut failures = Vec::new();

    for path in paths {
        let source = fs::read_to_string(&path).unwrap();
        let relative_path = relative_library_path(&root, &path);

        // first pass must parse and format
        let first = match format_library_source(&path, &source) {
            Ok(first) => first,
            Err(error) => {
                print_corpus_section(
                    "library formatter corpus: first pass parse failure",
                    relative_path,
                );
                print_library_parse_diagnostics(&path, &source);
                failures.push(CorpusFailure::new(relative_path, CORPUS_FIRST_PASS, error));
                continue;
            }
        };

        // second pass must parse and reach a fixed point
        let second = match format_library_source(&path, &first) {
            Ok(second) => second,
            Err(error) => {
                print_corpus_section(
                    "library formatter corpus: second pass parse failure",
                    relative_path,
                );
                print_library_parse_diagnostics(&path, &first);
                failures.push(CorpusFailure::new(relative_path, CORPUS_SECOND_PASS, error));
                continue;
            }
        };

        if first != second {
            print_corpus_section("library formatter corpus: idempotence diff", relative_path);
            let diff_options = DiffOptions::new().with_path(relative_path.display().to_string());
            print_diff(&first, &second, &diff_options);
            failures.push(CorpusFailure::new(
                relative_path,
                CORPUS_IDEMPOTENCE_PASS,
                "second pass changed output",
            ));
        }
    }

    if !failures.is_empty() {
        print_corpus_summary(checked_file_count, &failures);

        return Err(format!(
            "library formatter corpus failed: {} of {checked_file_count} files",
            failures.len()
        ));
    }

    Ok(())
}
