use std::io;
use std::path::PathBuf;

use crate::PhysicalFileSystem;

use super::{WalkOptions, walk};

/// Match a glob-style `pattern` against raw `text` bytes.
///
/// `*` and `?` remain within one path segment, while `**` crosses path separators.
/// `**/` may consume zero directories, so `**/*.rs` also matches root files.
pub fn matches(pattern: &[u8], text: &[u8]) -> bool {
    matches_at(pattern, 0, text, 0, false)
}

/// Return whether a pattern can match the given text or one of its descendants.
pub fn matches_prefix(pattern: &[u8], text: &[u8]) -> bool {
    matches_at(pattern, 0, text, 0, true)
}

/// Match pattern and text suffixes at exact byte indices.
fn matches_at(
    pattern: &[u8],
    mut pattern_index: usize,
    text: &[u8],
    mut text_index: usize,
    is_prefix: bool,
) -> bool {
    let pattern_length = pattern.len();
    let text_length = text.len();
    let mut star_index: Option<usize> = None;
    let mut star_width: usize = 0;
    let mut match_index: usize = 0;

    // match pattern against text
    while text_index < text_length {
        // match the remaining pattern at every directory depth
        let is_recursive_directory = pattern_index + 2 < pattern_length
            && pattern[pattern_index] == b'*'
            && pattern[pattern_index + 1] == b'*'
            && is_separator(pattern[pattern_index + 2])
            && (pattern_index == 0 || is_separator(pattern[pattern_index - 1]));
        if is_recursive_directory {
            let remaining_pattern = pattern_index + 3;

            // try the current directory
            if matches_at(pattern, remaining_pattern, text, text_index, is_prefix) {
                return true;
            }

            // try each descendant directory
            while text_index < text_length {
                if is_separator(text[text_index])
                    && matches_at(pattern, remaining_pattern, text, text_index + 1, is_prefix)
                {
                    return true;
                }
                text_index += 1;
            }

            return false;
        }

        // match single character or '?'
        if pattern_index < pattern_length
            && (pattern[pattern_index] == text[text_index]
                || (is_separator(pattern[pattern_index]) && is_separator(text[text_index]))
                || (pattern[pattern_index] == b'?' && !is_separator(text[text_index])))
        {
            pattern_index += 1;
            text_index += 1;
        }
        // "**" matches any number of bytes, including '.'
        else if pattern_index + 1 < pattern_length
            && pattern[pattern_index] == b'*'
            && pattern[pattern_index + 1] == b'*'
        {
            star_index = Some(pattern_index);
            star_width = 2;
            match_index = text_index;
            pattern_index += 2;
        }
        // match within one path segment with '*'
        else if pattern_index < pattern_length && pattern[pattern_index] == b'*' {
            star_index = Some(pattern_index);
            star_width = 1;
            match_index = text_index;
            pattern_index += 1;
        }
        // backtrack to last '*' if needed
        else if let Some(star_pattern_index) = star_index {
            if match_index >= text_length {
                return false;
            }

            let matched = text[match_index];
            let is_single_star_end = star_width == 1 && is_separator(matched);
            if is_single_star_end {
                return false;
            }

            pattern_index = star_pattern_index + star_width;
            match_index += 1;
            text_index = match_index;
        }
        // no match
        else {
            return false;
        }
    }

    // accept remaining pattern bytes when matching a directory prefix
    if is_prefix {
        return true;
    }

    // skip trailing '*' in pattern
    while pattern_index < pattern_length && pattern[pattern_index] == b'*' {
        pattern_index += 1;
    }

    pattern_index == pattern_length
}

/// Return whether one byte separates path segments.
fn is_separator(byte: u8) -> bool {
    matches!(byte, b'/' | b'\\')
}

/// Collect filesystem entries matching a glob `pattern`.
pub fn glob(pattern: &str) -> io::Result<Vec<PathBuf>> {
    // split pattern into base directory and normalized pattern
    let (base_directory, normalized_pattern) = split_base_directory(pattern)?;

    // set up walk options for traversal
    let walk_options = WalkOptions {
        root: base_directory,
        ignore: None,
        glob: Some(vec![normalized_pattern]),
    };
    let mut paths = Vec::new();

    // collect all matching paths
    walk(&PhysicalFileSystem, &walk_options, |path| {
        paths.push(path.to_path_buf());
    })?;

    Ok(paths)
}

