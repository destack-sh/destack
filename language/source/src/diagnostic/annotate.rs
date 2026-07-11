use std::fmt;
use std::sync::Arc;

use destack_core::Color;

use crate::{AnnotateError, File, LabeledSpan, Span};

const HEADER_PREFIX: &str = "──▶";
const GUTTER: &str = " │ ";
const PRIMARY_GLYPH: char = '^';
const SECONDARY_GLYPH: char = '-';
const CONNECTOR: char = '│';
const ELIDE: &str = "··";

/// A function that colorizes a slice of source code.
///
/// Takes a file, byte range (start, end), and a brightness flag. When `bright`
/// is true, use full brightness colors; when false, use dimmed colors.
pub type SourceColorizer = Arc<dyn Fn(&File, u32, u32, bool) -> String + Send + Sync>;

/// One span annotated inside a source window.
#[derive(Debug, Clone)]
pub struct AnnotateSpan {
    /// The annotated span.
    pub span: Span,
    /// The label shown at the span, empty for a bare underline.
    pub label: String,
    /// Whether the span carries the diagnostic itself.
    pub is_primary: bool,
}

impl AnnotateSpan {
    /// Create one primary annotated span.
    pub fn primary(span: Span, label: impl Into<String>) -> Self {
        Self {
            span,
            label: label.into(),
            is_primary: true,
        }
    }

    /// Create one secondary annotated span.
    pub fn secondary(span: Span, label: impl Into<String>) -> Self {
        Self {
            span,
            label: label.into(),
            is_primary: false,
        }
    }
}

impl From<&LabeledSpan> for AnnotateSpan {
    /// Annotate one labeled span as the primary span.
    fn from(labeled: &LabeledSpan) -> Self {
        Self::primary(labeled.span, labeled.label.clone())
    }
}

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
    /// Color for the primary highlight and its label.
    pub color_highlight: Color = Color::BrightYellow,
    /// Color for secondary highlights and their labels.
    pub color_secondary: Color = Color::BrightCyan,
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
            .field("color_secondary", &self.color_secondary)
            .field("colorizer", &self.colorizer.as_ref().map(|_| "..."))
            .finish()
    }
}

/// One label segment placed on a caret row.
#[derive(Debug, Clone)]
struct PlacedLabel {
    /// The column of the span start inside the visible slice.
    column: u32,
    /// The label text.
    label: String,
    /// Whether the label belongs to the primary span.
    is_primary: bool,
}

/// Annotate source lines around one or more labeled spans in one window.
///
/// All spans must come from `source`; the first primary span anchors the
/// header position. The output is one source window with an underline row
/// per annotated line and stacked connector rows when labels share a line:
///
/// ```text
/// ──▶ file.ds:2:9
///   │
/// 1 │ fn main() {
/// 2 │     let variable: int8 = value;
///   │         ^^^^^^^^  ---- expected because of this
///   │         │
///   │         this binding
/// 3 │ }
///   │
/// ```
pub fn annotate_file(
    source: &File,
    spans: &[AnnotateSpan],
    options: AnnotateOptions,
) -> Result<String, AnnotateError> {
    let Some(anchor) = spans
        .iter()
        .find(|span| span.is_primary)
        .or_else(|| spans.first())
    else {
        return Ok(String::new());
    };
    for span in spans {
        if source.id != span.span.file {
            return Err(AnnotateError::FileMismatch {
                source_file: source.id,
                span_file: span.span.file,
            });
        }
    }

    // compute the line range covered by every span
    let mut lines = Vec::with_capacity(spans.len());
    for span in spans {
        let start =
            source
                .get_position(span.span.start)
                .ok_or(AnnotateError::InvalidSpanStart {
                    file: source.id,
                    offset: span.span.start,
                })?;
        let end = source
            .get_position(span.span.end)
            .map(|(line, _)| line)
            .ok_or(AnnotateError::InvalidSpanEnd {
                file: source.id,
                offset: span.span.end,
            })?;
        lines.push((start.0, end));
    }
    let first_line = lines.iter().map(|(start, _)| *start).min().unwrap_or(0);
    let last_line = lines.iter().map(|(_, end)| *end).max().unwrap_or(0);
    let (anchor_line, anchor_col) =
        source
            .get_position(anchor.span.start)
            .ok_or(AnnotateError::InvalidSpanStart {
                file: source.id,
                offset: anchor.span.start,
            })?;

    // compute the source window
    let window_start = first_line.saturating_sub(options.prefix_lines as u32);
    let window_end = last_line
        .saturating_add(options.suffix_lines as u32)
        .min(source.line_count().saturating_sub(1));
    let width = (window_end + 1).to_string().len() as u32;

    let mut buffer = String::new();
    write_header(
        &mut buffer,
        source,
        anchor_line,
        anchor_col,
        width,
        &options,
    );
    write_separator(&mut buffer, width, &options);
    for line in window_start..=window_end {
        write_annotated_line(&mut buffer, source, line, width, spans, &lines, &options)?;
    }
    write_separator(&mut buffer, width, &options);

    Ok(buffer)
}

