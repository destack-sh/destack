use std::error::Error;
use std::fmt::{self, Display};
use std::sync::Arc;

use destack_core::StringPool;
use destack_dir::{Expression, LocalNodeId, NodeParentIndex, TokenSpan, Tree};
use destack_fir::format as fir_format;
use destack_parser::{Parser, ParserOptions, ParserTriviaMode};
use destack_repository::FormatterOptions;
use destack_source::{DiagnosticCollection, DiagnosticSeverity, File, LanguageType, Span};

use crate::{DestackFormatContext, DestackFormatOptions, statement_list};

const MAX_PARSE_ERROR_MESSAGES: usize = 8;

/// One formatter failure over one whole source file.
#[derive(Debug, Clone)]
pub struct FormatFileError {
    /// The error message.
    pub message: String,
}

/// One formatted file payload.
#[derive(Debug, Clone)]
pub struct FormattedFile {
    /// The formatted source text.
    pub text: String,
    /// Diagnostics produced while parsing the source.
    pub diagnostics: DiagnosticCollection,
}

/// One formatted source edit.
#[derive(Debug, Clone)]
pub struct FormatEdit {
    /// The replacement source text.
    pub text: String,
    /// The file-local byte span replaced by the text.
    pub span: Span,
}

/// One formatted range payload.
#[derive(Debug, Clone)]
pub struct FormattedRange {
    /// The selected edit when formatting found an overlapping root.
    pub edit: Option<FormatEdit>,
    /// Diagnostics produced while parsing the source.
    pub diagnostics: DiagnosticCollection,
}

impl Display for FormatFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for FormatFileError {}

/// Format one parser DIR tree.
pub fn format_file_tree(
    file: &File,
    tree: &Tree,
    tokens: &[TokenSpan],
    side_tokens: &[TokenSpan],
    roots: &[LocalNodeId<Expression>],
    strings: &StringPool,
    options: FormatterOptions,
) -> Result<String, FormatFileError> {
    let side_span = tree.decorator_span();
    let language_type = LanguageType::try_from(file.ty).map_err(|_| FormatFileError {
        message: format!("formatter received non-code file type: {:?}", file.ty),
    })?;
    let parents = NodeParentIndex::from_expression_roots(tree, roots);
    let options = DestackFormatOptions::from_formatter_options(options, language_type);
    let context = DestackFormatContext::new(
        options,
        file,
        tree,
        tokens,
        side_tokens,
        &side_span,
        strings,
        &parents,
    );

    render_program_roots(context, roots)
}

/// Format one full source file from authored text.
pub fn format_file_source(
    file: &File,
    source: &str,
    options: FormatterOptions,
) -> Result<String, FormatFileError> {
    let formatted = format_source(file, source, options)?;
    if formatted
        .diagnostics
        .has_diagnostics_of_severity(DiagnosticSeverity::Error)
    {
        return Err(format_diagnostic_error(&formatted.diagnostics));
    }

    Ok(formatted.text)
}

/// Format one full source file from authored text and return parser diagnostics.
pub fn format_source(
    file: &File,
    source: &str,
    options: FormatterOptions,
) -> Result<FormattedFile, FormatFileError> {
    let language_type = LanguageType::try_from(file.ty).map_err(|_| FormatFileError {
        message: format!("formatter received non-code file type: {:?}", file.ty),
    })?;
    let parser_file = parser_file(file, source);
    let mut parser = source_parser(parser_file.clone(), language_type);
    let expressions = parser.parse();
    let diagnostics = parser.diagnostics();
    if diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
        return Ok(FormattedFile {
            text: source.to_owned(),
            diagnostics,
        });
    }

    // build the formatter context
    let (tokens, side_tokens) = parser.take_token_spans();
    let side_span = parser.tree.decorator_span();
    parser.tree.index_parents();
    let strings = parser.publish_strings();
    let options = DestackFormatOptions::from_formatter_options(options, language_type);
    let context = DestackFormatContext::new(
        options,
        parser_file.as_ref(),
        &parser.tree,
        &tokens,
        &side_tokens,
        &side_span,
        strings,
        parser.tree.parents(),
    );
    let text = render_program_roots(context, &expressions)?;

    Ok(FormattedFile { text, diagnostics })
}

