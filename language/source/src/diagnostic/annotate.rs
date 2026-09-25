use std::borrow::Cow;
use std::fmt;
use std::sync::Arc;

use tspp_core::Color;
use tspp_unicode::UnicodeWidthChar;

use crate::{AnnotateError, File, LabeledSpan, Span};

const HEADER_PREFIX: &str = "──▶";
const GUTTER: &str = " │ ";
const PRIMARY_GLYPH: char = '^';
const SECONDARY_GLYPH: char = '-';
const CONNECTOR: &str = "│";
const OMISSION: &str = "··";
const TAB_SPACES: &str = "    ";

/// A function that colorizes a slice of source code.
///
/// Takes a file, byte range (start, end), and a brightness flag.
/// When `bright` is true, use full brightness colors; otherwise, use dimmed colors.
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

    /// Return the annotation kind.
    fn kind(&self) -> AnnotationKind {
        if self.is_primary {
            AnnotationKind::Primary
        } else {
            AnnotationKind::Secondary
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
    /// Maximum number of terminal columns to show; zero disables clipping.
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

/// The visual kind of one source annotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnnotationKind {
    /// The span carrying the diagnostic.
    Primary,
    /// One supporting span.
    Secondary,
}

impl AnnotationKind {
    /// Return the underline glyph for this kind.
    fn glyph(self) -> char {
        match self {
            Self::Primary => PRIMARY_GLYPH,
            Self::Secondary => SECONDARY_GLYPH,
        }
    }

    /// Return the configured color for this kind.
    fn color(self, options: &AnnotateOptions) -> Color {
        match self {
            Self::Primary => options.color_highlight,
            Self::Secondary => options.color_secondary,
        }
    }
}

/// The inclusive lines covered by one end-exclusive annotation span.
#[derive(Debug, Clone, Copy)]
struct AnnotationLines<'a> {
    /// The source annotation.
    annotation: &'a AnnotateSpan,
    /// The first covered line.
    first_line: u32,
    /// The last covered line.
    last_line: u32,
}

impl<'a> AnnotationLines<'a> {
    /// Resolve one annotation to its covered source lines.
    fn new(source: &File, annotation: &'a AnnotateSpan) -> Result<Self, AnnotateError> {
        if annotation.span.file != source.id {
            return Err(AnnotateError::FileMismatch {
                source_file: source.id,
                span_file: annotation.span.file,
            });
        }

        // resolve the inclusive first line
        let first_line = source
            .get_position(annotation.span.start)
            .map(|(line, _)| line)
            .ok_or(AnnotateError::InvalidSpanStart {
                file: source.id,
                offset: annotation.span.start,
            })?;

        // validate the end-exclusive span before moving inside it
        if annotation.span.end < annotation.span.start || annotation.span.end > source.len {
            return Err(AnnotateError::InvalidSpanEnd {
                file: source.id,
                offset: annotation.span.end,
            });
        }
        let last_offset = if annotation.span.is_empty() {
            annotation.span.end
        } else {
            annotation.span.end - 1
        };
        let last_line = source
            .get_position(last_offset)
            .map(|(line, _)| line)
            .ok_or(AnnotateError::InvalidSpanEnd {
                file: source.id,
                offset: annotation.span.end,
            })?;

        Ok(Self {
            annotation,
            first_line,
            last_line,
        })
    }

    /// Return whether this annotation covers one source line.
    fn contains(self, line: u32) -> bool {
        line >= self.first_line && line <= self.last_line
    }

    /// Return this annotation's visible segment on one covered line.
    fn segment(
        self,
        source: &File,
        line: u32,
        line_span: Span,
    ) -> Result<Option<LineAnnotation<'a>>, AnnotateError> {
        let start = if line == self.first_line {
            self.annotation.span.start
        } else {
            line_span.start
        };
        let end = if line == self.last_line {
            self.annotation.span.end
        } else {
            line_span.end
        };

        // retain exact single-line spans and trim multiline whitespace
        let span = if self.first_line == self.last_line {
            Some(Span::new(source.id, start, end))
        } else {
            trim_span(source, start, end)?
        };

