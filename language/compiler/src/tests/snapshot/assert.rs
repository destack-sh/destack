use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use destack_source::{DiffOptions, format_diff};

const BLESS_ENV: &str = "DESTACK_BLESS";

/// Original file contents at first bless, keyed by file, anchoring caller lines.
static BLESS_ORIGINALS: Mutex<Option<HashMap<PathBuf, String>>> = Mutex::new(None);

/// Assert one complete inline snapshot and bless its raw literal when requested.
#[track_caller]
pub(crate) fn assert_snapshot(actual: impl AsRef<str>, expected: &str) {
    let actual = actual.as_ref().trim_matches('\n');
    let expected = expected.trim_matches('\n');

    assert_snapshot_at(actual, expected, expected);
}

/// Assert one formatted inline snapshot and bless its original raw literal when requested.
#[track_caller]
pub(crate) fn assert_formatted_snapshot(actual: impl AsRef<str>, expected: &str, formatted: &str) {
    let actual = actual.as_ref().trim_matches('\n');
    let expected = expected.trim_matches('\n');
    let formatted = formatted.trim_matches('\n');

    assert_snapshot_at(actual, expected, formatted);
}

/// Compare one snapshot while retaining its source literal for blessing.
#[track_caller]
fn assert_snapshot_at(actual: &str, expected: &str, compared: &str) {
    if actual == compared {
        return;
    }

    // rewrite the caller's expected raw literal in bless mode
    if is_blessing() {
        let caller = std::panic::Location::caller();
        bless_snapshot(caller.file(), caller.line(), expected, actual);

        return;
    }

    let diff = format_diff(compared, actual, &DiffOptions::new());

    panic!("snapshot mismatch\n\n{diff}");
}

/// Return whether compiler snapshots should be blessed.
fn is_blessing() -> bool {
    std::env::var_os(BLESS_ENV).is_some_and(|value| !value.is_empty() && value != "0")
}

/// Rewrite the expected raw snapshot the calling test owns.
///
/// The caller's compiled line resolves to its enclosing test function in the
/// original file, and the rewrite targets that function's region in the
/// current file by name, so no rewrite can land in another test.
fn bless_snapshot(file: &str, line: u32, expected: &str, actual: &str) {
    // recover from panics under the lock; they occur before any mutation
    let mut originals = BLESS_ORIGINALS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let originals = originals.get_or_insert_with(HashMap::new);
    let path = source_path(file);
    let original = originals
        .entry(path.clone())
        .or_insert_with(|| std::fs::read_to_string(&path).expect("read snapshot source"))
        .clone();

    // name the calling test from the compiled caller line
    let call_offset = line_offset(&original, line);
    let name = enclosing_function_name(&original, call_offset).unwrap_or_else(|| {
        panic!(
            "snapshot at {}:{line} must sit inside one test function",
            path.display()
        )
    });

    // bound the rewrite to the named function's current region
    let source = std::fs::read_to_string(&path).expect("read snapshot source");
    let header = format!("fn {name}(");
    let region_start = source.find(&header).unwrap_or_else(|| {
        panic!(
            "snapshot function {name} is missing from {}",
            path.display()
        )
    });
    let region_end = source[region_start + header.len()..]
        .find("\nfn ")
        .map(|at| region_start + header.len() + at)
        .unwrap_or(source.len());
    let range = select_snapshot_range(&source, region_start, region_end, expected)
        .unwrap_or_else(|| {
            let candidates = raw_strings(&source)
                .into_iter()
                .filter(|(start, end)| *start >= region_start && *end <= region_end)
                .map(|(start, end)| {
                    let content = source[start..end].trim_matches('\n');
                    format!("  [{}..{}] {:?}", start, end, &content[..content.len().min(60)])
                })
                .collect::<Vec<_>>()
                .join("\n");
            panic!(
                "snapshot at {}:{line} must be one raw string in {name}\nexpected {:?}\nregion candidates:\n{candidates}",
                path.display(),
                &expected[..expected.len().min(60)],
            )
        });

    // preserve the conventional leading and trailing newline around snapshots
    let replacement = format!("\n{actual}\n");
    let mut source = source;
    source.replace_range(range.0..range.1, &replacement);
    std::fs::write(path, source).expect("write snapshot source");
}