/// Write one source line and its caret and label rows.
fn write_annotated_line(
    buffer: &mut String,
    source: &File,
    line: u32,
    width: u32,
    spans: &[AnnotateSpan],
    lines: &[(u32, u32)],
    options: &AnnotateOptions,
) -> Result<(), AnnotateError> {
    // collect the spans intersecting and starting on this line
    let mut intersecting = Vec::new();
    for (index, span) in spans.iter().enumerate() {
        let (start_line, end_line) = lines[index];
        if line >= start_line && line <= end_line {
            intersecting.push((index, span, start_line, end_line));
        }
    }
    let is_annotated = !intersecting.is_empty();

    // clip the line around the union of its annotated ranges
    let (visible, bounds, truncate_left, truncate_right) =
        visible_slice(source, line, &intersecting, options.line_width)?;
    let (visible_start, visible_end, line_start, line_end) = bounds;

    // source row
    write_gutter(buffer, Some(line), width, is_annotated, options);
    if truncate_left {
        buffer.push_str(ELIDE);
    }
    if options.use_color {
        if let Some(colorizer) = &options.colorizer {
            buffer.push_str(&colorizer(source, visible_start, visible_end, is_annotated));
        } else if is_annotated {
            buffer.push_str(&options.color_normal.apply(visible));
        } else {
            buffer.push_str(&options.color_dim.apply(visible));
        }
    } else {
        buffer.push_str(visible);
    }
    if truncate_right {
        buffer.push_str(ELIDE);
    }
    buffer.push('\n');
    if !is_annotated {
        return Ok(());
    }

    // caret row: overlay every intersecting span, primary wins overlaps
    let elide_offset = if truncate_left {
        ELIDE.chars().count() as u32
    } else {
        0
    };
    let visible_len = (visible_end - visible_start) + elide_offset;
    let mut cells: Vec<Option<bool>> = vec![None; visible_len as usize + 1];
    let mut labels = Vec::new();
    for (_, span, start_line, end_line) in &intersecting {
        let segment_start = if line == *start_line {
            span.span.start
        } else {
            line_start
        };
        let segment_end = if line == *end_line {
            span.span.end
        } else {
            line_end
        };
        let clipped_start = segment_start.max(visible_start).min(visible_end);
        let clipped_end = segment_end.max(visible_start).min(visible_end);
        let from = clipped_start - visible_start + elide_offset;
        // zero-width spans still show one caret
        let to = (clipped_end - visible_start + elide_offset).max(from + 1);
        for cell in from..to.min(visible_len + 1) {
            let cell = &mut cells[cell as usize];
            *cell = Some(span.is_primary || cell.unwrap_or(false));
        }
        if line == *start_line && !span.label.is_empty() {
            labels.push(PlacedLabel {
                column: from,
                label: span.label.clone(),
                is_primary: span.is_primary,
            });
        }
    }

    // labels sort by column; the rightmost label sits inline on the caret row
    labels.sort_by_key(|label| label.column);
    let inline = labels.pop();

    write_gutter(buffer, None, width, true, options);
    write_caret_cells(buffer, &cells, options);
    if let Some(inline) = &inline {
        buffer.push(' ');
        push_label_text(buffer, inline, options);
    }
    buffer.push('\n');

    // remaining labels stack under connector rows, right to left
    if !labels.is_empty() {
        write_gutter(buffer, None, width, true, options);
        write_connectors(buffer, &labels, labels.len(), options);
        buffer.push('\n');
        for index in (0..labels.len()).rev() {
            write_gutter(buffer, None, width, true, options);
            let column = write_connectors(buffer, &labels, index, options);
            for _ in column..labels[index].column {
                buffer.push(' ');
            }
            push_label_text(buffer, &labels[index], options);
            buffer.push('\n');
        }
    }

    Ok(())
}

