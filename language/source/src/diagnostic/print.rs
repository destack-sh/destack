use crate::{AnnotateOptions, DiagnosticCollection, FileRegistry, annotate_file, pluralize};

/// Options for printing diagnostics.
#[derive(Debug, Clone, Default)]
pub struct PrintOptions {
    /// Maximum line width for annotations.
    pub line_width: u32 = 100,
    /// Number of modules (for summary).
    pub module_count: Option<usize> = None,
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
}

/// Print diagnostics to stdout.
///
/// Prints all diagnostics with source annotations and a summary line.
pub fn print_diagnostics(
    files: &FileRegistry,
    diagnostics: &DiagnosticCollection,
    options: PrintOptions,
) {
    let annotate_options = AnnotateOptions::default().with_line_width(options.line_width);

    // individual diagnostics
    for diagnostic in diagnostics.iter() {
        let file = files.get(diagnostic.file_id);
        let annotate_options = annotate_options.with_highlight_color(diagnostic.severity.color());

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
                let original_options =
                    annotate_options.with_highlight_color(original_severity.color());
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
        eprintln!("{header}");
        eprintln!("{body}");
    }

    // summary
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
            eprintln!(
                "{}: {} from {} {}",
                color.apply_bold(&name),
                summary,
                module_count,
                pluralize(module_count, "module")
            );
        } else {
            eprintln!("{}: {}", color.apply_bold(&name), summary);
        }
    }
}
