//! Globbing and glob-style pattern matching with no external dependencies.

use std::path::{Path, PathBuf};

/// Check if `text` matches a glob `pattern` supporting '*' and '?' wildcards.
///
/// Both pattern and text are treated as byte strings; '*' matches any sequence
/// (including directory separators) and '?' matches any single byte.
pub fn matches(pattern: &[u8], mut pattern_idx: usize, text: &[u8], mut text_idx: usize) -> bool {
    let pattern_length = pattern.len();
    let text_length = text.len();
    let mut star_idx: Option<usize> = None;
    let mut match_idx: usize = 0;

    while text_idx < text_length {
        if pattern_idx < pattern_length
            && (pattern[pattern_idx] == b'?' || pattern[pattern_idx] == text[text_idx])
        {
            pattern_idx += 1;
            text_idx += 1;
        } else if pattern_idx < pattern_length && pattern[pattern_idx] == b'*' {
            star_idx = Some(pattern_idx);
            match_idx = text_idx;
            pattern_idx += 1;
        } else if let Some(si) = star_idx {
            pattern_idx = si + 1;
            match_idx += 1;
            text_idx = match_idx;
        } else {
            return false;
        }
    }

    while pattern_idx < pattern_length && pattern[pattern_idx] == b'*' {
        pattern_idx += 1;
    }
    pattern_idx == pattern_length
}

/// Glob for file paths matching `pattern`, similar to the `glob` crate.
///
/// The `pattern` can be absolute or relative to the current working directory.
/// Wildcards '*' and '?' are supported. Directory separators may be matched by '*'.
/// Returns a vector of matching file paths. Errors during traversal are ignored.
pub fn glob(pattern: &str) -> Vec<PathBuf> {
    let (base_dir, normalized_pattern) = split_base_directory(pattern);
    let mut result: Vec<PathBuf> = Vec::new();

    // stack-based DFS to avoid recursion
    let mut directories_to_visit: Vec<PathBuf> = vec![base_dir.clone()];
    while let Some(directory) = directories_to_visit.pop() {
        let read_dir = match std::fs::read_dir(&directory) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for entry in read_dir.flatten() {
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if file_type.is_dir() {
                directories_to_visit.push(path.clone());
            }
            // match against unix-like text; absolute patterns are matched against
            // absolute strings, relative patterns against absolute strings too
            let candidate_text = unix_path(&path);
            if matches(
                normalized_pattern.as_bytes(),
                0,
                candidate_text.as_bytes(),
                0,
            ) {
                result.push(path);
            }
        }
    }
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
    let base_path = if base.is_empty() {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    } else {
        PathBuf::from(base)
    };
    (base_path, normalized)
}

fn unix_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;

    #[test]
    fn matches_basic_patterns() {
        assert!(matches(b"*.rs", 0, b"main.rs", 0));
        assert!(matches(b"src/*.rs", 0, b"src/lib.rs", 0));
        assert!(matches(b"src/*/mod.rs", 0, b"src/foo/mod.rs", 0));
        assert!(matches(b"?ain.rs", 0, b"main.rs", 0));
        assert!(!matches(b"*.rs", 0, b"main.py", 0));
    }

    #[test]
    fn glob_finds_paths() {
        let base = tempdir();
        let src = base.join("src");
        let _ = fs::create_dir_all(src.join("sub"));
        write_file(&src.join("lib.rs"), "");
        write_file(&src.join("main.rs"), "");
        write_file(&src.join("sub").join("mod.rs"), "");

        let pattern = format!("{}/**/*.rs", unix_path(&base));
        let mut paths = glob(&pattern);
        paths.sort();

        assert!(paths.iter().any(|p| p.ends_with("lib.rs")));
        assert!(paths.iter().any(|p| p.ends_with("main.rs")));
        assert!(paths.iter().any(|p| p.ends_with("mod.rs")));
    }

    fn write_file(path: &Path, content: &str) {
        let _ = fs::create_dir_all(path.parent().unwrap());
        let mut f = File::create(path).unwrap();
        let _ = f.write_all(content.as_bytes());
    }

    fn tempdir() -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "destack_fs_{}",
            std::time::SystemTime::now().elapsed().unwrap().as_nanos()
        ));
        let _ = fs::create_dir_all(&p);
        p
    }
}
