use std::path::{Path, PathBuf};

use super::validate_query_path;

/// One strict unified patch for a query fixture file.
#[derive(Debug, Clone)]
pub(super) struct QueryPatch {
    /// The workspace-relative file path.
    pub(super) path: PathBuf,
    /// The ordered unified patch hunks.
    hunks: Vec<PatchHunk>,
}

impl QueryPatch {
    /// Parse one strict unified patch block.
    pub(super) fn parse(language: &str, source: &str) -> Result<Self, String> {
        let path = parse_patch_path(language)?;
        let mut hunks = Vec::new();
        let mut lines = source.split_inclusive('\n').peekable();

        // parse every complete hunk
        while let Some(header) = lines.next() {
            let header = header.strip_suffix('\n').unwrap_or(header);
            let (old_start, old_count, new_start, new_count) = parse_hunk_header(header)?;
            let mut hunk_lines: Vec<PatchLine> = Vec::new();

            // collect lines until the next hunk header
            while let Some(line) = lines.peek().copied() {
                if line.starts_with("@@ ") {
                    break;
                }
                lines.next();

                // mark the preceding line as lacking its final newline
                if line.trim_end_matches(['\r', '\n']) == "\\ No newline at end of file" {
                    let previous = hunk_lines.last_mut().ok_or_else(|| {
                        format!(
                            "query patch '{}' has a newline marker without a preceding line",
                            path.display()
                        )
                    })?;
                    previous.remove_newline(&path)?;

                    continue;
                }

                let marker = line.as_bytes().first().copied();
                let line = match marker {
                    Some(b' ') => PatchLine::Context(line[1..].to_string()),
                    Some(b'-') => PatchLine::Remove(line[1..].to_string()),
                    Some(b'+') => PatchLine::Add(line[1..].to_string()),
                    _ => {
                        return Err(format!(
                            "query patch '{}' has invalid hunk line '{}'",
                            path.display(),
                            line.trim_end()
                        ));
                    }
                };
                hunk_lines.push(line);
            }

            let hunk = PatchHunk {
                old_start,
                old_count,
                new_start,
                new_count,
                lines: hunk_lines,
            };
            hunk.validate(&path)?;
            hunks.push(hunk);
        }
        if hunks.is_empty() {
            return Err(format!(
                "query patch '{}' has no unified hunks",
                path.display()
            ));
        }

        Ok(Self { path, hunks })
    }

    /// Apply this patch exactly to one source version.
    pub(super) fn apply(&self, source: &str) -> Result<String, String> {
        let source_lines = source.split_inclusive('\n').collect::<Vec<_>>();
        let mut source_index = 0;
        let mut output = String::with_capacity(source.len());
        let mut output_lines = 0;

        // apply hunks in source order without fuzzy matching
        for hunk in &self.hunks {
            let old_index = range_index(hunk.old_start, hunk.old_count, &self.path, "source")?;
            if old_index < source_index || old_index > source_lines.len() {
                return Err(format!(
                    "query patch '{}' hunk starts outside its current source",
                    self.path.display()
                ));
            }
            for line in &source_lines[source_index..old_index] {
                output.push_str(line);
                output_lines += 1;
            }
            source_index = old_index;

            let new_index = range_index(hunk.new_start, hunk.new_count, &self.path, "target")?;
            if new_index != output_lines {
                return Err(format!(
                    "query patch '{}' hunk target starts at line {}, expected {}",
                    self.path.display(),
                    new_index + 1,
                    output_lines + 1
                ));
            }

            // require every context and removed line to match exactly
            for line in &hunk.lines {
                match line {
                    PatchLine::Context(expected) => {
                        require_source_line(&self.path, &source_lines, source_index, expected)?;
                        output.push_str(expected);
                        source_index += 1;
                        output_lines += 1;
                    }
                    PatchLine::Remove(expected) => {
                        require_source_line(&self.path, &source_lines, source_index, expected)?;
                        source_index += 1;
                    }
                    PatchLine::Add(line) => {
                        output.push_str(line);
                        output_lines += 1;
                    }
                }
            }
        }

        // retain source after the final hunk
        for line in &source_lines[source_index..] {
            output.push_str(line);
        }

        Ok(output)
    }
}

/// One strict unified patch hunk.
#[derive(Debug, Clone)]
struct PatchHunk {
    /// The one-based old source start.
    old_start: usize,
    /// The old source line count.
    old_count: usize,
    /// The one-based new source start.
    new_start: usize,
    /// The new source line count.
    new_count: usize,
    /// The hunk lines.
    lines: Vec<PatchLine>,
}

