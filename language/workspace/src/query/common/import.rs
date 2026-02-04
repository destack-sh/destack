use std::path::Path;

use destack_ast::{DependencyKind, DependencyMode, Expression, TokenType};
use destack_source::{Edit, FileId, PathExt, Span};

use crate::Session;
use crate::query::common::{QueryContext, relative_path};

/// Information about an existing import in the file.
#[derive(Debug, Clone)]
pub struct ExistingImport {
    /// The import path (e.g., "./utils", "react").
    pub path: String,
    /// Whether this is a type only import statement.
    pub is_type_only: bool,
    /// Start byte offset of the import statement.
    pub start: u32,
    /// End byte offset of the import statement.
    pub end: u32,
    /// Position of the closing brace `}` (for inserting new specifiers).
    pub closing_brace_pos: Option<u32>,
    /// Whether this is a namespace import (`import * as foo`).
    pub is_namespace: bool,
    /// Existing specifier names.
    pub specifiers: Vec<String>,
}

/// Resolve the brace span for an import clause.
pub(crate) fn import_clause_brace_span(
    ctx: &QueryContext<'_>,
    import_span: Span,
    target_span: Option<Span>,
) -> Option<(Span, Span)> {
    // resolve the limit before the target string
    let target_limit = target_span
        .map(|span| span.start)
        .unwrap_or(import_span.end);

    // track the last brace pair before the target
    let mut open_brace = None;
    let mut close_brace = None;

    // scan tokens inside the import span
    for token in &ctx.ast.tokens {
        if token.span.file != ctx.file_id {
            continue;
        }

        if token.span.start < import_span.start {
            continue;
        }

        if token.span.start >= target_limit {
            break;
        }

        match token.token.ty {
            TokenType::OpenBrace => {
                open_brace = Some(token.span);
                close_brace = None;
            }
            TokenType::CloseBrace => {
                if open_brace.is_some() {
                    close_brace = Some(token.span);
                }
            }
            _ => {}
        }
    }

    let open_brace = open_brace?;
    let close_brace = close_brace?;

    if open_brace.start >= close_brace.start {
        return None;
    }

    Some((open_brace, close_brace))
}

/// The mode for a new import edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportEditMode {
    /// A value import.
    Value,
    /// A type only import.
    Type,
}

/// Import group category for ordering (matches formatter).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImportGroup {
    /// Builtin modules with protocol prefix (e.g. `node:fs`, `bun:test`).
    Builtin = 0,
    /// External packages (e.g. `lodash`, `@org/pkg`, `react`).
    Package = 1,
    /// Path aliases (e.g. `@/utils`, `~/lib`, `#internal`).
    Alias = 2,
    /// Relative imports (e.g. `./foo`, `../bar`).
    Relative = 3,
}

impl ImportGroup {
    /// Categorize an import target path into a group.
    pub fn from_path(path: &str) -> Self {
        // builtin protocols: "protocol:module" but not URLs ("protocol://...")
        if let Some(colon_pos) = path.find(':')
            && !path[colon_pos..].starts_with("://")
        {
            return ImportGroup::Builtin;
        }

        // relative imports
        if path.starts_with("./") || path.starts_with("../") || path.starts_with('/') {
            return ImportGroup::Relative;
        }

        // alias imports: @/ ~/ # (but not scoped packages like @org/pkg)
        if path.starts_with("@/") || path.starts_with("~/") || path.starts_with('#') {
            return ImportGroup::Alias;
        }

        // everything else is a package
        ImportGroup::Package
    }
}

