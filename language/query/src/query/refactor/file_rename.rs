use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{BatchEdit, Edit, File, FileEdit, FileId, ModuleId, PathExt, Span};
use serde::{Deserialize, Serialize};
use {destack_ast as ast, destack_dir as dir};

use crate::common::{
    QueryContext, build_import_display_path_with_options, common_suffix_len, is_alias_specifier,
    module_specifier_in_expression, normalize_separators, relative_path,
    resolve_module_id_for_import_target, rewrite_path_with_common_suffix, split_alias_prefix,
    string_literal_span_in_enclosing, strip_module_extension, strip_path_extension,
    strip_path_suffix,
};
use destack_workspace::{ImportEdgeKind, ModuleSpecifier, Session};

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

    // collect edits grouped by file id
    let mut edits_by_file: HashMap<FileId, Vec<Edit>> = HashMap::new();

    // scan modules for import and re export targets
    for module_ref in session.modules.iter() {
        let module = module_ref.read();
        let Some(ctx) = crate::query_context(session, &module) else {
            continue;
        };

        // resolve file content for literal edits
        let Some(file) = file_for_rename(session, ctx.file_id) else {
            continue;
        };

        // scan import and re export expressions
        let dir_tree = ctx.tree();
        let mut dir_targets = HashMap::new();
        for (expr_id, expression) in dir_tree.iter_nodes_of_type::<dir::Expression>() {
            // collect resolved targets from dir expressions
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

        for expr_id in ctx.ast.tree.iter_nodes::<ast::Expression>() {
            // resolve the module specifier and dependency kind
            let expression = ctx.ast.tree.get(expr_id);
            let Some((target, kind)) = module_specifier_in_expression(&ctx.ast.tree, expression)
            else {
                continue;
            };

            // resolve the specifier text
            let specifier_text = ctx.ast.strings.get(target).to_string();

            // resolve the target module id
            let target_module_id = resolve_rename_target_module_id(
                session,
                &ctx,
                &dir_targets,
                expr_id.id,
                &specifier_text,
                kind,
            );
            // resolve the updated specifier text
            let updated_specifier = if let Some(target_module_id) = target_module_id {
                // resolve the target path and rename entry
                let target_module = session.modules.get(target_module_id);
                let target_module = target_module.read();
                let (target_path, new_path) = if let Some(target_path) = target_module.path.as_ref()
                {
                    let target_path = target_path.normalize();
                    if let Some(new_path) = rename_map.get(&target_path) {
                        (target_path, new_path.clone())
                    } else if let Some((target_path, new_path)) =
                        match_directory_rename_target(session, &rename_map, &target_path)
                    {
                        (target_path, new_path)
                    } else if let Some((target_path, new_path)) =
                        match_file_uri_rename_entry(session, &rename_map, &specifier_text)
                    {
                        (target_path, new_path)
                    } else if let Some((target_path, new_path)) =
                        match_absolute_rename_entry(session, &rename_map, &specifier_text)
                    {
                        (target_path, new_path)
                    } else {
                        continue;
                    }
                } else if let Some((target_path, new_path)) =
                    match_file_uri_rename_entry(session, &rename_map, &specifier_text)
                {
                    (target_path, new_path)
                } else if let Some((target_path, new_path)) =
                    match_absolute_rename_entry(session, &rename_map, &specifier_text)
                {
                    (target_path, new_path)
                } else {
                    continue;
                };

                // resolve package metadata for package specifiers
                let package = session.packages.get(target_module.package_id);
                let package = package.read();
                let package_name = package.name.as_deref();
                let package_dir = package.path.as_deref();

                rewrite_import_specifier(
                    session,
                    &file,
                    &specifier_text,
                    &target_path,
                    &new_path,
                    package_name,
                    package_dir,
                )
            } else if let Some((target_path, new_path)) =
                match_file_uri_rename_entry(session, &rename_map, &specifier_text)
            {
                rewrite_import_specifier(
                    session,
                    &file,
                    &specifier_text,
                    &target_path,
                    &new_path,
                    None,
                    None,
                )
            } else if let Some((target_path, new_path)) =
                match_absolute_rename_entry(session, &rename_map, &specifier_text)
            {
                rewrite_import_specifier(
                    session,
                    &file,
                    &specifier_text,
                    &target_path,
                    &new_path,
                    None,
                    None,
                )
            } else {
                let Some((target_path, new_path)) =
                    match_alias_rename_entry(&rename_map, &specifier_text)
                else {
                    continue;
                };

                rewrite_import_specifier(
                    session,
                    &file,
                    &specifier_text,
                    &target_path,
                    &new_path,
                    None,
                    None,
                )
            };
            let Some(updated_specifier) = updated_specifier else {
                continue;
            };
            if updated_specifier == specifier_text {
                continue;
            }

            // resolve the string literal span for the import target
            let ast_span = ctx.ast.tree.source_map.get_main_or_enclosing(expr_id.id);
            let enclosing = Span::new(ctx.file_id, ast_span.start, ast_span.end);
            let span = string_literal_span_in_enclosing(
                &file,
                &ctx.ast.tokens,
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
    ctx: &QueryContext<'_>,
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
    let relative_module = if specifier.starts_with("./") || specifier.starts_with("../") {
        Some(ctx.module_id)
    } else {
        None
    };
    let target_id = session.strings.intern(specifier);
    let cache_key = (relative_module, target_id, ImportEdgeKind::Import, None);
    if let Some(targets) = ctx.dir.imported_modules.get(&cache_key).copied() {
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

    // resolve relative and absolute specifiers directly
    if specifier.starts_with("./")
        || specifier.starts_with("../")
        || specifier.starts_with('/')
        || specifier.starts_with("file://")
    {
        return resolve_module_id_for_import_target(session, ctx, specifier);
    }

    None
}

/// Resolve a rename entry for file uri specifiers.
fn match_file_uri_rename_entry(
    session: &Session,
    rename_map: &HashMap<PathBuf, PathBuf>,
    specifier: &str,
) -> Option<(PathBuf, PathBuf)> {
    if !specifier.starts_with("file://") {
        return None;
    }

    let parsed = ModuleSpecifier::parse(specifier);
    let path_part = parsed.path.trim_start_matches("file://");
    if path_part.is_empty() {
        return None;
    }

    let path = Path::new(path_part).normalize();
    match_path_rename_entry(session, rename_map, &path)
}

/// Resolve a rename entry for absolute path specifiers.
fn match_absolute_rename_entry(
    session: &Session,
    rename_map: &HashMap<PathBuf, PathBuf>,
    specifier: &str,
) -> Option<(PathBuf, PathBuf)> {
    let parsed = ModuleSpecifier::parse(specifier);
    let path = Path::new(parsed.path());
    if !path.is_absolute() {
        return None;
    }

    let path = path.normalize();
    match_path_rename_entry(session, rename_map, &path)
}

/// Resolve a rename entry for alias specifiers when dir resolution fails.
fn match_alias_rename_entry(
    rename_map: &HashMap<PathBuf, PathBuf>,
    specifier: &str,
) -> Option<(PathBuf, PathBuf)> {
    // extract the alias suffix
    let (_prefix, suffix) = split_alias_prefix(specifier)?;
    let suffix = suffix.trim_start_matches('/');
    if suffix.is_empty() {
        return None;
    }

    // match rename entries by suffix
    let suffix_path = Path::new(suffix);
    for (old_path, new_path) in rename_map {
        if strip_path_suffix(old_path, suffix_path).is_some() {
            return Some((old_path.clone(), new_path.clone()));
        }

        if suffix_path.extension().is_none() {
            let Some(old_no_ext) = strip_path_extension(old_path) else {
                continue;
            };
            if strip_path_suffix(&old_no_ext, suffix_path).is_some() {
                return Some((old_path.clone(), new_path.clone()));
            }
        }
    }

    None
}

/// Resolve a rename entry for directory moves.
fn match_directory_rename_target(
    session: &Session,
    rename_map: &HashMap<PathBuf, PathBuf>,
    target_path: &Path,
) -> Option<(PathBuf, PathBuf)> {
    let workspace_root = session.workspace.read().root.normalize();
    let absolute_target = if target_path.is_absolute() {
        target_path.to_path_buf()
    } else {
        workspace_root.join(target_path)
    };

    for (old_path, new_path) in rename_map {
        if !is_directory_rename_entry(session, old_path, new_path) {
            continue;
        }

        let absolute_old = if old_path.is_absolute() {
            old_path.to_path_buf()
        } else {
            workspace_root.join(old_path)
        };

        let Ok(relative) = absolute_target.strip_prefix(&absolute_old) else {
            continue;
        };
        if relative.as_os_str().is_empty() {
            continue;
        }

        let updated = if new_path.is_absolute() {
            new_path.join(relative)
        } else {
            workspace_root.join(new_path).join(relative)
        };

        return Some((target_path.to_path_buf(), updated));
    }

    None
}

/// Match a specifier path to the best rename map entry.
fn match_path_rename_entry(
    session: &Session,
    rename_map: &HashMap<PathBuf, PathBuf>,
    specifier_path: &Path,
) -> Option<(PathBuf, PathBuf)> {
    // check for exact path matches first
    if let Some(new_path) = rename_map.get(specifier_path) {
        return Some((specifier_path.to_path_buf(), new_path.clone()));
    }

    // resolve directory rename entries
    let workspace_root = session.workspace.read().root.normalize();

    // check normalized file rename entries, including extensionless specifiers
    for (old_path, new_path) in rename_map {
        let absolute_old = if old_path.is_absolute() {
            old_path.to_path_buf()
        } else {
            workspace_root.join(old_path)
        };
        let absolute_new = if new_path.is_absolute() {
            new_path.to_path_buf()
        } else {
            workspace_root.join(new_path)
        };

        if specifier_path == absolute_old {
            return Some((absolute_old, absolute_new));
        }

        if specifier_path.extension().is_none()
            && let Some(old_no_extension) = strip_path_extension(&absolute_old)
            && specifier_path == old_no_extension
        {
            return Some((absolute_old, absolute_new));
        }

        // match absolute specifiers against workspace-root relative rename suffixes
        if specifier_path.is_absolute() {
            let old_suffix = old_path
                .strip_prefix(&workspace_root)
                .unwrap_or(old_path.as_path());

            if strip_path_suffix(specifier_path, old_suffix).is_some() {
                return Some((old_path.clone(), new_path.clone()));
            }

            if specifier_path.extension().is_none()
                && let Some(old_no_extension) = strip_path_extension(old_suffix)
                && strip_path_suffix(specifier_path, &old_no_extension).is_some()
            {
                return Some((old_path.clone(), new_path.clone()));
            }

            let shared_suffix = common_suffix_len(specifier_path, old_path);
            if shared_suffix >= 2 {
                return Some((old_path.clone(), new_path.clone()));
            }

            if specifier_path.extension().is_none()
                && let Some(old_no_extension) = strip_path_extension(old_path)
            {
                let shared_suffix_no_extension =
                    common_suffix_len(specifier_path, &old_no_extension);
                if shared_suffix_no_extension >= 2 {
                    return Some((old_path.clone(), new_path.clone()));
                }
            }
        }
    }

    for (old_path, new_path) in rename_map {
        if !is_directory_rename_entry(session, old_path, new_path) {
            continue;
        }

        let absolute_old = if old_path.is_absolute() {
            old_path.to_path_buf()
        } else {
            workspace_root.join(old_path)
        };

        if let Ok(relative) = specifier_path.strip_prefix(&absolute_old)
            && !relative.as_os_str().is_empty()
        {
            let updated = if new_path.is_absolute() {
                new_path.join(relative)
            } else {
                workspace_root.join(new_path).join(relative)
            };
            return Some((specifier_path.to_path_buf(), updated));
        }
    }

    None
}

/// Check whether a rename entry represents a directory move.
fn is_directory_rename_entry(session: &Session, old_path: &Path, new_path: &Path) -> bool {
    if session
        .fs
        .metadata(old_path)
        .map(|meta| meta.is_directory)
        .unwrap_or(false)
    {
        return true;
    }
    if session
        .fs
        .metadata(new_path)
        .map(|meta| meta.is_directory)
        .unwrap_or(false)
    {
        return true;
    }

    old_path.extension().is_none() && new_path.extension().is_none()
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

/// Rewrite an import specifier for a renamed target file.
fn rewrite_import_specifier(
    session: &Session,
    file: &File,
    specifier: &str,
    old_path: &Path,
    new_path: &Path,
    package_name: Option<&str>,
    package_dir: Option<&Path>,
) -> Option<String> {
    // require a source path to compute relative specifiers
    let _source_path = file.path.as_ref()?;

    // parse out query and fragment parts
    let parsed = ModuleSpecifier::parse(specifier);

    // preserve file specifiers by rewriting the file uri path
    if specifier.starts_with("file://") {
        let specifier_path = parsed.path.trim_start_matches("file://");
        if specifier_path.is_empty() {
            return None;
        }

        let updated_path =
            rewrite_path_with_common_suffix(Path::new(specifier_path), old_path, new_path);
        let mut updated = file_uri_for_path(&updated_path);
        if let Some(query) = parsed.query.as_ref() {
            updated.push_str(query);
        }
        if let Some(fragment) = parsed.fragment.as_ref() {
            updated.push_str(fragment);
        }

        return Some(updated);
    }

    // preserve extension style when present
    let spec_path = parsed.path();
    let has_extension = Path::new(spec_path).extension().is_some();
    let strip_extension = !has_extension;

    // rewrite relative specifiers
    let updated = if specifier.starts_with("./") || specifier.starts_with("../") {
        build_import_display_path_with_options(
            session,
            file.id,
            &new_path.to_string_lossy(),
            strip_extension,
        )
    } else if spec_path.starts_with('/') {
        // rewrite absolute specifiers while preserving the original root
        let updated_path =
            rewrite_path_with_common_suffix(Path::new(spec_path), old_path, new_path);
        let mut updated = normalize_separators(&updated_path.to_string_lossy());
        if strip_extension {
            updated = strip_module_extension(&updated);
        }
        updated
    } else if is_alias_specifier(spec_path) {
        // rewrite alias specifiers based on old target paths
        let mut updated = rewrite_alias_specifier(spec_path, old_path, new_path)?;
        if strip_extension {
            updated = strip_module_extension(&updated);
        }
        updated
    } else if let (Some(package_name), Some(package_dir)) = (package_name, package_dir) {
        // rewrite package specifiers that match the target package
        let package_prefix = format!("{package_name}/");
        let specifier_suffix = spec_path.strip_prefix(&package_prefix)?;
        if specifier_suffix.is_empty() {
            return None;
        }

        let relative = relative_path(package_dir, new_path)?;
        let mut relative_str = normalize_separators(&relative.to_string_lossy());
        if strip_extension {
            relative_str = strip_module_extension(&relative_str);
        }
        if relative_str.is_empty() || relative_str == ".." || relative_str.starts_with("../") {
            return None;
        }

        format!("{package_name}/{relative_str}")
    } else {
        return None;
    };

    // reattach query or fragment suffixes
    let mut updated = updated;
    if let Some(query) = parsed.query.as_ref() {
        updated.push_str(query);
    }
    if let Some(fragment) = parsed.fragment.as_ref() {
        updated.push_str(fragment);
    }

    Some(updated)
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

/// Build a file uri from a path for specifiers.
fn file_uri_for_path(path: &Path) -> String {
    let path_str = normalize_separators(&path.to_string_lossy());
    if path.is_absolute() && !path_str.starts_with('/') {
        return format!("file:///{path_str}");
    }

    format!("file://{path_str}")
}

/// Rewrite an alias specifier based on old and new target paths.
fn rewrite_alias_specifier(specifier: &str, old_path: &Path, new_path: &Path) -> Option<String> {
    let (prefix, suffix) = split_alias_prefix(specifier)?;
    let suffix = suffix.trim_start_matches('/');
    if suffix.is_empty() {
        return None;
    }

    // normalize the paths for comparisons
    let old_path = old_path.normalize();
    let new_path = new_path.normalize();
    let suffix_path = Path::new(suffix);

    // resolve the alias root by trimming the suffix from the old path
    let alias_root = strip_path_suffix(&old_path, suffix_path).or_else(|| {
        if suffix_path.extension().is_some() {
            return None;
        }

        let old_path_no_ext = strip_path_extension(&old_path)?;
        strip_path_suffix(&old_path_no_ext, suffix_path)
    })?;

    // compute the new alias relative path
    let relative = relative_path(&alias_root, &new_path)?;
    if relative
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return None;
    }

    let relative_str = normalize_separators(&relative.to_string_lossy());
    Some(format!("{prefix}{relative_str}"))
}