/// Format one selected source range from authored text.
pub fn format_source_range(
    file: &File,
    source: &str,
    options: FormatterOptions,
    start: u32,
    end: u32,
) -> Result<FormattedRange, FormatFileError> {
    let language_type = LanguageType::try_from(file.ty).map_err(|_| FormatFileError {
        message: format!("formatter received non-code file type: {:?}", file.ty),
    })?;
    let parser_file = parser_file(file, source);
    let mut parser = source_parser(parser_file.clone(), language_type);
    let expressions = parser.parse();
    let diagnostics = parser.diagnostics();
    if diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
        return Ok(FormattedRange {
            edit: None,
            diagnostics,
        });
    }

    // find roots that overlap the selected byte range
    let overlapping = expressions
        .iter()
        .filter(|expression| {
            let span = parser.tree.get_span(**expression);
            span.start < end && span.end > start
        })
        .copied()
        .collect::<Vec<_>>();
    if overlapping.is_empty() {
        return Ok(FormattedRange {
            edit: None,
            diagnostics,
        });
    }

    // compute the exact replacement span
    let first_span = parser.tree.get_span(overlapping[0]);
    let last_index = overlapping.len() - 1;
    let last_span = parser.tree.get_span(overlapping[last_index]);
    let span = Span::new(file.id, first_span.start, last_span.end);
    let span = format_replacement_span(source, span);

    // build the formatter context
    let (tokens, side_tokens) = parser.take_token_spans();
    let side_span = parser.tree.decorator_span();
    parser.tree.index_parents();
    let strings = parser.publish_strings();
    let options = DestackFormatOptions::from_formatter_options(options, language_type);
    let context = DestackFormatContext::new(
        options,
        parser_file.as_ref(),
        &parser.tree,
        &tokens,
        &side_tokens,
        &side_span,
        strings,
        parser.tree.parents(),
    );
    let mut text = render_program_roots(context, &overlapping)?;

    // keep EOF range formatting newline terminated
    let is_at_end = last_span.end >= parser_file.len.saturating_sub(1);
    if is_at_end && !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }

    Ok(FormattedRange {
        edit: Some(FormatEdit { text, span }),
        diagnostics,
    })
}

/// Build a parser file from an input file and source text.
fn parser_file(file: &File, source: &str) -> Arc<File> {
    Arc::new(File::from_text(
        file.id,
        file.name.clone(),
        file.uri.clone(),
        file.path.clone(),
        file.ty,
        source.to_owned(),
    ))
}

/// Build a parser configured for source formatting.
fn source_parser(file: Arc<File>, language_type: LanguageType) -> Parser {
    Parser::lex_file_with_options(
        file,
        language_type,
        ParserOptions {
            trivia_mode: ParserTriviaMode::Full,
            retain_parentheses: false,
        },
        Arc::new(StringPool::new()),
    )
}

/// Convert parser diagnostics to a formatter error.
fn format_diagnostic_error(diagnostics: &DiagnosticCollection) -> FormatFileError {
    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        .collect::<Vec<_>>();
    let error_count = errors.len();
    let mut messages = errors
        .iter()
        .take(MAX_PARSE_ERROR_MESSAGES)
        .map(|diagnostic| diagnostic.message.clone())
        .collect::<Vec<_>>();

    if error_count > MAX_PARSE_ERROR_MESSAGES {
        messages.push(format!("... {error_count} total parse errors"));
    }

    FormatFileError {
        message: messages.join("\n"),
    }
}

/// Extend one formatted root span over its source statement tail.
fn format_replacement_span(source: &str, span: Span) -> Span {
    let mut end = span.end as usize;

    // consume same-line trivia before an optional semicolon
    end = consume_horizontal_whitespace(source, end);
    if source[end..].starts_with(';') {
        end += ';'.len_utf8();
    }

    // consume the line ending owned by the formatted root
    end = consume_one_line_ending(source, end);

    Span::new(span.file, span.start, end as u32)
}

/// Consume horizontal whitespace from one byte offset.
fn consume_horizontal_whitespace(source: &str, offset: usize) -> usize {
    let mut offset = offset;
    while let Some(current) = source[offset..].chars().next() {
        if !current.is_whitespace() || is_line_terminator(current) {
            break;
        }

        offset += current.len_utf8();
    }

    offset
}

