use std::path::{Component, Path, PathBuf};

use destack_dir as dir;
use destack_source::{FileId, Patch, PathExt, Span};

use crate::format::ImportGroup;
use crate::{ModuleQueryContext, ProgramQueryContext, SymbolUse, strip_module_extension};

/// Information about an existing import in the file.
#[derive(Debug, Clone)]
struct ExistingImport {
    /// The import path.
    path: String,
    /// Whether this is a type only import statement.
    is_type_only: bool,
    /// Start byte offset of the import statement.
    start: u32,
    /// End byte offset of the import statement.
    end: u32,
    /// Position of the closing brace.
    closing_brace_pos: Option<u32>,
    /// Whether this is a namespace import.
    is_namespace: bool,
    /// Existing specifier names.
    specifiers: Vec<String>,
}

impl ExistingImport {
    /// Return whether this import already contains one specifier.
    fn contains_specifier(&self, symbol_name: &str) -> bool {
        self.specifiers
            .iter()
            .any(|specifier| specifier == symbol_name)
    }

    /// Return whether this import can accept a specifier of the requested form.
    fn accepts_form(&self, import_form: ImportEditForm) -> bool {
        match import_form {
            ImportEditForm::Value => !self.is_type_only,
            ImportEditForm::Type => true,
        }
    }

    /// Return insertion text for one added named specifier.
    fn specifier_insert_text(&self, symbol_name: &str) -> String {
        if self.specifiers.is_empty() {
            format!(" {symbol_name} ")
        } else {
            format!(", {symbol_name}")
        }
    }
}

/// The syntactic bounds of one import clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ImportClauseBounds {
    /// The clause opening brace.
    pub open_brace: Span,
    /// The clause closing brace.
    pub close_brace: Span,
    /// The effective clause end boundary.
    pub end_boundary: Span,
}

impl ImportClauseBounds {
    /// Return this clause's opening and closing brace spans.
    fn brace_span(self) -> (Span, Span) {
        (self.open_brace, self.close_brace)
    }
}

/// The syntactic form for a new import edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImportEditForm {
    /// A value import.
    Value,
    /// A type only import.
    Type,
}

impl ImportEditForm {
    /// Resolve the auto import form for one requested use and exported symbol kind.
    pub(crate) fn auto_import(
        requested_use: Option<SymbolUse>,
        symbol_kind: dir::SymbolKind,
    ) -> Self {
        if requested_use != Some(SymbolUse::Type) {
            return Self::Value;
        }

        if symbol_kind.can_be_used_as_type() && !symbol_kind.can_be_used_as_value() {
            return Self::Type;
        }

        Self::Value
    }

    /// Build edits to insert a new import statement.
    fn new_import_edits(
        self,
        file_id: FileId,
        symbol_name: &str,
        import_path: &str,
        existing_imports: &[ExistingImport],
    ) -> Vec<Patch> {
        let new_group = ImportGroup::from_path(import_path);
        let import_text = self.import_text(symbol_name, import_path);
        let insert_pos = Self::insert_position(import_path, new_group, existing_imports);

        vec![Patch::insert(file_id, insert_pos, import_text)]
    }

    /// Return the import statement text for this import form.
    fn import_text(self, symbol_name: &str, import_path: &str) -> String {
        match self {
            Self::Value => format!("import {{ {symbol_name} }} from \"{import_path}\";\n"),
            Self::Type => format!("import type {{ {symbol_name} }} from \"{import_path}\";\n"),
        }
    }

    /// Find the correct position to insert a new import.
    fn insert_position(
        import_path: &str,
        new_group: ImportGroup,
        existing_imports: &[ExistingImport],
    ) -> u32 {
        if existing_imports.is_empty() {
            return 0;
        }

        for existing in existing_imports {
            let existing_group = ImportGroup::from_path(&existing.path);
            if new_group < existing_group {
                return existing.start;
            }
            if new_group == existing_group && import_path < existing.path.as_str() {
                return existing.start;
            }
        }

        match existing_imports.last() {
            Some(existing) => existing.end,
            None => 0,
        }
    }
}

