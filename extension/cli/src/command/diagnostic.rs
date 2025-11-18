use crate::console;
use dyst_dir::Session;
use dyst_source::{AnnotateOptions, DiagnosticSeverity, annotate_source, pluralize};

/// Print diagnostics (and suggestions) to the console.
pub(crate) fn print_diagnostics<'a>(session: &'a Session<'a>, min_severity: DiagnosticSeverity) {
    let options =
        AnnotateOptions::default().with_line_width(session.language.formatting.line_width as u32);

    // individual diagnostics
    for diagnostic in session.diagnostics.iter() {
        if diagnostic.severity < min_severity {
            continue;
        }
        let Some(file) = session.files.get(diagnostic.file_id) else {
            console::error(&format!("no source for diagnostic: {diagnostic:?}"));
            continue;
        };
        let options = options.with_highlight_color(diagnostic.severity.color());
        let header_preamble = options.color_highlight.apply_bold(&format!(
            "{} {}",
            diagnostic.severity.family_name().to_ascii_lowercase(),
            diagnostic.code,
        ));
        let header_message = options.color_normal.apply(&diagnostic.message);
        let header = format!("{header_preamble}: {header_message}");
        let body = annotate_source(file, &diagnostic.primary_span, options);
        console::error(&header);
        console::print(&body);
    }

    // summary
    let counts = session.diagnostics.count_diagnostics_by_severity();
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
            session.modules.len(),
            pluralize(session.modules.len(), "module")
        ));
    }
}
