use destack_artifact::PackageDependency;
use destack_core::StringId;
use destack_dir as dir;
use destack_source::{FileId, ModuleId, Patch, Span};

use crate::source::{
    canonical_module_path, export_keys_selecting_module, package_specifier, path_text,
    relative_path,
};
use crate::{ModuleQueryContext, ProgramQueryContext, QueryError, QueryResult};

/// One binding introduced by an auto import.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum ImportBinding {
    /// A default import binding.
    Default {
        /// The local binding name.
        name: String,
    },
    /// A named import binding.
    Named {
        /// The exported and local binding name.
        name: String,
    },
}

impl ImportBinding {
    /// Return the local binding name.
    pub(crate) fn name(&self) -> &str {
        match self {
            Self::Default { name } | Self::Named { name } => name,
        }
    }
}

/// The import group used for declaration ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ImportGroup {
    /// External packages.
    Package,
    /// Relative imports.
    Relative,
}

impl ImportGroup {
    /// Return the group for one import path.
    fn from_path(path: &str) -> Self {
        if path.starts_with("./") || path.starts_with("../") {
            return Self::Relative;
        }

        Self::Package
    }
}

/// One existing import declaration.
#[derive(Debug, Clone)]
struct ExistingImport {
    /// The import path.
    path: String,
    /// Start byte offset of the import statement.
    start: u32,
    /// End byte offset of the import statement.
    end: u32,
    /// Position of the closing brace.
    closing_brace_offset: Option<u32>,
    /// Position before the first namespace or named binding.
    binding_offset: Option<u32>,
    /// Position of the `from` keyword.
    from_offset: Option<u32>,
    /// Whether this is a namespace import.
    is_namespace: bool,
    /// Existing default binding name.
    default_name: Option<String>,
    /// Existing named binding names.
    named: Vec<String>,
}

impl ExistingImport {
    /// Return whether this import already contains one named binding.
    fn contains_named(&self, symbol_name: &str) -> bool {
        self.named.iter().any(|specifier| specifier == symbol_name)
    }

    /// Return insertion text for one added named binding.
    fn named_insert_text(&self, symbol_name: &str) -> String {
        if self.named.is_empty() {
            format!(" {symbol_name} ")
        } else {
            format!(", {symbol_name}")
        }
    }
}

/// The authored braces of one import clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ImportClauseBounds {
    /// The clause opening brace.
    pub open_brace: Span,
    /// The clause closing brace.
    pub close_brace: Span,
}

