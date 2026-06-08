use std::sync::Arc;

use destack_source::{
    DiagnosticCollection, DiagnosticLabel, FileId, PrintOptions, print_diagnostics,
};
use destack_repository::{Repository, Revision};

/// Render one diagnostic collection as stable tripleslash rows.
pub(crate) fn render_diagnostics(
    repository: &Repository,
    revision: Revision,
    diagnostics: &DiagnosticCollection,
) -> String {
    let mut lines = Vec::new();

    for diagnostic in diagnostics.iter() {
        lines.push(format!(
            "/// @diagnostic.{} code={} message={}",
            diagnostic.severity.family_name(),
            diagnostic.code,
            quote(&diagnostic.message)
        ));

        lines.push(render_label(repository, revision, &diagnostic.primary));
    }

    lines.join("\n")
}

/// Render one diagnostic collection with source annotations.
pub(crate) fn render_source_diagnostics(
    repository: &Repository,
    revision: Revision,
    diagnostics: &DiagnosticCollection,
) -> String {
    let lines = Arc::new(parking_lot::Mutex::new(Vec::new()));
    let writer_lines = Arc::clone(&lines);
    let writer = Arc::new(move |line: &str| {
        writer_lines.lock().push(line.to_string());
    });
    let options = PrintOptions::new()
        .with_color(false)
        .with_line_writer(writer);
    let file_for_id = |file_id: FileId| repository.file(revision, file_id).ok().flatten();

    print_diagnostics(&file_for_id, diagnostics, options).expect("diagnostics should render");

    lines.lock().join("\n")
}

/// Render one diagnostic label.
fn render_label(repository: &Repository, revision: Revision, label: &DiagnosticLabel) -> String {
    let file = repository
        .file(revision, label.span.file)
        .expect("diagnostic snapshot file lookup should work")
        .expect("diagnostic snapshot file should exist");
    let (line, column) = file
        .get_position(label.span.start)
        .expect("diagnostic snapshot position should exist");
    let source = file.get_line_str(line).unwrap_or_default().trim();
    let line = line + 1;
    let column = column + 1;

    format!(
        "/// @diagnostic.label line={line} column={column} source={}",
        quote(source)
    )
}

/// Quote one snapshot field value.
fn quote(value: &str) -> String {
    format!("{value:?}")
}