/// Extract a base directory prefix without wildcards to limit traversal.
fn split_base_directory(pattern: &str) -> io::Result<(PathBuf, String)> {
    // normalize separators to '/'
    let normalized = pattern.replace('\\', "/");

    // find earliest wildcard position
    let mut first_wildcard = None;
    for (index, character) in normalized.char_indices() {
        if matches!(character, '*' | '?') {
            first_wildcard = Some(index);
            break;
        }
    }
    let prefix_end = first_wildcard.unwrap_or(normalized.len());
    let prefix = &normalized[..prefix_end];

    // retain complete directory segments
    let base_end = prefix.rfind('/').map(|index| index + 1).unwrap_or(0);
    let base = if base_end > 0 {
        &normalized[..base_end]
    } else {
        ""
    };

    // use current directory if no base found
    let base_directory = if base.is_empty() {
        std::env::current_dir()?
    } else {
        PathBuf::from(base)
    };

    Ok((base_directory, normalized))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    use crate::{FileSystem, TemporaryPhysicalFileSystem};

    /// Match wildcards within and across path segments.
    #[test]
    fn test_matches_basic_patterns() {
        assert!(matches(b"*.rs", b"main.rs"));
        assert!(matches(b"src/*.rs", b"src/lib.rs"));
        assert!(matches(b"src/*/mod.rs", b"src/foo/mod.rs"));
        assert!(matches(b"src/*/mod.rs", b"src/foo.bar/mod.rs"));
        assert!(matches(b"src\\*\\mod.rs", b"src/foo/mod.rs"));
        assert!(matches(b"bridge/*", b"bridge/zed"));
        assert!(!matches(b"bridge/*", b"bridge/zed/grammars/destack"));
        assert!(matches(b"bridge/**", b"bridge/zed/grammars/destack"));
        assert!(matches(b"?ain.rs", b"main.rs"));
        assert!(!matches(b"*.rs", b"main.py"));
        assert!(matches(b"**/*.d.tspp", b"src/foo/bar/declaration.d.tspp"));
        assert!(!matches(b"**/*.tspp", b"src/foo/bar/declaration.ts"));
    }

    /// Match directory prefixes that can reach complete glob matches.
    #[test]
    fn test_matches_pattern_prefixes() {
        assert!(matches_prefix(b"language/*", b"language"));
        assert!(matches_prefix(b"language/*", b"language/library"));
        assert!(!matches_prefix(b"language/*", b"language/library/src"));
        assert!(!matches_prefix(b"language/*", b".claude"));
        assert!(matches_prefix(b"**/package/*", b"vendor/project/package"));
    }

    /// Find root and nested files through one recursive pattern.
    #[test]
    fn test_glob_finds_paths() {
        // create a temporary directory structure
        let fs = TemporaryPhysicalFileSystem::new_with_prefix("file_glob");
        fs.create_dir_all(Path::new("src/sub"))
            .expect("create glob directories");
        fs.create_dir_all(Path::new(".git/info"))
            .expect("create Git directories");
        fs.create_dir_all(Path::new(".claude/worktrees/checkout"))
            .expect("create ignored directories");
        fs.write_text_or_error("root.rs", "");
        fs.write_text_or_error("src/lib.rs", "");
        fs.write_text_or_error("src/main.rs", "");
        fs.write_text_or_error("src/sub/mod.rs", "");
        fs.write_text_or_error(".git/info/exclude", "**/.claude/worktrees/\n");
        fs.write_text_or_error(".claude/worktrees/checkout/foreign.rs", "");

        // search for all .rs files recursively
        let pattern = format!("{}/**/*.rs", fs.root().to_string_lossy());
        let mut paths = glob(&pattern).expect("collect matching paths");
        paths.sort();

        // compare the complete path selection
        let mut expected = vec![
            fs.path_for("root.rs"),
            fs.path_for("src/lib.rs"),
            fs.path_for("src/main.rs"),
            fs.path_for("src/sub/mod.rs"),
        ];
        expected.sort();
        assert_eq!(paths, expected);
    }

    /// Match a recursive pattern at root and nested levels.
    #[test]
    fn test_matches_double_star_root_level() {
        assert!(matches(b"**/*.rs", b"file.rs"));
        assert!(matches(b"**/*.rs", b"dir/file.rs"));
        assert!(!matches(b"**/*.rs", b"file.py"));
    }

    /// Match dotted files beneath a recursive directory pattern.
    #[test]
    fn test_matches_directory_exclude_with_trailing_double_star() {
        assert!(matches(
            b"**/fixtures/**",
            b"packages/astro/e2e/fixtures/errors/src/components/JSSyntaxError.js"
        ));
        assert!(matches(
            b"**/fixtures/**",
            b"fixtures/error-case/index.test.tsx"
        ));
    }

    /// Find dotted files beneath a recursive directory pattern.
    #[test]
    fn test_glob_finds_fixture_paths_with_extensions() {
        // create a fixture directory with extension based files
        let fs = TemporaryPhysicalFileSystem::new_with_prefix("file_glob_fixtures");
        fs.create_dir_all(Path::new(
            "packages/astro/e2e/fixtures/errors/src/components",
        ))
        .expect("create fixture directories");
        fs.write_text_or_error(
            "packages/astro/e2e/fixtures/errors/src/components/JSSyntaxError.js",
            "",
        );

        // collect all files under any fixtures directory
        let pattern = format!("{}/**/fixtures/**", fs.root().to_string_lossy());
        let paths = glob(&pattern).expect("collect matching paths");

        // compare the complete path selection
        let expected =
            vec![fs.path_for("packages/astro/e2e/fixtures/errors/src/components/JSSyntaxError.js")];
        assert_eq!(paths, expected);
    }

    /// Report a missing traversal root.
    #[test]
    fn test_glob_reports_missing_root() {
        let fs = TemporaryPhysicalFileSystem::new_with_prefix("file_glob_missing");
        let pattern = format!("{}/missing/**/*.tspp", fs.root().to_string_lossy());

        let error = glob(&pattern).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    }
}
