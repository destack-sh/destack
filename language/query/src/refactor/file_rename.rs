use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use destack_source::{BatchEdit, Edit, File, FileEdit, FileId, PathExt, ProfileId, Span};
use serde::{Deserialize, Serialize};

use super::specifier::{SpecifierPolicy, apply_rename_to_specifier, match_specifier_rename};
use crate::core::{WorkspaceQueryContext, specifier_candidates_for_rename_paths};
use crate::source::string_literal_span_in_enclosing;

/// A file rename entry for refactor queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileRenameEntry {
    /// The old path before the rename.
    pub old_path: PathBuf,
    /// The new path after the rename.
    pub new_path: PathBuf,
}

/// Request payload for file rename edits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenameFilesRequest {
    /// Profiles that should participate in specifier rewrites.
    pub profile_ids: Vec<ProfileId>,
    /// The file rename entries to apply.
    pub renames: Vec<FileRenameEntry>,
}

/// Response payload for file rename queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenameFilesResponse {
    /// File rename edit, if available.
    pub edit: Option<BatchEdit>,
}

/// Resolve file rename edits across the workspace.
pub fn rename_files(
    ctx: &WorkspaceQueryContext<'_>,
    renames: &[FileRenameEntry],
) -> Option<BatchEdit> {
    let repository = ctx.repository();
    let revision = ctx.revision();

    // normalize rename targets
    let mut rename_map = HashMap::new();
    for rename in renames {
        let old_path = rename.old_path.normalize();
        let new_path = rename.new_path.normalize();
        if old_path == new_path {
            continue;
        }
        rename_map.insert(old_path, new_path);
    }
    // return early when nothing changed
    if rename_map.is_empty() {
        return None;
    }

    // shared specifier policy
    let workspace_root = repository.workspace_root().to_path_buf().normalize();
    let specifier_policy = SpecifierPolicy {
        fs: &**repository.file_system(),
        workspace_root: &workspace_root,
    };

    // collect edits grouped by file id
    let mut edits_by_file: HashMap<FileId, Vec<Edit>> = HashMap::new();

    let mut entries_by_module = HashMap::new();
    let specifier_entries = specifier_candidates_for_rename_paths(ctx, rename_map.keys().cloned());
    for (profile_id, entry) in specifier_entries {
        entries_by_module
            .entry((profile_id, entry.module_id))
            .or_insert_with(Vec::new)
            .push(entry);
    }

    // apply edits per owning module and profile
    for ((profile_id, module_id), entries) in entries_by_module {
        let Some(module) = repository.module(revision, module_id).ok().flatten() else {
            continue;
        };

        // resolve file content for literal edits
        let Some(file) = file_for_rename(ctx, module.file_id) else {
            continue;
        };

        let Some(module_ctx) = ctx.module_context(module.id, profile_id) else {
            continue;
        };
        let dir = module_ctx.dir();
        for entry in &entries {
            // resolve the updated specifier text
            let rename_match = if let Some(target_module_id) = entry.target_module_id {
                // prefer the semantic target path when it is available
                let Some(target_module) =
                    repository.module(revision, target_module_id).ok().flatten()
                else {
                    continue;
                };

                // resolve package metadata for package specifiers
                let package = repository
                    .package(revision, target_module.package_id)
                    .ok()
                    .flatten();
                match_specifier_rename(
                    &specifier_policy,
                    &rename_map,
                    file.path.as_deref(),
                    &entry.specifier,
                    target_module.path.as_deref(),
                    package.as_ref().and_then(|package| package.name.as_deref()),
                    package.as_ref().and_then(|package| package.path.as_deref()),
                )
            } else {
                match_specifier_rename(
                    &specifier_policy,
                    &rename_map,
                    file.path.as_deref(),
                    &entry.specifier,
                    None,
                    None,
                    None,
                )
            };
            let Some(rename_match) = rename_match else {
                continue;
            };

            // rewrite the literal text using the matched rename
            let updated_specifier =
                apply_rename_to_specifier(file.path.as_deref(), &entry.specifier, &rename_match);
            let Some(updated_specifier) = updated_specifier else {
                continue;
            };
            if updated_specifier == entry.specifier {
                continue;
            }

            // resolve the string literal span for the import target
            let source_span = dir
                .source_index()
                .get_main_or_enclosing(entry.source_node_id);
            let enclosing = Span::new(module.file_id, source_span.start, source_span.end);
            let span =
                string_literal_span_in_enclosing(&file, dir.tokens(), enclosing, &entry.specifier)
                    .unwrap_or(enclosing);
            let literal = file.span_str(span);
            if literal.is_empty() {
                continue;
            }

            // build and store the edit
            let new_text = wrap_string_literal(literal, &updated_specifier);
            edits_by_file
                .entry(span.file)
                .or_default()
                .push(Edit::replace(span, new_text));
        }
    }

    // return early when no edits exist
    if edits_by_file.is_empty() {
        return None;
    }

    // build a batch edit from per file edits
    let mut batch_edit = BatchEdit::new();
    for (file_id, edits) in edits_by_file {
        let mut file_edit = FileEdit::with_edits(file_id, edits);
        file_edit.sort();
        batch_edit.push(file_edit);
    }

    Some(batch_edit)
}

/// Resolve file content for file rename edits.
fn file_for_rename(ctx: &WorkspaceQueryContext<'_>, file_id: FileId) -> Option<Arc<File>> {
    let repository = ctx.repository();
    let file = repository.file(ctx.revision(), file_id).ok().flatten()?;
    if file.has_line_index() {
        return Some(file);
    }

    // load file content from the filesystem when repository text is unavailable
    let path = file.path.as_ref()?;
    let content = repository.file_system().read_to_string(path).ok()?;
    let loaded = File::from_text(
        file.id,
        file.name.clone(),
        file.uri.clone(),
        file.path.clone(),
        file.ty,
        content,
    );

    Some(Arc::new(loaded))
}

/// Wrap a specifier string using the original literal quote style.
fn wrap_string_literal(literal: &str, specifier: &str) -> String {
    let quote = match literal.as_bytes().first().copied() {
        Some(b'\'') => '\'',
        Some(b'`') => '`',
        _ => '"',
    };

    format!("{quote}{specifier}{quote}")
}
