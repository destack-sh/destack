use std::error::Error;
use std::fmt::{self, Display};
use std::sync::Arc;

use destack_ast::{Expression, LocalNodeId, NodeParentIndex};
use destack_css::{CssFormatOptions, format_stylesheet, parse_css};
use destack_fir::format as fir_format;
use destack_html::{HtmlFormatOptions, format_document, parse_html};
use destack_parser::Parser;
use destack_source::{DiagnosticSeverity, File, FileType, LanguageType};
use destack_workspace::FormatterOptions;

use crate::{DestackFormatContext, DestackFormatOptions, statement_list};

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

/// Format one full source file from authored text.
pub fn format_file_source(
    file: &File,
    source: &str,
    options: FormatterOptions,
) -> Result<String, FormatFileError> {
    match file.ty {
        FileType::Css => format_css_file_source(file, source, options),
        FileType::Html => format_html_file_source(file, source, options),
        _ => format_parser_file_source(file, options),
    }
}

/// Format one CSS source file.
fn format_css_file_source(
    file: &File,
    source: &str,
    options: FormatterOptions,
) -> Result<String, FormatFileError> {
    // parse and format
    let (tree, stylesheet) = parse_css(file, source).map_err(|error| FormatFileError {
        message: error.message,
    })?;
    let options = css_format_options(options);
    let mut formatted =
        format_stylesheet(&tree, stylesheet, options).map_err(|error| FormatFileError {
            message: error.to_string(),
        })?;

    // keep text outputs newline terminated
    if !formatted.is_empty() && !formatted.ends_with('\n') {
        formatted.push('\n');
    }

    Ok(formatted)
}

/// Format one HTML source file.
fn format_html_file_source(
    file: &File,
    source: &str,
    options: FormatterOptions,
) -> Result<String, FormatFileError> {
    // parse and format
    let (tree, document) = parse_html(file, source);
    let options = html_format_options(options);
    let mut formatted =
        format_document(&tree, document, options).map_err(|error| FormatFileError {
            message: error.to_string(),
        })?;

    // keep text outputs newline terminated
    if !formatted.is_empty() && !formatted.ends_with('\n') {
        formatted.push('\n');
    }

    Ok(formatted)
}

/// Format one parser-driven source file.
fn format_parser_file_source(
    file: &File,
    options: FormatterOptions,
) -> Result<String, FormatFileError> {
    // parse the source file
    let language_type = LanguageType::from(file.ty);
    let mut parser = Parser::lex_file(Arc::new(file.clone()), language_type);
    let expressions = parser.parse();

    // fail loudly on parse errors
    if parser
        .diagnostics
        .has_diagnostics_of_severity(DiagnosticSeverity::Error)
    {
        let message = parser
            .diagnostics
            .iter()
            .into_iter()
            .map(|diagnostic| diagnostic.message.clone())
            .collect::<Vec<_>>()
            .join("\n");

        return Err(FormatFileError { message });
    }

    // build the formatter context
    let (tokens, side_tokens) = parser.take_tokens();
    let side_span = parser.compute_side_span();
    let strings = parser.strings.into_immutable();
    let parents = NodeParentIndex::from_tree(&parser.tree);
    let options = DestackFormatOptions::from_formatter_options(options, language_type);
    let context = DestackFormatContext::new(
        options,
        file,
        &parser.tree,
        &tokens,
        &side_tokens,
        &side_span,
        &strings,
        parents,
    );

    render_program_roots(&context, &expressions)
}

/// Render one parsed root list through the main formatter.
fn render_program_roots<'a>(
    context: &DestackFormatContext<'a>,
    expressions: &'a [LocalNodeId<Expression>],
) -> Result<String, FormatFileError> {
    // format the parsed roots
    let mut result = if expressions.is_empty() {
        String::new()
    } else {
        let formatted =
            fir_format!(context.clone(), [statement_list(expressions)]).map_err(|error| {
                FormatFileError {
                    message: error.to_string(),
                }
            })?;
        let printed = formatted.print().map_err(|error| FormatFileError {
            message: error.to_string(),
        })?;

        printed.as_str().to_string()
    };

    // keep text outputs newline terminated
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }

    Ok(result)
}

/// Convert workspace options into CSS format options.
fn css_format_options(options: FormatterOptions) -> CssFormatOptions {
    CssFormatOptions::pretty()
        .with_line_ending(options.line_ending)
        .with_indent_style(options.indent_style)
        .with_indent_width(options.indent_width)
        .with_line_width(options.line_width.min(u8::MAX as u16) as u8)
}

/// Convert workspace options into HTML format options.
fn html_format_options(options: FormatterOptions) -> HtmlFormatOptions {
    HtmlFormatOptions::pretty()
        .with_line_ending(options.line_ending)
        .with_indent_style(options.indent_style)
        .with_indent_width(options.indent_width)
        .with_line_width(options.line_width.min(u8::MAX as u16) as u8)
}

#[cfg(test)]
mod tests {
    use super::format_file_source;
    use destack_source::{File, FileId, FileType, Uri};
    use destack_workspace::FormatterOptions;

    /// Format one CSS file through the shared formatter entrypoint.
    #[test]
    fn test_format_css_file_source() {
        let file = File::from_text(
            FileId::new(1),
            "style.css".to_string(),
            Uri::from_string("test:///style.css"),
            None,
            FileType::Css,
            "@media screen{.button{color:red;background:blue}}".to_string(),
        );

        let formatted =
            format_file_source(&file, file.text(), FormatterOptions::default()).unwrap();

        assert_eq!(
            formatted,
            "@media screen {\n    .button {\n        color: red;\n        background: blue;\n    }\n}\n"
        );
    }

    /// Format one HTML file through the shared formatter entrypoint.
    #[test]
    fn test_format_html_file_source() {
        let file = File::from_text(
            FileId::new(1),
            "index.html".to_string(),
            Uri::from_string("test:///index.html"),
            None,
            FileType::Html,
            "<div><span>hello</span><p>world</p></div>".to_string(),
        );

        let formatted =
            format_file_source(&file, file.text(), FormatterOptions::default()).unwrap();

        assert_eq!(
            formatted,
            "<div>\n    <span>hello</span>\n    <p>world</p>\n</div>\n"
        );
    }
}