impl ModuleQueryContext<'_> {
    /// Resolve the local alias text for one explicit import alias symbol.
    pub(crate) fn local_import_alias_name(&self, symbol_id: dir::GlobalSymbolId) -> Option<String> {
        let module = self.module_context(symbol_id.module_id);
        let declaration = {
            let symbols = module.symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            symbol.declaration?
        };

        if declaration.local_id.ty != dir::NodeType::DependencyItem {
            return None;
        }

        let item_id = declaration
            .local_id
            .try_into_typed::<dir::DependencyItem>()
            .unwrap_or_else(|_| {
                panic!(
                    "import alias declaration is not a dependency item: {:?}",
                    declaration.local_id
                )
            });
        let item = module.view().get::<dir::DependencyItem>(item_id);
        let local_name_id = item.local_import_alias_name()?;

        Some(module.strings().get(local_name_id).to_string())
    }

    /// Collect default import aliases whose imported default export resolves to one symbol.
    pub(crate) fn default_import_alias_symbols(
        &self,
        program: &ProgramQueryContext<'_>,
        canonical_id: dir::GlobalSymbolId,
    ) -> Vec<dir::GlobalSymbolId> {
        let mut symbols = Vec::new();

        for module_id in program.referencing_modules(canonical_id) {
            let module = self.module_context(module_id);

            let symbols_in_module = module.symbols();
            for symbol_index in 0..symbols_in_module.symbol_count() {
                let local_symbol_id = dir::LocalSymbolId::new(symbol_index);
                let symbol_id = dir::GlobalSymbolId::new(module.module_id(), local_symbol_id);
                if symbol_id == canonical_id {
                    continue;
                }

                if module
                    .local_default_import_alias_name(local_symbol_id)
                    .is_none()
                {
                    continue;
                }

                if module.canonical_symbol(symbol_id) != canonical_id {
                    continue;
                }

                symbols.push(symbol_id);
            }
        }

        symbols
    }

    /// Build edit(s) to add an import for a symbol.
    pub(crate) fn build_import_edits(
        &self,
        symbol_name: &str,
        import_path: &str,
        import_form: ImportEditForm,
    ) -> Vec<Patch> {
        let file_id = self.file_id();
        let existing_imports = self.collect_existing_imports();

        if let Some(existing) = existing_imports
            .iter()
            .find(|import| import.path == import_path)
        {
            if existing.contains_specifier(symbol_name) {
                return Vec::new();
            }

            if existing.is_namespace {
                return import_form.new_import_edits(
                    file_id,
                    symbol_name,
                    import_path,
                    &existing_imports,
                );
            }

            if existing.accepts_form(import_form) {
                if let Some(brace_pos) = existing.closing_brace_pos {
                    let insert_text = existing.specifier_insert_text(symbol_name);
                    return vec![Patch::insert(file_id, brace_pos, insert_text)];
                }
            }
        }

        import_form.new_import_edits(file_id, symbol_name, import_path, &existing_imports)
    }

    /// Build a display path for one import.
    pub(crate) fn import_display_path(&self, module_path: &str) -> String {
        self.import_display_path_with_options(module_path, true)
    }

    /// Build a display path for one import with optional extension stripping.
    pub(crate) fn import_display_path_with_options(
        &self,
        module_path: &str,
        strip_extension: bool,
    ) -> String {
        let repository = self.repository();
        let revision = self.revision();
        let source_file = repository
            .file(revision, self.file_id())
            .unwrap_or_else(|error| panic!("failed to read import source file: {error}"))
            .unwrap_or_else(|| panic!("missing import source file {:?}", self.file_id()));
        let Some(source_path) = source_file.path.as_ref() else {
            return module_path.to_string();
        };
        let Some(source_dir) = source_path.parent() else {
            return module_path.to_string();
        };

        let target_path = Path::new(module_path);
        if let Some(display_path) =
            self.external_package_display_path(source_path, target_path, strip_extension)
        {
            return display_path;
        }

        let relative =
            relative_path(source_dir, target_path).unwrap_or_else(|| target_path.normalize());
        let mut display_path = normalize_path_separators(&relative.to_string_lossy());
        if !display_path.starts_with("./") && !display_path.starts_with("../") {
            display_path = format!("./{display_path}");
        }
        if strip_extension {
            display_path = strip_module_extension(&display_path);
        }

        display_path
    }

    /// Resolve the bounds for a complete import clause.
    pub(crate) fn import_clause_bounds(
        &self,
        import_span: Span,
        target_span: Option<Span>,
    ) -> Option<ImportClauseBounds> {
        let target_limit = target_span
            .map(|span| span.start)
            .unwrap_or(import_span.end);
        let mut open_brace = None;
        let mut close_brace = None;

        for token in self.tokens() {
            if token.span.file != self.file_id() {
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
                dir::TokenType::CloseBrace if open_brace.is_some() => {
                    close_brace = Some(token.span);
                }
                _ => {}
            }
        }

        let open_brace = open_brace?;
        let close_brace = close_brace?;
        let end_boundary = close_brace;
        if open_brace.start >= end_boundary.start {
            return None;
        }

        Some(ImportClauseBounds {
            open_brace,
            close_brace,
            end_boundary,
        })
    }

    /// Resolve the brace span for an import clause.
    pub(crate) fn import_clause_brace_span(
        &self,
        import_span: Span,
        target_span: Option<Span>,
    ) -> Option<(Span, Span)> {
        let bounds = self.import_clause_bounds(import_span, target_span)?;

        Some(bounds.brace_span())
    }

    /// Collect existing imports from one source query surface.
    fn collect_existing_imports(&self) -> Vec<ExistingImport> {
        let mut imports = Vec::new();

        for node_id in self.tree().iter_nodes::<dir::Expression>() {
            let expr = self.tree().get(node_id);
            let dir::Expression::Import {
                target,
                items,
                form,
                ..
            } = expr
            else {
                continue;
            };

            let path = self.strings().get(*target).to_string();
            let span = self.tree().source_index.get(node_id.id);
            let is_type_only = *form == dir::DependencyForm::Type;
            let items = items.as_deref().unwrap_or(&[]);
            let is_namespace = items.iter().any(|item_id| {
                let item = self.tree().get(*item_id);
                item.binding() == Some(dir::DependencyBinding::Namespace)
            });
            let specifiers = items
                .iter()
                .filter_map(|item_id| {
                    let item = self.tree().get(*item_id);
                    if item.binding() == Some(dir::DependencyBinding::Namespace) {
                        return None;
                    }

                    item.local_string_key()
                        .map(|id| self.strings().get(id).to_string())
                })
                .collect::<Vec<_>>();
            let target_span = self.tree().source_index.get_main(node_id.id);
            let closing_brace_pos = if is_namespace {
                None
            } else {
                self.import_clause_brace_span(span, target_span)
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

        imports.sort_by_key(|import| import.start);
        imports
    }

    /// Resolve the local binding name for one default import symbol inside this module.
    fn local_default_import_alias_name(
        &self,
        local_symbol_id: dir::LocalSymbolId,
    ) -> Option<String> {
        let declaration = {
            let symbols = self.symbols();
            let symbol = symbols.get_symbol(local_symbol_id);
            symbol.declaration?
        };
        if declaration.local_id.ty != dir::NodeType::DependencyItem {
            return None;
        }

        let item_id = declaration
            .local_id
            .try_into_typed::<dir::DependencyItem>()
            .unwrap_or_else(|_| {
                panic!(
                    "default import declaration is not a dependency item: {:?}",
                    declaration.local_id
                )
            });
        let local_name_id = self
            .view()
            .get::<dir::DependencyItem>(item_id)
            .default_import_alias_name()?;

        Some(self.strings().get(local_name_id).to_string())
    }

    /// Build one package-name display path for an external package target.
    fn external_package_display_path(
        &self,
        source_path: &Path,
        target_path: &Path,
        strip_extension: bool,
    ) -> Option<String> {
        let repository = self.repository();
        let revision = self.revision();
        let target_package = repository
            .module_ids(revision)
            .unwrap_or_else(|error| panic!("failed to read module ids: {error}"))
            .into_iter()
            .filter_map(|module_id| {
                let module = repository
                    .module(revision, module_id)
                    .unwrap_or_else(|error| panic!("failed to read module {module_id:?}: {error}"))
                    .unwrap_or_else(|| panic!("missing module {module_id:?}"));
                let package = repository
                    .package(revision, module.package_id)
                    .unwrap_or_else(|error| {
                        panic!("failed to read package {:?}: {error}", module.package_id)
                    })
                    .unwrap_or_else(|| panic!("missing package {:?}", module.package_id));
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

        if source_path.starts_with(target_package_path) {
            return None;
        }
        if !target_package_path
            .components()
            .any(|component| component.as_os_str() == "node_modules")
        {
            return None;
        }

        let package_relative = target_path
            .strip_prefix(target_package_path)
            .unwrap_or_else(|_| {
                panic!(
                    "target path {target_path:?} is not under package path {target_package_path:?}"
                )
            });
        let package_relative = normalize_path_separators(&package_relative.to_string_lossy());
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

/// Compute a relative path between two paths when possible.
fn relative_path(from_directory: &Path, to_path: &Path) -> Option<PathBuf> {
    // normalize input paths
    let from_directory = from_directory.normalize();
    let to_path = to_path.normalize();

    // collect normalized path components
    let from_components = normalized_components(&from_directory);
    let to_components = normalized_components(&to_path);

    // find the shared prefix length
    let mut common = 0usize;
    while common < from_components.len()
        && common < to_components.len()
        && from_components[common] == to_components[common]
    {
        common += 1;
    }

    // refuse to compute across unrelated absolute roots
    if common == 0 && from_directory.is_absolute() && to_path.is_absolute() {
        return None;
    }

    // build the relative path segments
    let mut relative = PathBuf::new();
    for _ in common..from_components.len() {
        relative.push("..");
    }
    for component in &to_components[common..] {
        relative.push(component);
    }

    Some(relative)
}

/// Normalize path separators to forward slashes.
fn normalize_path_separators(path: &str) -> String {
    path.replace('\\', "/")
}

/// Collect normalized path components for stable comparisons.
fn normalized_components(path: &Path) -> Vec<String> {
    let mut components = Vec::new();

    // translate platform specific components into normalized strings
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => {
                components.push(prefix.as_os_str().to_string_lossy().to_string());
            }
            Component::RootDir => {
                components.push("/".to_string());
            }
            Component::Normal(part) => {
                components.push(part.to_string_lossy().to_string());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                components.push("..".to_string());
            }
        }
    }

    components
}