/// Write connector columns for the first `count` pending labels.
///
/// Returns the column after the last written connector.
fn write_connectors(
    buffer: &mut String,
    labels: &[PlacedLabel],
    count: usize,
    options: &AnnotateOptions,
) -> u32 {
    let mut column = 0;
    for label in labels.iter().take(count) {
        for _ in column..label.column {
            buffer.push(' ');
        }
        let color = label_color(label, options);
        if options.use_color {
            buffer.push_str(&color.apply_bold(&CONNECTOR.to_string()));
        } else {
            buffer.push(CONNECTOR);
        }
        column = label.column + 1;
    }

    column
}

/// Write one caret row from overlaid cells, coloring by span kind.
fn write_caret_cells(buffer: &mut String, cells: &[Option<bool>], options: &AnnotateOptions) {
    let mut run = String::new();
    let mut run_kind: Option<bool> = None;
    let flush = |buffer: &mut String, run: &mut String, kind: Option<bool>| {
        if run.is_empty() {
            return;
        }
        if options.use_color {
            let color = match kind {
                Some(true) => options.color_highlight,
                Some(false) => options.color_secondary,
                None => options.color_meta,
            };
            buffer.push_str(&color.apply_bold(run));
        } else {
            buffer.push_str(run);
        }
        run.clear();
    };

    for cell in cells {
        if *cell != run_kind {
            flush(buffer, &mut run, run_kind);
            run_kind = *cell;
        }
        run.push(match cell {
            Some(true) => PRIMARY_GLYPH,
            Some(false) => SECONDARY_GLYPH,
            None => ' ',
        });
    }
    // trailing blank cells never print
    while run.ends_with(' ') {
        run.pop();
    }
    flush(buffer, &mut run, run_kind);
}

/// Write one label text in its span color.
fn push_label_text(buffer: &mut String, label: &PlacedLabel, options: &AnnotateOptions) {
    if options.use_color {
        let color = label_color(label, options);
        buffer.push_str(&color.apply_bold(&label.label));
    } else {
        buffer.push_str(&label.label);
    }
}

/// Return the color for one placed label.
fn label_color(label: &PlacedLabel, options: &AnnotateOptions) -> Color {
    if label.is_primary {
        options.color_highlight
    } else {
        options.color_secondary
    }
}

/// Write the header row: `──▶ name:line:col` aligned to the gutter.
fn write_header(
    buffer: &mut String,
    source: &File,
    line: u32,
    col: u32,
    width: u32,
    options: &AnnotateOptions,
) {
    for _ in 0..width {
        buffer.push(' ');
    }
    let location = format!(
        "{}:{}:{}",
        source.uri.as_ref(),
        line.saturating_add(1),
        col.saturating_add(1)
    );
    if options.use_color {
        buffer.push_str(&options.color_meta.apply(HEADER_PREFIX));
        buffer.push(' ');
        buffer.push_str(&options.color_normal.apply(&location));
    } else {
        buffer.push_str(HEADER_PREFIX);
        buffer.push(' ');
        buffer.push_str(&location);
    }
    buffer.push('\n');
}

