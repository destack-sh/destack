use crate::{TsppFormatContext, TsppFormatter};
use tspp_fir::format::{FormatResult, copied_text};
use tspp_fir::prelude::{empty_line, hard_line_break};
use tspp_fir::write;
use tspp_source::Span;

/// Return the raw line prefix before one byte offset.
pub(super) fn line_prefix_text<'a>(
    context: &'a TsppFormatContext<'a>,
    offset: u32,
) -> Option<&'a str> {
    let (line_index, column) = context.file.get_position(offset)?;
    let line_span = context.file.get_line_span(line_index)?;
    let line_text = context.span_str(line_span);

    line_text.get(..column as usize)
}

/// Return source text dedented relative to its containing line.
fn source_span_text(context: &TsppFormatContext<'_>, span: Span) -> String {
    let source = context.span_str(span);

    // retain source when its line position is unavailable
    if context.file.get_position(span.start).is_none() {
        return source.to_owned();
    }

    // retain inline source and source without a measurable prefix
    let Some(prefix) = line_prefix_text(context, span.start) else {
        return source.to_owned();
    };
    if prefix.is_empty() || !prefix.trim().is_empty() {
        return source.to_owned();
    }

    source
        .split('\n')
        .map(|line| line.strip_prefix(prefix).unwrap_or(line).to_owned())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Write one authored source span with formatter-managed indentation.
pub(crate) fn write_source_span<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    span: Span,
) -> FormatResult<()> {
    // source spans already contain their own comments
    f.context_mut()
        .comments_mut()
        .skip_comments_before(span.end);

    // normalize indentation relative to the formatter-owned position
    let source = source_span_text(f.context(), span);
    let source = if !f.context().span_starts_on_own_line(span) {
        dedent_common_leading_whitespace(source.as_str(), 1)
    } else {
        dedent_common_leading_whitespace(source.as_str(), 0)
    };
    let source = if source.trim().is_empty() {
        source
            .chars()
            .filter(|character| *character == '\n')
            .collect::<String>()
    } else {
        source
    };

    // transcribe source lines into formatter line elements
    let mut segment_start = 0usize;
    while segment_start < source.len() {
        let Some(relative_newline_index) = source[segment_start..].find('\n') else {
            write!(f, [copied_text(&source[segment_start..])])?;
            break;
        };

        let newline_index = segment_start + relative_newline_index;
        if segment_start < newline_index {
            write!(f, [copied_text(&source[segment_start..newline_index])])?;
        }

        let mut newline_run_end = newline_index;
        let bytes = source.as_bytes();
        while newline_run_end < bytes.len() && bytes[newline_run_end] == b'\n' {
            newline_run_end += 1;
        }

        // preserve empty lines represented by each newline run
        let mut newline_count = newline_run_end - newline_index;
        while newline_count >= 2 {
            write!(f, [empty_line()])?;
            newline_count -= 2;
        }
        if newline_count == 1 {
            write!(f, [hard_line_break()])?;
        }

        segment_start = newline_run_end;
    }

    Ok(())
}

/// Remove shared indentation beginning with one line index.
fn dedent_common_leading_whitespace(raw: &str, start_line: usize) -> String {
    // collect all non-empty lines that contribute indentation
    let lines = raw.lines().collect::<Vec<_>>();
    if lines.len() <= start_line {
        return raw.to_owned();
    }

    // find the common indentation prefix
    let mut common_prefix: Option<&str> = None;
    for line in lines.iter().skip(start_line) {
        if line.trim().is_empty() {
            continue;
        }

        let prefix_end = line
            .char_indices()
            .find_map(|(index, character)| {
                if character == ' ' || character == '\t' {
                    None
                } else {
                    Some(index)
                }
            })
            .unwrap_or(line.len());
        let prefix = &line[..prefix_end];

        match common_prefix {
            None => common_prefix = Some(prefix),
            Some(current_prefix) => {
                let mut shared_len = 0usize;
                let current_iter = current_prefix.chars();
                let mut next_iter = prefix.chars();
                for current_character in current_iter {
                    let Some(next_character) = next_iter.next() else {
                        break;
                    };
                    if current_character != next_character {
                        break;
                    }
                    shared_len += current_character.len_utf8();
                }
                common_prefix = Some(&current_prefix[..shared_len]);
            }
        }
    }

    let Some(common_prefix) = common_prefix else {
        return raw.to_owned();
    };
    if common_prefix.is_empty() {
        return raw.to_owned();
    }

    // strip the prefix from the selected line range
    lines
        .into_iter()
        .enumerate()
        .map(|(index, line)| {
            if index < start_line {
                line.to_owned()
            } else {
                line.strip_prefix(common_prefix).unwrap_or(line).to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
