use std::path::Path;
use std::sync::Arc;

use destack_ast::{NodeParentIndex, TokenSpan};
use destack_fir::format as fir_format;
use destack_formatter::{DestackFormatContext, DestackFormatOptions, statement_list};
use destack_parser::{Parser, ParserOptions, source_colorizer};
use destack_source::{
    DiagnosticCollection, DiagnosticSeverity, DiffOptions, File, FileId, FileType, LanguageType,
    PrintOptions, Uri, print_diff,
};
use destack_workspace::{FormatterOptions, QuoteStyle};

use crate::core::format_diagnostics;

use crate::conformance::CaseOutcome;

/// Run formatter conformance on a single source file.
pub(super) fn run_formatter_case(
    path: &Path,
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
    let first_pass = match format_once(path, &source, file_type, formatter_options, show_diff) {
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
        let second_pass =
            match format_once(path, &first_pass, file_type, formatter_options, show_diff) {
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
    let file_id = FileId::from_logical_path(path);
    let file = Arc::new(File::from_text(
        file_id,
        name,
        uri,
        Some(path.to_path_buf()),
        file_type,
        source.to_string(),
    ));
    let file_for_id = |current_file_id| {
        if current_file_id == file_id {
            Some(file.clone())
        } else {
            None
        }
    };

    // parse source and fail on syntax errors
    let language_type =
        LanguageType::try_from(file_type).expect("file type has no parser language");
    let mut parser = Parser::lex_file_with_options(
        file.clone(),
        language_type,
        ParserOptions {
            preserve_parenthesized_wrappers: false,
            ..ParserOptions::default()
        },
    );
    let expressions = parser.parse();

    let has_errors = parser
        .diagnostics
        .to_vec()
        .into_iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error);
    if has_errors {
        if show_diff {
            let mut diagnostics = DiagnosticCollection::new();
            for diagnostic in parser.diagnostics.to_vec() {
                diagnostics.insert(diagnostic);
            }
            let options = PrintOptions::new().with_colorizer(source_colorizer());
            let rendered = format_diagnostics(&file_for_id, &diagnostics, options);
            println!("parse diagnostics for {}:\n{rendered}", path.display());
        }
        return Err(());
    }

    // format parsed expressions
    let (tokens, side_tokens) = parser.take_tokens();
    Ok(format_expressions(
        &parser,
        &tokens,
        &side_tokens,
        &expressions,
        &file,
        language_type,
        formatter_options,
    ))
}

/// Build formatter options for JS/TS conformance baselines.
pub(super) fn default_conformance_formatter_options() -> FormatterOptions {
    FormatterOptions::default()
        .with_indent_width(2)
        .with_line_width(80)
        .with_quote_style(QuoteStyle::Double)
}

/// Format parsed expressions into source output.
fn format_expressions(
    parser: &Parser,
    tokens: &Vec<TokenSpan>,
    side_tokens: &Vec<TokenSpan>,
    expressions: &[destack_ast::LocalNodeId<destack_ast::Expression>],
    file: &File,
    language_type: LanguageType,
    formatter: FormatterOptions,
) -> String {
    // build formatter context
    let side_span = parser.compute_side_span();
    let strings = parser.strings.clone().into_immutable();
    let parents = NodeParentIndex::from_expression_roots(&parser.tree, expressions);
    let format_options = DestackFormatOptions::from_formatter_options(formatter, language_type);
    let context = DestackFormatContext::new(
        format_options,
        file,
        &parser.tree,
        tokens,
        side_tokens,
        &side_span,
        &strings,
        parents,
    );

    // format statements
    let mut result = if expressions.is_empty() {
        String::new()
    } else {
        let formatted = fir_format!(context.clone(), [statement_list(expressions)]).unwrap();
        let printed = formatted.print().unwrap();
        printed.as_str().to_string()
    };

    // normalize missing final newline
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }

    result
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
