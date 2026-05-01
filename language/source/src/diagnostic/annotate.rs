#![allow(clippy::too_many_arguments)]

use std::fmt;
use std::sync::Arc;

use destack_core::Color;

use crate::{AnnotateError, File, LabeledSpan};

const HEADER_PREFIX: &str = "──▶";
const BODY_PREFIX: &str = " │ ";
const HIGHLIGHT: char = '^';
const ELIDE: &str = "··";

/// A function that colorizes a slice of source code.
///
/// Takes a file, byte range (start, end), and a brightness flag. When `bright`
/// is true, use full brightness colors; when false, use dimmed colors.
pub type SourceColorizer = Arc<dyn Fn(&File, u32, u32, bool) -> String + Send + Sync>;

/// Options controlling how annotation is rendered.
#[derive(Clone, Default)]
pub struct AnnotateOptions {
    /// Maximum number of characters to show from a source line. 0 disables clipping.
    pub line_width: u32 = 100,
    /// Number of context lines to show before the start line.
    pub prefix_lines: u8 = 2,
    /// Number of context lines to show after the end line.
    pub suffix_lines: u8 = 2,
    /// Whether to emit ANSI color escape sequences.
    pub use_color: bool = true,
    /// Color for normal text.
    pub color_normal: Color = Color::BrightWhite,
    /// Color for dim/less prominent text.
    pub color_dim: Color = Color::White,
    /// Color for metadata like line numbers and separators.
    pub color_meta: Color = Color::BrightMagenta,
    /// Color for highlights and labels.
    pub color_highlight: Color = Color::BrightYellow,
    /// Optional syntax colorizer for source code.
    pub colorizer: Option<SourceColorizer> = None,
}

impl AnnotateOptions {
    /// Create a new annotate options with the default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum line width.
    pub fn with_line_width(mut self, line_width: u32) -> Self {
        self.line_width = line_width;
        self
    }

    /// Set the highlight color.
    pub fn with_highlight_color(mut self, color: Color) -> Self {
        self.color_highlight = color;
        self
    }

    /// Set the surrounding lines.
    pub fn with_context_lines(mut self, prefix_lines: u8, suffix_lines: u8) -> Self {
        self.prefix_lines = prefix_lines;
        self.suffix_lines = suffix_lines;
        self
    }

    /// Set the source colorizer for syntax highlighting.
    pub fn with_colorizer(mut self, colorizer: SourceColorizer) -> Self {
        self.colorizer = Some(colorizer);
        self
    }
}

impl fmt::Debug for AnnotateOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnnotateOptions")
            .field("line_width", &self.line_width)
            .field("prefix_lines", &self.prefix_lines)
            .field("suffix_lines", &self.suffix_lines)
            .field("use_color", &self.use_color)
            .field("color_normal", &self.color_normal)
            .field("color_dim", &self.color_dim)
            .field("color_meta", &self.color_meta)
            .field("color_highlight", &self.color_highlight)
            .field("colorizer", &self.colorizer.as_ref().map(|_| "..."))
            .finish()
    }
}

/// Annotate source lines around a labeled span.
///
/// The `span` must come from the same `source`. The output includes a header
/// with file name and 1-based line and column, a body with the requested
/// surrounding lines, the original code, and a highlight caret line with the
/// label attached on the first highlighted line:
///
/// ```text
/// ==> file.ds:2:5
///  |
///  | 1 | fn main() {
///  | 2 |     let variable = 42;
///  |   |         ^^^^^^^^ label: explanation
///  | 3 | }
///  |
/// ```
///
/// Prefix and suffix line counts control how many lines are shown before and
/// after the highlighted region.
/// Lines longer than `max_line_width` are clipped to keep the highlight visible.
pub fn annotate_file(
    source: &File,
    span: &LabeledSpan,
    options: AnnotateOptions,
) -> Result<String, AnnotateError> {
    if source.id != span.span.file {
        return Err(AnnotateError::FileMismatch {
            source_file: source.id,
            span_file: span.span.file,
        });
    }

    // compute span bounds
    let Some((span_start_line, start_col)) = source.get_position(span.span.start) else {
        return Err(AnnotateError::InvalidSpanStart {
            file: source.id,
            offset: span.span.start,
        });
    };
    let span_end_line = source
        .get_position(span.span.end)
        .map(|(line, _)| line)
        .ok_or(AnnotateError::InvalidSpanEnd {
            file: source.id,
            offset: span.span.end,
        })?;

    // compute source window
    let (window_start_line, window_end_line) = get_source_window(
        source,
        span_start_line,
        span_end_line,
        options.prefix_lines,
        options.suffix_lines,
    );
    let width = (window_end_line + 1).to_string().len() as u32;

    // build string
    let mut buffer = String::new();
    write_header(
        &mut buffer,
        source,
        span_start_line,
        start_col,
        options.use_color,
        options.color_normal,
        options.color_meta,
    );
    write_body_separator(&mut buffer, options.use_color, options.color_meta);
    for line in window_start_line..=window_end_line {
        let (line_str, bounds, truncate_left, truncate_right) = get_visible_source_slice(
            source,
            line,
            span,
            span_start_line,
            span_end_line,
            options.line_width,
        )?;
        let is_in_span = line >= span_start_line && line <= span_end_line;
        write_source_line(
            &mut buffer,
            source,
            line,
            width,
            line_str,
            bounds,
            is_in_span,
            truncate_left,
            truncate_right,
            &options,
        );
        if is_in_span {
            let (offset_spaces, highlight_count, left_trunc, right_trunc) =
                get_highlight_offset(bounds, span, line, span_start_line, span_end_line);
            write_highlight_line(
                &mut buffer,
                width,
                offset_spaces,
                highlight_count,
                line == span_start_line,
                &span.label,
                left_trunc,
                right_trunc,
                options.use_color,
                options.color_meta,
                options.color_highlight,
            );
        }
    }
    write_body_separator(&mut buffer, options.use_color, options.color_meta);

    Ok(buffer)
}

