use std::path::{Component, Path, PathBuf};

use indexmap::IndexMap;
use tspp_source::{FileType, LanguageType};

use super::FixturePositionEdge;

/// One named byte position or range in a query file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FixtureAnchor {
    /// The inclusive start byte offset.
    pub(super) start: u32,
    /// The exclusive end byte offset.
    pub(super) end: u32,
}

/// One workspace file and its named query anchors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct QueryFile {
    /// The workspace-relative file path.
    pub(super) path: PathBuf,
    /// The fixture source including anchor declarations.
    pub(super) annotated_source: String,
    /// The exact source text without anchor declarations.
    pub(super) source: String,
    /// The named source anchors in declaration order.
    pub(super) anchors: IndexMap<String, FixtureAnchor>,
}

impl QueryFile {
    /// Parse one annotated workspace file.
    pub(super) fn parse(path: impl Into<PathBuf>, annotated_source: &str) -> Result<Self, String> {
        let path = path.into();
        validate_query_path(&path)?;

        // preserve non-code workspace files exactly
        let language_type = LanguageType::from_path(&path);
        if language_type.is_none() {
            return Ok(Self {
                path,
                annotated_source: annotated_source.to_string(),
                source: annotated_source.to_string(),
                anchors: IndexMap::new(),
            });
        }

        let mut source = String::with_capacity(annotated_source.len());
        let mut anchors = IndexMap::new();
        let mut range_starts = IndexMap::new();
        let mut previous_line = None;

        // remove anchor declaration lines while preserving exact source bytes
        for line in annotated_source.split_inclusive('\n') {
            if let Some(anchor) = parse_anchor_line(line, previous_line)? {
                if let Some(name) = anchor.name.strip_suffix(":start") {
                    if name.is_empty() {
                        return Err("query range start has no name".to_string());
                    }
                    if range_starts
                        .insert(name.to_string(), anchor.range)
                        .is_some()
                    {
                        return Err(format!(
                            "query file '{}' declares range start '{}' more than once",
                            path.display(),
                            name
                        ));
                    }
                } else if let Some(name) = anchor.name.strip_suffix(":end") {
                    let start = range_starts.shift_remove(name).ok_or_else(|| {
                        format!(
                            "query file '{}' declares range end '{}' without a start",
                            path.display(),
                            name
                        )
                    })?;
                    if start.start > anchor.range.end {
                        return Err(format!(
                            "query file '{}' range '{}' ends before it starts",
                            path.display(),
                            name
                        ));
                    }
                    let range = FixtureAnchor {
                        start: start.start,
                        end: anchor.range.end,
                    };
                    if anchors.insert(name.to_string(), range).is_some() {
                        return Err(format!(
                            "query file '{}' declares anchor '{}' more than once",
                            path.display(),
                            name
                        ));
                    }
                } else if anchors.insert(anchor.name.clone(), anchor.range).is_some() {
                    return Err(format!(
                        "query file '{}' declares anchor '{}' more than once",
                        path.display(),
                        anchor.name
                    ));
                }
                continue;
            }

            let line_start = u32::try_from(source.len()).map_err(|_| {
                format!(
                    "query file '{}' exceeds the source offset width",
                    path.display()
                )
            })?;
            let line_source = line.strip_suffix('\n').unwrap_or(line);
            let line_source = line_source.strip_suffix('\r').unwrap_or(line_source);
            previous_line = Some(SourceLine {
                start: line_start,
                source: line_source,
            });
            source.push_str(line);
        }
        if let Some((name, _)) = range_starts.first() {
            return Err(format!(
                "query file '{}' declares range start '{}' without an end",
                path.display(),
                name
            ));
        }

        Ok(Self {
            path,
            annotated_source: annotated_source.to_string(),
            source,
            anchors,
        })
    }

    /// Return whether this fixture file is a source module.
    pub(super) fn is_code(&self) -> bool {
        LanguageType::from_path(&self.path).is_some()
    }

    /// Return whether one fixture language denotes this file type.
    pub(super) fn accepts_language(&self, language: &str) -> bool {
        match FileType::from_path(&self.path) {
            Some(FileType::Tspp | FileType::TsppDeclaration) => {
                matches!(language, "tspp")
            }
            Some(FileType::Text) => matches!(language, "text" | "txt"),
            Some(FileType::Toml) => language == "toml",
            Some(FileType::Yaml) => matches!(language, "yaml" | "yml"),
            Some(FileType::Json) => language == "json",
            Some(FileType::Dotenv) => language == "env",
            _ => false,
        }
    }

    /// Return one required named anchor.
    pub(super) fn anchor(&self, name: &str) -> Result<FixtureAnchor, String> {
        self.anchors.get(name).copied().ok_or_else(|| {
            format!(
                "query file '{}' does not declare anchor '{name}'",
                self.path.display()
            )
        })
    }

    /// Return the unique anchor for one exact source range.
    pub(super) fn range_name(&self, start: u32, end: u32) -> Option<&str> {
        let mut names = self.anchors.iter().filter_map(|(name, anchor)| {
            (anchor.start == start && anchor.end == end).then_some(name.as_str())
        });
        let name = names.next()?;
        if names.next().is_some() {
            return None;
        }

        Some(name)
    }

