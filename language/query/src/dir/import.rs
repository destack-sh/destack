use std::path::Path;

use destack_core::StringId;
use destack_dir as dir;
use destack_source::{Edit, FileId, PathExt, Span};

use crate::core::path::{normalize_separators, relative_path};
use crate::core::{DirQueryContext, ModuleQueryContext, WorkspaceQueryContext};
use crate::format::ImportGroup;

/// Information about an existing import in the file.
#[derive(Debug, Clone)]
pub(crate) struct ExistingImport {
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

/// The syntactic bounds of one import clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ImportClauseBounds {
    /// The clause opening brace.
    pub open_brace: Span,
    /// The explicit closing brace when it is present.
    pub close_brace: Option<Span>,
    /// The effective clause end boundary.
    pub end_boundary: Span,
}

/// Check whether a symbol matches a requested symbol space filter.
pub(crate) fn matches_symbol_space_filter(
    symbol_kind: dir::SymbolKind,
    filter: Option<dir::SymbolSpace>,
) -> bool {
    let Some(filter) = filter else {
        return true;
    };

    symbol_kind.is_visible_in(filter)
}

/// Check whether an exported lookup space matches a requested symbol space filter.
pub(crate) fn matches_export_space_filter(
    export_space: dir::SymbolSpace,
    filter: Option<dir::SymbolSpace>,
) -> bool {
    let Some(filter) = filter else {
        return true;
    };

    export_space == filter
}

/// Check whether a symbol matches an explicit import-clause space filter.
pub(crate) fn matches_import_clause_space_filter(
    symbol_kind: dir::SymbolKind,
    filter: Option<dir::SymbolSpace>,
) -> bool {
    let Some(filter) = filter else {
        return true;
    };

    matches_symbol_space_filter(symbol_kind, Some(filter))
}

impl ModuleQueryContext<'_> {
    /// Resolve the local alias text for one explicit import alias symbol.
    pub(crate) fn local_import_alias_name(&self, symbol_id: dir::GlobalSymbolId) -> Option<String> {
        let ctx = self.module_context(symbol_id.module_id)?;

        // read the symbol declaration
        let declaration = {
            let symbols = ctx.dir().symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            symbol.declaration?
        };

        if declaration.local_id.ty != dir::NodeType::DependencyItem {
            return None;
        }

        let item_id: dir::LocalNodeId<dir::DependencyItem> =
            declaration.local_id.try_into().ok()?;
        let dir_tree = ctx.dir().view();
        let item = dir_tree.get::<dir::DependencyItem>(item_id);
        let local_name_id = item.local_import_alias_name()?;

        Some(ctx.dir().strings().get(local_name_id).to_string())
    }

    /// Build a display path for one import.
    pub(crate) fn import_display_path(&self, module_path: &str) -> String {
        self.import_display_path_with_options(module_path, true)
    }

    /// Collect default import aliases whose imported default export resolves to one symbol.
    pub(crate) fn collect_default_import_alias_symbols_for_export(
        &self,
        workspace: &WorkspaceQueryContext<'_>,
        canonical_id: dir::GlobalSymbolId,
    ) -> Vec<dir::GlobalSymbolId> {
        let mut symbols = Vec::new();

        // scan modules that reference the target
        for module_id in workspace.modules_referencing_symbol(canonical_id) {
            let Some(ctx) = self.module_context(module_id) else {
                continue;
            };
            let dir = ctx.dir();

            let symbols_in_module = dir.symbols();
            for symbol_index in 0..symbols_in_module.symbol_count() {
                let local_symbol_id = dir::LocalSymbolId::new(symbol_index);
                let symbol_id = dir::GlobalSymbolId::new(dir.module_id(), local_symbol_id);
                if symbol_id == canonical_id {
                    continue;
                }

                let local_alias_name =
                    dir.local_default_import_alias_name_in_context(local_symbol_id);
                if local_alias_name.is_none() {
                    continue;
                }

                if dir.canonical_symbol(symbol_id) != canonical_id {
                    continue;
                }

                symbols.push(symbol_id);
            }
        }

        symbols
    }

    /// Build edit(s) to add an import for a symbol.
    ///
    /// If there's an existing import from the same path, merges into it.
    /// Otherwise, inserts a new import at the appropriate position based on import groups.
    pub(crate) fn build_import_edits(
        &self,
        symbol_name: &str,
        import_path: &str,
        import_form: ImportEditSpace,
    ) -> Vec<Edit> {
        // collect existing imports for the file
        let file_id = self.file_id();
        let existing_imports = self.dir().collect_existing_imports();

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
                    import_form,
                );
            }

            // decide whether to merge into the existing import
            let can_merge = match import_form {
                ImportEditSpace::Value => !existing.is_type_only,
                ImportEditSpace::Type => true,
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
        build_new_import_edit(
            file_id,
            symbol_name,
            import_path,
            &existing_imports,
            import_form,
        )
    }

    /// Build a display path for one import with optional extension stripping.
    pub(crate) fn import_display_path_with_options(
        &self,
        module_path: &str,
        strip_extension: bool,
    ) -> String {
        let repository = self.repository();
        let revision = self.revision();

        // resolve the source file path
        let Some(source_file) = repository.file(revision, self.file_id()).ok().flatten() else {
            return module_path.to_string();
        };
        let Some(source_path) = source_file.path.as_ref() else {
            return module_path.to_string();
        };

        // resolve the source directory
        let Some(source_dir) = source_path.parent() else {
            return module_path.to_string();
        };

        // prefer package-name specifiers for external package targets
        let target_path = std::path::Path::new(module_path);
        if let Some(display_path) =
            self.build_external_package_display_path(source_path, target_path, strip_extension)
        {
            return display_path;
        }

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

        display_path
    }
}