/// Collect existing imports from a file's AST.
pub fn collect_existing_imports(session: &Session, file_id: FileId) -> Vec<ExistingImport> {
    // get module for this file
    let Some(module) = crate::query::common::get_module_by_file_id(session, file_id) else {
        return Vec::new();
    };
    let module = module.read();
    let Some(ctx) = session.query_context(&module) else {
        return Vec::new();
    };

    // prepare the import collection
    let mut imports = Vec::new();

    // iterate over import expressions in the AST
    for node_id in ctx.ast.tree.iter_nodes::<Expression>() {
        let expr = ctx.ast.tree.get(node_id);

        if let Expression::Import {
            target,
            items,
            kind,
            ..
        } = expr
        {
            // resolve import path, span, and kind
            let path = ctx.ast.strings.get(*target).to_string();
            let span = ctx.ast.tree.source_map.get(node_id.id);
            let is_type_only = *kind == DependencyKind::Type;

            // check if it's a namespace import
            let is_namespace = items.iter().any(|item_id| {
                let item = ctx.ast.tree.get(*item_id);
                item.mode == DependencyMode::Namespace
            });

            // collect specifier names
            let specifiers: Vec<String> = items
                .iter()
                .filter_map(|item_id| {
                    let item = ctx.ast.tree.get(*item_id);
                    if item.mode == DependencyMode::Namespace {
                        return None;
                    }
                    // use alias if present, otherwise name
                    item.alias
                        .or(item.name.map(|name| name.string()))
                        .map(|id| ctx.ast.strings.get(id).to_string())
                })
                .collect();

            // find closing brace position by scanning tokens
            let target_span = ctx.ast.tree.source_map.get_main(node_id.id);
            let closing_brace_pos = if is_namespace {
                None
            } else {
                import_clause_brace_span(&ctx, span, target_span).map(|(_, close)| close.start)
            };

            imports.push(ExistingImport {
                path,
                is_type_only,
                start: span.start,
                end: span.end,
                closing_brace_pos,
                is_namespace,
                specifiers,
            });
        }
    }

    // sort by position
    imports.sort_by_key(|i| i.start);

    // return collected imports
    imports
}

/// Build edit(s) to add an import for a symbol.
///
/// If there's an existing import from the same path, merges into it.
/// Otherwise, inserts a new import at the appropriate position based on import groups.
pub fn build_import_edits(
    session: &Session,
    file_id: FileId,
    symbol_name: &str,
    import_path: &str,
) -> Vec<Edit> {
    // default to value import edits
    build_import_edits_with_mode(
        session,
        file_id,
        symbol_name,
        import_path,
        ImportEditMode::Value,
    )
}

/// Build edit(s) to add an import for a symbol with a mode.
///
/// If there's an existing import from the same path, merges into it.
/// Otherwise, inserts a new import at the appropriate position based on import groups.
pub fn build_import_edits_with_mode(
    session: &Session,
    file_id: FileId,
    symbol_name: &str,
    import_path: &str,
    mode: ImportEditMode,
) -> Vec<Edit> {
    // collect existing imports for the file
    let existing_imports = collect_existing_imports(session, file_id);

    // check if there's already an import from this path
    if let Some(existing) = existing_imports.iter().find(|i| i.path == import_path) {
        // skip if already imported
        if existing.specifiers.contains(&symbol_name.to_string()) {
            return Vec::new();
        }

        // can't merge into namespace imports
        if existing.is_namespace {
            return build_new_import_edit(
                file_id,
                symbol_name,
                import_path,
                &existing_imports,
                mode,
            );
        }

        // decide whether to merge into the existing import
        let can_merge = match mode {
            ImportEditMode::Value => !existing.is_type_only,
            ImportEditMode::Type => true,
        };

        if can_merge {
            let brace_pos = existing.closing_brace_pos;
            if let Some(brace_pos) = brace_pos {
                // insert before closing brace, `{ a }` becomes `{ a, b }`
                let insert_text = if existing.specifiers.is_empty() {
                    format!(" {symbol_name} ")
                } else {
                    format!(", {symbol_name}")
                };

                return vec![Edit::insert(file_id, brace_pos, insert_text)];
            }
        }
    }

    // no existing import, add new one
    build_new_import_edit(file_id, symbol_name, import_path, &existing_imports, mode)
}

/// Build a display path for an import.
///
/// Tries to compute a relative path from the current file to the target module.
pub fn build_import_display_path(session: &Session, file_id: FileId, module_path: &str) -> String {
    build_import_display_path_with_options(session, file_id, module_path, true)
}

/// Build a display path for an import with optional extension stripping.
///
/// Tries to compute a relative path from the current file to the target module.
pub fn build_import_display_path_with_options(
    session: &Session,
    file_id: FileId,
    module_path: &str,
    strip_extension: bool,
) -> String {
    // resolve the source file path
    let source_file = session.files.get(file_id);
    let Some(source_path) = source_file.path.as_ref() else {
        return module_path.to_string();
    };

    // resolve the source directory
    let Some(source_dir) = source_path.parent() else {
        return module_path.to_string();
    };

    // compute a relative path to the target module
    let target_path = std::path::Path::new(module_path);
    // compute a relative path to the target module
    let relative =
        relative_path(source_dir, target_path).unwrap_or_else(|| target_path.normalize());
    let mut display_path = normalize_separators(&relative.to_string_lossy());

    // ensure a relative prefix for local imports
    if !display_path.starts_with("./") && !display_path.starts_with("../") {
        display_path = format!("./{display_path}");
    }

    // strip common source extensions when requested
    if strip_extension {
        display_path = strip_module_extension(&display_path);
    }

    // return the normalized display path
    display_path
}

