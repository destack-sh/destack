use tspp_fir::format::{FormatError, FormatResult};
use tspp_repository::FormatterOptions;
use tspp_source::{File, FileId, FileType, Uri};

use crate::{TsppFormatOptions, format_file_source};

/// Resolve a fenced code block language to a source file type.
pub(super) fn fenced_code_file_type(lang: &str) -> Option<FileType> {
    let lang = lang.trim().split_ascii_whitespace().next()?;

    let file_type = match lang {
        "tspp" => FileType::Tspp,
        "d.tspp" => FileType::TsppDeclaration,
        _ => return None,
    };

    Some(file_type)
}

/// Format embedded TS++ code.
pub(super) fn format_embedded_code(
    code: &str,
    print_width: usize,
    format_options: &TsppFormatOptions,
    file_type: FileType,
) -> FormatResult<String> {
    let width = print_width.clamp(1, 320) as u16;
    let options = embedded_options(format_options, width);

    format_embedded_source(code, file_type, options)
}

/// Format a snippet as one complete source file.
fn format_embedded_source(
    source: &str,
    file_type: FileType,
    options: FormatterOptions,
) -> FormatResult<String> {
    let extension = file_type.extension().ok_or(FormatError::SyntaxError {
        message: "embedded documentation language has no file extension",
    })?;
    let name = format!("documentation.{extension}");
    let uri = format!("memory:///{name}");
    let file_id = FileId::from_logical_str(&uri);

    let file = File::from_text(
        file_id,
        name,
        Uri::from_string(uri),
        None,
        file_type,
        source.to_owned(),
    )
    .map_err(|_| FormatError::SyntaxError {
        message: "embedded TS++ documentation is too large",
    })?;
    let mut formatted =
        format_file_source(&file, file.text(), options).map_err(|_| FormatError::SyntaxError {
            message: "embedded TS++ documentation could not be formatted",
        })?;

    let trimmed_length = formatted.trim_end().len();
    formatted.truncate(trimmed_length);

    Ok(formatted)
}

/// Build formatter options for an embedded snippet.
fn embedded_options(format_options: &TsppFormatOptions, line_width: u16) -> FormatterOptions {
    FormatterOptions {
        line_ending: format_options.line_ending,
        indent_style: format_options.indent_style,
        indent_width: format_options.indent_width,
        line_width,
    }
}
