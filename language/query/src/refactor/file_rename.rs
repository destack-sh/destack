use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use destack_source::{BatchEdit, Edit, File, FileEdit, FileId, ModuleId, PathExt, Span};
use destack_workspace::Revision;
use serde::{Deserialize, Serialize};

use super::specifier::{SpecifierPolicy, apply_rename_to_specifier, match_specifier_rename};
use crate::core::{
    SpecifierEntry, specifier_candidates_for_rename_paths, with_source_query_for_module,
};
use crate::source::string_literal_span_in_enclosing;
use destack_workspace::Repository;

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
    /// The file rename entries to apply.
    pub renames: Vec<FileRenameEntry>,
}

/// Response payload for file rename queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenameFilesResponse {
    /// File rename result, if available.
    pub result: Option<FileRenameResult>,
}

/// Result of a file rename query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileRenameResult {
    /// All edits to apply.
    pub edits: BatchEdit,
}

impl FileRenameResult {
    /// Create an empty file rename result.
    pub fn empty() -> Self {
        Self {
            edits: BatchEdit::new(),
        }
    }

    /// Create a file rename result from a batch edit.
    pub fn from_edits(edits: BatchEdit) -> Self {
        Self { edits }
    }

    /// Whether there are any edits.
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    /// Total number of edits.
    pub fn edit_count(&self) -> usize {
        self.edits.total_edits()
    }

    /// Number of files affected.
    pub fn file_count(&self) -> usize {
        self.edits.file_count()
    }
}

/// Resolve file rename edits across the workspace.
pub fn rename_files(
    repository: &Repository,
    revision: Revision,
    renames: &[FileRenameEntry],
) -> Option<FileRenameResult> {
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

    // collect candidate specifier entries from the workspace index
    let profile_ids = repository.profile_ids(revision).unwrap_or_default();
    let specifier_entries = specifier_candidates_for_rename_paths(
        repository,
        revision,
        &profile_ids,
        rename_map.keys().cloned(),
    );
    let mut entries_by_module: HashMap<ModuleId, Vec<SpecifierEntry>> = HashMap::new();
    for entry in specifier_entries {
        entries_by_module
            .entry(entry.module_id)
            .or_default()
            .push(entry);
    }

    // apply edits per owning module
    for (module_id, entries) in entries_by_module {
        let Some(module) = repository.module(revision, module_id).ok().flatten() else {
            continue;
        };

        // resolve file content for literal edits
        let Some(file) = file_for_rename(repository, revision, module.file_id) else {
            continue;
        };

        let Some(()) = with_source_query_for_module(repository, revision, module.id, |parsed| {
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
                let updated_specifier = apply_rename_to_specifier(
                    file.path.as_deref(),
                    &entry.specifier,
                    &rename_match,
                );
                let Some(updated_specifier) = updated_specifier else {
                    continue;
                };
                if updated_specifier == entry.specifier {
                    continue;
                }

                // resolve the string literal span for the import target
                let source_span = parsed
                    .source_map()
                    .get_main_or_enclosing(entry.source_node_id);
                let enclosing = Span::new(module.file_id, source_span.start, source_span.end);
                let span = string_literal_span_in_enclosing(
                    &file,
                    parsed.tokens(),
                    enclosing,
                    &entry.specifier,
                )
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
        }) else {
            continue;
        };
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

    Some(FileRenameResult::from_edits(batch_edit))
}

/// Resolve file content for file rename edits.
fn file_for_rename(
    repository: &Repository,
    revision: Revision,
    file_id: FileId,
) -> Option<Arc<File>> {
    let file = repository.file(revision, file_id).ok().flatten()?;
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use super::super::specifier::{
        SpecifierPolicy, SpecifierRenameMatch, apply_rename_to_specifier, match_specifier_rename,
    };
    use destack_artifact::DiskCacheStore;
    use destack_source::{FileSystem, PathExt, PhysicalFileSystem};
    use destack_workspace::{HostEnvironment, Repository};

    /// Match absolute target paths against workspace relative rename entries.
    #[test]
    fn test_match_path_rename_entry_for_absolute_target() {
        let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
        let repository = Repository::new(
            PathBuf::from("/test"),
            Arc::new(DiskCacheStore::new()),
            file_system,
            HostEnvironment::capture_process(),
        );
        let workspace_root = repository.workspace_root().to_path_buf().normalize();
        let policy = SpecifierPolicy {
            fs: &**repository.file_system(),
            workspace_root: &workspace_root,
        };
        let rename_map = HashMap::from([(
            PathBuf::from("src/utils/foo.ds"),
            PathBuf::from("src/utils/bar.ds"),
        )]);

        assert_eq!(
            match_specifier_rename(
                &policy,
                &rename_map,
                Some(Path::new("/test/src/main.ds")),
                "@/unused",
                Some(Path::new("/test/src/utils/foo.ds")),
                None,
                None,
            ),
            Some(SpecifierRenameMatch {
                old_path: PathBuf::from("/test/src/utils/foo.ds"),
                new_path: PathBuf::from("/test/src/utils/bar.ds"),
                package_name: None,
                package_directory: None,
            })
        );
    }

    /// Rewrite alias specifiers from renamed workspace targets.
    #[test]
    fn test_rewrite_import_specifier_for_alias() {
        assert_eq!(
            apply_rename_to_specifier(
                Some(Path::new("/test/src/main.ds")),
                "@/utils/foo",
                &SpecifierRenameMatch {
                    old_path: PathBuf::from("src/utils/foo.ds"),
                    new_path: PathBuf::from("src/utils/bar.ds"),
                    package_name: None,
                    package_directory: None,
                },
            ),
            Some("@/utils/bar".to_string())
        );
    }

    /// Rewrite package specifiers from renamed package targets.
    #[test]
    fn test_rewrite_import_specifier_for_package() {
        assert_eq!(
            apply_rename_to_specifier(
                Some(Path::new("/test/src/main.ds")),
                "my_pkg/utils/foo",
                &SpecifierRenameMatch {
                    old_path: PathBuf::from("node_modules/my_pkg/utils/foo.ds"),
                    new_path: PathBuf::from("node_modules/my_pkg/utils/bar.ds"),
                    package_name: Some("my_pkg".to_string()),
                    package_directory: Some(PathBuf::from("node_modules/my_pkg")),
                },
            ),
            Some("my_pkg/utils/bar".to_string())
        );
    }
}