/// Write the header line: `==> name:line:col`.
#[inline]
fn write_header(
    buffer: &mut String,
    source: &File,
    line: u32,
    col: u32,
    use_color: bool,
    color_normal: Color,
    color_meta: Color,
) {
    if use_color {
        // prefix
        buffer.push_str(&color_normal.apply(HEADER_PREFIX));
        buffer.push_str(&color_normal.apply(" "));
        // name
        buffer.push_str(&color_normal.apply(source.uri.as_ref()));
        buffer.push_str(&color_normal.apply(":"));
        // line
        buffer.push_str(&color_meta.apply(&line.saturating_add(1).to_string()));
        buffer.push_str(&color_normal.apply(":"));
        // column
        buffer.push_str(&color_meta.apply(&col.saturating_add(1).to_string()));
    } else {
        // prefix
        buffer.push_str(HEADER_PREFIX);
        buffer.push(' ');
        // name
        buffer.push_str(source.uri.as_ref());
        buffer.push(':');
        // line
        buffer.push_str(&line.saturating_add(1).to_string());
        buffer.push(':');
        // column
        buffer.push_str(&col.saturating_add(1).to_string());
    }
    buffer.push('\n');
}

/// Write a blank body separator line: ` | `.
#[inline]
fn write_body_separator(buffer: &mut String, use_color: bool, color_meta: Color) {
    if use_color {
        buffer.push_str(&color_meta.apply(BODY_PREFIX));
    } else {
        buffer.push_str(BODY_PREFIX);
    }
    buffer.push('\n');
}

/// Write a line with source code, including the line number column.
#[inline]
fn write_source_line(
    buffer: &mut String,
    source: &File,
    line: u32,
    width: u32,
    line_str: &str,
    bounds: (u32, u32, u32, u32),
    is_in_span: bool,
    truncate_left: bool,
    truncate_right: bool,
    options: &AnnotateOptions,
) {
    let (absolute_visible_start, absolute_visible_end, _, _) = bounds;

    // prefix
    if options.use_color {
        buffer.push_str(&options.color_meta.apply(BODY_PREFIX));
    } else {
        buffer.push_str(BODY_PREFIX);
    }
    let line_num = line.saturating_add(1).to_string();
    for _ in 0..(width - line_num.len() as u32) {
        buffer.push(' ');
    }

    // line number
    if options.use_color {
        if is_in_span {
            buffer.push_str(&options.color_meta.apply(&line_num));
        } else {
            buffer.push_str(&options.color_dim.apply(&line_num));
        }
        buffer.push_str(&options.color_meta.apply(" │ "));
    } else {
        buffer.push_str(&line_num);
        buffer.push_str(" │ ");
    }

    // content with elision markers
    if truncate_left {
        buffer.push_str(ELIDE);
    }

    // colorize source (highlighted lines are bright, context lines are dimmed)
    if options.use_color {
        if let Some(colorizer) = &options.colorizer {
            let colorized = colorizer(
                source,
                absolute_visible_start,
                absolute_visible_end,
                is_in_span,
            );
            buffer.push_str(&colorized);
        } else if is_in_span {
            buffer.push_str(&options.color_normal.apply(line_str));
        } else {
            buffer.push_str(&options.color_dim.apply(line_str));
        }
    } else {
        buffer.push_str(line_str);
    }

    if truncate_right {
        buffer.push_str(ELIDE);
    }
    buffer.push('\n');
}

