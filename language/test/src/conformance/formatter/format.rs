use std::path::Path;
use std::sync::Arc;

use destack_formatter::format_source;
use destack_parser::source_colorizer;
use destack_repository::FormatterOptions;
use destack_source::{
    DiagnosticSeverity, DiffOptions, File, FileId, FileType, PrintOptions, Uri, print_diff,
};

use crate::core::format_diagnostics;

use crate::conformance::CaseOutcome;

/// Run formatter conformance on a single source file.
pub(super) fn run_formatter_case(
    path: &Path,
    logical_path: &str,
    file_type: FileType,
    expected_output: Option<&str>,
    formatter_options: FormatterOptions,
    expect_error: bool,
    check_idempotence: bool,
    show_diff: bool,
) -> CaseOutcome {
    // read source content
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(_) => return CaseOutcome::FailedRead,
    };

    // format the source once
    let first_pass = match format_once(
        path,
        logical_path,
        &source,
        file_type,
        formatter_options,
        show_diff,
    ) {
        Ok(formatted) => formatted,
        Err(_) => {
            if expect_error {
                return CaseOutcome::Passed;
            }
            return CaseOutcome::FailedParse;
        }
    };

    // fail when error was expected but parsing succeeded
    if expect_error {
        return CaseOutcome::FailedParse;
    }

    // compare with expected output when available
    if let Some(expected_output) = expected_output {
        let expected_output = normalize_output(expected_output);
        let first_pass = normalize_output(&first_pass);
        if first_pass != expected_output {
            if show_diff {
                println!("diff for {}", path.display());
                print_diff(&expected_output, &first_pass, &DiffOptions::new());
            }
            return CaseOutcome::FailedOutput;
        }
    }

    // require idempotence after parity
    if check_idempotence {
        let second_pass = match format_once(
            path,
            logical_path,
            &first_pass,
            file_type,
            formatter_options,
            show_diff,
        ) {
            Ok(formatted) => formatted,
            Err(_) => return CaseOutcome::FailedIdempotence,
        };

        let first_pass = normalize_output(&first_pass);
        let second_pass = normalize_output(&second_pass);
        if first_pass == second_pass {
            CaseOutcome::Passed
        } else {
            if show_diff {
                println!("idempotence diff for {}", path.display());
                print_diff(&first_pass, &second_pass, &DiffOptions::new());
            }
            CaseOutcome::FailedIdempotence
        }
    }
    // no idempotence required
    else {
        CaseOutcome::Passed
    }
}

/// Format one source string as if it came from a file.
fn format_once(
    path: &Path,
    logical_path: &str,
    source: &str,
    file_type: FileType,
    formatter_options: FormatterOptions,
    show_diff: bool,
) -> Result<String, ()> {
    // materialize a source file in the in memory registry
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("input")
        .to_string();
    let uri = Uri::from_path(path);
    let file_id = FileId::from_logical_str(logical_path);
    let file = Arc::new(
        File::from_text(
            file_id,
            name,
            uri,
            Some(path.to_path_buf()),
            file_type,
            source.to_string(),
        )
        .expect("conformance source should load"),
    );
    let file_for_id = |current_file_id| {
        if current_file_id == file_id {
            Some(file.clone())
        } else {
            None
        }
    };

    // exercise the production whole-file formatter path
    let formatted = format_source(&file, source, formatter_options).map_err(|_| ())?;

    // fail on parser diagnostics
    let has_errors = formatted
        .diagnostics
        .to_vec()
        .into_iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error);
    if has_errors {
        if show_diff {
            let options = PrintOptions::new().with_colorizer(source_colorizer());
            let rendered = format_diagnostics(&file_for_id, &formatted.diagnostics, options);
            println!("parse diagnostics for {}:\n{rendered}", path.display());
        }
        return Err(());
    }

    Ok(formatted.text)
}

/// Build formatter options for JS/TS conformance baselines.
pub(super) fn default_conformance_formatter_options() -> FormatterOptions {
    FormatterOptions::default()
        .with_indent_width(2)
        .with_line_width(80)
}

/// Normalize output before comparisons.
fn normalize_output(content: &str) -> String {
    // strip trailing whitespace and normalize final newline
    let lines: Vec<&str> = content.lines().map(|line| line.trim_end()).collect();
    let mut result = lines.join("\n");

    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }
    result
}
