use std::collections::HashMap;

use destack_terminal::console;
use dyst_source::{
    AnnotateOptions, Color, DiagnosticCollector, File, FileId, LanguageOptions, annotate_source,
};

/// Print diagnostics (and suggestions) to the console.
pub(crate) fn print_diagnostics(
    source_by_id: &HashMap<FileId, &File>,
    language: LanguageOptions,
    diagnostics: &DiagnosticCollector,
) {
    let options = AnnotateOptions {
        max_line_width: language.formatting.line_width as u32,
        prefix_lines: 1,
        suffix_lines: 1,
        use_color: true,
    };

    // print diagnostics
    for diagnostic in diagnostics.iter() {
        let source = source_by_id
            .get(&diagnostic.source)
            .unwrap_or_else(|| panic!("no source for diagnostic: {diagnostic:?}"));
        let annotated = annotate_source(source, &diagnostic.primary_span, options);
        let diagnostic_header =
            Color::Red.apply_bold(&format!("{}: {}", diagnostic.code, diagnostic.message));
        console::error(&diagnostic_header);
        console::info(&annotated);
    }
}