        Ok(span.map(|span| LineAnnotation {
            annotation: self.annotation,
            span,
        }))
    }
}

/// One annotation segment visible on a rendered line.
#[derive(Debug, Clone, Copy)]
struct LineAnnotation<'a> {
    /// The source annotation.
    annotation: &'a AnnotateSpan,
    /// The content byte span.
    span: Span,
}

/// One source line prepared for rendering.
#[derive(Debug, Clone, Copy)]
struct SourceLine<'a> {
    /// The source text.
    text: &'a str,
    /// The source byte span.
    span: Span,
    /// Whether source text was omitted on the left.
    is_truncated_left: bool,
    /// Whether source text was omitted on the right.
    is_truncated_right: bool,
}

impl<'a> SourceLine<'a> {
    /// Load one source line.
    fn new(source: &'a File, line: u32) -> Result<Self, AnnotateError> {
        let text = source
            .get_line_str(line)
            .ok_or(AnnotateError::MissingLineText {
                file: source.id,
                line,
            })?;
        let span = source
            .get_line_span(line)
            .ok_or(AnnotateError::MissingLineSpan {
                file: source.id,
                line,
            })?;

        Ok(Self {
            text,
            span,
            is_truncated_left: false,
            is_truncated_right: false,
        })
    }

    /// Clip this line around its annotations.
    fn clip(
        self,
        annotations: &[LineAnnotation<'_>],
        max_width: u32,
    ) -> Result<Self, AnnotateError> {
        let byte_length = self.span.len();
        let mut byte_start = 0;
        let mut byte_end = byte_length;

        // clip long lines around the last annotated byte
        if max_width > 0 && display_width(self.text) > max_width {
            let annotation_byte_end = annotations
                .iter()
                .map(|annotation| annotation.span.end.saturating_sub(self.span.start))
                .max()
                .unwrap_or(0);
            let required_byte_end = annotation_byte_end.min(byte_length) as usize;
            let required =
                self.text
                    .get(..required_byte_end)
                    .ok_or(AnnotateError::InvalidVisibleSlice {
                        file: self.span.file,
                        start: self.span.start,
                        end: self.span.start + required_byte_end as u32,
                    })?;

            if display_width(required) <= max_width {
                byte_end = prefix_byte_end(self.text, max_width) as u32;
            } else {
                byte_end = required_byte_end as u32;
                byte_start = suffix_byte_start(required, max_width) as u32;
            }
        }

        // retain one valid UTF-8 slice
        let text = self
            .text
            .get(byte_start as usize..byte_end as usize)
            .ok_or(AnnotateError::InvalidVisibleSlice {
                file: self.span.file,
                start: self.span.start + byte_start,
                end: self.span.start + byte_end,
            })?;

        Ok(Self {
            text,
            span: Span::new(
                self.span.file,
                self.span.start + byte_start,
                self.span.start + byte_end,
            ),
            is_truncated_left: byte_start > 0,
            is_truncated_right: byte_end < byte_length,
        })
    }

    /// Return the terminal column of one visible byte offset.
    fn column(self, offset: u32) -> Result<u32, AnnotateError> {
        if offset < self.span.start || offset > self.span.end {
            return Err(AnnotateError::InvalidVisibleSlice {
                file: self.span.file,
                start: self.span.start,
                end: offset,
            });
        }
        let prefix = self.text.get(..(offset - self.span.start) as usize).ok_or(
            AnnotateError::InvalidVisibleSlice {
                file: self.span.file,
                start: self.span.start,
                end: offset,
            },
        )?;

        Ok(display_width(prefix))
    }
}

/// One label placed on an underline row.
#[derive(Debug, Clone, Copy)]
struct PlacedLabel<'a> {
    /// The column of the span start inside the visible slice.
    column: u32,
    /// The label text.
    label: &'a str,
    /// The annotation kind.
    kind: AnnotationKind,
}