impl ModuleQueryContext<'_> {
    /// Return whether one symbol is an explicit local import alias.
    pub(crate) fn is_local_import_alias(&self, symbol_id: dir::GlobalSymbolId) -> bool {
        self.local_import_alias_name_id(symbol_id).is_some()
    }

    /// Resolve the local alias text for one explicit import alias symbol.
    pub(crate) fn local_import_alias_name(&self, symbol_id: dir::GlobalSymbolId) -> Option<String> {
        let local_name_id = self.local_import_alias_name_id(symbol_id)?;

        Some(self.strings().get(local_name_id).to_string())
    }

    /// Resolve the local name id for one explicit import alias symbol.
    fn local_import_alias_name_id(&self, symbol_id: dir::GlobalSymbolId) -> Option<StringId> {
        if symbol_id.module_id != self.module_id() {
            return None;
        }

        let declaration = {
            let symbols = self.symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            symbol.declaration?
        };

        if declaration.local_id.ty != dir::NodeType::DependencyItem {
            return None;
        }

        let item_id = dir::LocalNodeId::<dir::DependencyItem>::new(declaration.local_id.id);
        let item = self.view().get::<dir::DependencyItem>(item_id);

        item.local_import_alias_name()
    }

    /// Build edits that add an import for one symbol.
    pub(crate) fn build_import_edits(
        &self,
        file_id: FileId,
        binding: &ImportBinding,
        import_path: &str,
    ) -> QueryResult<Vec<Patch>> {
        let existing_imports = self.collect_existing_imports(file_id)?;

        if let Some(existing) = existing_imports
            .iter()
            .find(|import| import.path == import_path)
        {
            match binding {
                // retain an existing default import or add one before its remaining bindings
                ImportBinding::Default { name } => {
                    if existing.default_name.as_deref() == Some(name) {
                        return Ok(Vec::new());
                    }
                    if existing.default_name.is_some() {
                        return Ok(Vec::new());
                    }
                    if let Some(position) = existing.binding_offset {
                        let text = format!("{name}, ");

                        return Ok(vec![Patch::insert(file_id, position, text)]);
                    }
                }

                // retain an existing named import or add one inside its clause
                ImportBinding::Named { name } => {
                    if existing.contains_named(name) {
                        return Ok(Vec::new());
                    }
                    if !existing.is_namespace
                        && let Some(offset) = existing.closing_brace_offset
                    {
                        let text = existing.named_insert_text(name);

                        return Ok(vec![Patch::insert(file_id, offset, text)]);
                    }
                    if !existing.is_namespace
                        && existing.default_name.is_some()
                        && let Some(position) = existing.from_offset
                    {
                        let text = format!(", {{ {name} }} ");

                        return Ok(vec![Patch::insert(file_id, position, text)]);
                    }
                }
            }
        }

        // insert one plain import because imports preserve the declaration's symbol space
        let group = ImportGroup::from_path(import_path);
        let position = import_insert_position(import_path, group, &existing_imports);
        let text = match binding {
            ImportBinding::Default { name } => {
                format!("import {name} from \"{import_path}\";\n")
            }
            ImportBinding::Named { name } => {
                format!("import {{ {name} }} from \"{import_path}\";\n")
            }
        };

        Ok(vec![Patch::insert(file_id, position, text)])
    }

    /// Resolve the bounds for a complete import clause.
    pub(crate) fn import_clause_bounds(
        &self,
        import_span: Span,
        target_span: Option<Span>,
    ) -> QueryResult<Option<ImportClauseBounds>> {
        let target_limit = match target_span {
            Some(span) => span.start,
            None => import_span.end,
        };
        let mut open_brace = None;
        let mut close_brace = None;

        for token in self.tokens(import_span.file)? {
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

        let Some(open_brace) = open_brace else {
            return Ok(None);
        };
        let Some(close_brace) = close_brace else {
            return Ok(None);
        };
        if open_brace.start >= close_brace.start {
            return Ok(None);
        }

        Ok(Some(ImportClauseBounds {
            open_brace,
            close_brace,
        }))
    }

    /// Collect existing imports from one source file.
    fn collect_existing_imports(&self, file_id: FileId) -> QueryResult<Vec<ExistingImport>> {
        let mut imports = Vec::new();
        let view = self.view();

        for node_id in view.iter_nodes::<dir::Expression>() {
            let expression = view.get(node_id);
            let dir::Expression::Import { target, items, .. } = expression else {
                continue;
            };

            // ignore generated imports because they have no editable authored source
            let source_id = view.get_source(node_id);
            let Some(span) = self.source_index().try_get(source_id) else {
                continue;
            };
            let path = self.strings().get(*target).to_string();
            if span.file != file_id {
                continue;
            }
            let items = match items {
                Some(items) => items.as_slice(),
                None => &[],
            };
            let is_namespace = items.iter().any(|item_id| {
                let item = view.get(*item_id);
                item.binding() == Some(dir::DependencyBinding::Namespace)
            });
            let mut default_name = None;
            let mut named = Vec::new();
            let mut namespace_item = None;
            for item_id in items {
                let item = view.get(*item_id);
                let node = item_id.into_global_any(self.module_id());
                match item.binding() {
                    Some(dir::DependencyBinding::Default) => {
                        let name = item
                            .local_string_key()
                            .ok_or(QueryError::missing(format!("import item name: {node:?}")))?;
                        default_name = Some(self.strings().get(name).to_string());
                    }
                    Some(dir::DependencyBinding::Named) => {
                        let name = item
                            .local_string_key()
                            .ok_or(QueryError::missing(format!("import item name: {node:?}")))?;
                        named.push(self.strings().get(name).to_string());
                    }
                    Some(dir::DependencyBinding::Namespace) => namespace_item = Some(*item_id),
                    None => {}
                }
            }
            let target_span = self.source_index().get_main(source_id);
            let bounds = self.import_clause_bounds(span, target_span)?;
            let closing_brace_offset = bounds.map(|bounds| bounds.close_brace.start);
            let binding_offset = if let Some(item_id) = namespace_item {
                let source_id = view.get_source(item_id);
                let node = item_id.into_global_any(self.module_id());
                let span = self
                    .source_index()
                    .try_get(source_id)
                    .ok_or(QueryError::missing(format!("import item span: {node:?}")))?;

                Some(span.start)
            } else {
                bounds.map(|bounds| bounds.open_brace.start)
            };
            let from_offset = self.tokens(span.file)?.find_map(|token| {
                if token.span.start < span.start || token.span.end > span.end {
                    return None;
                }

                (token.token.keyword() == Some(dir::Keyword::From)).then_some(token.span.start)
            });

            imports.push(ExistingImport {
                path,
                start: span.start,
                end: span.end,
                closing_brace_offset,
                binding_offset,
                from_offset,
                is_namespace,
                default_name,
                named,
            });
        }

        imports.sort_by_key(|import| import.start);

        Ok(imports)
    }
}