    /// Return the unique anchor edge for one exact source offset.
    pub(super) fn position_name(&self, offset: u32) -> Option<(&str, FixturePositionEdge)> {
        let mut points = self.anchors.iter().filter_map(|(name, anchor)| {
            (anchor.start == offset && anchor.end == offset)
                .then_some((name.as_str(), FixturePositionEdge::Start))
        });
        if let Some(point) = points.next() {
            if points.next().is_none() {
                return Some(point);
            }

            return None;
        }

        let mut edges = self.anchors.iter().filter_map(|(name, anchor)| {
            if anchor.start == offset {
                Some((name.as_str(), FixturePositionEdge::Start))
            } else if anchor.end == offset {
                Some((name.as_str(), FixturePositionEdge::End))
            } else {
                None
            }
        });
        let edge = edges.next()?;
        if edges.next().is_some() {
            return None;
        }

        Some(edge)
    }
}

/// One parsed anchor declaration.
struct AnchorDeclaration {
    /// The declared anchor name.
    name: String,
    /// The declared source range.
    range: FixtureAnchor,
}

/// One preceding source line available to an anchor declaration.
#[derive(Clone, Copy)]
struct SourceLine<'a> {
    /// The line start in the clean source.
    start: u32,
    /// The line source without its line ending.
    source: &'a str,
}

/// Parse one source anchor declaration line.
fn parse_anchor_line(
    line: &str,
    previous: Option<SourceLine<'_>>,
) -> Result<Option<AnchorDeclaration>, String> {
    let line = line.strip_suffix('\n').unwrap_or(line);
    let line = line.strip_suffix('\r').unwrap_or(line);
    let Some(source_marker_start) = line.find('^') else {
        return Ok(None);
    };
    let prefix = &line[..source_marker_start];
    if !prefix.trim().is_empty() {
        return Ok(None);
    }
    let marker_start = source_marker_start;
    let marker_width = line[source_marker_start..]
        .bytes()
        .take_while(|byte| *byte == b'^')
        .count();
    let name_start = source_marker_start + marker_width;
    let name_source = &line[name_start..];

    // distinguish fixture annotations from source operators
    let has_separator = name_source.starts_with(' ') || name_source.starts_with('\t');
    if !has_separator {
        return Ok(None);
    }
    let name = name_source.trim();

    // require a source line and one identifier-like name
    let previous =
        previous.ok_or_else(|| "query anchor has no preceding source line".to_string())?;
    if name.is_empty() {
        return Err("query anchor has no name".to_string());
    }
    if name.chars().any(char::is_whitespace) {
        return Err(format!("query anchor '{name}' contains whitespace"));
    }
    if name.contains("->") {
        return Err(format!(
            "query anchor '{name}' uses an implicit target; declare the target in the query"
        ));
    }

    // resolve the byte range against the preceding source line
    let start = marker_start;
    let mut end = start
        .checked_add(marker_width)
        .ok_or_else(|| "query anchor end overflowed".to_string())?;
    if start == previous.source.len() && marker_width == 1 {
        end = start;
    } else if end > previous.source.len() {
        return Err(format!(
            "query anchor '{name}' exceeds its preceding source line"
        ));
    }
    if !previous.source.is_char_boundary(start) || !previous.source.is_char_boundary(end) {
        return Err(format!(
            "query anchor '{name}' does not follow UTF-8 byte boundaries"
        ));
    }

    // convert the validated local range to clean source offsets
    let start = u32::try_from(start).map_err(|_| "query anchor start overflowed".to_string())?;
    let end = u32::try_from(end).map_err(|_| "query anchor end overflowed".to_string())?;
    let start = previous
        .start
        .checked_add(start)
        .ok_or_else(|| "query anchor start overflowed".to_string())?;
    let end = previous
        .start
        .checked_add(end)
        .ok_or_else(|| "query anchor end overflowed".to_string())?;

    Ok(Some(AnchorDeclaration {
        name: name.to_string(),
        range: FixtureAnchor { start, end },
    }))
}

/// Return a stable display path for one query file.
pub(super) fn display_query_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Require one normalized workspace-relative query file path.
pub(super) fn validate_query_path(path: &Path) -> Result<(), String> {
    let is_normal = !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)));
    if !is_normal {
        return Err(format!(
            "query file '{}' must use a normalized workspace-relative path",
            path.display()
        ));
    }

    Ok(())
}

/// Return one required declared query file.
pub(super) fn require_query_file<'a>(
    path: &Path,
    files: &'a IndexMap<PathBuf, QueryFile>,
) -> Result<&'a QueryFile, String> {
    validate_query_path(path)?;

    files
        .get(path)
        .ok_or_else(|| format!("query file '{}' is not declared", path.display()))
}

/// Parse one file block with an exact trailing marker.
pub(super) fn parse_marked_file_tag<'a>(
    language: &'a str,
    expected: &str,
) -> Option<(&'a str, &'a str)> {
    let mut words = language.split_whitespace();
    let language = words.next()?;
    let path = words.next()?;
    let marker = words.next()?;
    if marker != expected || words.next().is_some() {
        return None;
    }

    Some((language, path))
}

/// Parse one file-qualified query anchor.
pub(super) fn parse_query_anchor<'a>(
    value: &'a str,
    noun: &str,
) -> Result<(&'a str, &'a str), String> {
    let Some((path, anchor)) = value.split_once('#') else {
        return Err(format!("query {noun} '{value}' must be '<file>#<anchor>'"));
    };
    if path.is_empty() || anchor.is_empty() || anchor.contains('#') {
        return Err(format!("query {noun} '{value}' must be '<file>#<anchor>'"));
    }

    Ok((path, anchor))
}