/// Annotate source lines around one or more labeled spans in one window.
///
/// All spans must come from `source`.
/// The first primary span anchors the header position.
/// The output contains underline rows for visible content and stacked rows for shared labels:
///
/// ```text
/// ──▶ file.tspp:2:9
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
    if spans.is_empty() {
        return Ok(String::new());
    }
    let anchor_index = spans.iter().position(|span| span.is_primary).unwrap_or(0);

    // resolve every span once before rendering lines
    let annotations = spans
        .iter()
        .map(|annotation| AnnotationLines::new(source, annotation))
        .collect::<Result<Vec<_>, _>>()?;
    let first_line = annotations
        .iter()
        .map(|annotation| annotation.first_line)
        .min()
        .unwrap_or(0);
    let last_line = annotations
        .iter()
        .map(|annotation| annotation.last_line)
        .max()
        .unwrap_or(0);

    // place the header at the first primary annotation
    let anchor = annotations[anchor_index];
    let anchor_line = anchor.first_line;
    let anchor_line_span =
        source
            .get_line_span(anchor_line)
            .ok_or(AnnotateError::MissingLineSpan {
                file: source.id,
                line: anchor_line,
            })?;
    let anchor_prefix = source_slice(source, anchor_line_span.start, anchor.annotation.span.start)?;
    let anchor_column = display_width(anchor_prefix);

    // compute the source window
    let window_first_line = first_line.saturating_sub(options.prefix_lines as u32);
    let window_last_line = last_line
        .saturating_add(options.suffix_lines as u32)
        .min(source.line_count().saturating_sub(1));
    let gutter_width = (window_last_line + 1).to_string().len() as u32;

    let mut buffer = String::new();
    write_header(
        &mut buffer,
        source,
        anchor_line,
        anchor_column,
        gutter_width,
        &options,
    );
    write_separator(&mut buffer, gutter_width, &options);
    for line in window_first_line..=window_last_line {
        write_annotated_line(
            &mut buffer,
            source,
            line,
            gutter_width,
            &annotations,
            &options,
        )?;
    }
    write_separator(&mut buffer, gutter_width, &options);

    Ok(buffer)
}

