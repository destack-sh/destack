use crate::console;
use dyst_dir::Session;
use dyst_source::{AnnotateOptions, DiagnosticSeverity, annotate_source};

/// Print diagnostics (and suggestions) to the console.
pub(crate) fn print_diagnostics<'a>(session: &'a Session<'a>, min_severity: DiagnosticSeverity) {
    let options =
        AnnotateOptions::default().with_line_width(session.language.formatting.line_width as u32);
    for diagnostic in session.diagnostics.iter() {
        if diagnostic.severity < min_severity {
            continue;
        }
        let Some(file) = session.files.get(diagnostic.file_id) else {
            console::error(&format!("no source for diagnostic: {diagnostic:?}"));
            continue;
        };
        let options = options.with_highlight_color(diagnostic.severity.color());
        let header = options.color_highlight.apply_bold(&format!(
            "{}[{}]: {}",
            diagnostic.severity.family_name().to_ascii_lowercase(),
            diagnostic.code,
            diagnostic.message
        ));
        let body = annotate_source(file, &diagnostic.primary_span, options);
        console::error(&header);
        console::print(&body);
    }
}
