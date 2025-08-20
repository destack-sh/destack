use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::glob::matches_glob;
use crate::ignore::IgnoreSet;

const FILE_BUFFER_SIZE: usize = 512 * 1024; // 512KB

/// Describe how to identify a language.
#[derive(Debug, Clone)]
pub struct LanguageConfiguration {
    /// Logical name (e.g., "Rust").
    pub name: String,
    /// File endings including dot (e.g., [".rs"]).
    pub endings: Vec<String>,
    /// Optional comment markers.
    pub comment: Option<CommentStyle>,
}

/// Comment style markers.
#[derive(Debug, Clone)]
pub struct CommentStyle {
    /// Single-line comment prefixes (e.g., ["//", "#"]).
    pub line: Vec<String>,
    /// Optional block comment delimiters (start, end) pairs (e.g., [("/*", "*/")]).
    pub block: Vec<(String, String)>,
}

/// Options for counting.
#[derive(Debug, Clone)]
pub struct Options {
    /// Root directory to scan.
    pub root: PathBuf,
    /// Languages to consider.
    pub languages: Vec<LanguageConfiguration>,
    /// Optional glob-like file patterns to include (e.g., ["**/*.rs", "*.py"]).
    pub patterns: Vec<String>,
    /// Path names to skip during traversal (exact match).
    pub ignore_paths: Vec<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            languages: Vec::new(),
            patterns: Vec::new(),
            ignore_paths: Vec::new(),
        }
    }
}

/// Result statistics.
#[derive(Debug, Clone)]
pub struct Statistics {
    /// Total number of files matched.
    pub total_files: u64,
    /// Total number of lines across matched files.
    pub total_lines: u64,
    /// Lines by language name.
    pub lines_by_language: HashMap<String, u64>,
    /// Files by language name.
    pub files_by_language: HashMap<String, u64>,
}

impl Statistics {
    /// Create new empty Statistics.
    pub(crate) fn new() -> Self {
        Self {
            total_files: 0,
            total_lines: 0,
            lines_by_language: HashMap::new(),
            files_by_language: HashMap::new(),
        }
    }
}

/// Count lines according to options.
///
/// Scans `options.root` recursively, matches files by extension suffix against
/// provided `LanguageConfiguration.endings`, and counts lines including comments.
/// Comment configuration is accepted for future use.
pub fn count(options: &Options) -> io::Result<Statistics> {
    let mut statistics = Statistics::new();

    // extension -> language name map for quick lookup
    let mut ext_to_lang: Vec<(String, &str)> = Vec::new();
    for lang in &options.languages {
        for e in &lang.endings {
            ext_to_lang.push((e.clone(), lang.name.as_str()));
        }
    }

    // bail if no languages, nothing to do
    if ext_to_lang.is_empty() {
        return Ok(statistics);
    }

    // walk the directory tree non-recursively (using a manual stack)
    let mut dir_stack: Vec<PathBuf> = vec![options.root.clone()];
    let mut ignore_set = IgnoreSet::new();
    while let Some(dir) = dir_stack.pop() {
        ignore_set.load_dir(&dir);
        let read_dir = match std::fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for entry in read_dir {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(t) => t,
                Err(_) => continue,
            };
            let is_dir = file_type.is_dir();
            if ignore_set.is_ignored(&options.root, &path, is_dir) {
                continue;
            }

            // skip configured directories
            if is_dir {
                if let Some(name) = path.file_name().and_then(|s| s.to_str())
                    && options.ignore_paths.iter().any(|d| d == name)
                {
                    continue;
                }
                dir_stack.push(path);
                continue;
            }
            if !file_type.is_file() {
                continue;
            }

            // optional pre-filter by glob patterns
            if !options.patterns.is_empty() {
                let text = path.to_string_lossy();
                if !options
                    .patterns
                    .iter()
                    .any(|p| matches_glob(p.as_bytes(), 0, text.as_bytes(), 0))
                {
                    continue;
                }
            }

            // count lines for matching language
            if let Some((lang_name, _)) = match_language(&path, &ext_to_lang) {
                let lines = count_file_lines(&path).unwrap_or_default();
                statistics.total_files += 1;
                statistics.total_lines += lines;
                *statistics
                    .lines_by_language
                    .entry(lang_name.to_string())
                    .or_insert(0) += lines;
                *statistics
                    .files_by_language
                    .entry(lang_name.to_string())
                    .or_insert(0) += 1;
            }
        }
    }

    Ok(statistics)
}

/// Find matching language for file path by checking endings.
fn match_language<'a>(
    path: &Path,
    ext_to_language: &'a [(String, &str)],
) -> Option<(&'a str, &'a str)> {
    let fname = path.file_name()?.to_str()?;
    for (ending, lang) in ext_to_language.iter() {
        if fname.ends_with(ending) {
            return Some((*lang, ending.as_str()));
        }
    }
    None
}

/// Count lines in a single file.
fn count_file_lines(path: &Path) -> io::Result<u64> {
    let f = File::open(path)?;
    let mut reader = BufReader::with_capacity(FILE_BUFFER_SIZE, f);
    let mut buf = String::new();
    let mut count: u64 = 0;
    loop {
        buf.clear();
        let n = reader.read_line(&mut buf)?;
        if n == 0 {
            break;
        }
        count += 1;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::Instant;

    #[test]
    fn test_count_simple() {
        // count lines in simple test files
        let tmp = tempfile_dir();
        let a = tmp.join("a.rs");
        let b = tmp.join("b.py");
        write_file(&a, "line1\nline2\n");
        write_file(&b, "# c\nprint()\n");

        let options = Options {
            root: tmp.clone(),
            languages: vec![
                LanguageConfiguration {
                    name: "rs".into(),
                    endings: vec![".rs".into()],
                    comment: None,
                },
                LanguageConfiguration {
                    name: "py".into(),
                    endings: vec![".py".into()],
                    comment: None,
                },
            ],
            patterns: Vec::new(),
            ignore_paths: Vec::new(),
        };
        let s = count(&options).unwrap();

        assert_eq!(s.total_files, 2);
        assert_eq!(s.total_lines, 4);
        assert_eq!(*s.lines_by_language.get("rs").unwrap(), 2);
        assert_eq!(*s.lines_by_language.get("py").unwrap(), 2);
    }

    /// Write content to file, creating parent directories as needed.
    fn write_file(path: &Path, content: &str) {
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        let mut f = File::create(path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    /// Create unique temporary directory for testing.
    fn tempfile_dir() -> PathBuf {
        let base = std::env::temp_dir();
        let mut p = base.clone();
        let unique = format!("destack_tokei_{}", Instant::now().elapsed().as_nanos());
        p.push(unique);
        let _ = std::fs::create_dir_all(&p);
        p
    }
}
