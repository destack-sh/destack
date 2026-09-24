use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::path::PathBuf;

use destack_artifact::{ArtifactKey, DirBound, DirExpanded, DirImported, DirParsed, DirView};
use destack_dir as dir;
use destack_repository::{ArtifactReader, Repository, Revision};
use destack_serde::Reflect;
use destack_source::{FileId, FilePatch, Patch, PatchSet, PathExt};
use serde::{Deserialize, Serialize};

use super::specifier::{rename_specifier, renamed_target_path, workspace_path};
use crate::{Module, QueryError, QueryResult};

/// One source file or directory rename.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FileRename {
    /// The old path before the rename.
    pub old_path: PathBuf,
    /// The new path after the rename.
    pub new_path: PathBuf,
}

/// A file rename request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RenameFilesRequest {
    /// The file rename entries to apply.
    pub renames: Vec<FileRename>,
}

/// A file rename response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RenameFilesResponse {
    /// File rename edit, if available.
    pub edit: Option<PatchSet>,
}

/// Resolve file rename edits across exact module profiles.
pub fn rename_files(
    request: RenameFilesRequest,
    repository: &Repository,
    revision: Revision,
    modules: &[Module],
    require_artifacts: &(dyn Fn(&[ArtifactKey]) -> QueryResult<()> + Sync),
) -> QueryResult<RenameFilesResponse> {
    let workspace_root = repository.path().to_path_buf().normalize();

    // normalize rename targets against the workspace root
    let mut rename_map = BTreeMap::new();
    for rename in request.renames {
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
        return Ok(RenameFilesResponse { edit: None });
    }

    // build one replacement per exact authored span
    let mut edits_by_span = BTreeMap::new();
    for selected in modules {
        // read the source module and the DIR artifacts used by specifier resolution
        let module_id = selected.module_id;
        let roots = [
            ArtifactKey::dir_parsed(module_id),
            ArtifactKey::dir_imported(module_id, selected.profile_id),
            ArtifactKey::dir_expanded(module_id, selected.profile_id),
        ];
        require_artifacts(&roots)?;
        let source_module = repository
            .module(revision, module_id)?
            .ok_or_else(|| QueryError::missing(format!("repository module {module_id:?}")))?;
        let artifacts = ArtifactReader::new(repository, revision);
        let key = (module_id, selected.profile_id);
        let stages = DirView::expanded(
            artifacts.read::<DirParsed>(module_id)?,
            artifacts.read::<DirBound>(key)?,
            artifacts.read::<DirImported>(key)?,
            artifacts.read::<DirExpanded>(key)?,
        );
        let parsed = &stages.parsed;
        let view = stages.tree();
        let source_index = &parsed.tree.source_index;
        let module_table = stages.modules();

        // resolve file content for literal edits
        let file_id = source_module.file_id;
        let file = repository
            .file(revision, file_id)?
            .ok_or_else(|| QueryError::missing(format!("source file {file_id:?}")))?;
        let source_path = source_module
            .path
            .as_deref()
            .map(|path| workspace_path(&workspace_root, path));
        let renamed_source_path = match source_path.as_deref() {
            Some(path) => renamed_target_path(path, &rename_map)?,
            None => None,
        };
        let is_source_renamed = renamed_source_path.is_some();
        let source_path = renamed_source_path.as_deref().or(source_path.as_deref());

        // visit authored import and re-export specifiers
        for (expression_id, expression) in view.iter_nodes::<dir::Expression>() {
            let (text, relation) = match expression {
                dir::Expression::Import { target, .. } => (*target, dir::ModuleRelation::Import),
                dir::Expression::Export {
                    target: Some(target),
                    ..
                } => (*target, dir::ModuleRelation::ReExport),
                _ => continue,
            };
            let source = expression_id.into_global_any(module_id);
            let Some(target_module_id) = module_table.target_for_source(source, relation) else {
                continue;
            };
            let target_module =
                repository
                    .module(revision, target_module_id)?
                    .ok_or_else(|| {
                        QueryError::missing(format!("repository module {target_module_id:?}"))
                    })?;
            let Some(target_path) = target_module.path.as_deref() else {
                continue;
            };
            let target_path = workspace_path(&workspace_root, target_path);

            // require every resolved authored specifier to retain its source span
            let source_id = view.get_source(expression_id);
            if source_index.try_get(source_id).is_none() {
                continue;
            }
            let span = source_index.get_main(source_id).ok_or_else(|| {
                QueryError::invalid(format!(
                    "resolved module specifier {source:?} has no authored main span"
                ))
            })?;
            let text = repository.string_pool().get(text);

            // resolve source and target movement independently
            let renamed_target_path = renamed_target_path(&target_path, &rename_map)?;
            if !is_source_renamed && renamed_target_path.is_none() {
                continue;
            }
            let target_path = if let Some(path) = renamed_target_path.as_deref() {
                path
            } else {
                &target_path
            };

            // rewrite the literal from the exact upstream resolution
            let updated_specifier =
                rename_specifier(source_path, target_path, text)?.ok_or_else(|| {
                    QueryError::invalid(format!(
                        "specifier {text:?} at {span:?} cannot represent its resolved target rename"
                    ))
                })?;
            if updated_specifier == text {
                continue;
            }

            // require the authored span to belong to the source module
            if span.file != file_id {
                return Err(QueryError::conflict(format!(
                    "module {module_id:?} uses file {file_id:?}, but its specifier uses {:?}",
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
        return Ok(RenameFilesResponse { edit: None });
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

    Ok(RenameFilesResponse {
        edit: Some(batch_edit),
    })
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