/// Write one source line and its caret and label rows.
fn write_annotated_line(
    buffer: &mut String,
    source: &File,
    line: u32,
    gutter_width: u32,
    annotations: &[AnnotationLines<'_>],
    options: &AnnotateOptions,
) -> Result<(), AnnotateError> {
    // trim each covering annotation to this line's content
    let source_line = SourceLine::new(source, line)?;
    let mut segments = Vec::new();
    for annotation in annotations
        .iter()
        .copied()
        .filter(|annotation| annotation.contains(line))
    {
        if let Some(segment) = annotation.segment(source, line, source_line.span)? {
            segments.push(segment);
        }
    }
    let is_annotated = !segments.is_empty();

    // clip the source around its visible annotations
    let source_line = source_line.clip(&segments, options.line_width)?;

    // source row
    write_gutter(
        buffer,
        Some(line),
        gutter_width,
        is_annotated,
        !source_line.text.is_empty(),
        options,
    );
    if source_line.is_truncated_left {
        buffer.push_str(OMISSION);
    }
    if options.use_color {
        if let Some(colorizer) = &options.colorizer {
            let colored = colorizer(
                source,
                source_line.span.start,
                source_line.span.end,
                is_annotated,
            );
            let colored = expand_tabs(&colored);
            buffer.push_str(&colored);
        } else if is_annotated {
            let text = expand_tabs(source_line.text);
            buffer.push_str(&options.color_normal.apply(&text));
        } else {
            let text = expand_tabs(source_line.text);
            buffer.push_str(&options.color_dim.apply(&text));
        }
    } else {
        let text = expand_tabs(source_line.text);
        buffer.push_str(&text);
    }
    if source_line.is_truncated_right {
        buffer.push_str(OMISSION);
    }
    buffer.push('\n');
    if !is_annotated {
        return Ok(());
    }

    // caret row: overlay annotations with primary spans winning overlaps
    let omission_width = if source_line.is_truncated_left {
        OMISSION.chars().count() as u32
    } else {
        0
    };
    let rendered_width = display_width(source_line.text) + omission_width;
    let mut cells = vec![None; rendered_width as usize + 1];
    let mut labels = Vec::new();
    for segment in segments {
        let start = segment
            .span
            .start
            .clamp(source_line.span.start, source_line.span.end);
        let end = segment
            .span
            .end
            .clamp(source_line.span.start, source_line.span.end);
        let start_column = source_line.column(start)? + omission_width;
        let end_column = source_line.column(end)? + omission_width;

        // zero-width spans still show one caret
        let end_column = end_column.max(start_column + 1);
        let kind = segment.annotation.kind();
        for cell in start_column..end_column.min(rendered_width + 1) {
            let cell = &mut cells[cell as usize];
            if kind == AnnotationKind::Primary || cell.is_none() {
                *cell = Some(kind);
            }
        }

        // place the label on the first nonempty multiline segment
        let before_segment =
            source_slice(source, segment.annotation.span.start, segment.span.start)?;
        let is_first_visible_segment = before_segment.trim().is_empty();
        if is_first_visible_segment && !segment.annotation.label.is_empty() {
            labels.push(PlacedLabel {
                column: start_column,
                label: &segment.annotation.label,
                kind,
            });
        }
    }

    // sort labels by column and retain the rightmost label inline
    labels.sort_by_key(|label| label.column);
    let inline_label = labels.pop();

    write_gutter(buffer, None, gutter_width, true, true, options);
    write_caret_cells(buffer, &cells, options);
    if let Some(inline_label) = &inline_label {
        buffer.push(' ');
        write_label(buffer, inline_label, options);
    }
    buffer.push('\n');

    // stack remaining labels under connector rows from right to left
    if !labels.is_empty() {
        write_gutter(buffer, None, gutter_width, true, true, options);
        write_connectors(buffer, &labels, labels.len(), options);
        buffer.push('\n');
        for index in (0..labels.len()).rev() {
            write_gutter(buffer, None, gutter_width, true, true, options);
            let column = write_connectors(buffer, &labels, index, options);
            for _ in column..labels[index].column {
                buffer.push(' ');
            }
            write_label(buffer, &labels[index], options);
            buffer.push('\n');
        }
    }

    Ok(())
}

/// Return one valid source slice between absolute byte offsets.
fn source_slice(source: &File, start: u32, end: u32) -> Result<&str, AnnotateError> {
    source
        .text()
        .get(start as usize..end as usize)
        .ok_or(AnnotateError::InvalidVisibleSlice {
            file: source.id,
            start,
            end,
        })
}

/// Trim whitespace from one multiline annotation span.
fn trim_span(source: &File, start: u32, end: u32) -> Result<Option<Span>, AnnotateError> {
    let text = source_slice(source, start, end)?;
    let leading_bytes = text.len() - text.trim_start().len();
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }

    let start = start + leading_bytes as u32;
    let end = start + text.len() as u32;

    Ok(Some(Span::new(source.id, start, end)))
}

/// Return text with tabs expanded to diagnostic indentation.
fn expand_tabs(text: &str) -> Cow<'_, str> {
    if text.contains('\t') {
        Cow::Owned(text.replace('\t', TAB_SPACES))
    } else {
        Cow::Borrowed(text)
    }
}

/// Return the terminal column width of one source string.
fn display_width(text: &str) -> u32 {
    text.chars().map(character_width).sum()
}

/// Return the terminal width of one source character.
fn character_width(character: char) -> u32 {
    if character == '\t' {
        TAB_SPACES.len() as u32
    } else {
        u32::from(character.terminal_display_width())
    }
}

/// Return the byte end of the longest prefix within the terminal width.
fn prefix_byte_end(text: &str, max_width: u32) -> usize {
    let mut width = 0;
    for (index, character) in text.char_indices() {
        let next_width = width + character_width(character);
        if next_width > max_width {
            return index;
        }
        width = next_width;
    }

    text.len()
}

