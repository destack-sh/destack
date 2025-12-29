use std::fmt;
use std::sync::Arc;

use destack_base::pluralize;

use crate::{AnnotateOptions, DiagnosticCollection, FileRegistry, SourceColorizer, annotate_file};

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

    /// Set the line writer for diagnostic output.
    pub fn with_line_writer(mut self, line_writer: LineWriter) -> Self {
        self.line_writer = Some(line_writer);
        self
    }
}

/// Print diagnostics to the configured line writer, defaulting to stderr.
///
/// Prints all diagnostics with source annotations and a summary line.
pub fn print_diagnostics(
    files: &FileRegistry,
    diagnostics: &DiagnosticCollection,
    options: PrintOptions,
) {
    let mut annotate_options = AnnotateOptions::default().with_line_width(options.line_width);
    if let Some(colorizer) = options.colorizer.clone() {
        annotate_options = annotate_options.with_colorizer(colorizer);
    }

    // individual diagnostics
    for diagnostic in diagnostics.iter() {
        let file = files.get(diagnostic.file_id);
        let annotate_options = annotate_options
            .clone()
            .with_highlight_color(diagnostic.severity.color());

        // header preamble (severity + code)
        let header_preamble = annotate_options.color_highlight.apply_bold(&format!(
            "{} {}",
            diagnostic.severity.family_name().to_ascii_lowercase(),
            diagnostic.code,
        ));

        let header_message = annotate_options.color_normal.apply(&diagnostic.message);
        let header = {
            // include original code/severity if it exists and differs
            if let Some(original_code) = &diagnostic.original_code
                && let Some(original_severity) = diagnostic.original_severity
                && (*original_code != diagnostic.code || original_severity != diagnostic.severity)
            {
                let original_options = annotate_options
                    .clone()
                    .with_highlight_color(original_severity.color());
                let header_preamble_original = original_options
                    .color_highlight
                    .apply_bold(&original_code.to_string());
                let header_preamble = format!("{header_preamble} ({header_preamble_original})");
                format!("{header_preamble}: {header_message}")
            } else {
                format!("{header_preamble}: {header_message}")
            }
        };

        let body = annotate_file(&file, &diagnostic.primary_span, annotate_options);
        write_line(&options, &header);
        write_block(&options, &body);
    }

    // summary (skip if caller will print their own)
    if options.skip_summary {
        return;
    }
    let counts = diagnostics.count_diagnostics_by_severity();
    if !counts.is_empty() {
        // derive families and pluralize with naive 's'
        let mut parts: Vec<String> = Vec::new();
        for (severity, count) in counts.iter().rev() {
            let name = severity.family_name().to_ascii_lowercase();
            let color = severity.color();
            let colored_count = color.apply_bold(&format!("{count}"));
            let colored_name = color.apply_bold(&pluralize(*count, name));
            parts.push(format!("{colored_count} {colored_name}"));
        }
        let summary = parts.join(", ");

        // summary line
        let highest_severity = counts.keys().max().unwrap();
        let color = highest_severity.color();
        let name = highest_severity.family_name().to_ascii_lowercase();

        if let Some(module_count) = options.module_count {
            write_line(
                &options,
                &format!(
                    "{}: {} from {} {}",
                    color.apply_bold(&name),
                    summary,
                    module_count,
                    pluralize(module_count, "module")
                ),
            );
        } else {
            write_line(
                &options,
                &format!("{}: {}", color.apply_bold(&name), summary),
            );
        }
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
