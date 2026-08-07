use destack_dir as dir;
use destack_source::Span;

use crate::{ModuleQueryContext, ProgramQueryContext, QueryError, QueryResult};

impl ModuleQueryContext<'_> {
    /// Resolve a display name for one declaration.
    pub(crate) fn declaration_display_name(
        &self,
        declaration: &dir::Declaration,
    ) -> Option<String> {
        declaration
            .name()
            .map(|name| self.strings().get(name.string()).to_string())
    }

    /// Resolve the global symbol declared by one DIR node.
    pub(crate) fn global_node_symbol(
        &self,
        node_id: dir::LocalNodeIdAny,
    ) -> QueryResult<Option<dir::GlobalSymbolId>> {
        let symbol = self
            .node_symbol(node_id)?
            .map(|symbol_id| symbol_id.into_global(self.module_id()));

        Ok(symbol)
    }

    /// Return the exact definition member carried by one indexed symbol.
    pub(crate) fn definition_member<'a>(
        &'a self,
        program: &ProgramQueryContext<'_>,
        symbol: dir::GlobalSymbolId,
    ) -> QueryResult<
        Option<(
            dir::GlobalSymbolId,
            &'a dir::Definition,
            &'a dir::DefinitionMember,
        )>,
    > {
        if symbol.module_id != self.module_id() {
            return Err(QueryError::invalid(format!(
                "local member symbol: {symbol:?}, {:?}",
                self.module_id()
            )));
        }

        // select the declaring definition through the persisted member index
        let Some(entry) = program.member_index(self.module_id())?.symbol_entry(symbol) else {
            return Ok(None);
        };
        let declaring = entry.declaring;
        let definition = self
            .definitions()?
            .definition(declaring)
            .ok_or(QueryError::missing(format!(
                "member declaring definition: {symbol:?}, {declaring:?}"
            )))?;
        let member = definition
            .member(symbol)
            .ok_or(QueryError::missing(format!(
                "indexed definition member: {symbol:?}, {declaring:?}"
            )))?;

        Ok(Some((declaring, definition, member)))
    }
}

