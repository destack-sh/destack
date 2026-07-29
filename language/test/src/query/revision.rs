use std::path::PathBuf;

use destack_query::QueryMethod;
use indexmap::IndexMap;

use crate::mdtest::{MdTestCase, RawCodeBlock};

use super::{
    QueryAssertion, QueryFile, QueryPatch, parse_marked_file_tag, require_query_file,
    validate_query_path,
};

/// One exact workspace revision in a query fixture.
#[derive(Debug, Clone)]
pub(super) struct QueryRevision {
    /// The atomic changes that produce this revision.
    pub(super) changes: Vec<QueryChange>,
    /// The query assertions executed at this revision.
    pub(super) assertions: Vec<QueryAssertion>,
}

/// One exact workspace change in a query fixture.
#[derive(Debug, Clone)]
pub(super) enum QueryChange {
    /// Add one complete file.
    Add(QueryFile),
    /// Replace one complete file.
    Set(QueryFile),
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

impl QueryRevision {
    /// Parse every workspace revision and its exact query responses.
    pub(super) fn parse(
        markdown: &MdTestCase,
        files: &IndexMap<PathBuf, QueryFile>,
        method: QueryMethod,
    ) -> Result<Vec<Self>, String> {
        if !markdown.bullet_items.is_empty() {
            return Err("query fixture has unsupported bullet expectations".to_string());
        }

        let mut files = files.clone();
        let mut revisions = Vec::new();
        let mut revision = Self {
            changes: Vec::new(),
            assertions: Vec::new(),
        };
        let mut is_change_group_open = false;
        let mut index = 0;

        // consume assertions and workspace changes in declaration order
        while let Some(block) = markdown.extra_blocks.get(index) {
            if block.language.starts_with("query ") {
                let (assertion, applied_files, next_index) =
                    QueryAssertion::parse(&markdown.extra_blocks, index, &files, method)?;
                revision.assertions.push(assertion);
                is_change_group_open = false;
                index = next_index;

                // use one returned edit as the next revision
                if let Some(applied_files) = applied_files {
                    revisions.push(revision);
                    let changes = applied_files.into_iter().map(QueryChange::Set).collect();
                    revision = Self {
                        changes,
                        assertions: Vec::new(),
                    };
                    for change in &revision.changes {
                        change.apply(&mut files)?;
                    }
                }
            } else if QueryChange::matches(block) {
                // finish the preceding observed revision
                if !revision.assertions.is_empty() {
                    revisions.push(revision);
                    revision = Self {
                        changes: Vec::new(),
                        assertions: Vec::new(),
                    };
                    is_change_group_open = true;
                } else if !is_change_group_open {
                    return Err("query fixture revision has no assertions".to_string());
                }

                // append one change to the current atomic batch
                let change = QueryChange::parse(block, &files)?;
                change.apply(&mut files)?;
                revision.changes.push(change);
                index += 1;
            } else {
                return Err(format!(
                    "query fixture expected a query or workspace change block, found '{}'",
                    block.language
                ));
            }
        }
        if revision.assertions.is_empty() {
            return Err("query fixture revision has no assertions".to_string());
        }
        revisions.push(revision);

        Ok(revisions)
    }
}

impl QueryChange {
    /// Return whether one raw block declares a workspace change.
    fn matches(block: &RawCodeBlock) -> bool {
        change_file_tag(&block.language).is_some()
            || add_file_tag(&block.language).is_some()
            || block.language.starts_with("diff ")
            || block.language.starts_with("remove ")
            || block.language.starts_with("move ")
    }

    /// Parse one exact workspace change.
    fn parse(block: &RawCodeBlock, files: &IndexMap<PathBuf, QueryFile>) -> Result<Self, String> {
        // replace one complete existing file
        if let Some((language, path)) = change_file_tag(&block.language) {
            let path = PathBuf::from(path);
            let file = require_query_file(&path, files)?;
            if !file.accepts_language(language) {
                return Err(format!(
                    "source change block '{}' uses the wrong language for '{}'",
                    block.language,
                    path.display()
                ));
            }

            QueryFile::parse(path, &block.content).map(Self::Set)
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

            Ok(Self::Add(file))
        }
        // apply one strict patch to an existing file
        else if block.language.starts_with("diff ") {
            let patch = QueryPatch::parse(&block.language, &block.content)?;
            let file = require_query_file(&patch.path, files)?;
            let annotated_source = patch.apply(&file.annotated_source)?;
            let file = QueryFile::parse(patch.path, &annotated_source)?;

            Ok(Self::Set(file))
        }
        // remove one existing file
        else if block.language.starts_with("remove ") {
            let path = parse_remove(&block.language, &block.content)?;
            require_query_file(&path, files)?;

            Ok(Self::Remove(path))
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

            Ok(Self::Move { from, to })
        }
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

            // replace one existing file
            Self::Set(file) => {
                if !files.contains_key(&file.path) {
                    return Err(format!(
                        "query fixture file '{}' does not exist",
                        file.path.display()
                    ));
                }
                files.insert(file.path.clone(), file.clone());
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

/// Parse one complete source replacement tag.
fn change_file_tag(language: &str) -> Option<(&str, &str)> {
    parse_marked_file_tag(language, "change")
}

/// Parse one complete added file tag.
fn add_file_tag(language: &str) -> Option<(&str, &str)> {
    parse_marked_file_tag(language, "add")
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
