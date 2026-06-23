use std::sync::Arc;

use destack_lsp_types as lsp;
use destack_source::{File, TextChange, TextPosition, TextRange, WATCHABLE_FILE_TYPES};
use destack_workspace::FileImage;

/// Globs for config files tracked by the LSP.
pub(super) const CONFIG_GLOBS: [&str; 1] = ["**/destack.json"];

/// Build a source file from an image payload.
pub(super) fn file_from_image(image: &FileImage) -> Option<Arc<File>> {
    let content = image.content.as_ref()?;
    let file = File::from_text(
        image.id,
        image.name.clone(),
        image.uri.clone(),
        image.path.clone(),
        image.file_type,
        content.to_string(),
    );

    Some(Arc::new(file))
}

/// Build file watcher patterns for the client.
pub(super) fn file_watchers() -> Vec<lsp::FileSystemWatcher> {
    let mut watchers = Vec::new();
    for pattern in tracked_file_globs() {
        watchers.push(lsp::FileSystemWatcher {
            glob_pattern: pattern.to_string().into(),
            kind: None,
        });
    }

    watchers
}

/// Build the set of file globs tracked by the LSP.
pub(super) fn tracked_file_globs() -> Vec<&'static str> {
    let mut patterns = Vec::new();
    for file_type in WATCHABLE_FILE_TYPES {
        for pattern in file_type.globs() {
            if !patterns.contains(pattern) {
                patterns.push(pattern);
            }
        }
    }

    // append config globs
    for pattern in CONFIG_GLOBS {
        if !patterns.contains(&pattern) {
            patterns.push(pattern);
        }
    }

    patterns
}

/// Convert LSP text changes into source text changes.
pub(super) fn text_changes_from_lsp(
    changes: Vec<lsp::TextDocumentContentChangeEvent>,
) -> Vec<TextChange> {
    changes
        .into_iter()
        .map(|change| TextChange {
            range: change.range.map(|range| TextRange {
                start: TextPosition {
                    line: range.start.line,
                    character: range.start.character,
                },
                end: TextPosition {
                    line: range.end.line,
                    character: range.end.character,
                },
            }),
            text: change.text,
        })
        .collect()
}

/// Normalize line endings to LF.
pub(super) fn normalize_line_endings(content: String) -> String {
    if content.contains('\r') {
        content.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        content
    }
}