/// The import space for a new import edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImportEditSpace {
    /// A value import.
    Value,
    /// A type only import.
    Type,
}

impl ImportEditSpace {
    /// Resolve the auto import edit space for one requested space and exported symbol space.
    pub(crate) fn for_auto_import(
        requested_space: Option<dir::SymbolSpace>,
        symbol_space: dir::SymbolSpace,
    ) -> Self {
        if requested_space != Some(dir::SymbolSpace::Type) {
            return Self::Value;
        }

        if symbol_space == dir::SymbolSpace::Type {
            return Self::Type;
        }

        Self::Value
    }
}

/// Resolve a module specifier and dependency form for a source expression.
pub(crate) fn module_specifier_in_expression(
    expression: &dir::Expression,
) -> Option<(StringId, dir::DependencyForm)> {
    match expression {
        dir::Expression::Import { target, form, .. } => Some((*target, *form)),
        dir::Expression::Export { target, form, .. } => target.map(|target| (target, *form)),
        _ => None,
    }
}

/// Resolve a module name from a file path.
pub(crate) fn module_name_from_path(path: &Path) -> Option<String> {
    // read the file name
    let file_name = path.file_name()?.to_string_lossy();

    // strip known module extensions
    Some(strip_module_extension(file_name.as_ref()))
}

