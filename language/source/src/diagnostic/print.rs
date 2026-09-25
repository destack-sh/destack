use std::fmt;
use std::sync::Arc;

use tspp_core::{Color, pluralize};

use crate::{
    AnnotateOptions, AnnotateSpan, Applicability, DiagnosticCollection, DiagnosticLabel,
    DiagnosticRenderError, DiagnosticSeverity, DiagnosticSuggestion, DiffOptions, File, FileId,
    SourceColorizer, annotate_file, apply_file_patch, format_diff,
};

/// Write a diagnostic line.
type LineWriter = Arc<dyn Fn(&str) + Send + Sync>;

/// Options for printing diagnostics.
#[derive(Clone, Default)]
pub struct PrintOptions {
    /// Maximum line width for annotations.
    pub line_width: u32 = 100,
    /// Number of modules (for summary).
    pub module_count: Option<usize> = None,
    /// Optional syntax colorizer for source code.
    pub colorizer: Option<SourceColorizer> = None,
    /// Skip printing the summary line.
    pub skip_summary: bool = false,
    /// Whether to emit ANSI color escape sequences.
    pub use_color: bool = true,
    /// Optional line writer for diagnostic output.
    pub line_writer: Option<LineWriter> = None,
}

impl fmt::Debug for PrintOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PrintOptions")
            .field("line_width", &self.line_width)
            .field("module_count", &self.module_count)
            .field("colorizer", &self.colorizer.as_ref().map(|_| "..."))
            .field("skip_summary", &self.skip_summary)
            .field("use_color", &self.use_color)
            .field("line_writer", &self.line_writer.as_ref().map(|_| "..."))
            .finish()
    }
}

impl PrintOptions {
    /// Create a new print options with the default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the line width.
    pub fn with_line_width(mut self, line_width: u32) -> Self {
        self.line_width = line_width;
        self
    }

    /// Set the module count for the summary.
    pub fn with_module_count(mut self, module_count: usize) -> Self {
        self.module_count = Some(module_count);
        self
    }

    /// Set the source colorizer for syntax highlighting.
    pub fn with_colorizer(mut self, colorizer: SourceColorizer) -> Self {
        self.colorizer = Some(colorizer);
        self
    }

    /// Skip printing the summary line (caller will print their own).
    pub fn with_skip_summary(mut self, skip: bool) -> Self {
        self.skip_summary = skip;
        self
    }

    /// Set whether ANSI color escape sequences are emitted.
    pub fn with_color(mut self, use_color: bool) -> Self {
        self.use_color = use_color;
        self
    }

    /// Set the line writer for diagnostic output.
    pub fn with_line_writer(mut self, line_writer: LineWriter) -> Self {
        self.line_writer = Some(line_writer);
        self
    }
}

