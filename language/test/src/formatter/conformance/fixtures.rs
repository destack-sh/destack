use std::path::{Path, PathBuf};

use destack_source::FileType;

/// Return whether a file type is in formatter conformance scope.
pub(super) fn is_formattable_file_type(file_type: FileType) -> bool {
    matches!(
        file_type,
        FileType::JavaScript
            | FileType::JavaScriptXml
            | FileType::TypeScript
            | FileType::TypeScriptXml
            | FileType::TypeScriptDeclaration
    )
}

/// Return whether a directory should be ignored during fixture discovery.
pub(super) fn should_skip_directory(name: &str) -> bool {
    name.starts_with('.') || name == "node_modules" || name == "__snapshots__" || name == "staging"
}

/// Return whether a fixture file should be ignored.
pub(super) fn should_skip_fixture_file(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");

    // ignore snapshot and helper files
    if file_name.ends_with(".snap")
        || file_name.ends_with(".prettier-snap")
        || file_name.ends_with(".snap.md")
        || file_name.ends_with(".snap-original")
    {
        return true;
    }

    // ignore suite runner helper scripts
    if matches!(file_name, "format.test.js" | "mod.rs" | "lib.rs") {
        return true;
    }

    false
}

/// Return whether a test path likely expects parser failure.
pub(super) fn expect_error_from_path(test_name: &str) -> bool {
    test_name.contains("/error/")
        || test_name.contains("/errors/")
        || test_name.contains("/invalid/")
        || test_name.contains("/malformed/")
        || test_name.contains("/fail/")
}

/// Build a sibling path with an additional suffix.
pub(super) fn sibling_with_suffix(path: &Path, suffix: &str) -> Option<PathBuf> {
    let file_name = path.file_name()?.to_str()?;
    let sibling_name = format!("{file_name}{suffix}");
    Some(path.parent()?.join(sibling_name))
}
