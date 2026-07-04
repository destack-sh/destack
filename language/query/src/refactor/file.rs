use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use destack_serde::Reflect;
use destack_source::{File, FileId, FilePatch, Patch, PatchSet, PathExt, ProfileId};
use serde::{Deserialize, Serialize};

use super::specifier::{SpecifierPolicy, SpecifierRenames};
use crate::ProgramQueryContext;
use crate::source::string_literal_span_in_enclosing;

/// A file rename entry for refactor queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FileRenameEntry {
    /// The old path before the rename.
    pub old_path: PathBuf,
    /// The new path after the rename.
    pub new_path: PathBuf,
}

/// Request payload for file rename edits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RenameFilesRequest {
    /// Profiles that should participate in specifier rewrites.
    pub profile_ids: Vec<ProfileId>,
    /// The file rename entries to apply.
    pub renames: Vec<FileRenameEntry>,
}

/// Response payload for file rename queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RenameFilesResponse {
    /// File rename edit, if available.
    pub edit: Option<PatchSet>,
}

impl ProgramQueryContext<'_> {
    /// Resolve file content for file rename edits.
    fn read_rename_file(&self, file_id: FileId) -> Arc<File> {
        let repository = self.repository();
        let file = repository
            .file(self.revision(), file_id)
            .unwrap_or_else(|error| panic!("failed to read renamed file {file_id:?}: {error}"))
            .unwrap_or_else(|| panic!("missing renamed file {file_id:?}"));
        if file.has_line_index() {
            return file;
        }

        // load file content from the filesystem when repository text is unavailable
        let path = file
            .path
            .as_ref()
            .unwrap_or_else(|| panic!("missing path for renamed file {file_id:?}"));
        let content = repository
            .file_system()
            .read_to_string(path)
            .unwrap_or_else(|error| panic!("failed to read renamed file {path:?}: {error}"));
        let loaded = File::from_text(
            file.id,
            file.name.clone(),
            file.uri.clone(),
            file.path.clone(),
            file.ty,
            content,
        );

        Arc::new(loaded)
    }
}

/// One source string literal used for specifier rewrites.
struct StringLiteral<'a> {
    /// The source literal text.
    text: &'a str,
}

impl<'a> StringLiteral<'a> {
    /// Create a string literal view over source text.
    fn new(text: &'a str) -> Self {
        Self { text }
    }

    /// Return this literal's quote character.
    fn quote(&self) -> char {
        match self.text.as_bytes().first().copied() {
            Some(b'\'') => '\'',
            Some(b'`') => '`',
            _ => '"',
        }
    }

    /// Return new literal text with the original quote style.
    fn replace_text(&self, specifier: &str) -> String {
        let quote = self.quote();

        format!("{quote}{specifier}{quote}")
    }
}

impl ProgramQueryContext<'_> {
    /// Resolve file rename edits across the workspace.
    pub fn rename_files(&self, renames: &[FileRenameEntry]) -> Option<PatchSet> {
        let repository = self.repository();
        let revision = self.revision();

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
        let workspace_root = repository.path().to_path_buf().normalize();
        let specifier_policy = SpecifierPolicy {
            fs: &**repository.file_system(),
            workspace_root: &workspace_root,
        };
        let specifier_renames = SpecifierRenames::new(specifier_policy, &rename_map);

        // collect edits grouped by file id
        let mut edits_by_file: HashMap<FileId, Vec<Patch>> = HashMap::new();

        let mut entries_by_module = HashMap::new();
        let specifier_entries = self.renamed_specifiers(rename_map.keys().cloned());
        for (profile_id, entry) in specifier_entries {
            entries_by_module
                .entry((profile_id, entry.source.module_id))
                .or_insert_with(Vec::new)
                .push(entry);
        }

        // apply edits per owning module and profile
        for ((profile_id, module_id), entries) in entries_by_module {
            let module = repository
                .module(revision, module_id)
                .unwrap_or_else(|error| {
                    panic!("failed to read renamed module {module_id:?}: {error}")
                })
                .unwrap_or_else(|| panic!("missing renamed module {module_id:?}"));

            // resolve file content for literal edits
            let file = self.read_rename_file(module.file_id);

            let module = self.module_context(module.id, profile_id);
            for entry in &entries {
                // resolve the updated specifier text
                let rename_match = if let Some(target_module_id) = entry.target_module {
                    // prefer the semantic target path when it is available
                    let target_module = repository
                        .module(revision, target_module_id)
                        .unwrap_or_else(|error| {
                            panic!(
                                "failed to read renamed target module {target_module_id:?}: {error}"
                            )
                        })
                        .unwrap_or_else(|| {
                            panic!("missing renamed target module {target_module_id:?}")
                        });

                    // resolve package metadata for package specifiers
                    let package = repository
                        .package(revision, target_module.package_id)
                        .unwrap_or_else(|error| {
                            panic!(
                                "failed to read renamed target package {:?}: {error}",
                                target_module.package_id
                            )
                        })
                        .unwrap_or_else(|| {
                            panic!(
                                "missing renamed target package {:?}",
                                target_module.package_id
                            )
                        });
                    specifier_renames.match_specifier(
                        file.path.as_deref(),
                        &entry.text,
                        target_module.path.as_deref(),
                        package.name.as_deref(),
                        package.path.as_deref(),
                    )
                } else {
                    specifier_renames.match_specifier(
                        file.path.as_deref(),
                        &entry.text,
                        None,
                        None,
                        None,
                    )
                };
                let Some(rename_match) = rename_match else {
                    continue;
                };

                // rewrite the literal text using the matched rename
                let updated_specifier = rename_match.apply(file.path.as_deref(), &entry.text);
                let Some(updated_specifier) = updated_specifier else {
                    continue;
                };
                if updated_specifier == entry.text {
                    continue;
                }

                // resolve the string literal span for the import target
                let enclosing = entry.span;
                let span = string_literal_span_in_enclosing(
                    &file,
                    module.tokens(),
                    enclosing,
                    &entry.text,
                )?;
                let literal = file.span_str(span);
                if literal.is_empty() {
                    continue;
                }

                // build and store the edit
                let new_text = StringLiteral::new(literal).replace_text(&updated_specifier);
                edits_by_file
                    .entry(span.file)
                    .or_default()
                    .push(Patch::replace(span, new_text));
            }
        }

        // return early when no edits exist
        if edits_by_file.is_empty() {
            return None;
        }

        // build a batch edit from per file edits
        let mut batch_edit = PatchSet::new();
        for (file_id, edits) in edits_by_file {
            let mut file_edit = FilePatch::with_patches(file_id, edits);
            file_edit.sort();
            batch_edit.push(file_edit);
        }

        Some(batch_edit)
    }
}
