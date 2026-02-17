#[cfg(feature = "format-api")]
use std::path::{Path, PathBuf};
#[cfg(feature = "format-api")]
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
#[cfg(feature = "format-api")]
use {
    destack_ast as ast, destack_formatter as formatter, destack_parser as parser,
    destack_source as source,
};

#[cfg(feature = "format-api")]
use super::error::parse_optional_input;
use super::error::{js_error, to_js_value};
use super::{IndentStyle, LineEnding};

/// Options for format operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

#[cfg(feature = "format-api")]
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

#[cfg(not(feature = "format-api"))]
impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            line_ending: LineEnding::LineFeed,
            indent_style: IndentStyle::Space,
            indent_width: 2,
            line_width: 80,
        }
    }
}

/// Result payload for format operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatResult {
    /// Formatted output code.
    pub code: String,
}

/// Get the default format options.
#[wasm_bindgen(js_name = defaultFormatOptions)]
pub fn default_format_options() -> Result<JsValue, JsValue> {
    to_js_value(&FormatOptions::default())
}

/// Format a file synchronously.
#[wasm_bindgen(js_name = formatSync)]
pub fn format_sync(
    path: String,
    content: String,
    options: Option<JsValue>,
) -> Result<JsValue, JsValue> {
    // format api enabled
    #[cfg(feature = "format-api")]
    {
        // decode options and normalize source path
        let options: FormatOptions = parse_optional_input(options)?;
        let path = PathBuf::from(path);

        // build a source file for parser and formatter
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

        // parse the file and fail fast on parser errors
        let mut parser = parser::Parser::lex_file(file.clone(), language);
        let expressions: Vec<ast::LocalNodeId<ast::Expression>> = parser.parse();
        if !parser.errors.is_empty() {
            return Err(js_error(format!(
                "failed to format '{}': parser reported {} errors",
                path.display(),
                parser.errors.len()
            )));
        }

        // collect parser artifacts needed by formatter
        let side_span: source::MultiSpan = parser.compute_side_span();
        let (tokens, side_tokens) = parser.take_tokens();
        let tree = parser.tree;
        let strings = parser.strings.into_immutable();

        // create the formatter context
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

        // render and print the formatted output
        let formatted = destack_fir::format!(context, [formatter::statement_list(&expressions)])
            .map_err(|error| js_error(format!("failed to format '{}': {error}", path.display())))?;
        let printed = formatted
            .print()
            .map_err(|error| js_error(format!("failed to print '{}': {error}", path.display())))?;

        // encode the format response payload
        return to_js_value(&FormatResult {
            code: printed.as_str().to_string(),
        });
    }

    // format api disabled
    #[cfg(not(feature = "format-api"))]
    {
        let _ = (path, content, options);
        Err(js_error(format_feature_disabled_message()))
    }
}

/// Parse and validate a file type for formatting.
#[cfg(feature = "format-api")]
fn format_file_type(path: &Path) -> Result<source::FileType, JsValue> {
    let file_type = source::FileType::from_path(path).ok_or_else(|| {
        js_error(format!(
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
        _ => Err(js_error(format!(
            "unsupported source file type '{file_type:?}' for '{}'",
            path.display()
        ))),
    }
}

/// Return a stable error message for format api disabled builds.
#[cfg(not(feature = "format-api"))]
fn format_feature_disabled_message() -> &'static str {
    "format APIs are disabled in this wasm build: enable the 'format-api' cargo feature"
}
