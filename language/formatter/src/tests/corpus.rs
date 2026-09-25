use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tspp_dir::Tree;
use tspp_parser::{CommentRetention, ParseOptions, Parser, source_colorizer};
use tspp_repository::FormatterOptions;
use tspp_source::{
    DiffOptions, File, FileId, FileType, LanguageType, ModuleId, PackageId, PrintOptions, Uri,
    print_diagnostics, print_diff,
};

use crate::format_file_source;

const SECTION_SEPARATOR: &str =
    "================================================================================";
const CHECKED_IN_PASS: &str = "checked-in";
const SECOND_PASS: &str = "second pass";
const DRIFT_PASS: &str = "drift";
const IDEMPOTENCE_PASS: &str = "idempotence";

/// One library corpus failure.
#[derive(Debug)]
struct LibraryFailure {
    /// The library-relative path.
    path: String,
    /// The failed formatter pass.
    pass: &'static str,
    /// The failure detail.
    detail: String,
}

impl LibraryFailure {
    /// Create one failure.
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

/// Return the checked-in library corpus root.
fn library_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../library")
}

/// Collect every checked-in library `.tspp` source file.
fn library_sources(root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_library_sources(root, &mut paths);

    paths
}

/// Collect every checked-in library `.tspp` source file.
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
        else if path
            .extension()
            .is_some_and(|extension| extension == "tspp")
        {
            paths.push(path);
        }
    }
}

/// Return one path relative to the library root.
fn relative_library_path<'a>(root: &Path, path: &'a Path) -> &'a Path {
    path.strip_prefix(root)
        .expect("expected source below library root")
}

/// Build a formatter file for one library path.
///
/// The file identity derives from the library-relative logical path, while the
/// URI and physical path keep the absolute location for diagnostics.
fn library_file(path: &Path, logical_path: &Path, source: &str) -> File {
    let file_name = path
        .file_name()
        .expect("expected library file name")
        .to_string_lossy()
        .to_string();
    let path_text = path.to_string_lossy();

    File::from_text(
        FileId::from_logical_path(logical_path),
        file_name,
        Uri::from_string(path_text.as_ref()),
        Some(path.to_path_buf()),
        FileType::Tspp,
        source.to_string(),
    )
    .expect("library source should load")
}

/// Format one checked-in library source.
fn format_library_source(path: &Path, logical_path: &Path, source: &str) -> Result<String, String> {
    let file = library_file(path, logical_path, source);

    format_file_source(&file, source, FormatterOptions::default()).map_err(|error| error.message)
}

/// Print one corpus diagnostics section header.
fn print_section(title: &str, path: &Path) {
    eprintln!();
    eprintln!("{SECTION_SEPARATOR}");
    eprintln!("=== {title}");
    eprintln!("=== {}", path.display());
    eprintln!("{SECTION_SEPARATOR}");
    eprintln!();
}

/// Count failures for one pass.
fn failure_count(failures: &[LibraryFailure], pass: &str) -> usize {
    failures
        .iter()
        .filter(|failure| failure.pass == pass)
        .count()
}

/// Print one corpus failure summary.
fn print_summary(checked_file_count: usize, failures: &[LibraryFailure]) {
    print_section("library formatter corpus: summary", Path::new("."));

    let checked_in_failures = failure_count(failures, CHECKED_IN_PASS);
    let second_pass_failures = failure_count(failures, SECOND_PASS);
    let drift_failures = failure_count(failures, DRIFT_PASS);
    let idempotence_failures = failure_count(failures, IDEMPOTENCE_PASS);

    eprintln!(
        "checked {checked_file_count} files, found {} failures",
        failures.len()
    );
    eprintln!("checked-in parse failures: {checked_in_failures}");
    eprintln!("second pass parse failures: {second_pass_failures}");
    eprintln!("drift failures: {drift_failures}");
    eprintln!("idempotence failures: {idempotence_failures}");
    eprintln!();

    for failure in failures {
        eprintln!("{}", failure.summary());
    }
}