/// Print diagnostics to the configured line writer, defaulting to stderr.
///
/// Prints all diagnostics with source annotations and a summary line.
pub fn print_diagnostics<F>(
    file_for_id: &F,
    diagnostics: &DiagnosticCollection,
    options: PrintOptions,
) -> Result<(), DiagnosticRenderError>
where
    F: Fn(FileId) -> Option<Arc<File>>,
{
    let mut annotate_options = AnnotateOptions::default().with_line_width(options.line_width);
    annotate_options.use_color = options.use_color;
    if let Some(colorizer) = options.colorizer.clone() {
        annotate_options = annotate_options.with_colorizer(colorizer);
    }

    // individual diagnostics
    for diagnostic in diagnostics.iter() {
        let primary = diagnostic.primary_label();
        let file = file_for_label(file_for_id, primary)?;
        let annotate_options = annotate_options
            .clone()
            .with_highlight_color(diagnostic.severity.color());

        // render the severity, canonical id, and message
        let header_preamble = color_bold(
            &options,
            diagnostic.severity.color(),
            &format!(
                "{}[{}]",
                diagnostic.severity.family_name().to_ascii_lowercase(),
                diagnostic.id,
            ),
        );
        let header_message = color_bold(&options, Color::BrightWhite, &diagnostic.message);
        write_line(&options, &format!("{header_preamble}: {header_message}"));

        // gather same-file span labels into the primary window
        let primary_span = primary.target.span();
        let mut spans = Vec::new();
        if let Some(span) = primary_span {
            spans.push(AnnotateSpan::primary(
                span,
                primary.message.clone().unwrap_or_default(),
            ));
        }
        let mut detached = Vec::new();
        for label in diagnostic.labels() {
            match label.target.span() {
                Some(span) if label.target.file() == primary.target.file() => {
                    spans.push(AnnotateSpan::secondary(
                        span,
                        label.message.clone().unwrap_or_default(),
                    ));
                }
                _ => detached.push(label),
            }
        }
        // whole-file primaries point at their file without a window
        if !spans.is_empty() {
            let body = annotate_file(&file, &spans, annotate_options.clone())?;
            write_block(&options, &body);
        } else {
            let text = format!(
                "  {} {}",
                color_text(&options, Color::BrightMagenta, "──▶"),
                color_text(&options, Color::BrightWhite, &file.name),
            );
            write_block(&options, &text);
        }

        // span labels in other files render their own window, file
        //  labels point at their file, and labels whose source is
        //  unavailable degrade to a bare note
        let detached_options = annotate_options.clone().with_context_lines(1, 1);
        for label in detached {
            let message = label.message.clone().unwrap_or_default();
            match (label.target.span(), file_for_label(file_for_id, label)) {
                (Some(span), Ok(detached_file)) => {
                    let span = AnnotateSpan::secondary(span, message);
                    let detached_body =
                        annotate_file(&detached_file, &[span], detached_options.clone())?;
                    write_block(&options, &detached_body);
                }
                (None, Ok(detached_file)) => {
                    let text = format!(
                        "  {} {}: {}",
                        color_text(&options, Color::BrightMagenta, "──▶"),
                        color_text(&options, Color::BrightWhite, &detached_file.name),
                        color_text(&options, Color::BrightWhite, &message),
                    );
                    write_block(&options, &text);
                }
                (_, Err(_)) => {
                    let text = format!(
                        " {} {} {}",
                        color_text(&options, Color::BrightMagenta, "="),
                        color_bold(&options, Color::BrightWhite, "note:"),
                        color_text(&options, Color::BrightWhite, &message),
                    );
                    write_block(&options, &text);
                }
            }
        }

        // notes and helps align under the window gutter
        for note in diagnostic.notes() {
            let text = format!(
                " {} {} {}",
                color_text(&options, Color::BrightMagenta, "="),
                color_bold(&options, Color::BrightWhite, "note:"),
                color_text(&options, Color::BrightWhite, &note.message)
            );
            write_line(&options, &text);
        }
        for help in diagnostic.helps() {
            let text = format!(
                " {} {} {}",
                color_text(&options, Color::BrightMagenta, "="),
                color_bold(&options, Color::BrightCyan, "help:"),
                color_text(&options, Color::BrightCyan, &help.message)
            );
            write_line(&options, &text);
        }

        // suggestions
        for suggestion in &diagnostic.suggestions {
            write_suggestion(file_for_id, &options, suggestion)?;
        }
    }

    // point at the explain command for the reported error ids
    let mut ids: Vec<&str> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        .map(|diagnostic| diagnostic.id.as_str())
        .collect();
    ids.sort_unstable();
    ids.dedup();
    if let Some(first) = ids.first() {
        let trailer = format!("for more information about an error, run `tspp explain {first}`");
        write_line(&options, &color_text(&options, Color::White, &trailer));
    }

    // summary (skip if caller will print their own)
    if options.skip_summary {
        return Ok(());
    }
    let counts = diagnostics.count_diagnostics_by_severity();
    if !counts.is_empty() {
        // derive families and pluralize with naive 's'
        let mut parts: Vec<String> = Vec::new();
        for (severity, count) in counts.iter().rev() {
            let name = severity.family_name().to_ascii_lowercase();
            let color = severity.color();
            let colored_count = color_bold(&options, color, &format!("{count}"));
            let colored_name = color_bold(&options, color, &pluralize(*count, name));
            parts.push(format!("{colored_count} {colored_name}"));
        }
        let summary = parts.join(", ");

        // summary line
        let Some(highest_severity) = counts.keys().max() else {
            return Ok(());
        };
        let color = highest_severity.color();
        let name = highest_severity.family_name().to_ascii_lowercase();

        if let Some(module_count) = options.module_count {
            write_line(
                &options,
                &format!(
                    "{}: {} from {} {}",
                    color_bold(&options, color, &name),
                    summary,
                    module_count,
                    pluralize(module_count, "module")
                ),
            );
        } else {
            write_line(
                &options,
                &format!("{}: {}", color_bold(&options, color, &name), summary),
            );
        }
    }

    Ok(())
}

