use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::path::PathBuf;

use destack_repository::{Repository, Revision};
use destack_serde::Reflect;
use destack_source::{FileId, FilePatch, Patch, PatchSet, PathExt};
use serde::{Deserialize, Serialize};

use super::specifier::{module_specifiers, rename_specifier, renamed_target_path, workspace_path};
use crate::{Module, QueryError, QueryResult};

/// One source file or directory rename.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FileRename {
    /// The old path before the rename.
    pub old_path: PathBuf,
    /// The new path after the rename.
    pub new_path: PathBuf,
}

/// Request payload for file rename edits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RenameFilesRequest {
    /// The file rename entries to apply.
    pub renames: Vec<FileRename>,
}

/// Response payload for file rename queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RenameFilesResponse {
    /// File rename edit, if available.
    pub edit: Option<PatchSet>,
}

/// Resolve file rename edits across exact module profiles.
pub fn rename_files(
    repository: &Repository,
    revision: Revision,
    modules: &[Module],
    renames: &[FileRename],
) -> QueryResult<Option<PatchSet>> {
    let workspace_root = repository.path().to_path_buf().normalize();

    // normalize rename targets against the workspace root
    let mut rename_map = BTreeMap::new();
    for rename in renames {
        let old_path = workspace_path(&workspace_root, &rename.old_path);
        let new_path = workspace_path(&workspace_root, &rename.new_path);
        if old_path == new_path {
            continue;
        }

        match rename_map.entry(old_path) {
            Entry::Vacant(entry) => {
                entry.insert(new_path);
            }
            Entry::Occupied(entry) if entry.get() == &new_path => {}
            Entry::Occupied(entry) => {
                return Err(QueryError::conflict(format!(
                    "path {} is renamed to both {} and {}",
                    entry.key().display(),
                    entry.get().display(),
                    new_path.display()
                )));
            }
        }
    }

    // return early when nothing changed
    if rename_map.is_empty() {
        return Ok(None);
    }

    // collect exact resolved specifiers across the selected module profiles
    let mut entries_by_module = BTreeMap::new();
    for module in modules {
        let specifiers =
            module_specifiers(repository, revision, module.module_id, module.profile_id)?;

        entries_by_module
            .entry(module.module_id)
            .or_insert_with(Vec::new)
            .extend(specifiers);
    }

    // build one replacement per exact authored span
    let mut edits_by_span = BTreeMap::new();
    for (module_id, mut entries) in entries_by_module {
        entries.sort_by(|left, right| {
            (
                left.span.start,
                left.span.end,
                left.text.as_str(),
                &left.target_path,
            )
                .cmp(&(
                    right.span.start,
                    right.span.end,
                    right.text.as_str(),
                    &right.target_path,
                ))
        });
        entries.dedup();

        // read the source module and its authored file
        let module = repository
            .module(revision, module_id)?
            .ok_or_else(|| QueryError::missing(format!("repository module {module_id:?}")))?;

        // resolve file content for literal edits
        let file_id = module.file_id;
        let file = repository
            .file(revision, file_id)?
            .ok_or_else(|| QueryError::missing(format!("source file {file_id:?}")))?;
        let source_path = module
            .path
            .as_deref()
            .map(|path| workspace_path(&workspace_root, path));
        let renamed_source_path = match source_path.as_deref() {
            Some(path) => renamed_target_path(path, &rename_map)?,
            None => None,
        };
        let is_source_renamed = renamed_source_path.is_some();
        let source_path = renamed_source_path.as_deref().or(source_path.as_deref());

        for entry in &entries {
            // resolve source and target movement independently
            let renamed_target_path = renamed_target_path(&entry.target_path, &rename_map)?;
            if !is_source_renamed && renamed_target_path.is_none() {
                continue;
            }
            let target_path = if let Some(path) = renamed_target_path.as_deref() {
                path
            } else {
                &entry.target_path
            };

            // rewrite the literal from the exact upstream resolution
            let updated_specifier = rename_specifier(source_path, target_path, &entry.text)
                .ok_or_else(|| {
                    QueryError::invalid(format!(
                        "specifier {:?} at {:?} cannot represent its resolved target rename",
                        entry.text, entry.span
                    ))
                })?;
            if updated_specifier == entry.text {
                continue;
            }

            // use the exact authored specifier span stored by the index
            let span = entry.span;
            if span.file != file_id {
                return Err(QueryError::conflict(format!(
                    "module {module_id:?} uses file {file_id:?}, but its indexed specifier uses {:?}",
                    span.file
                )));
            }

            let literal = file
                .get_span_str(span)
                .ok_or_else(|| QueryError::invalid(format!("source span {span:?}")))?;
            let literal = StringLiteral::parse(literal)
                .ok_or_else(|| QueryError::invalid(format!("specifier literal at {span:?}")))?;

            // reject contradictory profile-specific replacements
            let new_text = literal.replace_text(&updated_specifier);
            let key = (span.file, span.start, span.end);
            match edits_by_span.entry(key) {
                Entry::Vacant(entry) => {
                    entry.insert((span, new_text));
                }
                Entry::Occupied(entry) if entry.get().1 == new_text => {}
                Entry::Occupied(entry) => {
                    return Err(QueryError::conflict(format!(
                        "specifier at {span:?} requires replacements {:?} and {new_text:?}",
                        entry.get().1
                    )));
                }
            }
        }
    }

    // return early when no edits exist
    if edits_by_span.is_empty() {
        return Ok(None);
    }

    // group exact replacements into deterministic file edits
    let mut edits_by_file: BTreeMap<FileId, Vec<Patch>> = BTreeMap::new();
    for (_, (span, new_text)) in edits_by_span {
        edits_by_file
            .entry(span.file)
            .or_default()
            .push(Patch::replace(span, new_text));
    }

    // build one batch edit
    let mut batch_edit = PatchSet::new();
    for (file_id, edits) in edits_by_file {
        let mut file_edit = FilePatch::with_patches(file_id, edits);
        file_edit.sort();
        batch_edit.push(file_edit);
    }

    Ok(Some(batch_edit))
}

/// One source string literal used for specifier rewrites.
struct StringLiteral<'a> {
    /// The source literal text.
    text: &'a str,
}

impl<'a> StringLiteral<'a> {
    /// Parse one complete authored string literal.
    fn parse(text: &'a str) -> Option<Self> {
        let first = text.as_bytes().first()?;
        let last = text.as_bytes().last()?;
        let is_quote = matches!(first, b'\'' | b'"');

        (is_quote && first == last).then_some(Self { text })
    }

    /// Return new literal text with the original quote style.
    fn replace_text(&self, specifier: &str) -> String {
        let quote = self.text.as_bytes()[0] as char;
        let mut escaped = String::with_capacity(specifier.len());

        // escape characters that cannot appear literally inside the selected quote
        for character in specifier.chars() {
            match character {
                '\\' => escaped.push_str("\\\\"),
                character if character == quote => {
                    escaped.push('\\');
                    escaped.push(character);
                }
                '\n' => escaped.push_str("\\n"),
                '\r' => escaped.push_str("\\r"),
                '\t' => escaped.push_str("\\t"),
                character if character.is_control() => {
                    escaped.extend(character.escape_default());
                }
                character => escaped.push(character),
            }
        }

        format!("{quote}{escaped}{quote}")
    }
}
