//! Provide glob-style pattern matching without pulling additional crates.

use std::path::PathBuf;

use crate::walk::{WalkOptions, walk};

/// Match a glob-style `pattern` against raw `text` bytes.
/// The matcher works over byte slices so callers can supply UTF-8 or filesystem-encoded data:
/// - `*` keeps its classic greedy semantics
/// - `?` matches a single byte, and
/// - `**/` segment optionally consumes a directory
/// - separator so that `**/*.rs` matches files in the root directory as well as nested subdirectories.
pub fn matches(pattern: &[u8], mut pattern_idx: usize, text: &[u8], mut text_idx: usize) -> bool {
    let pattern_length = pattern.len();
    let text_length = text.len();
    let mut star_idx: Option<usize> = None;
    let mut match_idx: usize = 0;

    // match pattern against text
    while text_idx < text_length {
        // handle "**/" at root or after a slash
        if pattern_idx + 2 < pattern_length
            && pattern[pattern_idx] == b'*'
            && pattern[pattern_idx + 1] == b'*'
            && pattern[pattern_idx + 2] == b'/'
            && (pattern_idx == 0 || pattern[pattern_idx - 1] == b'/')
        {
            // try the branch where we skip the "/" after a double star
            if matches(pattern, pattern_idx + 3, text, text_idx) {
                return true;
            }
        }

        // match single character or '?'
        if pattern_idx < pattern_length
            && (pattern[pattern_idx] == b'?' || pattern[pattern_idx] == text[text_idx])
        {
            pattern_idx += 1;
            text_idx += 1;
        }
        // handle '*'
        else if pattern_idx < pattern_length && pattern[pattern_idx] == b'*' {
            star_idx = Some(pattern_idx);
            match_idx = text_idx;
            pattern_idx += 1;
        }
        // backtrack to last '*' if needed
        else if let Some(si) = star_idx {
            pattern_idx = si + 1;
            match_idx += 1;
            text_idx = match_idx;
        }
        // no match
        else {
            return false;
        }
    }

    // skip trailing '*' in pattern
    while pattern_idx < pattern_length && pattern[pattern_idx] == b'*' {
        pattern_idx += 1;
    }
    pattern_idx == pattern_length
}

/// Collect filesystem entries matching a glob `pattern`.
///
/// The walker honours ignore rules through [`walk`]. Patterns are processed
/// using [`matches`], so callers automatically get the extended `**/` semantics
/// at the workspace root.
pub fn glob(pattern: &str) -> Vec<PathBuf> {
    // split pattern into base directory and normalized pattern
    let (base_dir, normalized_pattern) = split_base_directory(pattern);
    // set up walk options for traversal
    let walk_options = WalkOptions {
        root: base_dir,
        ignore: None,
        glob: Some(vec![normalized_pattern]),
    };
    let mut result = Vec::new();
    // collect all matching paths
    walk(&walk_options, |path| result.push(path.to_path_buf()));
    result
}

/// Extract a base directory prefix without wildcards to limit traversal.
fn split_base_directory(pattern: &str) -> (PathBuf, String) {
    // normalize separators to '/'
    let normalized = pattern.replace('\\', "/");

    // find earliest wildcard position
    let mut first_wildcard: Option<usize> = None;
    for (idx, ch) in normalized.char_indices() {
        if ch == '*' || ch == '?' {
            first_wildcard = Some(idx);
            break;
        }
    }
    let scan_upto = first_wildcard.unwrap_or(normalized.len());
    let prefix = &normalized[..scan_upto];

    // base is up to the last '/'
    let base_end = prefix.rfind('/').map(|i| i + 1).unwrap_or(0);
    let base = if base_end > 0 {
        &normalized[..base_end]
    } else {
        ""
    };

    // use current directory if no base found
    let base_path = if base.is_empty() {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    } else {
        PathBuf::from(base)
    };
    (base_path, normalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;

    #[test]
    /// Exercise simple wildcard cases.
    fn test_matches_basic_patterns() {
        assert!(matches(b"*.rs", 0, b"main.rs", 0));
        assert!(matches(b"src/*.rs", 0, b"src/lib.rs", 0));
        assert!(matches(b"src/*/mod.rs", 0, b"src/foo/mod.rs", 0));
        assert!(matches(b"?ain.rs", 0, b"main.rs", 0));
        assert!(!matches(b"*.rs", 0, b"main.py", 0));
        assert!(matches(b"**/*.d.ds", 0, b"src/foo/bar/declaration.d.ds", 0));
        assert!(!matches(b"**/*.ds", 0, b"src/foo/bar/declarationd.d.ds", 0));
    }

    #[test]
    /// Exercise glob traversal including recursive and root-level matches.
    fn test_glob_finds_paths() {
        // create a temporary directory structure
        let base = tempdir();
        let src = base.join("src");
        let _ = fs::create_dir_all(src.join("sub"));
        write_file(&base.join("root.rs"), "");
        write_file(&src.join("lib.rs"), "");
        write_file(&src.join("main.rs"), "");
        write_file(&src.join("sub").join("mod.rs"), "");

        // search for all .rs files recursively
        let pattern = format!("{}/**/*.rs", base.to_string_lossy());
        let mut paths = glob(&pattern);
        paths.sort();

        // check that all expected files are found
        assert!(paths.iter().any(|p| p.ends_with("root.rs")));
        assert!(paths.iter().any(|p| p.ends_with("lib.rs")));
        assert!(paths.iter().any(|p| p.ends_with("main.rs")));
        assert!(paths.iter().any(|p| p.ends_with("mod.rs")));
    }

    #[test]
    /// Validate root-level double star matching without a directory separator.
    fn test_matches_double_star_root_level() {
        assert!(matches(b"**/*.rs", 0, b"file.rs", 0));
        assert!(matches(b"**/*.rs", 0, b"dir/file.rs", 0));
        assert!(!matches(b"**/*.rs", 0, b"file.py", 0));
    }

    fn write_file(path: &Path, content: &str) {
        let _ = fs::create_dir_all(path.parent().unwrap());
        let mut f = File::create(path).unwrap();
        let _ = f.write_all(content.as_bytes());
    }

    fn tempdir() -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "destack_file_{}",
            std::time::SystemTime::now().elapsed().unwrap().as_nanos()
        ));
        let _ = fs::create_dir_all(&p);
        p
    }
}