fn file_for_label<F>(
    file_for_id: &F,
    label: &DiagnosticLabel,
) -> Result<Arc<File>, DiagnosticRenderError>
where
    F: Fn(FileId) -> Option<Arc<File>>,
{
    let file_id = label.target.file();
    let file = file_for_id(file_id).ok_or(DiagnosticRenderError::MissingFile { file: file_id })?;
    let actual = file.blob();

    if actual != label.blob {
        return Err(DiagnosticRenderError::BlobMismatch {
            file: file_id,
            expected: label.blob,
            actual,
        });
    }

    Ok(file)
}

fn write_suggestion<F>(
    file_for_id: &F,
    options: &PrintOptions,
    suggestion: &DiagnosticSuggestion,
) -> Result<(), DiagnosticRenderError>
where
    F: Fn(FileId) -> Option<Arc<File>>,
{
    let (label, qualifier) = match suggestion.applicability {
        Applicability::Automatic => ("fix:", ""),
        Applicability::Unsafe => ("suggestion:", " (may change behavior)"),
        Applicability::Dangerous => ("suggestion:", " (requires review)"),
    };
    let message = format!(
        " {} {} {}{qualifier}",
        color_text(options, Color::BrightMagenta, "="),
        color_bold(options, Color::BrightGreen, label),
        color_text(options, Color::BrightGreen, &suggestion.message)
    );
    write_line(options, &message);

    for file_patch in &suggestion.patches.files {
        let file = file_for_id(file_patch.file).ok_or(DiagnosticRenderError::MissingFile {
            file: file_patch.file,
        })?;
        let updated = apply_file_patch(&file, file_patch)?;
        if updated == file.text() {
            continue;
        }

        let diff_options = DiffOptions::new()
            .with_context(1)
            .with_path(file.uri.as_ref())
            .with_color(options.use_color);
        let diff = format_diff(file.text(), &updated, &diff_options);
        write_block(options, &diff);
    }

    Ok(())
}

fn color_text(options: &PrintOptions, color: Color, text: &str) -> String {
    if options.use_color {
        color.apply(text)
    } else {
        text.to_string()
    }
}

fn color_bold(options: &PrintOptions, color: Color, text: &str) -> String {
    if options.use_color {
        color.apply_bold(text)
    } else {
        text.to_string()
    }
}

fn write_line(options: &PrintOptions, line: &str) {
    if let Some(writer) = &options.line_writer {
        writer(line);
    } else {
        eprintln!("{line}");
    }
}

