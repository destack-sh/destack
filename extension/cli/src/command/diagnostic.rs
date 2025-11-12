use crate::console;
use dyst_dir::Session;
use dyst_source::{
    AnnotateOptions, Color, DiagnosticSeverity,
    annotate_source,
};

/// Print diagnostics (and suggestions) to the console.
pub(crate) fn print_diagnostics<'a>(session: &'a Session<'a>, min_severity: DiagnosticSeverity) {
    let options = AnnotateOptions {
        max_line_width: session.language.formatting.line_width as u32,
        prefix_lines: 1,
        suffix_lines: 1,
        use_color: true,
    };

    // print diagnostics
    for diagnostic in session.diagnostics.iter() {
        if diagnostic.severity < min_severity {
            continue;
        }
        let Some(file) = session.files.get(diagnostic.file_id) else {
            console::error(&format!("no source for diagnostic: {diagnostic:?}"));
            continue;
        };
        let header = Color::Red.apply_bold(&format!("{}: {}", diagnostic.code, diagnostic.message));
        let body = annotate_source(file, &diagnostic.primary_span, options);
        console::error(&header);
        console::info(&body);
    }
}
