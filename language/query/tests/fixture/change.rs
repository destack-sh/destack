use std::ops::Range;
use std::path::PathBuf;

use indexmap::IndexMap;

use crate::QueryBlock;

use super::{
    FixtureAnchor, QueryFile, QueryPatch, parse_marked_file_tag, require_query_file,
    validate_query_path,
};

/// One exact workspace change in a query fixture.
#[derive(Debug, Clone)]
pub(super) enum QueryChange {
    /// Add one complete file.
    Add(QueryFile),
    /// Edit one existing text file.
    Edit(QueryFile),
    /// Remove one file.
    Remove(PathBuf),
    /// Move one file.
    Move {
        /// The existing path.
        from: PathBuf,
        /// The new path.
        to: PathBuf,
    },
}

impl QueryChange {
    /// Return whether one raw block declares a workspace change.
    pub(super) fn matches(block: &QueryBlock) -> bool {
        TextAction::parse(&block.language).is_some()
            || add_file_tag(&block.language).is_some()
            || block.language.starts_with("diff ")
            || block.language.starts_with("remove ")
            || block.language.starts_with("move ")
    }

    /// Return whether one source block expands into successive revisions.
    pub(super) fn is_sequential(block: &QueryBlock) -> bool {
        TextAction::parse(&block.language).is_some_and(|(_, _, action)| action.is_sequential())
    }

    /// Parse the changes declared by one workspace block.
    pub(super) fn parse(
        block: &QueryBlock,
        files: &IndexMap<PathBuf, QueryFile>,
    ) -> Result<Vec<Self>, String> {
        // edit one existing source file
        if let Some((language, path, action)) = TextAction::parse(&block.language) {
            let path = PathBuf::from(path);
            let current = require_query_file(&path, files)?;
            if !current.accepts_language(language) {
                return Err(format!(
                    "source change block '{}' uses the wrong language for '{}'",
                    block.language,
                    path.display()
                ));
            }
            let target = QueryFile::parse(path, &block.content)?;
            let changes = match action {
                TextAction::Change => vec![Self::edit(current, target)?],
                TextAction::Type => Self::type_text(current, &target)?,
                TextAction::Backspace => Self::backspace(current, &target)?,
                TextAction::Delete => Self::delete(current, &target)?,
            };

            Ok(changes)
        }
        // add one complete new file
        else if let Some((language, path)) = add_file_tag(&block.language) {
            let path = PathBuf::from(path);
            if files.contains_key(&path) {
                return Err(format!(
                    "query fixture file '{}' already exists",
                    path.display()
                ));
            }
            let file = QueryFile::parse(path, &block.content)?;
            if !file.accepts_language(language) {
                return Err(format!(
                    "source add block '{}' uses the wrong language for '{}'",
                    block.language,
                    file.path.display()
                ));
            }

            Ok(vec![Self::Add(file)])
        }
        // apply one strict patch to an existing file
        else if block.language.starts_with("diff ") {
            let patch = QueryPatch::parse(&block.language, &block.content)?;
            let current = require_query_file(&patch.path, files)?;
            let annotated_source = patch.apply(&current.annotated_source)?;
            let target = QueryFile::parse(patch.path, &annotated_source)?;

            Ok(vec![Self::edit(current, target)?])
        }
        // remove one existing file
        else if block.language.starts_with("remove ") {
            let path = parse_remove(&block.language, &block.content)?;
            require_query_file(&path, files)?;

            Ok(vec![Self::Remove(path)])
        }
        // move one existing file
        else {
            let (from, to) = parse_move(&block.language, &block.content)?;
            require_query_file(&from, files)?;
            if files.contains_key(&to) {
                return Err(format!(
                    "query fixture file '{}' already exists",
                    to.display()
                ));
            }

            Ok(vec![Self::Move { from, to }])
        }
    }

    /// Build one edit between complete fixture files.
    pub(super) fn edit(current: &QueryFile, target: QueryFile) -> Result<Self, String> {
        require_same_path(current, &target)?;
        changed_ranges(&current.source, &target.source)?;

        Ok(Self::Edit(target))
    }

    /// Build one revision for every scalar inserted into a source replacement.
    fn type_text(current: &QueryFile, target: &QueryFile) -> Result<Vec<Self>, String> {
        require_same_path(current, target)?;
        let (current_range, target_range) = changed_ranges(&current.source, &target.source)?;
        let replacement = &target.source[target_range.clone()];
        if replacement.is_empty() {
            return Err(format!(
                "query typing for '{}' inserts no source",
                current.path.display()
            ));
        }
        let sources = replacement
            .char_indices()
            .map(|(index, character)| index + character.len_utf8())
            .map(|end| replaced_source(current, &current_range, &replacement[..end]))
            .collect::<Vec<_>>();

        Self::sequence(current, target, sources)
    }