impl PatchHunk {
    /// Validate the declared line counts.
    fn validate(&self, path: &Path) -> Result<(), String> {
        let old_count = self
            .lines
            .iter()
            .filter(|line| !matches!(line, PatchLine::Add(_)))
            .count();
        let new_count = self
            .lines
            .iter()
            .filter(|line| !matches!(line, PatchLine::Remove(_)))
            .count();
        if old_count != self.old_count || new_count != self.new_count {
            return Err(format!(
                "query patch '{}' hunk declares -{},{} +{},{} but contains {old_count} old and {new_count} new lines",
                path.display(),
                self.old_start,
                self.old_count,
                self.new_start,
                self.new_count
            ));
        }

        Ok(())
    }
}

/// One line within a unified patch hunk.
#[derive(Debug, Clone)]
enum PatchLine {
    /// One unchanged context line.
    Context(String),
    /// One removed source line.
    Remove(String),
    /// One added target line.
    Add(String),
}

impl PatchLine {
    /// Remove the final newline required by a unified newline marker.
    fn remove_newline(&mut self, path: &Path) -> Result<(), String> {
        let content = match self {
            Self::Context(content) | Self::Remove(content) | Self::Add(content) => content,
        };
        if content.pop() != Some('\n') {
            return Err(format!(
                "query patch '{}' newline marker follows a line without a newline",
                path.display()
            ));
        }
        if content.ends_with('\r') {
            content.pop();
        }

        Ok(())
    }
}

/// Parse one patch block path.
fn parse_patch_path(language: &str) -> Result<PathBuf, String> {
    let mut words = language.split_whitespace();
    if words.next() != Some("diff") {
        return Err(format!(
            "query patch block '{language}' must be 'diff <path>'"
        ));
    }
    let path = words
        .next()
        .ok_or_else(|| "query patch has no file path".to_string())?;
    if words.next().is_some() {
        return Err(format!(
            "query patch block '{language}' must be 'diff <path>'"
        ));
    }
    let path = PathBuf::from(path);
    validate_query_path(&path)?;

    Ok(path)
}

/// Parse one unified hunk header.
fn parse_hunk_header(header: &str) -> Result<(usize, usize, usize, usize), String> {
    let Some(header) = header.strip_prefix("@@ -") else {
        return Err(format!(
            "query patch expected a unified hunk, found '{header}'"
        ));
    };
    let Some((ranges, _)) = header.split_once(" @@") else {
        return Err(format!(
            "query patch has invalid hunk header '@@ -{header}'"
        ));
    };
    let Some((old, new)) = ranges.split_once(" +") else {
        return Err(format!("query patch has invalid hunk ranges '{ranges}'"));
    };
    let (old_start, old_count) = parse_hunk_range(old)?;
    let (new_start, new_count) = parse_hunk_range(new)?;

    Ok((old_start, old_count, new_start, new_count))
}

/// Parse one unified hunk range.
fn parse_hunk_range(range: &str) -> Result<(usize, usize), String> {
    let (start, count) = match range.split_once(',') {
        Some((start, count)) => (start, count),
        None => (range, "1"),
    };
    let start = start
        .parse::<usize>()
        .map_err(|_| format!("query patch has invalid hunk start '{start}'"))?;
    let count = count
        .parse::<usize>()
        .map_err(|_| format!("query patch has invalid hunk count '{count}'"))?;
    if count > 0 && start == 0 {
        return Err("query patch non-empty hunk range starts at zero".to_string());
    }

    Ok((start, count))
}

/// Convert one unified range start to a zero-based line index.
fn range_index(start: usize, count: usize, path: &Path, noun: &str) -> Result<usize, String> {
    if count == 0 {
        Ok(start)
    } else {
        start.checked_sub(1).ok_or_else(|| {
            format!(
                "query patch '{}' {noun} range starts at zero",
                path.display()
            )
        })
    }
}

/// Require one exact source line at a hunk cursor.
fn require_source_line(
    path: &Path,
    source: &[&str],
    index: usize,
    expected: &str,
) -> Result<(), String> {
    let actual = source.get(index).copied().ok_or_else(|| {
        format!(
            "query patch '{}' reads beyond its current source",
            path.display()
        )
    })?;
    if actual != expected {
        return Err(format!(
            "query patch '{}' differs at source line {}\nexpected: {expected:?}\nactual:   {actual:?}",
            path.display(),
            index + 1
        ));
    }

    Ok(())
}