fn write_block(options: &PrintOptions, block: &str) {
    for line in block.split('\n') {
        write_line(options, line);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::{
        Applicability, Diagnostic, DiagnosticCollection, DiagnosticLabel, DiagnosticSuggestion,
        DiagnosticTarget, File, FileId, FilePatch, FileType, Patch, PrintOptions, Span, Uri,
        print_diagnostics,
    };

    /// Capture diagnostic printer output as one string.
    fn render(diagnostics: DiagnosticCollection, file: Arc<File>) -> String {
        let lines = Arc::new(Mutex::new(Vec::<String>::new()));
        let output = Arc::clone(&lines);
        let writer = Arc::new(move |line: &str| {
            output.lock().unwrap().push(line.to_string());
        });
        let file_for_id = |file_id| {
            if file_id == file.id {
                Some(Arc::clone(&file))
            } else {
                None
            }
        };
        let options = PrintOptions::new()
            .with_color(false)
            .with_skip_summary(true)
            .with_line_writer(writer);

        print_diagnostics(&file_for_id, &diagnostics, options).unwrap();

        lines.lock().unwrap().join("\n")
    }

    /// Render suggestion labels and patch diffs.
    #[test]
    fn test_prints_suggestion_labels_and_diff() {
        let file_id = FileId::new(1);
        let file = Arc::new(
            File::from_text(
                file_id,
                "<test>".to_string(),
                Uri::from_string("<test>"),
                None,
                FileType::Tspp,
                "let value = 1;".to_string(),
            )
            .expect("test source should load"),
        );
        let let_span = Span::new(file_id, 0, 3);
        let blob = file.blob();
        let mut file_patch = FilePatch::new(file_id);
        file_patch.push(Patch::replace(let_span, "const"));
        let suggestion =
            DiagnosticSuggestion::new("use `const`", file_patch.into(), Applicability::Automatic)
                .label(DiagnosticLabel::message(
                    blob,
                    DiagnosticTarget::Span(let_span),
                    "replace `let` with `const`",
                ));
        let diagnostic = Diagnostic::warning(
            "prefer-const",
            "variable is never reassigned",
            DiagnosticLabel::message(blob, DiagnosticTarget::Span(let_span), "use const"),
        )
        .suggestion(suggestion);
        let diagnostics = DiagnosticCollection::from_diagnostics(vec![diagnostic]);

        let rendered = render(diagnostics, file);

        let expected = concat!(
            "warning[prefer-const]: variable is never reassigned\n",
            " ──▶ <test>:1:1\n",
            "  │\n",
            "1 │ let value = 1;\n",
            "  │ ^^^ use const\n",
            "  │\n",
            "\n",
            " = fix: use `const`\n",
            "--- a/<test>\n",
            "+++ b/<test>\n",
            "\n",
            "-   1│ let value = 1;\n",
            "+   1│ const value = 1;\n",
        );
        assert_eq!(rendered, expected);
    }

    /// Render secondary labels, notes, and helps in one window.
    #[test]
    fn test_prints_labels_notes_and_helps() {
        let file_id = FileId::new(1);
        let file = Arc::new(
            File::from_text(
                file_id,
                "<test>".to_string(),
                Uri::from_string("<test>"),
                None,
                FileType::Tspp,
                "const overflows: int8 = 300;".to_string(),
            )
            .expect("test source should load"),
        );
        let blob = file.blob();
        let value_span = Span::new(file_id, 24, 27);
        let annotation_span = Span::new(file_id, 17, 21);
        let diagnostic = Diagnostic::error(
            "not-assignable",
            "type '300' is not assignable to type 'int8'",
            DiagnosticLabel::message(
                blob,
                DiagnosticTarget::Span(value_span),
                "this value does not fit",
            ),
        )
        .label(DiagnosticLabel::message(
            blob,
            DiagnosticTarget::Span(annotation_span),
            "expected `int8` because of this annotation",
        ))
        .note("`int8` holds values in -128..=127")
        .help("widen the annotation or use a fitting value");
        let diagnostics = DiagnosticCollection::from_diagnostics(vec![diagnostic]);

        let rendered = render(diagnostics, file);

        let expected = concat!(
            "error[not-assignable]: type '300' is not assignable to type 'int8'\n",
            " ──▶ <test>:1:25\n",
            "  │\n",
            "1 │ const overflows: int8 = 300;\n",
            "  │                  ----   ^^^ this value does not fit\n",
            "  │                  │\n",
            "  │                  expected `int8` because of this annotation\n",
            "  │\n",
            "\n",
            " = note: `int8` holds values in -128..=127\n",
            " = help: widen the annotation or use a fitting value\n",
            "for more information about an error, run `tspp explain not-assignable`",
        );
        assert_eq!(rendered, expected);
    }
}