/// Write the highlight line containing carets and the optional label.
fn write_highlight_line(
    buffer: &mut String,
    width: u32,
    offset_spaces: u32,
    highlight_count: u32,
    is_first: bool,
    label: &str,
    truncate_left: bool,
    truncate_right: bool,
    use_color: bool,
    color_meta: Color,
    color_highlight: Color,
) {
    // prefix
    if use_color {
        buffer.push_str(&color_meta.apply(BODY_PREFIX));
    } else {
        buffer.push_str(BODY_PREFIX);
    }
    for _ in 0..width {
        buffer.push(' ');
    }
    if use_color {
        buffer.push_str(&color_meta.apply(" │ "));
    } else {
        buffer.push_str(" │ ");
    }
    for _ in 0..offset_spaces {
        buffer.push(' ');
    }
    // highlight (with elision markers if truncated)
    let show_elide = truncate_left || truncate_right;
    let mut caret_text = String::new();
    if show_elide {
        caret_text.push_str(ELIDE);
    }
    for _ in 0..highlight_count {
        caret_text.push(HIGHLIGHT);
    }
    if show_elide {
        caret_text.push_str(ELIDE);
    }
    if use_color {
        buffer.push_str(&color_highlight.apply_bold(&caret_text));
    } else {
        buffer.push_str(&caret_text);
    }
    // label
    if is_first && !label.is_empty() {
        buffer.push(' ');
        if use_color {
            buffer.push_str(&color_highlight.apply_bold(label));
        } else {
            buffer.push_str(label);
        }
    }

    buffer.push('\n');
}

/// Get the first and last line indices to display (inclusive).
#[inline]
fn get_source_window(
    source: &File,
    start_line: u32,
    end_line: u32,
    prefix_lines: u8,
    suffix_lines: u8,
) -> (u32, u32) {
    let first = start_line.saturating_sub(prefix_lines as u32);
    let last_needed = end_line.saturating_add(suffix_lines as u32);
    let last_available = source.line_count().saturating_sub(1);
    (first, last_needed.min(last_available))
}

/// Compute the visible source code slice for a line and return it with absolute bounds.
///
/// Bounds are (absolute_visible_start, absolute_visible_end, line_start, line_end).
fn get_visible_source_slice<'a>(
    source: &'a File,
    current_line: u32,
    span: &LabeledSpan,
    start_line: u32,
    end_line: u32,
    max_line_width: u32,
) -> Result<(&'a str, (u32, u32, u32, u32), bool, bool), AnnotateError> {
    let line_text = source
        .get_line_str(current_line)
        .ok_or(AnnotateError::MissingLineText {
            file: source.id,
            line: current_line,
        })?;
    let line_span = source
        .get_line_span(current_line)
        .ok_or(AnnotateError::MissingLineSpan {
            file: source.id,
            line: current_line,
        })?;
    let line_len_bytes = line_span.len();

    // determine slice bounds within the line
    let mut slice_start_in_line: u32 = 0;
    let mut slice_end_in_line: u32 = line_len_bytes;

    // truncate long lines to fit within max_line_width
    if max_line_width > 0 && line_len_bytes > max_line_width {
        // calculate where the highlight appears within this line
        let highlight_start_in_line = if current_line == start_line {
            (span.span.start).saturating_sub(line_span.start)
        } else {
            0
        };
        let highlight_end_in_line = if current_line == end_line {
            (span.span.end).saturating_sub(line_span.start)
        } else if current_line >= start_line && current_line <= end_line {
            line_len_bytes
        } else {
            highlight_start_in_line
        };

        // position the slice to show the highlight
        let needed_end = highlight_end_in_line.min(line_len_bytes);
        if needed_end <= max_line_width {
            slice_start_in_line = 0;
            slice_end_in_line = max_line_width;
        } else {
            slice_end_in_line = needed_end;
            slice_start_in_line = slice_end_in_line.saturating_sub(max_line_width);
        }

        // adjust slice bounds to respect UTF-8 character boundaries
        let abs_slice_start = line_span.start + slice_start_in_line;
        let abs_slice_end = line_span.start + slice_end_in_line;
        let mut start_b = abs_slice_start;
        while start_b > line_span.start && !source.text().is_char_boundary(start_b as usize) {
            start_b -= 1;
        }
        let mut end_b = abs_slice_end;
        while end_b < line_span.end && !source.text().is_char_boundary(end_b as usize) {
            end_b += 1;
        }
        slice_start_in_line = start_b - line_span.start;
        slice_end_in_line = (end_b - line_span.start).min(line_len_bytes);
    }

    // extract the visible portion and compute absolute bounds
    let visible = line_text
        .get(slice_start_in_line as usize..slice_end_in_line as usize)
        .ok_or(AnnotateError::InvalidVisibleSlice {
            file: source.id,
            start: slice_start_in_line,
            end: slice_end_in_line,
        })?;
    let absolute_visible_start = line_span.start + slice_start_in_line;
    let absolute_visible_end = line_span.start + slice_end_in_line;
    let truncate_left = slice_start_in_line > 0;
    let truncate_right = slice_end_in_line < line_len_bytes;
    Ok((
        visible,
        (
            absolute_visible_start,
            absolute_visible_end,
            line_span.start,
            line_span.end,
        ),
        truncate_left,
        truncate_right,
    ))
}

