use destack_terminal::console;
use dyst_source::{
    AnnotateOptions, Color, DiagnosticCollector, File, FileId, LanguageOptions, DiagnosticSeverity,
    annotate_source,
};

/// Print diagnostics (and suggestions) to the console.
pub(crate) fn print_diagnostics<'a>(
    diagnostics: &DiagnosticCollector,
    language: LanguageOptions,
    min_severity: DiagnosticSeverity,
    get_source: impl Fn(FileId) -> Option<&'a File>,
) {
    let options = AnnotateOptions {
        max_line_width: language.formatting.line_width as u32,
        prefix_lines: 1,
        suffix_lines: 1,
        use_color: true,
    };

    // print diagnostics
    for diagnostic in diagnostics.iter() {
        if diagnostic.severity < min_severity {
            continue;
        }
        let file = get_source(diagnostic.file_id)
            .unwrap_or_else(|| panic!("no source for diagnostic: {diagnostic:?}"));
        let header = Color::Red.apply_bold(&format!("{}: {}", diagnostic.code, diagnostic.message));
        let body = annotate_source(file, &diagnostic.primary_span, options);
        console::error(&header);
        console::info(&body);
    }
}
