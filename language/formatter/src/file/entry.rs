use std::error::Error;
use std::fmt::{self, Display};
use std::sync::Arc;

use destack_core::StringPool;
use destack_dir::{Expression, LocalNodeId, NodeParentIndex, TokenSpan, Tree};
use destack_fir::format as fir_format;
use destack_parser::{Parser, ParserOptions, ParserTriviaMode};
use destack_repository::FormatterOptions;
use destack_source::{File, LanguageType};

use crate::{DestackFormatContext, DestackFormatOptions, statement_list};

const MAX_PARSE_ERROR_MESSAGES: usize = 8;

/// One formatter failure over one whole source file.
#[derive(Debug, Clone)]
pub struct FormatFileError {
    /// The error message.
    pub message: String,
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
    let side_span = Parser::compute_side_span_from_tree(tree);
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
        parents,
    );

    render_program_roots(&context, roots)
}

/// Format one full source file from authored text.
pub fn format_file_source(
    file: &File,
    source: &str,
    options: FormatterOptions,
) -> Result<String, FormatFileError> {
    // parse the source file
    let language_type = LanguageType::try_from(file.ty).map_err(|_| FormatFileError {
        message: format!("formatter received non-code file type: {:?}", file.ty),
    })?;
    // parse with side tokens for formatting
    let parser_file = Arc::new(File::from_text(
        file.id,
        file.name.clone(),
        file.uri.clone(),
        file.path.clone(),
        file.ty,
        source.to_owned(),
    ));
    let mut parser = Parser::lex_file_with_options(
        parser_file.clone(),
        language_type,
        ParserOptions {
            trivia_mode: ParserTriviaMode::Full,
            preserve_parenthesized_wrappers: false,
            ..ParserOptions::default()
        },
        Arc::new(StringPool::new()),
    );
    let expressions = parser.parse();
    fail_on_parse_errors(&parser)?;

    // finalize retained comments before formatting
    parser.attach_comments();

    // build the formatter context
    let (tokens, side_tokens) = parser.take_token_spans();
    let side_span = parser.compute_side_span();
    let strings = parser.strings.as_ref();
    let parents = NodeParentIndex::from_expression_roots(&parser.tree, &expressions);
    let options = DestackFormatOptions::from_formatter_options(options, language_type);
    let context = DestackFormatContext::new(
        options,
        parser_file.as_ref(),
        &parser.tree,
        &tokens,
        &side_tokens,
        &side_span,
        strings,
        parents,
    );

    render_program_roots(&context, &expressions)
}

/// Return an error if the parser produced parse errors.
fn fail_on_parse_errors(parser: &Parser) -> Result<(), FormatFileError> {
    let errors = &parser.errors;
    if !errors.is_empty() {
        let error_count = errors.len();
        let mut messages = errors
            .iter()
            .take(MAX_PARSE_ERROR_MESSAGES)
            .map(|error| parser.diagnostic(error).message)
            .collect::<Vec<_>>();

        if error_count > MAX_PARSE_ERROR_MESSAGES {
            messages.push(format!("... {error_count} total parse errors"));
        }

        let message = messages.join("\n");

        return Err(FormatFileError { message });
    }

    Ok(())
}

/// Render one parsed root list through the main formatter.
fn render_program_roots<'a>(
    context: &DestackFormatContext<'a>,
    expressions: &'a [LocalNodeId<Expression>],
) -> Result<String, FormatFileError> {
    // format the parsed roots
    let formatted =
        fir_format!(context.clone(), [statement_list(expressions)]).map_err(|error| {
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
    use super::format_file_source;
    use destack_repository::FormatterOptions;
    use destack_source::{File, FileId, FileType, Uri};

    /// Source formatting should use the provided source text.
    #[test]
    fn test_format_file_source_uses_provided_source() {
        let file = File::from_text(
            FileId::new(1),
            "main.ts".to_string(),
            Uri::from_string("test:///main.ts"),
            None,
            FileType::TypeScript,
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
            "main.ts".to_string(),
            Uri::from_string("test:///main.ts"),
            None,
            FileType::TypeScript,
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
}
