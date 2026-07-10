use destack_repository::FormatterOptions;
use destack_source::{File, FileId, FileType, Uri};

use crate::{DestackFormatOptions, format_file_source};

use super::serialize::truncate_trim_end;

/// Resolve a fenced code block language to a source file type.
pub(super) fn fenced_code_file_type(lang: &str) -> Option<FileType> {
    let lang = lang.trim().split_ascii_whitespace().next()?;

    let file_type = match lang {
        "ds" | "destack" => FileType::Destack,
        "d.ds" | "destack-declaration" => FileType::DestackDeclaration,
        _ => return None,
    };

    Some(file_type)
}

/// Count unescaped backticks on a line and update template literal depth.
pub(super) fn update_template_depth(line: &str, mut depth: u32) -> u32 {
    let bytes = line.as_bytes();
    let mut index = 0;
    let mut expression_brace_depth: smallvec::SmallVec<[u32; 4]> = smallvec::SmallVec::new();

    while index < bytes.len() {
        // skip escaped characters
        if bytes[index] == b'\\' {
            index += 2;
            continue;
        }

        // update nested template depth
        if bytes[index] == b'`' {
            if expression_brace_depth.is_empty() {
                depth = if depth == 0 { depth + 1 } else { depth - 1 };
            } else {
                expression_brace_depth.push(0);
            }
        }
        // enter `${...}` while inside a template
        else if depth > 0 && bytes[index] == b'$' && bytes.get(index + 1) == Some(&b'{') {
            expression_brace_depth.push(0);
            index += 2;
            continue;
        }
        // track nested expression braces
        else if !expression_brace_depth.is_empty() && bytes[index] == b'{' {
            if let Some(last) = expression_brace_depth.last_mut() {
                *last += 1;
            }
        }
        // close `${...}` or one nested expression brace
        else if !expression_brace_depth.is_empty()
            && bytes[index] == b'}'
            && let Some(last) = expression_brace_depth.last_mut()
        {
            if *last == 0 {
                expression_brace_depth.pop();
            } else {
                *last -= 1;
            }
        }

        index += 1;
    }

    depth
}

/// Try to format embedded code using the surrounding language.
pub(super) fn format_embedded_code(
    code: &str,
    print_width: usize,
    format_options: &DestackFormatOptions,
    file_type: FileType,
) -> Option<String> {
    let width = print_width.clamp(1, 320) as u16;
    let base_options = embedded_options(format_options, width);
    let trimmed = code.trim();
    if starts_object_literal_example(trimmed) {
        return format_embedded_object_literal(trimmed, &base_options, file_type);
    }
    if trimmed.starts_with('{') {
        return None;
    }

    format_embedded_source(code, file_type, base_options)
}

/// Return whether brace-starting example code can be formatted as an object literal.
fn starts_object_literal_example(trimmed: &str) -> bool {
    let Some(rest) = trimmed.strip_prefix('{') else {
        return false;
    };

    // object members cannot begin with a second `{`
    let Some(first) = rest.trim_start().chars().next() else {
        return false;
    };

    matches!(
        first,
        '"' | '\'' | '[' | '.' | '_' | '$' | '0'..='9' | 'a'..='z' | 'A'..='Z'
    )
}

/// Try to format an object literal snippet.
fn format_embedded_object_literal(
    trimmed: &str,
    base_options: &FormatterOptions,
    file_type: FileType,
) -> Option<String> {
    // quoted keys need a json formatter to preserve spelling
    let has_quoted_keys = trimmed.contains('"') && trimmed.find('"') < trimmed.find('}');
    if has_quoted_keys {
        return None;
    }

    let wrapped = format!("({trimmed})");
    let formatted = format_embedded_source(&wrapped, file_type, *base_options)?;
    let formatted = formatted.trim_end();

    if let Some(inner) = formatted
        .strip_prefix('(')
        .and_then(|inner| inner.strip_suffix(");"))
    {
        let trimmed_inner = inner.trim();
        if !trimmed_inner.ends_with('}') {
            return None;
        }

        return Some(String::from(inner));
    }

    None
}

/// Format a snippet as one complete source file.
fn format_embedded_source(
    source: &str,
    file_type: FileType,
    options: FormatterOptions,
) -> Option<String> {
    let extension = file_type.extension()?;
    let name = format!("jsdoc.{extension}");
    let uri = format!("memory:///{name}");
    let file_id = FileId::from_logical_str(&uri);

    let file = File::from_text(
        file_id,
        name,
        Uri::from_string(uri),
        None,
        file_type,
        source.to_owned(),
    );
    let mut formatted = format_file_source(&file, file.text(), options).ok()?;

    truncate_trim_end(&mut formatted);

    Some(formatted)
}

/// Build formatter options for an embedded snippet.
fn embedded_options(format_options: &DestackFormatOptions, line_width: u16) -> FormatterOptions {
    FormatterOptions {
        line_ending: format_options.line_ending,
        indent_style: format_options.indent_style,
        indent_width: format_options.indent_width,
        line_width,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_fenced_code_file_type() {
        assert_eq!(fenced_code_file_type("ds"), Some(FileType::Destack));
        assert_eq!(
            fenced_code_file_type("d.ds"),
            Some(FileType::DestackDeclaration)
        );
        assert_eq!(fenced_code_file_type("ts"), None);
        assert_eq!(fenced_code_file_type("tsx"), None);
        assert_eq!(fenced_code_file_type("css"), None);
    }
}