impl ProgramQueryContext<'_> {
    /// Return whether one symbol belongs to the toolchain's built-in package.
    pub(crate) fn symbol_is_default_library(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<bool> {
        let package_id = symbol_id.module_id.package_id;
        let package = self
            .repository()
            .package(self.revision(), package_id)?
            .ok_or(destack_repository::RepositoryError::MissingPackage {
                package: package_id,
            })?;

        Ok(package.is_builtin)
    }

    /// Return every canonical symbol reached by dependency bindings.
    pub(crate) fn canonical_symbols(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<dir::GlobalSymbolId>> {
        let mut symbols = Vec::new();
        let mut path = Vec::new();
        self.collect_canonical_symbols(symbol_id, &mut path, &mut symbols)?;
        symbols.sort();
        symbols.dedup();

        Ok(symbols)
    }

    /// Return the one canonical symbol reached by dependency bindings.
    pub(crate) fn canonical_symbol(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<dir::GlobalSymbolId>> {
        let symbols = self.canonical_symbols(symbol_id)?;
        match symbols.as_slice() {
            [] => Ok(None),
            [symbol_id] => Ok(Some(*symbol_id)),
            _ => Err(QueryError::conflict(format!(
                "canonical symbol group: {symbol_id:?}"
            ))),
        }
    }

    /// Collect canonical symbols through one exact dependency path.
    fn collect_canonical_symbols(
        &self,
        symbol_id: dir::GlobalSymbolId,
        path: &mut Vec<dir::GlobalSymbolId>,
        symbols: &mut Vec<dir::GlobalSymbolId>,
    ) -> QueryResult<()> {
        if path.contains(&symbol_id) {
            return Err(QueryError::cycle(format!(
                "canonical symbol: {symbol_id:?}"
            )));
        }

        // retain declarations that are not import bindings
        let module = self.module(symbol_id.module_id)?;
        let binding = module.bindings()?.get_symbol(symbol_id.local_id);
        let Some(declaration) = binding.declaration else {
            symbols.push(symbol_id);

            return Ok(());
        };
        if declaration.local_id.ty != dir::NodeType::DependencyItem {
            symbols.push(symbol_id);

            return Ok(());
        }

        // follow every exact overload target
        let reference =
            module
                .resolved()?
                .references
                .get(declaration)
                .ok_or(QueryError::missing(format!(
                    "canonical reference: {declaration:?}"
                )))?;
        match reference {
            dir::Reference::Bound(targets) => {
                if targets.is_empty() {
                    return Err(QueryError::missing(format!(
                        "canonical reference: {declaration:?}"
                    )));
                }

                path.push(symbol_id);
                for target in targets {
                    self.collect_canonical_symbols(*target, path, symbols)?;
                }
                path.pop();
            }
            dir::Reference::Namespace(_) => symbols.push(symbol_id),
            dir::Reference::Projected { .. } => {
                return Err(QueryError::invalid(format!(
                    "canonical reference: {declaration:?}"
                )));
            }
            dir::Reference::Ambiguous(targets) => {
                path.push(symbol_id);
                for target in targets {
                    if let dir::ImportTarget::Symbol(target) = target {
                        self.collect_canonical_symbols(*target, path, symbols)?;
                    }
                }
                path.pop();
            }
            dir::Reference::Missing => {}
        }

        Ok(())
    }

    /// Resolve a symbol name string when possible.
    pub(crate) fn symbol_name(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<String>> {
        let module = self.module(symbol_id.module_id)?;
        let symbols = module.bindings()?;
        let symbol = symbols.get_symbol(symbol_id.local_id);

        Ok(symbol
            .name()
            .map(|name_id| module.strings().get(name_id).to_string()))
    }

    /// Return the definition span of a symbol.
    pub(crate) fn symbol_definition_span(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<Span>> {
        let Some(symbol_id) = self.canonical_symbol(symbol_id)? else {
            return Ok(None);
        };
        let module = self.module(symbol_id.module_id)?;

        module.symbol_local_definition_span(self, symbol_id)
    }
}

impl ModuleQueryContext<'_> {
    /// Return the local definition span of a symbol without canonical expansion.
    pub(crate) fn symbol_local_definition_span(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<Span>> {
        if symbol_id.module_id != self.module_id() {
            return Err(QueryError::invalid(format!(
                "local symbol: {:?}, {:?}",
                symbol_id,
                self.module_id()
            )));
        }

        let declaration = {
            let symbols = self.bindings()?;
            let symbol = symbols.get_symbol(symbol_id.local_id);
            symbol.declaration
        };

        if let Some(declaration) = declaration {
            let span = self
                .node_selection_span(self.view()?, declaration.local_id)?
                .ok_or(QueryError::missing(format!(
                    "symbol definition: {symbol_id:?}"
                )))?;

            return Ok(Some(span));
        }

        // read member selections from their exact definition source
        if let Some((_, _, member)) = self.definition_member(program, symbol_id)? {
            let source = member.source();
            let span = self
                .node_selection_span(self.view()?, source.local_id)?
                .ok_or(QueryError::missing(format!(
                    "member definition: {symbol_id:?}"
                )))?;

            return Ok(Some(span));
        }

        Ok(None)
    }

    /// Return the local declaration span of a symbol without canonical expansion.
    pub(crate) fn symbol_local_declaration_span(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<Span>> {
        if symbol_id.module_id != self.module_id() {
            return Err(QueryError::invalid(format!(
                "local symbol: {:?}, {:?}",
                symbol_id,
                self.module_id()
            )));
        }

        let symbols = self.bindings()?;
        let symbol = symbols.get_symbol(symbol_id.local_id);
        let source = match symbol.declaration {
            Some(declaration) => declaration.local_id,
            None => {
                let Some((_, _, member)) = self.definition_member(program, symbol_id)? else {
                    return Ok(None);
                };

                member.source().local_id
            }
        };
        let span = self.node_span(self.view()?, source)?;

        Ok(Some(span))
    }
}
