use std::fmt;
use std::sync::Arc;

use destack_core::{Color, pluralize};

use crate::{
    AnnotateOptions, Applicability, DiagnosticCollection, DiagnosticLabel, DiagnosticRenderError,
    DiagnosticSuggestion, DiffOptions, File, FileId, SourceColorizer, annotate_file,
    apply_file_edit, format_diff,
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

        // header preamble (severity + code)
        let header_preamble = color_bold(
            &options,
            diagnostic.severity.color(),
            &format!(
                "{} {}",
                diagnostic.severity.family_name().to_ascii_lowercase(),
                diagnostic.code,
            ),
        );

        // header message
        let header_message = color_text(&options, Color::BrightWhite, &diagnostic.message);
        let header = format!("{header_preamble}: {header_message}");

        // primary + body
        let primary = primary.to_labeled_span(&diagnostic.message);
        let body = annotate_file(&file, &primary, annotate_options.clone())?;
        write_line(&options, &header);
        write_block(&options, &body);

        // related labels
        let secondary_options = annotate_options
            .clone()
            .with_highlight_color(Color::BrightCyan)
            .with_context_lines(1, 1);
        for label in diagnostic.labels() {
            let related = label.to_labeled_span("related location");
            let secondary_file = file_for_label(file_for_id, label)?;
            let secondary_body =
                annotate_file(&secondary_file, &related, secondary_options.clone())?;
            write_block(&options, &secondary_body);
        }

        // notes
        for note in diagnostic.notes() {
            let note = format!("note: {}", note.message);
            let note = color_text(&options, Color::BrightWhite, &note);
            write_line(&options, &note);
        }

        // helps
        for help in diagnostic.helps() {
            let help = format!("help: {}", help.message);
            let help = color_text(&options, Color::BrightCyan, &help);
            write_line(&options, &help);
        }

        // suggestions
        for suggestion in &diagnostic.suggestions {
            write_suggestion(file_for_id, &options, &annotate_options, suggestion)?;
        }
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
    let file_id = label.span.file;
    let file = file_for_id(file_id).ok_or(DiagnosticRenderError::MissingFile { file: file_id })?;
    let actual = file.content_id();

    if actual != label.content {
        return Err(DiagnosticRenderError::ContentMismatch {
            file: file_id,
            expected: label.content,
            actual,
        });
    }

    Ok(file)
}

fn write_suggestion<F>(
    file_for_id: &F,
    options: &PrintOptions,
    annotate_options: &AnnotateOptions,
    suggestion: &DiagnosticSuggestion,
) -> Result<(), DiagnosticRenderError>
where
    F: Fn(FileId) -> Option<Arc<File>>,
{
    let applicability = match suggestion.applicability {
        Applicability::Automatic => "machine-applicable",
        Applicability::Unsafe => "unsafe",
        Applicability::Dangerous => "requires review",
    };
    let message = format!("help: {} ({applicability})", suggestion.message);
    let message = color_text(options, Color::BrightGreen, &message);
    write_line(options, &message);

    let suggestion_options = annotate_options
        .clone()
        .with_highlight_color(Color::BrightGreen)
        .with_context_lines(1, 1);
    for label in &suggestion.labels {
        let labeled_span = label.to_labeled_span("suggested change");
        let file = file_for_label(file_for_id, label)?;
        let body = annotate_file(&file, &labeled_span, suggestion_options.clone())?;
        write_block(options, &body);
    }

    for file_edit in &suggestion.edits.files {
        let file = file_for_id(file_edit.file).ok_or(DiagnosticRenderError::MissingFile {
            file: file_edit.file,
        })?;
        let updated = apply_file_edit(&file, file_edit)?;
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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::{
        Applicability, BatchEdit, Diagnostic, DiagnosticCollection, DiagnosticLabel,
        DiagnosticSuggestion, Edit, File, FileEdit, FileId, FileType, PrintOptions, Span, Uri,
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

    /// Render suggestion labels and edit diffs.
    #[test]
    fn test_prints_suggestion_labels_and_diff() {
        let file_id = FileId::new(1);
        let file = Arc::new(File::from_text(
            file_id,
            "<test>".to_string(),
            Uri::from_string("<test>"),
            None,
            FileType::Destack,
            "let value = 1;".to_string(),
        ));
        let let_span = Span::new(file_id, 0, 3);
        let content = file.content_id();
        let mut file_edit = FileEdit::new(file_id);
        file_edit.push(Edit::replace(let_span, "const"));
        let suggestion = DiagnosticSuggestion::new(
            "use `const`",
            BatchEdit::single(file_edit),
            Applicability::Automatic,
        )
        .label(DiagnosticLabel::message(
            content,
            let_span,
            "replace `let` with `const`",
        ));
        let diagnostic = Diagnostic::warning(
            "W001",
            "variable is never reassigned",
            DiagnosticLabel::message(content, let_span, "use const"),
        )
        .suggestion(suggestion);
        let diagnostics = DiagnosticCollection::from_diagnostics(vec![diagnostic]);

        let rendered = render(diagnostics, file);

        let expected = concat!(
            "warning W001: variable is never reassigned\n",
            "──▶ <test>:1:1\n",
            " │ \n",
            " │ 1 │ let value = 1;\n",
            " │   │ ^^^ use const\n",
            " │ \n",
            "\n",
            "help: use `const` (machine-applicable)\n",
            "──▶ <test>:1:1\n",
            " │ \n",
            " │ 1 │ let value = 1;\n",
            " │   │ ^^^ replace `let` with `const`\n",
            " │ \n",
            "\n",
            "--- a/<test>\n",
            "+++ b/<test>\n",
            "\n",
            "-   1│ let value = 1;\n",
            "+   1│ const value = 1;\n",
        );
        assert_eq!(rendered, expected);
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