impl ProgramQueryContext<'_> {
    /// Return every valid import specifier between two exact modules.
    pub(crate) fn import_specifiers(
        &self,
        source_module_id: ModuleId,
        target_module_id: ModuleId,
    ) -> QueryResult<Vec<String>> {
        if source_module_id == target_module_id {
            return Ok(Vec::new());
        }

        let repository = self.repository();
        let revision = self.revision();
        let source_module =
            repository
                .module(revision, source_module_id)?
                .ok_or(QueryError::missing(format!(
                    "repository module: {source_module_id:?}"
                )))?;
        // invert public exports of the resolved dependency across packages
        if source_module.package_id != target_module_id.package_id {
            let source_node = self.package_node(source_module.package_id)?;
            let mut specifiers = Vec::new();

            // invert each alias resolving to the target package
            for (alias, dependency) in &source_node.dependencies {
                let PackageDependency::Resolved(dependency) = dependency else {
                    continue;
                };
                if *dependency != target_module_id.package_id {
                    continue;
                }
                let dependency_node = self.package_node(*dependency)?;
                let keys = export_keys_selecting_module(
                    repository,
                    revision,
                    *dependency,
                    &dependency_node,
                    target_module_id,
                )?;
                for key in &keys {
                    specifiers.push(package_specifier(alias, key)?);
                }
            }

            // order and deduplicate the inverted specifiers
            specifiers.sort();
            specifiers.dedup();

            return Ok(specifiers);
        }

        // skip modules that have no exact default import path
        let Some(target_path) = canonical_module_path(repository, revision, target_module_id)?
        else {
            return Ok(Vec::new());
        };
        let source_path = source_module
            .path
            .as_deref()
            .ok_or(QueryError::missing(format!(
                "module path: {source_module_id:?}"
            )))?;
        let source_directory = source_path.parent().ok_or(QueryError::invalid(format!(
            "module import base: {source_module_id:?}"
        )))?;
        let relative = relative_path(source_directory, &target_path).ok_or(QueryError::invalid(
            format!("relative module path: {source_module_id:?} -> {target_module_id:?}"),
        ))?;
        let mut specifier = path_text(&relative)?;
        if !specifier.starts_with("./") && !specifier.starts_with("../") {
            specifier = format!("./{specifier}");
        }

        Ok(vec![specifier])
    }
}

/// Return the insertion position for one new import.
fn import_insert_position(
    import_path: &str,
    group: ImportGroup,
    existing_imports: &[ExistingImport],
) -> u32 {
    for existing in existing_imports {
        let existing_group = ImportGroup::from_path(&existing.path);
        if group < existing_group {
            return existing.start;
        }
        if group == existing_group && import_path < existing.path.as_str() {
            return existing.start;
        }
    }

    existing_imports.last().map_or(0, |existing| existing.end)
}
