use crate::console;
use dyst_dir::Program;
use dyst_source::{
    AnnotateOptions, DiagnosticCollection, DiagnosticOptions, annotate_file, pluralize,
};

use clap::Args;

#[derive(Args, Debug, Clone)]
pub struct DiagnosticOptionsArgs {
    /// Error on the given warning codes (like W001).
    #[arg(long, value_delimiter = ',', value_name = "CODES")]
    pub error_warnings: Vec<String>,

    /// Suppress the given error codes (like E001) as warnings.
    #[arg(long, value_delimiter = ',', value_name = "CODES")]
    pub suppress_errors: Vec<String>,

    /// Suppress the given warning codes (like W001).
    #[arg(long, value_delimiter = ',', value_name = "CODES")]
    pub suppress_warnings: Vec<String>,
}

impl From<DiagnosticOptionsArgs> for DiagnosticOptions {
    fn from(args: DiagnosticOptionsArgs) -> Self {
        DiagnosticOptions {
            error_warnings: args.error_warnings,
            suppress_errors: args.suppress_errors,
            suppress_warnings: args.suppress_warnings,
        }
    }
}

/// Print diagnostics (and suggestions) to the console.
pub(crate) fn print_diagnostics(program: &Program, diagnostics: &DiagnosticCollection) {
    let options =
        AnnotateOptions::default().with_line_width(program.language.formatting.line_width as u32);

    // individual diagnostics
    for diagnostic in diagnostics.iter() {
        let file = program.files.get(diagnostic.file_id);
        let options = options.with_highlight_color(diagnostic.severity.color());
        let header_preamble = options.color_highlight.apply_bold(&format!(
            "{} {}",
            diagnostic.severity.family_name().to_ascii_lowercase(),
            diagnostic.code,
        ));

        let header_message = options.color_normal.apply(&diagnostic.message);
        let header = {
            // include original code/severity if it exists
            if let Some(original_code) = diagnostic.original_code
                && let Some(original_severity) = diagnostic.original_severity
                && (original_code != diagnostic.code || original_severity != diagnostic.severity)
            {
                let options = options.with_highlight_color(original_severity.color());
                let header_preamble_original = options
                    .color_highlight
                    .apply_bold(&original_code.to_string());
                let header_preamble = format!("{header_preamble} ({header_preamble_original})");
                format!("{header_preamble}: {header_message}")
            } else {
                format!("{header_preamble}: {header_message}")
            }
        };

        let body = annotate_file(&file, &diagnostic.primary_span, options);
        console::error(&header);
        console::print(&body);
    }

    // summary
    let counts = diagnostics.count_diagnostics_by_severity();
    if !counts.is_empty() {
        console::print(""); // newline
        // derives families + pluralizes with naive 's'
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
        console::print(&format!(
            "{}: {} from {} {}",
            color.apply_bold(&name),
            summary,
            program.modules.len(),
            pluralize(program.modules.len(), "module")
        ));
    }
}