/// Strip a code module extension from an import path.
pub(crate) fn strip_module_extension(path: &str) -> String {
    // prefer compound extensions before simple ones
    let extensions = [".d.ts", ".d.ds", ".tsx", ".ts", ".jsx", ".js", ".ds"];

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
    import_form: ImportEditSpace,
) -> Vec<Edit> {
    // resolve the import group
    let new_group = ImportGroup::from_path(import_path);

    // choose the import text for the space
    let import_text = match import_form {
        ImportEditSpace::Value => format!("import {{ {symbol_name} }} from \"{import_path}\";\n"),
        ImportEditSpace::Type => {
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

impl DirQueryContext<'_> {
    /// Collect existing imports from one source query surface.
    fn collect_existing_imports(self) -> Vec<ExistingImport> {
        let source = self;

        // prepare the import collection
        let mut imports = Vec::new();

        // iterate over import expressions in the source DIR
        for node_id in source.tree().iter_nodes::<dir::Expression>() {
            let expr = source.tree().get(node_id);

            if let dir::Expression::Import {
                target,
                items,
                form,
                ..
            } = expr
            {
                // resolve import path, span, and form
                let path = source.strings().get(*target).to_string();
                let span = source.tree().source_index.get(node_id.id);
                let is_type_only = *form == dir::DependencyForm::Type;
                let items = items.as_deref().unwrap_or(&[]);

                // check if it's a namespace import
                let is_namespace = items.iter().any(|item_id| {
                    let item = source.tree().get(*item_id);
                    item.binding() == Some(dir::DependencyBinding::Namespace)
                });

                // collect specifier names
                let specifiers: Vec<String> = items
                    .iter()
                    .filter_map(|item_id| {
                        let item = source.tree().get(*item_id);
                        if item.binding() == Some(dir::DependencyBinding::Namespace) {
                            return None;
                        }

                        item.local_string_key()
                            .map(|id| source.strings().get(id).to_string())
                    })
                    .collect();

                // find closing brace position by scanning tokens
                let target_span = source.tree().source_index.get_main(node_id.id);
                let closing_brace_pos = if is_namespace {
                    None
                } else {
                    source
                        .import_clause_brace_span(span, target_span)
                        .map(|(_, close)| close.start)
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

        imports.sort_by_key(|import| import.start);

        imports
    }

    /// Check whether a symbol is a local import alias for a canonical target.
    pub(crate) fn is_dependency_alias_for_target(
        self,
        symbol_id: dir::GlobalSymbolId,
        canonical_target: dir::GlobalSymbolId,
    ) -> bool {
        let dir = self;

        // read the symbol declaration
        let declaration = {
            let symbols = dir.symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            symbol.declaration
        };
        let Some(declaration) = declaration else {
            return false;
        };

        // bail out when the declaration is not a dependency item
        if declaration.local_id.ty != dir::NodeType::DependencyItem {
            return false;
        }

        // resolve the dependency item node
        let Ok(item_id): Result<dir::LocalNodeId<dir::DependencyItem>, _> =
            declaration.local_id.try_into()
        else {
            return false;
        };

        // check for an alias that targets the canonical symbol
        let dir_tree = dir.view();
        let item = dir_tree.get::<dir::DependencyItem>(item_id);
        let (alias, binding) = match item {
            dir::DependencyItem::Binding { alias, binding, .. } => (alias, binding),
            _ => return false,
        };

        // require an explicit alias
        if alias.is_none() {
            return false;
        }

        // allow default imports to be renamed with their targets
        if *binding == dir::DependencyBinding::Default {
            return false;
        }

        // compare canonical targets
        let Some(target_symbol) = dir.dependency_symbol_target(item_id) else {
            return false;
        };
        let target_canonical = dir.canonical_symbol(target_symbol);
        target_canonical == canonical_target
    }

    /// Resolve the bounds for an import clause, even when the closing brace is missing.
    pub(crate) fn import_clause_bounds(
        self,
        import_span: Span,
        target_span: Option<Span>,
    ) -> Option<ImportClauseBounds> {
        let source = self;

        // resolve the limit before the target string
        let target_limit = target_span
            .map(|span| span.start)
            .unwrap_or(import_span.end);

        // track the last brace pair before the target
        let mut open_brace = None;
        let mut close_brace = None;

        // scan tokens inside the import span
        for token in source.tokens() {
            if token.span.file != source.file_id() {
                continue;
            }
            if token.span.start < import_span.start {
                continue;
            }
            if token.span.start >= target_limit {
                break;
            }

            match token.token.ty() {
                dir::TokenType::OpenBrace => {
                    open_brace = Some(token.span);
                    close_brace = None;
                }
                dir::TokenType::CloseBrace => {
                    if open_brace.is_some() {
                        close_brace = Some(token.span);
                    }
                }
                _ => {}
            }
        }

        let open_brace = open_brace?;
        let end_boundary =
            close_brace.unwrap_or_else(|| Span::new(open_brace.file, target_limit, target_limit));

        if open_brace.start >= end_boundary.start {
            return None;
        }

        Some(ImportClauseBounds {
            open_brace,
            close_brace,
            end_boundary,
        })
    }

    /// Resolve the local binding name for one default import symbol inside a query context.
    fn local_default_import_alias_name_in_context(
        self,
        local_symbol_id: dir::LocalSymbolId,
    ) -> Option<String> {
        let dir = self;

        let declaration = {
            let symbols = dir.symbols();
            let symbol = symbols.get_symbol(local_symbol_id);
            symbol.declaration?
        };

        if declaration.local_id.ty != dir::NodeType::DependencyItem {
            return None;
        }

        let item_id: dir::LocalNodeId<dir::DependencyItem> =
            declaration.local_id.try_into().ok()?;
        let local_name_id = dir
            .view()
            .get::<dir::DependencyItem>(item_id)
            .default_import_alias_name()?;

        Some(dir.strings().get(local_name_id).to_string())
    }

    /// Resolve the brace span for an import clause.
    pub(crate) fn import_clause_brace_span(
        self,
        import_span: Span,
        target_span: Option<Span>,
    ) -> Option<(Span, Span)> {
        let source = self;
        let bounds = source.import_clause_bounds(import_span, target_span)?;
        let close_brace = bounds.close_brace?;

        Some((bounds.open_brace, close_brace))
    }
}

impl ModuleQueryContext<'_> {
    /// Build one package-name display path for an external package target.
    fn build_external_package_display_path(
        &self,
        source_path: &Path,
        target_path: &Path,
        strip_extension: bool,
    ) -> Option<String> {
        let ctx = self;
        let repository = ctx.repository();
        let revision = ctx.revision();

        // resolve the owning package for the target path
        let target_package = repository
            .module_ids(revision)
            .ok()?
            .into_iter()
            .filter_map(|module_id| {
                let module = repository.module(revision, module_id).ok()??;
                let package = repository.package(revision, module.package_id).ok()??;
                let package_path = package.path.as_ref()?;
                if !target_path.starts_with(package_path) {
                    return None;
                }

                Some((package_path.as_os_str().len(), package))
            })
            .max_by_key(|(package_length, _)| *package_length)
            .map(|(_, package)| package)?;
        let target_package_path = target_package.path.as_ref()?;
        let target_package_name = target_package.name.as_ref()?;

        // keep relative imports inside the same package
        if source_path.starts_with(target_package_path) {
            return None;
        }

        // only rewrite dependency packages into bare package specifiers
        if !target_package_path
            .components()
            .any(|component| component.as_os_str() == "node_modules")
        {
            return None;
        }

        // build the package subpath from the package root
        let package_relative = target_path.strip_prefix(target_package_path).ok()?;
        let package_relative = normalize_separators(&package_relative.to_string_lossy());
        let package_relative = if strip_extension {
            strip_module_extension(&package_relative)
        } else {
            package_relative
        };

        if package_relative.is_empty() {
            return Some(target_package_name.to_string());
        }

        Some(format!("{target_package_name}/{package_relative}"))
    }
}
