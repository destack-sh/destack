use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use destack_resolver::{SpecifierPolicy, match_specifier_rename, rewrite_specifier_for_rename};
use destack_source::{BatchEdit, Edit, File, FileEdit, FileId, ModuleId, PathExt, Span};
use serde::{Deserialize, Serialize};
use {destack_ast as ast, destack_dir as dir};

use crate::ast::string_literal_span_in_enclosing;
use crate::core::{QueryContext, with_ast_query_for_module};
use crate::dir::module_specifier_in_expression;
use destack_artifact::ImportEdgeKind;
use destack_workspace::Session;

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
pub fn rename_files(session: &Session, renames: &[FileRenameEntry]) -> Option<FileRenameResult> {
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
    let workspace_root = session.workspace.read().root.normalize();
    let specifier_policy = SpecifierPolicy {
        fs: &*session.fs,
        workspace_root: &workspace_root,
    };

    // collect edits grouped by file id
    let mut edits_by_file: HashMap<FileId, Vec<Edit>> = HashMap::new();

    // scan modules for import and re export targets
    for module_ref in session.modules.iter() {
        let module = module_ref.as_ref();
        let query_context = crate::core::query_context(session, module);

        // resolve file content for literal edits
        let Some(file) = file_for_rename(session, module.file_id) else {
            continue;
        };

        let Some(()) = with_ast_query_for_module(session, module, |ast| {
            let mut dir_targets = HashMap::new();
            if let Some(ctx) = query_context.as_ref() {
                let dir_tree = ctx.dir().tree();
                for (expr_id, expression) in dir_tree.iter_nodes_of_type::<dir::Expression>() {
                    let target_module = match expression {
                        dir::Expression::Import { target_module, .. }
                        | dir::Expression::ReExport { target_module, .. } => Some(*target_module),
                        _ => None,
                    };
                    let Some(target_module) = target_module else {
                        continue;
                    };

                    let source_id = dir_tree.get_source(expr_id.id);
                    dir_targets.insert(source_id, target_module);
                }
            }

            for expr_id in ast.tree().iter_nodes::<ast::Expression>() {
                // resolve the module specifier and dependency kind
                let expression = ast.tree().get(expr_id);
                let Some((target, kind)) = module_specifier_in_expression(ast.tree(), expression)
                else {
                    continue;
                };

                // resolve the specifier text
                let specifier_text = ast.strings().get(target).to_string();

                // resolve the target module id
                let target_module_id = if let Some(ctx) = query_context.as_ref() {
                    resolve_rename_target_module_id(
                        session,
                        ctx,
                        &dir_targets,
                        expr_id.id,
                        &specifier_text,
                        kind,
                    )
                } else {
                    None
                };

                // resolve the updated specifier text
                let rename_match = if let Some(target_module_id) = target_module_id {
                    // prefer the semantic target path when it is available
                    let target_module = session.modules.get(target_module_id);
                    let target_module = target_module.as_ref();

                    // resolve package metadata for package specifiers
                    let package = session.packages.get(target_module.package_id);
                    let package = package.read();
                    match_specifier_rename(
                        &specifier_policy,
                        &rename_map,
                        file.path.as_deref(),
                        &specifier_text,
                        target_module.path.as_deref(),
                        package.name.as_deref(),
                        package.path.as_deref(),
                    )
                } else {
                    match_specifier_rename(
                        &specifier_policy,
                        &rename_map,
                        file.path.as_deref(),
                        &specifier_text,
                        None,
                        None,
                        None,
                    )
                };
                let Some(rename_match) = rename_match else {
                    continue;
                };

                // rewrite the literal text using the matched rename
                let updated_specifier = rewrite_specifier_for_rename(
                    file.path.as_deref(),
                    &specifier_text,
                    &rename_match,
                );
                let Some(updated_specifier) = updated_specifier else {
                    continue;
                };
                if updated_specifier == specifier_text {
                    continue;
                }

                // resolve the string literal span for the import target
                let ast_span = ast.source_map().get_main_or_enclosing(expr_id.id);
                let enclosing = Span::new(module.file_id, ast_span.start, ast_span.end);
                let span = string_literal_span_in_enclosing(
                    &file,
                    ast.tokens(),
                    enclosing,
                    &specifier_text,
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

/// Resolve a module id for a rename target.
fn resolve_rename_target_module_id(
    session: &Session,
    ctx: &QueryContext,
    dir_targets: &HashMap<u32, dir::ModuleTarget>,
    ast_node_id: u32,
    specifier: &str,
    kind: ast::DependencyKind,
) -> Option<ModuleId> {
    // prefer resolved dir targets when available
    if let Some(target_module) = dir_targets.get(&ast_node_id)
        && let dir::ModuleTarget::Module(module_id) = target_module
    {
        return Some(*module_id);
    }

    // check the dir import cache for resolved modules
    let target_id = session.strings.intern(specifier);
    let cache_key = (
        Some(ctx.module_id()),
        target_id,
        ImportEdgeKind::Import,
        None,
    );
    if let Some(targets) = ctx
        .dir()
        .resolved()
        .imported_modules
        .get(&cache_key)
        .copied()
    {
        let dependency_kind = match kind {
            ast::DependencyKind::Type => dir::DependencyKind::Type,
            ast::DependencyKind::Value => dir::DependencyKind::Value,
        };
        if let Some(module_target) = targets.for_kind(dependency_kind)
            && let Some(module_id) = module_target.module_id()
        {
            return Some(module_id);
        }
    }

    None
}

/// Resolve file content for file rename edits.
fn file_for_rename(session: &Session, file_id: FileId) -> Option<Arc<File>> {
    // return the file when content is loaded
    let file = session.files.get(file_id);
    if file.is_loaded() && file.line_start_offsets.is_some() {
        return Some(file);
    }

    // load file content from the filesystem when session text is unavailable
    let path = file.path.as_ref()?;
    let content = session.fs.read_to_string(path).ok()?;
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

    use destack_resolver::{
        SpecifierPolicy, SpecifierRenameMatch, match_specifier_rename, rewrite_specifier_for_rename,
    };
    use destack_source::PathExt;
    use destack_workspace::Session;

    /// Match absolute target paths against workspace relative rename entries.
    #[test]
    fn test_match_path_rename_entry_for_absolute_target() {
        let session = Session::new(PathBuf::from("/test"));
        let workspace_root = session.workspace.read().root.normalize();
        let policy = SpecifierPolicy {
            fs: &*session.fs,
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
            rewrite_specifier_for_rename(
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
            rewrite_specifier_for_rename(
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