/// Print parser diagnostics for one library source.
fn print_parse_diagnostics(path: &Path, logical_path: &Path, source: &str) {
    let file = Arc::new(library_file(path, logical_path, source));
    let file_id = file.id;
    let file_for_id = |current_file_id| {
        if current_file_id == file_id {
            Some(file.clone())
        } else {
            None
        }
    };

    let module_id = ModuleId::new(PackageId::new(0), file.id.0);
    let parser = Parser::new(
        file.clone(),
        LanguageType::Tspp,
        Tree::new(module_id),
        ParseOptions {
            comment_retention: CommentRetention::All,
            ..ParseOptions::default()
        },
    );
    let parse = parser.parse();

    let diagnostics = parse.diagnostics();
    let options = PrintOptions::new().with_colorizer(source_colorizer());
    let _ = print_diagnostics(&file_for_id, &diagnostics, options);
}

/// Record one parse failure.
fn record_parse_failure(
    failures: &mut Vec<LibraryFailure>,
    relative_path: &Path,
    path: &Path,
    source: &str,
    pass: &'static str,
    error: String,
) {
    print_section("library formatter corpus: parse failure", relative_path);
    print_parse_diagnostics(path, relative_path, source);
    failures.push(LibraryFailure::new(relative_path, pass, error));
}

/// Record one diff failure.
fn record_diff_failure(
    failures: &mut Vec<LibraryFailure>,
    relative_path: &Path,
    pass: &'static str,
    left: &str,
    right: &str,
    detail: &'static str,
) {
    print_section("library formatter corpus: diff", relative_path);
    let diff_options = DiffOptions::new().with_path(relative_path.display().to_string());
    print_diff(left, right, &diff_options);
    failures.push(LibraryFailure::new(relative_path, pass, detail));
}

/// Assert formatter output matches the checked-in library corpus.
#[test]
fn test_format_library() -> Result<(), String> {
    let root = library_root();
    let paths = library_sources(&root);
    let checked_file_count = paths.len();
    let mut failures = Vec::new();

    for path in paths {
        let source = fs::read_to_string(&path).expect("expected library source");
        let relative_path = relative_library_path(&root, &path);

        // format checked-in source
        let formatted = match format_library_source(&path, relative_path, &source) {
            Ok(formatted) => formatted,
            Err(error) => {
                record_parse_failure(
                    &mut failures,
                    relative_path,
                    &path,
                    &source,
                    CHECKED_IN_PASS,
                    error,
                );
                continue;
            }
        };

        // rewrite the checked-in corpus when explicitly requested
        if std::env::var_os("TSPP_FORMAT_UPDATE").is_some() {
            if formatted != source {
                fs::write(&path, &formatted).expect("expected to write library source");
            }

            continue;
        }

        // compare checked-in source
        if formatted != source {
            record_diff_failure(
                &mut failures,
                relative_path,
                DRIFT_PASS,
                &source,
                &formatted,
                "formatter output differs from checked-in source",
            );
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        print_summary(checked_file_count, &failures);
        Err(format!(
            "library formatter corpus drifted: {} of {checked_file_count} files",
            failures.len()
        ))
    }
}

/// Assert formatter output reaches a fixed point over the library corpus.
#[test]
fn test_format_library_idempotence() -> Result<(), String> {
    let root = library_root();
    let paths = library_sources(&root);
    let checked_file_count = paths.len();
    let mut failures = Vec::new();

    for path in paths {
        let source = fs::read_to_string(&path).expect("expected library source");
        let relative_path = relative_library_path(&root, &path);

        // first pass
        let first = match format_library_source(&path, relative_path, &source) {
            Ok(first) => first,
            Err(error) => {
                record_parse_failure(
                    &mut failures,
                    relative_path,
                    &path,
                    &source,
                    CHECKED_IN_PASS,
                    error,
                );
                continue;
            }
        };

        // second pass
        let second = match format_library_source(&path, relative_path, &first) {
            Ok(second) => second,
            Err(error) => {
                record_parse_failure(
                    &mut failures,
                    relative_path,
                    &path,
                    &first,
                    SECOND_PASS,
                    error,
                );
                continue;
            }
        };

        // compare fixed point
        if first != second {
            record_diff_failure(
                &mut failures,
                relative_path,
                IDEMPOTENCE_PASS,
                &first,
                &second,
                "second pass changed output",
            );
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        print_summary(checked_file_count, &failures);
        Err(format!(
            "library formatter corpus failed: {} of {checked_file_count} files",
            failures.len()
        ))
    }
}