    /// Build one revision for every scalar removed from right to left.
    fn backspace(current: &QueryFile, target: &QueryFile) -> Result<Vec<Self>, String> {
        let (range, deleted) = deletion(current, target, "backspace")?;
        let sources = deleted
            .char_indices()
            .rev()
            .map(|(end, _)| replaced_source(current, &range, &deleted[..end]))
            .collect::<Vec<_>>();

        Self::sequence(current, target, sources)
    }

    /// Build one revision for every scalar removed from left to right.
    fn delete(current: &QueryFile, target: &QueryFile) -> Result<Vec<Self>, String> {
        let (range, deleted) = deletion(current, target, "deletion")?;
        let sources = deleted
            .char_indices()
            .map(|(index, character)| index + character.len_utf8())
            .map(|start| replaced_source(current, &range, &deleted[start..]))
            .collect::<Vec<_>>();

        Self::sequence(current, target, sources)
    }

    /// Build successive edits from complete intermediate sources.
    fn sequence(
        current: &QueryFile,
        target: &QueryFile,
        sources: Vec<String>,
    ) -> Result<Vec<Self>, String> {
        let mut revisions = Vec::with_capacity(sources.len());

        // retain final anchors at every intermediate source
        for source in sources {
            let target = intermediate_target(target, source)?;
            revisions.push(Self::edit(current, target)?);
        }

        Ok(revisions)
    }

    /// Apply this change to one exact fixture file set.
    pub(super) fn apply(&self, files: &mut IndexMap<PathBuf, QueryFile>) -> Result<(), String> {
        match self {
            // add one absent file
            Self::Add(file) => {
                if files.contains_key(&file.path) {
                    return Err(format!(
                        "query fixture file '{}' already exists",
                        file.path.display()
                    ));
                }
                files.insert(file.path.clone(), file.clone());
            }

            // edit one existing file
            Self::Edit(target) => {
                require_query_file(&target.path, files)?;
                files.insert(target.path.clone(), target.clone());
            }

            // remove one existing file
            Self::Remove(path) => {
                if files.shift_remove(path).is_none() {
                    return Err(format!(
                        "query fixture file '{}' does not exist",
                        path.display()
                    ));
                }
            }

            // move one file to an absent path
            Self::Move { from, to } => {
                if files.contains_key(to) {
                    return Err(format!(
                        "query fixture file '{}' already exists",
                        to.display()
                    ));
                }
                let file = files.shift_remove(from).ok_or_else(|| {
                    format!("query fixture file '{}' does not exist", from.display())
                })?;
                let mut file = file;
                file.path = to.clone();
                files.insert(to.clone(), file);
            }
        }

        Ok(())
    }
}

/// One text action declared by a source block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextAction {
    /// Apply one complete replacement.
    Change,
    /// Insert one replacement scalar per revision.
    Type,
    /// Remove one scalar per revision from right to left.
    Backspace,
    /// Remove one scalar per revision from left to right.
    Delete,
}

impl TextAction {
    /// Parse one text action tag.
    fn parse(language: &str) -> Option<(&str, &str, Self)> {
        [
            ("change", Self::Change),
            ("type", Self::Type),
            ("backspace", Self::Backspace),
            ("delete", Self::Delete),
        ]
        .into_iter()
        .find_map(|(marker, action)| {
            parse_marked_file_tag(language, marker).map(|(language, path)| (language, path, action))
        })
    }

    /// Return whether this action produces successive revisions.
    fn is_sequential(self) -> bool {
        self != Self::Change
    }
}

/// Parse one complete added file tag.
fn add_file_tag(language: &str) -> Option<(&str, &str)> {
    parse_marked_file_tag(language, "add")
}

/// Return the changed ranges in two source strings.
fn changed_ranges(current: &str, target: &str) -> Result<(Range<usize>, Range<usize>), String> {
    let prefix = common_prefix(current, target);
    let suffix = common_suffix(&current[prefix..], &target[prefix..]);
    let current_end = current.len() - suffix;
    let target_end = target.len() - suffix;
    if current_end == prefix && target_end == prefix {
        return Err("query source change has identical input and output".to_string());
    }

    Ok((prefix..current_end, prefix..target_end))
}

