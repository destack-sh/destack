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
    let normalized = test_name.replace('\\', "/").to_ascii_lowercase();
    let segments: Vec<&str> = normalized.split('/').collect();

    // explicit error marker directories
    if segments.iter().copied().any(segment_is_error_marker) {
        return true;
    }

    // filenames in error suites often carry invalid or malformed prefixes
    let file_name = segments.last().copied().unwrap_or_default();
    let file_stem = file_name
        .split_once('.')
        .map_or(file_name, |(stem, _)| stem);
    file_stem_starts_with_error_marker(file_stem)
}

/// Return whether a path segment marks an error fixture bucket.
fn segment_is_error_marker(segment: &str) -> bool {
    matches!(
        segment,
        "error"
            | "errors"
            | "_errors_"
            | "invalid"
            | "malformed"
            | "fail"
            | "fails"
            | "failing"
            | "failure"
            | "syntax-error"
            | "syntax-errors"
            | "syntax_error"
            | "syntax_errors"
            | "early-error"
            | "early-errors"
            | "early_error"
            | "early_errors"
    ) || segment.starts_with("error-")
        || segment.ends_with("-error")
        || segment.starts_with("errors-")
        || segment.ends_with("-errors")
        || segment.starts_with("invalid-")
        || segment.ends_with("-invalid")
        || segment.starts_with("malformed-")
        || segment.ends_with("-malformed")
}

/// Return whether a file stem starts with a known error marker.
fn file_stem_starts_with_error_marker(file_stem: &str) -> bool {
    file_stem.starts_with("invalid-")
        || file_stem.starts_with("error-")
        || file_stem.starts_with("malformed-")
        || file_stem.starts_with("fail-")
}

/// Build a sibling path with an additional suffix.
pub(super) fn sibling_with_suffix(path: &Path, suffix: &str) -> Option<PathBuf> {
    let file_name = path.file_name()?.to_str()?;
    let sibling_name = format!("{file_name}{suffix}");
    Some(path.parent()?.join(sibling_name))
}

#[cfg(test)]
mod tests {
    use super::expect_error_from_path;

    #[test]
    fn test_expect_error_from_path_matches_error_directory() {
        assert!(expect_error_from_path(
            "js/_errors_/discard-binding/example.js"
        ));
        assert!(expect_error_from_path("js/errors/example.js"));
        assert!(expect_error_from_path("js/error/example.js"));
    }

    #[test]
    fn test_expect_error_from_path_matches_error_file_prefixes() {
        assert!(expect_error_from_path(
            "js/module/invalid-array-expression.js"
        ));
        assert!(expect_error_from_path("js/module/error-case.js"));
        assert!(expect_error_from_path("js/module/malformed-token.js"));
        assert!(expect_error_from_path("js/module/fail-case.js"));
    }

    #[test]
    fn test_expect_error_from_path_does_not_match_non_error_paths() {
        assert!(!expect_error_from_path("js/module/assignment/basic.js"));
        assert!(!expect_error_from_path(
            "typescript/type-parameters/variables.ts"
        ));
    }
}