/// Get the offset and length of the highlight within the visible slice.
fn get_highlight_offset(
    bounds: (u32, u32, u32, u32),
    span: &LabeledSpan,
    current_line: u32,
    start_line: u32,
    end_line: u32,
) -> (u32, u32, bool, bool) {
    let (absolute_visible_start, absolute_visible_end, line_start_byte, line_end_byte) = bounds;

    // determine the absolute span of the highlight on this line
    let segment_start_abs = if current_line == start_line {
        span.span.start
    } else {
        line_start_byte
    };
    let segment_end_abs = if current_line == end_line {
        span.span.end
    } else {
        line_end_byte
    };

    // clip the highlight to the visible portion
    let hl_start = segment_start_abs
        .max(absolute_visible_start)
        .min(absolute_visible_end);
    let hl_end = segment_end_abs
        .max(absolute_visible_start)
        .min(absolute_visible_end);

    // ensure zero-width spans show at least one character
    let mut highlight_count = hl_end.saturating_sub(hl_start);
    if highlight_count == 0 && current_line == start_line {
        highlight_count = 1;
    }

    let offset_spaces = hl_start.saturating_sub(absolute_visible_start);
    let truncate_left = segment_start_abs < absolute_visible_start;
    let truncate_right = segment_end_abs > absolute_visible_end;
    (
        offset_spaces,
        highlight_count,
        truncate_left,
        truncate_right,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FileId, FileType, Span, Uri};

    #[test]
    fn test_annotate_single_line() {
        let id = FileId::new(0);
        let content = r#"fn main() {
    let variable = 42;
}"#
        .to_string();
        let source = File::from_text(
            id,
            "<test>".to_string(),
            Uri::from_string("<test>"),
            None,
            FileType::Destack,
            content.clone(),
        );
        let start = content.find("variable").unwrap();
        let end = start + "variable".len();
        let span = LabeledSpan {
            span: Span::new(id, start as u32, end as u32),
            label: "variable name".to_string(),
        };
        let options = AnnotateOptions {
            line_width: 80,
            prefix_lines: 1,
            suffix_lines: 1,
            use_color: false,
            color_normal: Color::White,
            color_dim: Color::White,
            color_meta: Color::BrightMagenta,
            color_highlight: Color::BrightYellow,
            colorizer: None,
        };
        let annotated = annotate_file(&source, &span, options).unwrap();

        let expected = concat!(
            "──▶ <test>:2:9\n",
            " │ \n",
            " │ 1 │ fn main() {\n",
            " │ 2 │     let variable = 42;\n",
            " │   │         ^^^^^^^^ variable name\n",
            " │ 3 │ }\n",
            " │ \n",
        );

        assert_eq!(annotated, expected);
    }

    #[test]
    fn test_annotate_wrapped_line() {
        let content = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n";
        let id = FileId::new(0);
        let source = File::from_text(
            id,
            "<test>".to_string(),
            Uri::from_string("<test>"),
            None,
            FileType::Destack,
            content.to_string(),
        );
        let start = 150usize;
        let end = 155usize;
        let span = LabeledSpan {
            span: Span::new(id, start as u32, end as u32),
            label: "tail".to_string(),
        };
        let options = AnnotateOptions {
            line_width: 60,
            prefix_lines: 0,
            suffix_lines: 0,
            use_color: false,
            color_normal: Color::White,
            color_dim: Color::White,
            color_meta: Color::BrightMagenta,
            color_highlight: Color::BrightYellow,
            colorizer: None,
        };
        let annotated = annotate_file(&source, &span, options).unwrap();

        let expected = concat!(
            "──▶ <test>:1:151\n",
            " │ \n",
            " │ 1 │ ··aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa··\n",
            " │   │                                                        ^^^^^ tail\n",
            " │ \n",
        );

        assert_eq!(annotated, expected);
    }
}