/// Return the byte start of the longest suffix within the terminal width.
fn suffix_byte_start(text: &str, max_width: u32) -> usize {
    let mut start = text.len();
    let mut width = 0;
    for (index, character) in text.char_indices().rev() {
        let next_width = width + character_width(character);
        if next_width > max_width {
            break;
        }
        start = index;
        width = next_width;
    }

    start
}

/// Write connector columns for the first `count` pending labels.
///
/// Returns the column after the last written connector.
fn write_connectors(
    buffer: &mut String,
    labels: &[PlacedLabel<'_>],
    count: usize,
    options: &AnnotateOptions,
) -> u32 {
    let mut column = 0;
    for label in labels.iter().take(count) {
        for _ in column..label.column {
            buffer.push(' ');
        }
        if options.use_color {
            buffer.push_str(&label.kind.color(options).apply_bold(CONNECTOR));
        } else {
            buffer.push_str(CONNECTOR);
        }
        column = label.column + 1;
    }

    column
}

/// Write one caret row from overlaid cells, coloring by span kind.
fn write_caret_cells(
    buffer: &mut String,
    cells: &[Option<AnnotationKind>],
    options: &AnnotateOptions,
) {
    let mut run = String::new();
    let mut run_kind = None;

    // write each contiguous annotation kind as one colored run
    for cell in cells {
        if *cell != run_kind {
            write_caret_run(buffer, &mut run, run_kind, options);
            run_kind = *cell;
        }
        run.push(match cell {
            Some(kind) => kind.glyph(),
            None => ' ',
        });
    }

    // remove trailing blank cells and write the final run
    while run.ends_with(' ') {
        run.pop();
    }
    write_caret_run(buffer, &mut run, run_kind, options);
}

/// Write one contiguous caret run.
fn write_caret_run(
    buffer: &mut String,
    run: &mut String,
    kind: Option<AnnotationKind>,
    options: &AnnotateOptions,
) {
    if run.is_empty() {
        return;
    }

    // color annotations by kind and blank columns as metadata
    if options.use_color {
        let color = kind.map_or(options.color_meta, |kind| kind.color(options));
        buffer.push_str(&color.apply_bold(run));
    } else {
        buffer.push_str(run);
    }
    run.clear();
}

/// Write one label in its annotation color.
fn write_label(buffer: &mut String, label: &PlacedLabel<'_>, options: &AnnotateOptions) {
    if options.use_color {
        let color = label.kind.color(options);
        buffer.push_str(&color.apply_bold(label.label));
    } else {
        buffer.push_str(label.label);
    }
}

/// Write the header row: `──▶ name:line:col` aligned to the gutter.
fn write_header(
    buffer: &mut String,
    source: &File,
    line: u32,
    column: u32,
    gutter_width: u32,
    options: &AnnotateOptions,
) {
    for _ in 0..gutter_width {
        buffer.push(' ');
    }
    let location = format!(
        "{}:{}:{}",
        source.uri.as_ref(),
        line.saturating_add(1),
        column.saturating_add(1)
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
fn write_separator(buffer: &mut String, gutter_width: u32, options: &AnnotateOptions) {
    for _ in 0..gutter_width {
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
    gutter_width: u32,
    is_annotated: bool,
    is_padded: bool,
    options: &AnnotateOptions,
) {
    let number = line.map(|line| line.saturating_add(1).to_string());
    let padding = gutter_width - number.as_deref().map_or(0, str::len) as u32;
    for _ in 0..padding {
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
    let gutter = if is_padded { GUTTER } else { GUTTER.trim_end() };
    if options.use_color {
        buffer.push_str(&options.color_meta.apply(gutter));
    } else {
        buffer.push_str(gutter);
    }
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
            FileType::Tspp,
            content.to_string(),
        )
        .expect("test source should load")
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
        let expected = r#" ──▶ <test>:2:9
  │
1 │ fn main() {
2 │     let variable = 42;
  │         ^^^^^^^^ variable name
3 │ }
  │
"#;
        assert_eq!(annotated, expected);
    }

    #[test]
    fn test_annotate_multiline_content() {
        let content = r#"function append(target: int32[], source: int32[]): void {
    for (const value of source) {
        target.push(value);

    }
}"#;
        let source = test_file(content);
        let start = content.find("for").unwrap() as u32;
        let end = content.rfind("    }").unwrap() as u32 + 5;
        let span = AnnotateSpan::primary(Span::new(source.id, start, end), "");

        let annotated = annotate_file(&source, &[span], plain_options()).unwrap();

        let expected = r#" ──▶ <test>:2:5
  │
1 │ function append(target: int32[], source: int32[]): void {
2 │     for (const value of source) {
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │         target.push(value);
  │         ^^^^^^^^^^^^^^^^^^^
4 │
5 │     }
  │     ^
6 │ }
  │
"#;
        assert_eq!(annotated, expected);
    }

    #[test]
    fn test_annotate_multiline_label_after_empty_line() {
        let content = "\nvalue";
        let source = test_file(content);
        let span = AnnotateSpan::primary(Span::new(source.id, 0, content.len() as u32), "value");

        let annotated = annotate_file(&source, &[span], plain_options()).unwrap();

        let expected = r#" ──▶ <test>:1:1
  │
1 │
2 │ value
  │ ^^^^^ value
  │
"#;
        assert_eq!(annotated, expected);
    }

    #[test]
    fn test_annotate_span_ending_at_line_start() {
        let content = "value\nnext";
        let source = test_file(content);
        let span = AnnotateSpan::primary(Span::new(source.id, 0, 6), "value");

        let annotated = annotate_file(&source, &[span], plain_options()).unwrap();

        let expected = r#" ──▶ <test>:1:1
  │
1 │ value
  │ ^^^^^ value
2 │ next
  │
"#;
        assert_eq!(annotated, expected);
    }

    #[test]
    fn test_annotate_unicode_columns() {
        let content = "const café = value;";
        let source = test_file(content);
        let start = content.find("value").unwrap() as u32;
        let span = AnnotateSpan::primary(Span::new(source.id, start, start + 5), "value");

        let annotated = annotate_file(&source, &[span], plain_options()).unwrap();

        let expected = r#" ──▶ <test>:1:14
  │
1 │ const café = value;
  │              ^^^^^ value
  │
"#;
        assert_eq!(annotated, expected);
    }

    #[test]
    fn test_annotate_tab_columns() {
        let content = "\tlet value = 1;";
        let source = test_file(content);
        let start = content.find("value").unwrap() as u32;
        let span = AnnotateSpan::primary(Span::new(source.id, start, start + 5), "value");

        let annotated = annotate_file(&source, &[span], plain_options()).unwrap();

        let expected = r#" ──▶ <test>:1:9
  │
1 │     let value = 1;
  │         ^^^^^ value
  │
"#;
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

        let expected = r#" ──▶ <test>:1:25
  │
1 │ const overflows: int8 = 300;
  │                  ----   ^^^ this value does not fit
  │                  │
  │                  expected `int8` because of this annotation
  │
"#;
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

        let expected = r#" ──▶ <test>:2:12
  │
1 │ function get(): int8 {
  │                 ---- declared here
2 │     return 300;
  │            ^^^ does not fit
3 │ }
  │
"#;
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

        let expected = r#" ──▶ <test>:1:151
  │
1 │ ··aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa··
  │                                                          ^^^^^ tail
  │
"#;
        assert_eq!(annotated, expected);
    }

    #[test]
    fn test_annotate_wrapped_unicode_line() {
        let content = "界界界界a";
        let source = test_file(content);
        let start = content.find('a').unwrap() as u32;
        let span = AnnotateSpan::primary(Span::new(source.id, start, start + 1), "value");
        let options = AnnotateOptions {
            line_width: 5,
            prefix_lines: 0,
            suffix_lines: 0,
            use_color: false,
            ..AnnotateOptions::default()
        };

        let annotated = annotate_file(&source, &[span], options).unwrap();

        let expected = r#" ──▶ <test>:1:9
  │
1 │ ··界界a
  │       ^ value
  │
"#;
        assert_eq!(annotated, expected);
    }
}
