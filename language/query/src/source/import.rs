use tspp_artifact::PackageDependency;
use tspp_core::StringId;
use tspp_dir as dir;
use tspp_source::{FileId, ModuleId, Patch, Span};

use crate::source::{builtin_specifier, package_specifier, relative_module_specifier};
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

/// Authored import declarations from one source file.
pub(crate) struct ImportDeclarations {
    /// The source file containing the declarations.
    file_id: FileId,
    /// The declarations in source order.
    declarations: Vec<ImportDeclaration>,
}

/// One authored import declaration.
#[derive(Debug, Clone)]
struct ImportDeclaration {
    /// The import path.
    path: String,
    /// Start byte offset of the import statement.
    start: u32,
    /// End byte offset of the import statement.
    end: u32,
    /// Position for the first named binding.
    named_offset: Option<u32>,
    /// Position for an added default binding.
    default_offset: Option<u32>,
    /// Position after the default binding.
    default_end: Option<u32>,
    /// Position after the final named binding.
    named_end: Option<u32>,
    /// Whether this is a namespace import.
    is_namespace: bool,
    /// Existing default binding name.
    default_name: Option<String>,
    /// Existing named binding names.
    named: Vec<String>,
}

impl ImportDeclaration {
    /// Return whether this import already contains one named binding.
    fn contains_named(&self, symbol_name: &str) -> bool {
        self.named.iter().any(|specifier| specifier == symbol_name)
    }
}