/// Return the shared UTF-8 prefix length of two strings.
fn common_prefix(left: &str, right: &str) -> usize {
    left.chars()
        .zip(right.chars())
        .take_while(|(left, right)| left == right)
        .map(|(character, _)| character.len_utf8())
        .sum()
}

/// Return the shared UTF-8 suffix length of two strings.
fn common_suffix(left: &str, right: &str) -> usize {
    left.chars()
        .rev()
        .zip(right.chars().rev())
        .take_while(|(left, right)| left == right)
        .map(|(character, _)| character.len_utf8())
        .sum()
}

/// Replace one exact byte range in a fixture source.
fn replaced_source(current: &QueryFile, range: &Range<usize>, replacement: &str) -> String {
    let mut source = current.source.clone();
    source.replace_range(range.clone(), replacement);

    source
}

/// Return one pure deletion and its removed source.
fn deletion<'a>(
    current: &'a QueryFile,
    target: &QueryFile,
    operation: &str,
) -> Result<(Range<usize>, &'a str), String> {
    require_same_path(current, target)?;
    let (current_range, target_range) = changed_ranges(&current.source, &target.source)?;
    if !target_range.is_empty() || current_range.is_empty() {
        return Err(format!(
            "query {operation} for '{}' must remove one contiguous source range",
            current.path.display()
        ));
    }
    let deleted = &current.source[current_range.clone()];

    Ok((current_range, deleted))
}

/// Require two fixture files to name the same path.
fn require_same_path(current: &QueryFile, target: &QueryFile) -> Result<(), String> {
    if current.path != target.path {
        return Err(format!(
            "query source change maps '{}' to '{}'",
            current.path.display(),
            target.path.display()
        ));
    }

    Ok(())
}

/// Build one fixture file for an intermediate source revision.
fn intermediate_target(target: &QueryFile, source: String) -> Result<QueryFile, String> {
    if source == target.source {
        return Ok(target.clone());
    }
    let (current_range, target_range) = changed_ranges(&source, &target.source)?;
    let anchors = target
        .anchors
        .iter()
        .map(|(name, anchor)| {
            let anchor = FixtureAnchor {
                start: intermediate_offset(anchor.start, &current_range, &target_range),
                end: intermediate_offset(anchor.end, &current_range, &target_range),
            };

            (name.clone(), anchor)
        })
        .collect();

    Ok(QueryFile {
        path: target.path.clone(),
        annotated_source: source.clone(),
        source,
        anchors,
    })
}

/// Map one final offset into an intermediate source revision.
fn intermediate_offset(
    offset: u32,
    current_range: &Range<usize>,
    target_range: &Range<usize>,
) -> u32 {
    let offset = offset as usize;
    let offset = if offset <= target_range.start {
        offset
    } else if offset < target_range.end {
        let relative = offset - target_range.start;

        current_range.start + relative.min(current_range.len())
    } else {
        current_range.end + offset - target_range.end
    };

    offset as u32
}

/// Parse one exact file removal.
fn parse_remove(language: &str, content: &str) -> Result<PathBuf, String> {
    require_empty_change(language, content)?;
    let mut words = language.split_whitespace();
    if words.next() != Some("remove") {
        return Err(format!(
            "query removal block '{language}' must be 'remove <path>'"
        ));
    }
    let path = words
        .next()
        .ok_or_else(|| "query removal has no file path".to_string())?;
    if words.next().is_some() {
        return Err(format!(
            "query removal block '{language}' must be 'remove <path>'"
        ));
    }
    let path = PathBuf::from(path);
    validate_query_path(&path)?;

    Ok(path)
}

/// Parse one exact file move.
fn parse_move(language: &str, content: &str) -> Result<(PathBuf, PathBuf), String> {
    require_empty_change(language, content)?;
    let mut words = language.split_whitespace();
    if words.next() != Some("move") {
        return Err(format!(
            "query move block '{language}' must be 'move <from> <to>'"
        ));
    }
    let from = words
        .next()
        .ok_or_else(|| "query move has no source path".to_string())?;
    let to = words
        .next()
        .ok_or_else(|| "query move has no destination path".to_string())?;
    if words.next().is_some() {
        return Err(format!(
            "query move block '{language}' must be 'move <from> <to>'"
        ));
    }
    let from = PathBuf::from(from);
    let to = PathBuf::from(to);
    validate_query_path(&from)?;
    validate_query_path(&to)?;

    Ok((from, to))
}

/// Require one structural workspace change block to have no body.
fn require_empty_change(language: &str, content: &str) -> Result<(), String> {
    if !content.is_empty() {
        return Err(format!(
            "query workspace change block '{language}' must be empty"
        ));
    }

    Ok(())
}
