use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use destack_source::{DiffOptions, format_diff};

const BLESS_ENV: &str = "DESTACK_BLESS";

/// Line deltas from earlier rewrites, keyed by file and original caller line.
static BLESS_DELTAS: Mutex<Option<HashMap<PathBuf, Vec<(u32, i64)>>>> = Mutex::new(None);

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

/// Rewrite the raw snapshot nearest one call site.
fn bless_snapshot(file: &str, line: u32, expected: &str, actual: &str) {
    // recover from panics under the lock; they occur before any delta mutation
    let mut deltas = BLESS_DELTAS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let deltas = deltas.get_or_insert_with(HashMap::new);
    let path = source_path(file);
    let source = std::fs::read_to_string(&path).expect("read snapshot source");

    // correct the compiled caller line by earlier rewrites in this file
    let shift = deltas
        .get(&path)
        .map(|writes| {
            writes
                .iter()
                .filter(|(at, _)| *at < line)
                .map(|(_, delta)| *delta)
                .sum::<i64>()
        })
        .unwrap_or(0);
    let call_offset = line_offset(&source, (line as i64 + shift).max(1) as u32);
    let range = raw_strings(&source)
        .into_iter()
        .filter(|(start, end)| source[*start..*end].trim_matches('\n') == expected)
        .min_by_key(|(start, _)| start.abs_diff(call_offset))
        .unwrap_or_else(|| {
            panic!(
                "snapshot at {}:{line} must be one raw string",
                path.display()
            )
        });

    // preserve the conventional leading and trailing newline around snapshots
    let replacement = format!("\n{actual}\n");
    let removed = source[range.0..range.1].matches('\n').count() as i64;
    let added = replacement.matches('\n').count() as i64;
    deltas
        .entry(path.clone())
        .or_default()
        .push((line, added - removed));

    let mut source = source;
    source.replace_range(range.0..range.1, &replacement);
    std::fs::write(path, source).expect("write snapshot source");
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
    use super::raw_strings;

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