impl ImportDeclarations {
    /// Build the edit that introduces one available import binding.
    pub(crate) fn edit(&self, binding: &ImportBinding, import_path: &str) -> Option<Patch> {
        if let Some(existing) = self
            .declarations
            .iter()
            .find(|import| import.path == import_path)
        {
            match binding {
                // retain an existing default import or add one before its remaining bindings
                ImportBinding::Default { name } => {
                    if existing.default_name.as_deref() == Some(name) {
                        return None;
                    }
                    if existing.default_name.is_some() {
                        return None;
                    }
                    if let Some(position) = existing.default_offset {
                        let text = format!("{name}, ");

                        return Some(Patch::insert(self.file_id, position, text));
                    }
                }

                // retain an existing named import or add one inside its clause
                ImportBinding::Named { name } => {
                    if existing.contains_named(name) {
                        return None;
                    }
                    if !existing.is_namespace
                        && let Some(offset) = existing.named_end
                    {
                        let text = format!(", {name}");

                        return Some(Patch::insert(self.file_id, offset, text));
                    }
                    if !existing.is_namespace
                        && existing.default_name.is_some()
                        && let Some(position) = existing.default_end
                    {
                        let text = format!(", {{ {name} }}");

                        return Some(Patch::insert(self.file_id, position, text));
                    }
                    if !existing.is_namespace
                        && let Some(position) = existing.named_offset
                    {
                        let text = format!(" {name} ");

                        return Some(Patch::insert(self.file_id, position, text));
                    }
                }
            }
        }

        // preserve the declaration's symbol space
        let group = ImportGroup::from_path(import_path);
        let declaration = match binding {
            ImportBinding::Default { name } => {
                format!("import {name} from \"{import_path}\";")
            }
            ImportBinding::Named { name } => {
                format!("import {{ {name} }} from \"{import_path}\";")
            }
        };

        // place the declaration before its successor or after the final import
        let next = self.declarations.iter().find(|existing| {
            let existing_group = ImportGroup::from_path(&existing.path);

            group < existing_group
                || (group == existing_group && import_path < existing.path.as_str())
        });
        let (position, text) = if let Some(next) = next {
            (next.start, format!("{declaration}\n"))
        } else if let Some(previous) = self.declarations.last() {
            (previous.end, format!("\n{declaration}"))
        } else {
            (0, format!("{declaration}\n"))
        };

        Some(Patch::insert(self.file_id, position, text))
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
    pub(crate) fn is_local_import_alias(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<bool> {
        Ok(self.local_import_alias_name_id(symbol_id)?.is_some())
    }

    /// Resolve the local alias text for one explicit import alias symbol.
    pub(crate) fn local_import_alias_name(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<String>> {
        let local_name_id = self.local_import_alias_name_id(symbol_id)?;

        Ok(local_name_id.map(|name| self.strings().get(name).to_string()))
    }

    /// Resolve the local name id for one explicit import alias symbol.
    fn local_import_alias_name_id(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<StringId>> {
        if symbol_id.module_id != self.module_id() {
            return Ok(None);
        }

        let declaration = {
            let symbols = self.bindings()?;
            let symbol = symbols.get_symbol(symbol_id.local_id);
            let Some(declaration) = symbol.declaration else {
                return Ok(None);
            };

            declaration
        };

        if declaration.local_id.ty != dir::NodeType::DependencyItem {
            return Ok(None);
        }

        let item_id = dir::LocalNodeId::<dir::DependencyItem>::new(declaration.local_id.id);
        let item = self.view()?.get::<dir::DependencyItem>(item_id);

        Ok(item.local_import_alias_name())
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

    /// Collect authored imports from one source file.
    pub(crate) fn import_declarations(&self, file_id: FileId) -> QueryResult<ImportDeclarations> {
        let mut imports = Vec::new();
        let view = self.view()?;

        for node_id in view.iter_node_ids_of_type::<dir::Expression>() {
            let expression = view.get(node_id);
            let dir::Expression::Import { target, items, .. } = expression else {
                continue;
            };

            // ignore generated imports because they have no editable authored source
            let source_id = view.get_source(node_id);
            let Some(span) = self.source_index()?.try_get(source_id) else {
                continue;
            };
            let path = self.strings().get(*target).to_string();
            if span.file != file_id {
                continue;
            }

            // include the authored terminator in insertion ordering
            let statement_end = self
                .tokens(span.file)?
                .find(|token| token.span.start >= span.end)
                .filter(|token| token.token.ty() == dir::TokenType::Semicolon)
                .map_or(span.end, |token| token.span.end);
            let items = match items {
                Some(items) => items.as_slice(),
                None => &[],
            };
            let is_namespace = items.iter().any(|item_id| {
                let item = view.get(*item_id);
                item.binding() == Some(dir::DependencyBinding::Namespace)
            });
            let mut default_name = None;
            let mut default_end = None;
            let mut named = Vec::new();
            let mut named_end = None;
            let mut namespace_item = None;
            for item_id in items {
                let item = view.get(*item_id);
                let node = item_id.into_global_any(self.module_id());
                let source_id = view.get_source(*item_id);
                let span = self
                    .source_index()?
                    .try_get(source_id)
                    .ok_or(QueryError::missing(format!("import item span: {node:?}")))?;
                match item.binding() {
                    Some(dir::DependencyBinding::Default) => {
                        let name = item
                            .local_string_key()
                            .ok_or(QueryError::missing(format!("import item name: {node:?}")))?;
                        default_name = Some(self.strings().get(name).to_string());
                        default_end = Some(span.end);
                    }
                    Some(dir::DependencyBinding::Named) => {
                        let name = item
                            .local_string_key()
                            .ok_or(QueryError::missing(format!("import item name: {node:?}")))?;
                        named.push(self.strings().get(name).to_string());
                        named_end = Some(span.end);
                    }
                    Some(dir::DependencyBinding::Namespace) => namespace_item = Some(*item_id),
                    None => {}
                }
            }
            let target_span = self.source_index()?.get_main(source_id);
            let bounds = self.import_clause_bounds(span, target_span)?;
            let named_offset = bounds.map(|bounds| bounds.open_brace.end);
            let default_offset = if let Some(item_id) = namespace_item {
                let source_id = view.get_source(item_id);
                let node = item_id.into_global_any(self.module_id());
                let span = self
                    .source_index()?
                    .try_get(source_id)
                    .ok_or(QueryError::missing(format!("import item span: {node:?}")))?;

                Some(span.start)
            } else {
                bounds.map(|bounds| bounds.open_brace.start)
            };
            imports.push(ImportDeclaration {
                path,
                start: span.start,
                end: statement_end,
                named_offset,
                default_offset,
                default_end,
                named_end,
                is_namespace,
                default_name,
                named,
            });
        }

        imports.sort_by_key(|import| import.start);

        Ok(ImportDeclarations {
            file_id,
            declarations: imports,
        })
    }
}

impl ProgramQueryContext<'_> {
    /// Return every module specifier addressable from one source module.
    pub(crate) fn addressable_specifiers(
        &self,
        source_module_id: ModuleId,
    ) -> QueryResult<Vec<String>> {
        let repository = self.repository();
        let revision = self.revision();
        let source_module = repository
            .module(revision, source_module_id)?
            .ok_or_else(|| {
                QueryError::missing(format!("repository module: {source_module_id:?}"))
            })?;
        let source_path = source_module
            .path
            .as_deref()
            .ok_or_else(|| QueryError::missing(format!("module path: {source_module_id:?}")))?;
        let source_package = self.package_node(source_module.package_id)?;
        let mut specifiers = Vec::new();
        let mut builtin_packages = Vec::new();

        // collect relative modules and implicit builtin packages
        for target_module_id in self.module_ids() {
            let package_id = target_module_id.package_id;
            if package_id == source_module.package_id && *target_module_id != source_module_id {
                if let Some(specifier) =
                    relative_module_specifier(repository, revision, source_path, *target_module_id)?
                {
                    specifiers.push(specifier);
                }
            } else if repository.is_builtin_package(package_id)
                && !builtin_packages.contains(&package_id)
            {
                builtin_packages.push(package_id);
            }
        }

        // collect public specifiers from implicit builtin packages
        for package_id in builtin_packages {
            for (key, _) in self.package_module_exports(package_id)? {
                specifiers.push(builtin_specifier(&key)?);
            }
        }

        // collect public specifiers through each resolved direct dependency
        for (alias, dependency) in &source_package.dependencies {
            let PackageDependency::Resolved(package_id) = dependency else {
                continue;
            };
            for (key, _) in self.package_module_exports(*package_id)? {
                specifiers.push(package_specifier(alias, &key)?);
            }
        }

        // provide one stable canonical list
        specifiers.sort();
        specifiers.dedup();

        Ok(specifiers)
    }

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
        // invert public exports across packages
        if source_module.package_id != target_module_id.package_id {
            let source_node = self.package_node(source_module.package_id)?;
            let target_package_id = target_module_id.package_id;
            let mut specifiers = Vec::new();

            // invert the implicitly addressable builtin package
            if repository.is_builtin_package(target_package_id) {
                let exports = self.package_module_exports(target_package_id)?;
                for (key, module_id) in &exports {
                    if *module_id != target_module_id {
                        continue;
                    }
                    specifiers.push(builtin_specifier(key)?);
                }
            }

            // invert each alias resolving to the target package
            for (alias, dependency) in &source_node.dependencies {
                let PackageDependency::Resolved(dependency) = dependency else {
                    continue;
                };
                if *dependency != target_package_id {
                    continue;
                }
                let exports = self.package_module_exports(*dependency)?;
                for (key, module_id) in &exports {
                    if *module_id != target_module_id {
                        continue;
                    }
                    specifiers.push(package_specifier(alias, key)?);
                }
            }

            // order and deduplicate the inverted specifiers
            specifiers.sort();
            specifiers.dedup();

            return Ok(specifiers);
        }

        // build the canonical relative specifier inside one package
        let source_path = source_module
            .path
            .as_deref()
            .ok_or(QueryError::missing(format!(
                "module path: {source_module_id:?}"
            )))?;
        let specifier =
            relative_module_specifier(repository, revision, source_path, target_module_id)?;

        Ok(specifier.into_iter().collect())
    }
}
