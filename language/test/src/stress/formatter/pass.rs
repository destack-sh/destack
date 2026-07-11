use std::sync::Arc;
use std::time::{Duration, Instant};

use destack_core::StringPool;
use destack_dir::{Expression, LocalNodeId};
use destack_fir::format as fir_format;
use destack_formatter::{DestackFormatContext, DestackFormatOptions, statement_list};
use destack_parser::{Parser, ParserOptions, ParserTriviaMode};
use destack_repository::FormatterOptions;
use destack_source::{DiagnosticCollection, DiagnosticSeverity, File, FileId, LanguageType, Uri};

use super::document::FormatterDocumentStats;
use super::timing::FormatterTiming;
use crate::stress::{StressExpectation, StressFixture};

const BOUNDED_OUTPUT_MAX_BYTES: usize = 64 * 1024 * 1024;

/// One formatted stress pass with phase timings.
#[derive(Debug)]
pub(super) struct FormatterPass {
    /// The formatted output.
    pub(super) output: String,
    /// The phase timings for this pass.
    pub(super) timing: FormatterTiming,
}

/// Format one stress source while measuring formatter phases.
pub(super) fn format_pass(
    fixture: &StressFixture,
    source: &str,
    options: FormatterOptions,
) -> Result<FormatterPass, String> {
    let language_type = LanguageType::try_from(fixture.file_type).map_err(|_| {
        format!(
            "formatter received non-code file type: {:?}",
            fixture.file_type
        )
    })?;

    // parse source
    let parse_start = Instant::now();
    let file = Arc::new(stress_file(fixture, source)?);
    let mut parser = Parser::lex_file_with_options(
        file.clone(),
        language_type,
        ParserOptions {
            trivia_mode: ParserTriviaMode::Full,
            retain_parentheses: false,
        },
        Arc::new(StringPool::new()),
    );
    let expressions = parser.parse();
    let parse_elapsed = parse_start.elapsed();

    // build diagnostics
    let diagnostics_start = Instant::now();
    let diagnostics = parser.diagnostics();
    let diagnostics_elapsed = diagnostics_start.elapsed();
    if diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
        return Err(format_parse_error(&diagnostics));
    }

    // materialize token spans and side spans
    let token_spans_start = Instant::now();
    let (tokens, side_tokens) = parser.take_token_spans();
    let side_span = parser.tree.decorator_span();
    let token_spans_elapsed = token_spans_start.elapsed();

    // build parent index
    let parents_start = Instant::now();
    parser.tree.index_parents();
    let parents_elapsed = parents_start.elapsed();

    // convert formatter options
    let options_start = Instant::now();
    let format_options = DestackFormatOptions::from_formatter_options(options, language_type);
    let options_elapsed = options_start.elapsed();

    // publish parser-local strings
    let strings_start = Instant::now();
    let strings = parser.publish_strings();
    let strings_elapsed = strings_start.elapsed();

    // build formatter context
    let context_start = Instant::now();
    let context = DestackFormatContext::new(
        format_options,
        file.as_ref(),
        &parser.tree,
        &tokens,
        &side_tokens,
        &side_span,
        strings,
        parser.tree.parents(),
    );
    let context_elapsed = context_start.elapsed();

    let max_output_bytes =
        (fixture.expectation == StressExpectation::Bounded).then_some(BOUNDED_OUTPUT_MAX_BYTES);
    let (output, format_elapsed, print_elapsed, document) =
        render_profiled(context, &expressions, max_output_bytes)?;
    let timing = FormatterTiming {
        parse: parse_elapsed,
        diagnostics: diagnostics_elapsed,
        strings: strings_elapsed,
        token_spans: token_spans_elapsed,
        parents: parents_elapsed,
        options: options_elapsed,
        context: context_elapsed,
        format: format_elapsed,
        print: print_elapsed,
        document,
    };

    Ok(FormatterPass { output, timing })
}

/// Render one parsed source while separating document build from printing.
fn render_profiled<'a>(
    context: DestackFormatContext<'a>,
    expressions: &'a [LocalNodeId<Expression>],
    max_output_bytes: Option<usize>,
) -> Result<(String, Duration, Duration, FormatterDocumentStats), String> {
    // build formatter document
    let format_start = Instant::now();
    let formatted =
        fir_format!(context, [statement_list(expressions)]).map_err(|error| error.to_string())?;
    let format_elapsed = format_start.elapsed();
    let document = FormatterDocumentStats::from_nodes(formatted.document());

    // print formatter document
    let print_start = Instant::now();
    let mut print_options = formatted.context().options.print_options();
    if let Some(max_output_bytes) = max_output_bytes {
        let max_output_bytes = u32::try_from(max_output_bytes).map_err(|_| {
            format!("output byte limit {max_output_bytes} exceeds FIR marker range")
        })?;
        print_options = print_options.with_max_output_bytes(max_output_bytes);
    }
    let printed = formatted
        .print_with_options(print_options)
        .map_err(|error| error.to_string())?;
    let mut output = printed.as_str().to_string();
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }
    let print_elapsed = print_start.elapsed();

    Ok((output, format_elapsed, print_elapsed, document))
}

/// Convert parser diagnostics into a compact stress failure message.
fn format_parse_error(diagnostics: &DiagnosticCollection) -> String {
    const MAX_MESSAGES: usize = 8;

    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        .collect::<Vec<_>>();
    let error_count = errors.len();
    let mut messages = errors
        .iter()
        .take(MAX_MESSAGES)
        .map(|diagnostic| diagnostic.message.clone())
        .collect::<Vec<_>>();

    if error_count > MAX_MESSAGES {
        messages.push(format!("... {error_count} total parse errors"));
    }

    messages.join("\n")
}

/// Create the source file used for one formatter stress pass.
fn stress_file(fixture: &StressFixture, source: &str) -> Result<File, String> {
    let file_name = fixture.file_name()?;
    let file_id = FileId::from_logical_path(&fixture.logical_path());

    Ok(File::from_text(
        file_id,
        file_name.clone(),
        Uri::from_string(&file_name),
        None,
        fixture.file_type,
        source.to_string(),
    ))
}
