use std::path::Path;
use std::sync::Arc;

use destack_ast::{NodeParentIndex, TokenSpan};
use destack_fir::format as fir_format;
use destack_formatter::{DestackFormatContext, DestackFormatOptions, statement_list};
use destack_parser::{Parser, source_colorizer};
use destack_source::{
    DiagnosticCollection, DiagnosticSeverity, DiffOptions, File, FileRegistry, FileSystem,
    FileType, LanguageType, MemoryFileSystem, PrintOptions, Uri, print_diff,
};
use destack_workspace::{FormatterOptions, LinterOptions, Program, QuoteStyle};

use crate::harness::format_diagnostics;

use super::runner::TestOutcome;

/// Run formatter conformance on a single source file.
pub(super) fn run_formatter_case(
    path: &Path,
    file_type: FileType,
    expected_output: Option<&str>,
    formatter_options: FormatterOptions,
    expect_error: bool,
    show_diff: bool,
) -> TestOutcome {
    // read source content
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(_) => return TestOutcome::FailedRead,
    };

    // format the source once
    let first_pass = match format_once(path, &source, file_type, formatter_options, show_diff) {
        Ok(formatted) => formatted,
        Err(_) => {
            if expect_error {
                return TestOutcome::Passed;
            }
            return TestOutcome::FailedParse;
        }
    };

    // fail when error was expected but parsing succeeded
    if expect_error {
        return TestOutcome::FailedParse;
    }

    // compare with expected output when available
    if let Some(expected_output) = expected_output {
        let expected_output = normalize_output(expected_output);
        let first_pass = normalize_output(&first_pass);
        return if first_pass == expected_output {
            TestOutcome::Passed
        } else {
            if show_diff {
                println!("diff for {}", path.display());
                print_diff(&expected_output, &first_pass, &DiffOptions::new());
            }
            TestOutcome::FailedOutput
        };
    }

    // otherwise require idempotence
    let second_pass = match format_once(path, &first_pass, file_type, formatter_options, show_diff)
    {
        Ok(formatted) => formatted,
        Err(_) => return TestOutcome::FailedParse,
    };

    let first_pass = normalize_output(&first_pass);
    let second_pass = normalize_output(&second_pass);
    if first_pass == second_pass {
        TestOutcome::Passed
    } else {
        if show_diff {
            println!("idempotence diff for {}", path.display());
            print_diff(&first_pass, &second_pass, &DiffOptions::new());
        }
        TestOutcome::FailedIdempotence
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
    // set up a minimal program for parser and formatter execution
    let cwd = path
        .parent()
        .map_or_else(|| Path::new("/").to_path_buf(), |path| path.to_path_buf());
    let files = Arc::new(FileRegistry::new());
    let fs: Arc<dyn FileSystem> = Arc::new(MemoryFileSystem::new());
    let program = Arc::new(Program::from_options(
        formatter_options,
        LinterOptions::default(),
        cwd,
        fs,
        files,
    ));

    // materialize a source file in the in memory registry
    let file_id = program.files.next_id();
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("input")
        .to_string();
    let uri = Uri::from_path(path);
    let file = Arc::new(File::from_text(
        file_id,
        name,
        uri,
        Some(path.to_path_buf()),
        file_type,
        source.to_string(),
    ));
    program.files.insert((*file).clone());

    // parse source and fail on syntax errors
    let language_type = LanguageType::from(file_type);
    let mut parser = Parser::lex_file(file.clone(), language_type);
    let expressions = parser.parse();
    parser.finish();

    let has_errors = parser
        .diagnostics
        .iter()
        .into_iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error);
    if has_errors {
        if show_diff {
            let mut diagnostics = DiagnosticCollection::new();
            for diagnostic in parser.diagnostics.iter() {
                diagnostics.insert(diagnostic);
            }
            let options = PrintOptions::new().with_colorizer(source_colorizer());
            let rendered = format_diagnostics(&program.files, &diagnostics, options);
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
        program.formatter,
    ))
}

/// Build formatter options for JS and TS conformance baselines.
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
    let parents = NodeParentIndex::from_tree(&parser.tree);
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
