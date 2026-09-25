use std::error::Error;
use std::fmt::{self, Display};
use std::sync::Arc;

use tspp_core::StringPool;
use tspp_dir::{Comment, Expression, LocalNodeId, NodeParentIndex, TokenSpan, Tree};
use tspp_fir::format as fir_format;
use tspp_fir::format::Allocator;
use tspp_parser::{CommentRetention, Parse, ParseOptions, Parser};
use tspp_repository::FormatterOptions;
use tspp_source::{
    DiagnosticCollection, DiagnosticSeverity, File, LanguageType, ModuleId, PackageId, Span,
};

use crate::context::is_line_terminator;
use crate::{TsppFormatContext, TsppFormatOptions, statement_list};

const MAX_PARSE_ERROR_MESSAGES: usize = 8;

/// Documentation diagnostics that never block formatting.
const DOCUMENTATION_DIAGNOSTIC_IDS: &[&str] = &[
    "invalid-documentation-owner",
    "missing-documentation-target",
    "duplicate-documentation-target",
];

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

/// One authored source file parsed for formatting.
struct ParsedFile {
    /// The language variant used to parse the file.
    language_type: LanguageType,
    /// The completed source parse.
    parse: Parse,
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
    comments: &[Comment],
    roots: &[LocalNodeId<Expression>],
    strings: &StringPool,
    options: FormatterOptions,
) -> Result<String, FormatFileError> {
    let side_span = tree.decorator_span();
    let language_type = LanguageType::try_from(file.ty).map_err(|_| FormatFileError {
        message: format!("formatter received non-code file type: {:?}", file.ty),
    })?;
    let parents = NodeParentIndex::from_roots(tree, roots);
    let options = TsppFormatOptions::from_formatter_options(options, language_type);
    let context = TsppFormatContext::new(
        options, file, tree, tokens, comments, &side_span, strings, &parents,
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
    if has_blocking_diagnostics(&formatted.diagnostics) {
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
    // parse the source and preserve blocking diagnostics
    let parsed = parse_file(file, source)?;
    let diagnostics = parsed.parse.diagnostics();
    if has_blocking_diagnostics(&diagnostics) {
        return Ok(FormattedFile {
            text: source.to_owned(),
            diagnostics,
        });
    }

    // render every parsed root
    let ParsedFile {
        language_type,
        mut parse,
    } = parsed;
    let roots = std::mem::take(&mut parse.roots);
    let text = render_parsed_roots(language_type, &mut parse, &roots, &roots, options)?;

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
    // parse the source and preserve blocking diagnostics
    let parsed = parse_file(file, source)?;
    let diagnostics = parsed.parse.diagnostics();
    if has_blocking_diagnostics(&diagnostics) {
        return Ok(FormattedRange {
            edit: None,
            diagnostics,
        });
    }

    // find roots that overlap the selected byte range
    let overlapping = parsed
        .parse
        .roots
        .iter()
        .filter(|expression| {
            let span = parsed.parse.tree.get_span(**expression);
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
    let first_span = parsed.parse.tree.get_span(overlapping[0]);
    let last_index = overlapping.len() - 1;
    let last_span = parsed.parse.tree.get_span(overlapping[last_index]);
    let span = Span::new(file.id, first_span.start, last_span.end);
    let span = format_replacement_span(source, span);

    // render the overlapping roots in complete source context
    let ParsedFile {
        language_type,
        mut parse,
    } = parsed;
    let roots = std::mem::take(&mut parse.roots);
    let mut text = render_parsed_roots(language_type, &mut parse, &roots, &overlapping, options)?;

    // keep EOF range formatting newline terminated
    let is_at_end = last_span.end >= file.len.saturating_sub(1);
    if is_at_end && !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }

    Ok(FormattedRange {
        edit: Some(FormatEdit { text, span }),
        diagnostics,
    })
}

/// Parse one authored source file for formatting.
fn parse_file(file: &File, source: &str) -> Result<ParsedFile, FormatFileError> {
    let language_type = LanguageType::try_from(file.ty).map_err(|_| FormatFileError {
        message: format!("formatter received non-code file type: {:?}", file.ty),
    })?;

    // create a standalone parse that retains every authored comment
    let file = parser_file(file, source)?;
    let module_id = ModuleId::new(PackageId::new(0), file.id.0);
    let tree = Tree::new(module_id);
    let parser = Parser::new(
        file,
        language_type,
        tree,
        ParseOptions {
            comment_retention: CommentRetention::All,
            ..ParseOptions::default()
        },
    );
    let parse = parser.parse();

    Ok(ParsedFile {
        language_type,
        parse,
    })
}

/// Render selected roots from one parsed source file.
fn render_parsed_roots(
    language_type: LanguageType,
    parse: &mut Parse,
    program_roots: &[LocalNodeId<Expression>],
    selected_roots: &[LocalNodeId<Expression>],
    options: FormatterOptions,
) -> Result<String, FormatFileError> {
    // materialize parser-owned formatting state
    let tokens = parse.take_token_spans();
    let side_span = parse.tree.decorator_span();
    parse.tree.index_parents(program_roots);
    let strings = StringPool::new();
    strings.extend(&parse.strings);

    // render the selected roots against the complete source context
    let options = TsppFormatOptions::from_formatter_options(options, language_type);
    let context = TsppFormatContext::new(
        options,
        &parse.file,
        &parse.tree,
        &tokens,
        &parse.comments,
        &side_span,
        &strings,
        parse.tree.parents(),
    );

    render_program_roots(context, selected_roots)
}

/// Build a parser file from an input file and source text.
fn parser_file(file: &File, source: &str) -> Result<Arc<File>, FormatFileError> {
    let file = File::from_text(
        file.id,
        file.name.clone(),
        file.uri.clone(),
        file.path.clone(),
        file.ty,
        source.to_owned(),
    )
    .map_err(|error| FormatFileError {
        message: error.to_string(),
    })?;

    Ok(Arc::new(file))
}

/// Return whether parser diagnostics block formatting.
fn has_blocking_diagnostics(diagnostics: &DiagnosticCollection) -> bool {
    diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == DiagnosticSeverity::Error
            && !DOCUMENTATION_DIAGNOSTIC_IDS.contains(&diagnostic.id.as_str())
    })
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

/// Render one parsed root list through the main formatter.
fn render_program_roots<'a>(
    context: TsppFormatContext<'a>,
    expressions: &'a [LocalNodeId<Expression>],
) -> Result<String, FormatFileError> {
    let allocator = Allocator::default();

    // format the parsed roots
    let formatted =
        fir_format!(&allocator, context, [statement_list(expressions)]).map_err(|error| {
            FormatFileError {
                message: error.to_string(),
            }
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
    use tspp_repository::FormatterOptions;
    use tspp_source::{File, FileId, FileType, Span, Uri};

    /// Source formatting should use the provided source text.
    #[test]
    fn test_format_file_source_uses_provided_source() {
        let file = File::from_text(
            FileId::new(1),
            "main.tspp".to_string(),
            Uri::from_string("test:///main.tspp"),
            None,
            FileType::Tspp,
            "const stale=1".to_string(),
        )
        .expect("test source should load");

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
            "main.tspp".to_string(),
            Uri::from_string("test:///main.tspp"),
            None,
            FileType::Tspp,
            String::new(),
        )
        .expect("test source should load");

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
            "main.tspp".to_string(),
            Uri::from_string("test:///main.tspp"),
            None,
            FileType::Tspp,
            String::new(),
        )
        .expect("test source should load");

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
            "main.tspp".to_string(),
            Uri::from_string("test:///main.tspp"),
            None,
            FileType::Tspp,
            source.to_string(),
        )
        .expect("test source should load");

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