/// Write one blank gutter separator row.
fn write_separator(buffer: &mut String, width: u32, options: &AnnotateOptions) {
    for _ in 0..width {
        buffer.push(' ');
    }
    let gutter = GUTTER.trim_end();
    if options.use_color {
        buffer.push_str(&options.color_meta.apply(gutter));
    } else {
        buffer.push_str(gutter);
    }
    buffer.push('\n');
}

/// Write one gutter prefix, with the line number when given.
fn write_gutter(
    buffer: &mut String,
    line: Option<u32>,
    width: u32,
    is_annotated: bool,
    options: &AnnotateOptions,
) {
    let number = line.map(|line| line.saturating_add(1).to_string());
    let pad = width - number.as_deref().map_or(0, str::len) as u32;
    for _ in 0..pad {
        buffer.push(' ');
    }
    if let Some(number) = &number {
        if options.use_color {
            if is_annotated {
                buffer.push_str(&options.color_meta.apply(number));
            } else {
                buffer.push_str(&options.color_dim.apply(number));
            }
        } else {
            buffer.push_str(number);
        }
    }
    if options.use_color {
        buffer.push_str(&options.color_meta.apply(GUTTER));
    } else {
        buffer.push_str(GUTTER);
    }
}

/// Compute the visible slice of one line around its annotated ranges.
///
/// Bounds are (visible_start, visible_end, line_start, line_end) in absolute
/// byte offsets.
fn visible_slice<'a>(
    source: &'a File,
    line: u32,
    intersecting: &[(usize, &AnnotateSpan, u32, u32)],
    max_line_width: u32,
) -> Result<(&'a str, (u32, u32, u32, u32), bool, bool), AnnotateError> {
    let line_text = source
        .get_line_str(line)
        .ok_or(AnnotateError::MissingLineText {
            file: source.id,
            line,
        })?;
    let line_span = source
        .get_line_span(line)
        .ok_or(AnnotateError::MissingLineSpan {
            file: source.id,
            line,
        })?;
    let line_len = line_span.len();

    let mut slice_start: u32 = 0;
    let mut slice_end: u32 = line_len;

    // clip long lines around the union of annotated ranges
    if max_line_width > 0 && line_len > max_line_width {
        let union_end = intersecting
            .iter()
            .map(|(_, span, _, end_line)| {
                if line == *end_line {
                    span.span.end.saturating_sub(line_span.start)
                } else {
                    line_len
                }
            })
            .max()
            .unwrap_or(0);

        let needed_end = union_end.min(line_len);
        if needed_end <= max_line_width {
            slice_end = max_line_width;
        } else {
            slice_end = needed_end;
            slice_start = slice_end.saturating_sub(max_line_width);
        }

        // respect UTF-8 character boundaries
        let mut start = line_span.start + slice_start;
        while start > line_span.start && !source.text().is_char_boundary(start as usize) {
            start -= 1;
        }
        let mut end = line_span.start + slice_end;
        while end < line_span.end && !source.text().is_char_boundary(end as usize) {
            end += 1;
        }
        slice_start = start - line_span.start;
        slice_end = (end - line_span.start).min(line_len);
    }

    let visible = line_text
        .get(slice_start as usize..slice_end as usize)
        .ok_or(AnnotateError::InvalidVisibleSlice {
            file: source.id,
            start: slice_start,
            end: slice_end,
        })?;

    Ok((
        visible,
        (
            line_span.start + slice_start,
            line_span.start + slice_end,
            line_span.start,
            line_span.end,
        ),
        slice_start > 0,
        slice_end < line_len,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FileId, FileType, Uri};

    fn test_file(content: &str) -> File {
        File::from_text(
            FileId::new(0),
            "<test>".to_string(),
            Uri::from_string("<test>"),
            None,
            FileType::Destack,
            content.to_string(),
        )
    }

    fn plain_options() -> AnnotateOptions {
        AnnotateOptions {
            line_width: 80,
            prefix_lines: 1,
            suffix_lines: 1,
            use_color: false,
            ..AnnotateOptions::default()
        }
    }

    #[test]
    fn test_annotate_single_line() {
        let content = "fn main() {\n    let variable = 42;\n}";
        let source = test_file(content);
        let start = content.find("variable").unwrap() as u32;
        let span = AnnotateSpan::primary(Span::new(source.id, start, start + 8), "variable name");

        let annotated = annotate_file(&source, &[span], plain_options()).unwrap();

        let expected = concat!(
            " ──▶ <test>:2:9\n",
            "  │\n",
            "1 │ fn main() {\n",
            "2 │     let variable = 42;\n",
            "  │         ^^^^^^^^ variable name\n",
            "3 │ }\n",
            "  │\n",
        );
        assert_eq!(annotated, expected);
    }

    #[test]
    fn test_annotate_stacked_labels() {
        let content = "const overflows: int8 = 300;";
        let source = test_file(content);
        let annotation = content.find("int8").unwrap() as u32;
        let value = content.find("300").unwrap() as u32;
        let spans = [
            AnnotateSpan::primary(
                Span::new(source.id, value, value + 3),
                "this value does not fit",
            ),
            AnnotateSpan::secondary(
                Span::new(source.id, annotation, annotation + 4),
                "expected `int8` because of this annotation",
            ),
        ];

        let annotated = annotate_file(&source, &spans, plain_options()).unwrap();

        let expected = concat!(
            " ──▶ <test>:1:25\n",
            "  │\n",
            "1 │ const overflows: int8 = 300;\n",
            "  │                  ----   ^^^ this value does not fit\n",
            "  │                  │\n",
            "  │                  expected `int8` because of this annotation\n",
            "  │\n",
        );
        assert_eq!(annotated, expected);
    }

    #[test]
    fn test_annotate_labels_on_separate_lines() {
        let content = "function get(): int8 {\n    return 300;\n}";
        let source = test_file(content);
        let annotation = content.find("int8").unwrap() as u32;
        let value = content.find("300").unwrap() as u32;
        let spans = [
            AnnotateSpan::primary(Span::new(source.id, value, value + 3), "does not fit"),
            AnnotateSpan::secondary(
                Span::new(source.id, annotation, annotation + 4),
                "declared here",
            ),
        ];

        let annotated = annotate_file(&source, &spans, plain_options()).unwrap();

        let expected = concat!(
            " ──▶ <test>:2:12\n",
            "  │\n",
            "1 │ function get(): int8 {\n",
            "  │                 ---- declared here\n",
            "2 │     return 300;\n",
            "  │            ^^^ does not fit\n",
            "3 │ }\n",
            "  │\n",
        );
        assert_eq!(annotated, expected);
    }

    #[test]
    fn test_annotate_wrapped_line() {
        let content = format!("{}\n", "a".repeat(204));
        let source = test_file(&content);
        let span = AnnotateSpan::primary(Span::new(source.id, 150, 155), "tail");
        let options = AnnotateOptions {
            line_width: 60,
            prefix_lines: 0,
            suffix_lines: 0,
            use_color: false,
            ..AnnotateOptions::default()
        };

        let annotated = annotate_file(&source, &[span], options).unwrap();

        let expected = concat!(
            " ──▶ <test>:1:151\n",
            "  │\n",
            "1 │ ··aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa··\n",
            "  │                                                          ^^^^^ tail\n",
            "  │\n",
        );
        assert_eq!(annotated, expected);
    }
}
