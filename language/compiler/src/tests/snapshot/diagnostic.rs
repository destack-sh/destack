use std::sync::Arc;

use tspp_repository::{Repository, Revision};
use tspp_source::{
    Applicability, DiagnosticCollection, DiagnosticLabel, DiagnosticSuggestion, File, FileId,
    PrintOptions, apply_file_patch, print_diagnostics,
};

/// Render one diagnostic collection as stable tripleslash rows.
pub(crate) fn render_diagnostics(
    repository: &Repository,
    revision: Revision,
    diagnostics: &DiagnosticCollection,
) -> String {
    let mut lines = Vec::new();

    for diagnostic in diagnostics.iter() {
        lines.push(format!(
            "/// @diagnostic.{} id={} message={}",
            diagnostic.severity.family_name(),
            diagnostic.id,
            quote(&diagnostic.message)
        ));
        let primary_file = diagnostic.primary.target.file();
        lines.push(render_label(
            repository,
            revision,
            &diagnostic.primary,
            "label",
            primary_file,
        ));

        // secondary labels follow their diagnostic
        for label in diagnostic.labels() {
            lines.push(render_label(
                repository,
                revision,
                label,
                "related",
                primary_file,
            ));
        }

        // notes and helps pin as plain messages
        for note in diagnostic.notes() {
            lines.push(format!(
                "/// @diagnostic.note message={}",
                quote(&note.message)
            ));
        }
        for help in diagnostic.helps() {
            lines.push(format!(
                "/// @diagnostic.help message={}",
                quote(&help.message)
            ));
        }

        // suggestions pin their patched output
        for suggestion in &diagnostic.suggestions {
            lines.push(render_suggestion(repository, revision, suggestion));
        }
    }

    lines.join("\n")
}

/// Render one suggestion with its patched lines.
fn render_suggestion(
    repository: &Repository,
    revision: Revision,
    suggestion: &DiagnosticSuggestion,
) -> String {
    let applicability = match suggestion.applicability {
        Applicability::Automatic => "automatic",
        Applicability::Unsafe => "unsafe",
        Applicability::Dangerous => "dangerous",
    };

    // apply each file patch and keep only the lines that changed
    let mut patched_lines = Vec::new();
    for file_patch in &suggestion.patches.files {
        let file = repository
            .file(revision, file_patch.file)
            .expect("suggestion snapshot file lookup should work")
            .expect("suggestion snapshot file should exist");
        let updated =
            apply_file_patch(&file, file_patch).expect("suggestion snapshot patch should apply");
        for (before, after) in file.text().lines().zip(updated.lines()) {
            if before != after {
                patched_lines.push(after.trim().to_string());
            }
        }
    }

    format!(
        "/// @diagnostic.suggestion message={} applicability={applicability} patched={}",
        quote(&suggestion.message),
        quote(&patched_lines.join("\n"))
    )
}

/// Render one diagnostic collection with source annotations.
pub(crate) fn render_source_diagnostics(
    repository: &Repository,
    revision: Revision,
    diagnostics: &DiagnosticCollection,
    use_color: bool,
) -> String {
    let lines = Arc::new(parking_lot::Mutex::new(Vec::new()));
    let writer_lines = Arc::clone(&lines);
    let writer = Arc::new(move |line: &str| {
        writer_lines.lock().push(line.to_string());
    });
    let options = PrintOptions::new()
        .with_color(use_color)
        .with_colorizer(tspp_parser::source_colorizer())
        .with_line_writer(writer);
    let file_for_id = |file_id: FileId| repository.file(revision, file_id).ok().flatten();

    print_diagnostics(&file_for_id, diagnostics, options).expect("diagnostics should render");

    lines.lock().join("\n")
}

/// Render one diagnostic label under one row tag.
fn render_label(
    repository: &Repository,
    revision: Revision,
    label: &DiagnosticLabel,
    tag: &str,
    primary_file: FileId,
) -> String {
    let file = repository
        .file(revision, label.target.file())
        .expect("diagnostic snapshot file lookup should work")
        .expect("diagnostic snapshot file should exist");
    let message = match &label.message {
        Some(message) if tag != "label" => format!(" message={}", quote(message)),
        _ => String::new(),
    };

    // whole-file labels pin by file only
    let Some(span) = label.target.span() else {
        return format!("/// @diagnostic.{tag} file={}{message}", quote(&file.name));
    };

    // labels in other files name their file
    let file_field = match label.target.file() == primary_file {
        true => String::new(),
        false => format!(" file={}", quote(&file.name)),
    };

    // load the exact source revision named by the diagnostic label
    let memory = repository
        .open_blob(label.blob)
        .expect("diagnostic snapshot Blob should load");
    let file = File::from_blob(
        file.id,
        file.name.clone(),
        file.uri.clone(),
        file.path.clone(),
        file.ty,
        memory,
    )
    .expect("diagnostic snapshot source should load");
    let (line, column) = file
        .get_position(span.start)
        .expect("diagnostic snapshot position should exist");
    let span = file.get_span_str(span).unwrap_or_default().trim();
    let line_source = file.get_line_str(line).unwrap_or_default().trim();
    let line = line + 1;
    let column = column + 1;

    format!(
        "/// @diagnostic.{tag}{file_field} line={line} column={column} span={} line_source={}{message}",
        quote(span),
        quote(line_source)
    )
}

/// Quote one snapshot field value.
fn quote(value: &str) -> String {
    format!("{value:?}")
}