/// Return the name of the function enclosing one source offset.
fn enclosing_function_name(source: &str, offset: usize) -> Option<String> {
    let start = source[..offset].rfind("\nfn ")?;
    let name = source[start + 4..]
        .chars()
        .take_while(|character| character.is_alphanumeric() || *character == '_')
        .collect::<String>();

    (!name.is_empty()).then_some(name)
}

/// Return the workspace path for one compiler source location.
fn source_path(file: &str) -> PathBuf {
    let path = Path::new(file);
    if path.is_absolute() {
        return path.to_path_buf();
    }

    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

/// Return the expected raw string one test region owns.
fn select_snapshot_range(
    source: &str,
    region_start: usize,
    region_end: usize,
    expected: &str,
) -> Option<(usize, usize)> {
    raw_strings(source)
        .into_iter()
        .filter(|(start, end)| *start >= region_start && *end <= region_end)
        .filter(|(start, end)| source[*start..*end].trim_matches('\n') == expected)
        .min_by_key(|(start, _)| *start)
}

/// Return the byte offset of one-based source line.
fn line_offset(source: &str, line: u32) -> usize {
    source
        .match_indices('\n')
        .nth(line.saturating_sub(2) as usize)
        .map_or(0, |(offset, _)| offset + 1)
}

/// Return the content ranges of every raw string literal.
fn raw_strings(source: &str) -> Vec<(usize, usize)> {
    let bytes = source.as_bytes();
    let mut ranges = Vec::new();
    let mut cursor = 0;

    // scan raw string delimiters without interpreting Rust tokens
    while cursor < bytes.len() {
        if bytes[cursor] != b'r' {
            cursor += 1;

            continue;
        }

        let mut quote = cursor + 1;
        while bytes.get(quote) == Some(&b'#') {
            quote += 1;
        }
        if bytes.get(quote) != Some(&b'"') {
            cursor += 1;

            continue;
        }

        let hashes = quote - cursor - 1;
        let content = quote + 1;
        let Some(end) = raw_string_end(bytes, content, hashes) else {
            cursor += 1;

            continue;
        };

        ranges.push((content, end));
        cursor = end + hashes + 1;
    }

    ranges
}

/// Return the closing quote of one raw string literal.
fn raw_string_end(bytes: &[u8], mut cursor: usize, hashes: usize) -> Option<usize> {
    while cursor < bytes.len() {
        let delimiter_end = cursor + 1 + hashes;
        if bytes[cursor] == b'"'
            && delimiter_end <= bytes.len()
            && bytes[cursor + 1..delimiter_end]
                .iter()
                .all(|byte| *byte == b'#')
        {
            return Some(cursor);
        }

        cursor += 1;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{raw_strings, select_snapshot_range};

    /// Skip an identical literal that belongs to another test region.
    #[test]
    fn test_select_the_expected_literal_inside_the_region() {
        let source = r####"
first(r#"
"#);
second(r#"
"#);
"####;
        let region_start = source.find("second").unwrap();
        let range = select_snapshot_range(source, region_start, source.len(), "").unwrap();

        assert!(range.0 > region_start);
    }

    /// Find ordinary and hash-delimited raw string contents.
    #[test]
    fn test_find_raw_strings() {
        let source = r####"let a = r"one"; let b = r###"two "# three"###;"####;
        let contents = raw_strings(source)
            .into_iter()
            .map(|(start, end)| &source[start..end])
            .collect::<Vec<_>>();

        assert_eq!(contents, ["one", "two \"# three"]);
    }
}