/// Normalize path separators to forward slashes.
fn normalize_separators(path: &str) -> String {
    path.replace('\\', "/")
}

/// Resolve a module name from a file path.
pub(crate) fn module_name_from_path(path: &Path) -> Option<String> {
    // read the file name
    let file_name = path.file_name()?.to_string_lossy();

    // strip known module extensions
    Some(strip_module_extension(file_name.as_ref()))
}

/// Strip a code module extension from an import path.
/// Strip a code module extension from an import path.
pub fn strip_module_extension(path: &str) -> String {
    // prefer compound extensions before simple ones
    let extensions = [
        ".d.ts", ".d.mts", ".d.cts", ".d.ds", ".tsx", ".ts", ".mts", ".cts", ".jsx", ".js", ".mjs",
        ".cjs", ".ds",
    ];

    for extension in extensions {
        if let Some(stripped) = path.strip_suffix(extension) {
            return stripped.to_string();
        }
    }

    path.to_string()
}

/// Build an edit to insert a new import statement.
fn build_new_import_edit(
    file_id: FileId,
    symbol_name: &str,
    import_path: &str,
    existing_imports: &[ExistingImport],
    mode: ImportEditMode,
) -> Vec<Edit> {
    // resolve the import group
    let new_group = ImportGroup::from_path(import_path);

    // choose the import text for the mode
    let import_text = match mode {
        ImportEditMode::Value => format!("import {{ {symbol_name} }} from \"{import_path}\";\n"),
        ImportEditMode::Type => {
            format!("import type {{ {symbol_name} }} from \"{import_path}\";\n")
        }
    };

    // find the right position based on import groups
    let insert_pos = find_import_insert_position(import_path, new_group, existing_imports);

    // return the insertion edit
    vec![Edit::insert(file_id, insert_pos, import_text)]
}

/// Find the correct position to insert a new import.
fn find_import_insert_position(
    import_path: &str,
    new_group: ImportGroup,
    existing_imports: &[ExistingImport],
) -> u32 {
    // if no existing imports, insert at start
    if existing_imports.is_empty() {
        return 0;
    }

    // find the right position based on group ordering
    for existing in existing_imports {
        let existing_group = ImportGroup::from_path(&existing.path);

        // insert before first import of later group
        if new_group < existing_group {
            return existing.start;
        }

        // same group: alphabetical ordering
        if new_group == existing_group && import_path < existing.path.as_str() {
            return existing.start;
        }
    }

    // append after last import
    existing_imports.last().map(|i| i.end).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Order import groups by builtin, package, alias, then relative.
    #[test]
    fn test_import_group_ordering() {
        // compare group ordering
        assert!(ImportGroup::Builtin < ImportGroup::Package);
        assert!(ImportGroup::Package < ImportGroup::Alias);
        assert!(ImportGroup::Alias < ImportGroup::Relative);
    }

    /// Categorize import paths into the expected groups.
    #[test]
    fn test_import_group_categorization() {
        // compare group categorization for sample paths
        assert_eq!(ImportGroup::from_path("node:fs"), ImportGroup::Builtin);
        assert_eq!(ImportGroup::from_path("bun:test"), ImportGroup::Builtin);
        assert_eq!(ImportGroup::from_path("react"), ImportGroup::Package);
        assert_eq!(ImportGroup::from_path("@org/pkg"), ImportGroup::Package);
        assert_eq!(ImportGroup::from_path("lodash"), ImportGroup::Package);
        assert_eq!(ImportGroup::from_path("@/utils"), ImportGroup::Alias);
        assert_eq!(ImportGroup::from_path("~/lib"), ImportGroup::Alias);
        assert_eq!(ImportGroup::from_path("#internal"), ImportGroup::Alias);
        assert_eq!(ImportGroup::from_path("./local"), ImportGroup::Relative);
        assert_eq!(ImportGroup::from_path("../parent"), ImportGroup::Relative);
    }
}
