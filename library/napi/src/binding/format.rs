use std::path::{Path, PathBuf};
use std::sync::Arc;

use napi::Error;
use napi_derive::napi;
use {
    destack_ast as ast, destack_formatter as formatter, destack_parser as parser,
    destack_source as source,
};

use super::{IndentStyle, LineEnding};

/// Options for format operations.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct FormatOptions {
    /// The type of line ending to apply to the printed input.
    pub line_ending: LineEnding,
    /// The indent style.
    pub indent_style: IndentStyle,
    /// Spaces per indent.
    pub indent_width: u8,
    /// Maximum line length (best effort).
    pub line_width: u16,
}

impl Default for FormatOptions {
    fn default() -> Self {
        let options = formatter::DestackFormatOptions::default();
        Self {
            line_ending: match options.line_ending {
                source::LineEnding::LineFeed => LineEnding::LineFeed,
                source::LineEnding::CarriageReturnLineFeed => LineEnding::CarriageReturnLineFeed,
                source::LineEnding::CarriageReturn => LineEnding::CarriageReturn,
            },
            indent_style: match options.indent_style {
                source::IndentStyle::Tab => IndentStyle::Tab,
                source::IndentStyle::Space => IndentStyle::Space,
            },
            indent_width: options.indent_width,
            line_width: options.line_width,
        }
    }
}

/// Result payload for format operations.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct FormatResult {
    /// Formatted output code.
    pub code: String,
}

/// Get the default format options.
#[napi(js_name = "defaultFormatOptions")]
pub fn default_format_options() -> FormatOptions {
    FormatOptions::default()
}

/// Format a file synchronously.
#[napi(js_name = "formatSync")]
pub fn format_sync(
    path: String,
    content: String,
    options: Option<FormatOptions>,
) -> napi::Result<FormatResult> {
    let options = options.unwrap_or_default();
    let path = PathBuf::from(path);

    let file_type = format_file_type(&path)?;
    let language = source::LanguageType::from(file_type);
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "<format>".to_string());

    let file = source::File::from_text(
        source::FileId::new(0),
        file_name,
        source::Uri::from_path(&path),
        Some(path.clone()),
        file_type,
        content,
    );
    let file = Arc::new(file);

    let mut parser = parser::Parser::lex_file(file.clone(), language);
    let expressions: Vec<ast::LocalNodeId<ast::Expression>> = parser.parse();
    if !parser.errors.is_empty() {
        return Err(Error::from_reason(format!(
            "failed to format '{}': parser reported {} errors",
            path.display(),
            parser.errors.len()
        )));
    }

    let side_span: source::MultiSpan = parser.compute_side_span();
    let (tokens, side_tokens) = parser.take_tokens();
    let tree = parser.tree;
    let strings = parser.strings.into_immutable();

    let format_options = formatter::DestackFormatOptions::default()
        .with_line_ending(options.line_ending.into())
        .with_indent_style(options.indent_style.into())
        .with_indent_width(options.indent_width)
        .with_line_width(options.line_width);
    let context = formatter::DestackFormatContext::new(
        format_options,
        formatter::DestackFormatArtifacts {
            file: file.as_ref(),
            tree: &tree,
            tokens: &tokens,
            side_tokens: &side_tokens,
            side_span: &side_span,
            strings: &strings,
            parents: ast::NodeParentIndex::from_tree(&tree),
        },
    );

    let formatted = destack_fir::format!(context, [formatter::statement_list(&expressions)])
        .map_err(|error| {
            Error::from_reason(format!("failed to format '{}': {error}", path.display()))
        })?;
    let printed = formatted.print().map_err(|error| {
        Error::from_reason(format!("failed to print '{}': {error}", path.display()))
    })?;

    Ok(FormatResult {
        code: printed.as_str().to_string(),
    })
}

fn format_file_type(path: &Path) -> napi::Result<source::FileType> {
    let file_type = source::FileType::from_path(path).ok_or_else(|| {
        Error::from_reason(format!(
            "unsupported source file extension for '{}'",
            path.display()
        ))
    })?;

    match file_type {
        source::FileType::Destack
        | source::FileType::TypeScript
        | source::FileType::TypeScriptXml
        | source::FileType::JavaScript
        | source::FileType::JavaScriptXml => Ok(file_type),
        _ => Err(Error::from_reason(format!(
            "unsupported source file type '{file_type:?}' for '{}'",
            path.display()
        ))),
    }
}