/// Consume at most one source line ending from one byte offset.
fn consume_one_line_ending(source: &str, offset: usize) -> usize {
    let Some(current) = source[offset..].chars().next() else {
        return offset;
    };
    if !is_line_terminator(current) {
        return offset;
    }

    let offset = offset + current.len_utf8();
    if let Some(next) = source[offset..].chars().next()
        && ((current == '\r' && next == '\n') || (current == '\n' && next == '\r'))
    {
        return offset + next.len_utf8();
    }

    offset
}

/// Return whether one character is a line terminator.
fn is_line_terminator(current: char) -> bool {
    matches!(current, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// Render one parsed root list through the main formatter.
fn render_program_roots<'a>(
    context: DestackFormatContext<'a>,
    expressions: &'a [LocalNodeId<Expression>],
) -> Result<String, FormatFileError> {
    // format the parsed roots
    let formatted =
        fir_format!(context, [statement_list(expressions)]).map_err(|error| FormatFileError {
            message: error.to_string(),
        })?;
    let printed = formatted.print().map_err(|error| FormatFileError {
        message: error.to_string(),
    })?;
    let mut result = printed.as_str().to_string();

    // keep text outputs newline terminated
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::{format_file_source, format_source_range};
    use destack_repository::FormatterOptions;
    use destack_source::{File, FileId, FileType, Span, Uri};

    /// Source formatting should use the provided source text.
    #[test]
    fn test_format_file_source_uses_provided_source() {
        let file = File::from_text(
            FileId::new(1),
            "main.ds".to_string(),
            Uri::from_string("test:///main.ds"),
            None,
            FileType::Destack,
            "const stale=1".to_string(),
        );

        let formatted =
            format_file_source(&file, "const fresh=2", FormatterOptions::default()).unwrap();

        assert_eq!(
            formatted,
            r#"const fresh = 2;
"#
        );
    }

    /// Source formatting should preserve trailing file comments.
    #[test]
    fn test_format_file_source_preserves_eof_comments() {
        let file = File::from_text(
            FileId::new(1),
            "main.ds".to_string(),
            Uri::from_string("test:///main.ds"),
            None,
            FileType::Destack,
            String::new(),
        );

        let formatted = format_file_source(
            &file,
            r#"const value = 1;
// trailing
"#,
            FormatterOptions::default(),
        )
        .unwrap();

        assert_eq!(
            formatted,
            r#"const value = 1;
// trailing
"#
        );
    }

    /// Source formatting should preserve spacing in files containing only comments.
    #[test]
    fn test_format_file_source_preserves_comment_only_file_spacing() {
        let file = File::from_text(
            FileId::new(1),
            "main.ds".to_string(),
            Uri::from_string("test:///main.ds"),
            None,
            FileType::Destack,
            String::new(),
        );

        let formatted = format_file_source(
            &file,
            "// first\n\n/* second */\n/** third */\n",
            FormatterOptions::default(),
        )
        .unwrap();

        assert_eq!(formatted, "// first\n\n/* second */\n/** third */\n");
    }

    /// Range formatting should replace the selected source root.
    #[test]
    fn test_format_source_range_replaces_selected_root() {
        let file_id = FileId::new(1);
        let source = "const first=1;\nconst second=2;\n";
        let file = File::from_text(
            file_id,
            "main.ds".to_string(),
            Uri::from_string("test:///main.ds"),
            None,
            FileType::Destack,
            source.to_string(),
        );

        // select the second declaration
        let start = source.find("second").unwrap() as u32;
        let end = start + "second".len() as u32;
        let formatted =
            format_source_range(&file, source, FormatterOptions::default(), start, end).unwrap();
        let edit = formatted.edit.unwrap();

        assert_eq!(edit.span, Span::new(file_id, 15, 31));
        assert_eq!(
            edit.text,
            r#"const second = 2;
"#
        );

        // apply the edit to prove punctuation ownership
        let start = edit.span.start as usize;
        let end = edit.span.end as usize;
        let edited = format!("{}{}{}", &source[..start], edit.text, &source[end..]);
        assert_eq!(
            edited,
            r#"const first=1;
const second = 2;
"#
        );
    }
}
