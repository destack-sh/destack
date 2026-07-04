use destack_dir as dir;
use destack_source::EnclosingSpan;

use crate::ModuleQueryContext;

/// A scope and mark resolved for one cursor position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScopeAtOffset {
    /// The scope id.
    pub scope_id: dir::LocalScopeId,
    /// The scope mark within the scope.
    pub scope_mark: dir::LocalScopeMark,
}

impl ScopeAtOffset {
    /// Create one scope cursor hit.
    pub(crate) fn new(scope_id: dir::LocalScopeId, scope_mark: dir::LocalScopeMark) -> Self {
        Self {
            scope_id,
            scope_mark,
        }
    }
}

impl ModuleQueryContext<'_> {
    /// Resolve the best visible scope at one offset.
    pub(crate) fn scope_at_offset(&self, offset: u32) -> Option<ScopeAtOffset> {
        let enclosing = self.enclosing_spans_with_previous(offset);

        // prefer block scopes because statement and argument positions usually live there
        if let Some(scope) = self.enclosing_block_scope(&enclosing, offset) {
            return Some(scope);
        }

        // otherwise accept expression and owned declaration scopes
        if let Some(scope) = self.enclosing_scope(&enclosing, offset) {
            return Some(scope);
        }

        // use source parents when the cursor sits between mapped DIR spans
        self.parent_scope(&enclosing, offset)
    }

    /// Resolve the nearest enclosing block or owned declaration scope at one offset.
    pub(crate) fn block_scope_at_offset(&self, offset: u32) -> Option<ScopeAtOffset> {
        let enclosing = self.enclosing_spans_with_previous(offset);

        // prefer the nearest enclosing block scope
        if let Some(scope) = self.enclosing_block_scope(&enclosing, offset) {
            return Some(scope);
        }

        // otherwise use owned declaration scopes from mapped nodes
        if let Some(scope) = self.enclosing_declaration_scope(&enclosing, offset) {
            return Some(scope);
        }

        // use source parents when the cursor sits between mapped DIR spans
        self.parent_block_scope(&enclosing, offset)
    }

    /// Resolve the scope owned by one block span.
    pub(crate) fn source_block_scope(
        &self,
        source_block_id: u32,
        offset: u32,
    ) -> Option<ScopeAtOffset> {
        let view = self.view();
        let symbols = self.symbols();
        let dir_node_id = view.get_node_id_by_source_id(source_block_id)?;

        // only blocks own statement scopes
        if dir_node_id.ty != dir::NodeType::Block {
            return None;
        }

        let Ok(block_id) = dir_node_id.try_into_typed::<dir::Block>() else {
            return None;
        };

        let scope = self.node_scope(block_id.into_any())?;
        let scope_mark = self.scope_mark_at_offset(scope.id, offset, view, symbols);

        Some(ScopeAtOffset::new(scope.id, scope_mark))
    }

    /// Resolve the visible scope for one expression at an offset.
    pub(crate) fn expression_scope_at_offset(
        &self,
        expr_id: dir::LocalNodeId<dir::Expression>,
        offset: u32,
    ) -> Option<ScopeAtOffset> {
        self.node_scope(expr_id.into_any())?;

        self.scope_at_offset(offset)
    }

    /// Resolve the nearest enclosing block scope.
    fn enclosing_block_scope(
        &self,
        enclosing: &[EnclosingSpan],
        offset: u32,
    ) -> Option<ScopeAtOffset> {
        let view = self.view();
        let symbols = self.symbols();

        for enc in enclosing {
            let Some(dir_node_id) = view.get_node_id_by_source_id(enc.source_id) else {
                continue;
            };

            // blocks define the local statement scope
            if dir_node_id.ty == dir::NodeType::Block {
                let Ok(block_id) = dir_node_id.try_into_typed::<dir::Block>() else {
                    continue;
                };

                let Some(scope) = self.node_scope(block_id.into_any()) else {
                    continue;
                };
                let scope_mark = self.scope_mark_at_offset(scope.id, offset, view, symbols);

                return Some(ScopeAtOffset::new(scope.id, scope_mark));
            }
        }

        None
    }

    /// Resolve the best enclosing scope.
    fn enclosing_scope(&self, enclosing: &[EnclosingSpan], offset: u32) -> Option<ScopeAtOffset> {
        let view = self.view();
        let symbols = self.symbols();

        for enc in enclosing {
            let Some(dir_node_id) = view.get_node_id_by_source_id(enc.source_id) else {
                continue;
            };

            // expressions carry the surrounding lexical scope
            if dir_node_id.ty == dir::NodeType::Expression {
                let Ok(expr_id) = dir_node_id.try_into_typed::<dir::Expression>() else {
                    continue;
                };

                let Some(scope) = self.node_scope(expr_id.into_any()) else {
                    continue;
                };
                let scope_mark = self.scope_mark_at_offset(scope.id, offset, view, symbols);

                return Some(ScopeAtOffset::new(scope.id, scope_mark));
            }

            // owned declarations such as functions and classes expose their inner scope
            if dir_node_id.ty == dir::NodeType::Declaration {
                if let Some(scope_id) = self.declaration_scope(dir_node_id) {
                    let scope_mark = self.scope_mark_at_offset(scope_id, offset, view, symbols);

                    return Some(ScopeAtOffset::new(scope_id, scope_mark));
                }
            }
        }

        None
    }

    /// Resolve the nearest enclosing owned declaration scope.
    fn enclosing_declaration_scope(
        &self,
        enclosing: &[EnclosingSpan],
        offset: u32,
    ) -> Option<ScopeAtOffset> {
        let view = self.view();
        let symbols = self.symbols();

        for enc in enclosing {
            let Some(dir_node_id) = view.get_node_id_by_source_id(enc.source_id) else {
                continue;
            };

            if dir_node_id.ty != dir::NodeType::Declaration {
                continue;
            }

            let Some(scope_id) = self.declaration_scope(dir_node_id) else {
                continue;
            };
            let scope_mark = self.scope_mark_at_offset(scope_id, offset, view, symbols);

            return Some(ScopeAtOffset::new(scope_id, scope_mark));
        }

        None
    }

    /// Resolve one scope through source parents.
    fn parent_scope(&self, enclosing: &[EnclosingSpan], offset: u32) -> Option<ScopeAtOffset> {
        let start_id = enclosing.first().map(|enc| enc.source_id)?;

        let view = self.view();
        let symbols = self.symbols();

        for parent_id in self.parents().walk_parents_by_id(start_id) {
            let Some(dir_node_id) = view.get_node_id_by_source_id(parent_id) else {
                if self.tree().get_node_type(parent_id) == dir::NodeType::Declaration {
                    if let Some(scope_id) = self.source_declaration_scope(parent_id) {
                        let scope_mark = self.scope_mark_at_offset(scope_id, offset, view, symbols);

                        return Some(ScopeAtOffset::new(scope_id, scope_mark));
                    }
                }

                continue;
            };

            // blocks still win during source parent walks
            if dir_node_id.ty == dir::NodeType::Block {
                let Ok(block_id) = dir_node_id.try_into_typed::<dir::Block>() else {
                    continue;
                };

                let Some(scope) = self.node_scope(block_id.into_any()) else {
                    continue;
                };
                let scope_mark = self.scope_mark_at_offset(scope.id, offset, view, symbols);

                return Some(ScopeAtOffset::new(scope.id, scope_mark));
            }

            // expressions carry the surrounding lexical scope
            if dir_node_id.ty == dir::NodeType::Expression {
                let Ok(expr_id) = dir_node_id.try_into_typed::<dir::Expression>() else {
                    continue;
                };

                let Some(scope) = self.node_scope(expr_id.into_any()) else {
                    continue;
                };
                let scope_mark = self.scope_mark_at_offset(scope.id, offset, view, symbols);

                return Some(ScopeAtOffset::new(scope.id, scope_mark));
            }

            // owned declarations expose an inner scope
            if dir_node_id.ty == dir::NodeType::Declaration {
                if let Some(scope_id) = self.declaration_scope(dir_node_id) {
                    let scope_mark = self.scope_mark_at_offset(scope_id, offset, view, symbols);

                    return Some(ScopeAtOffset::new(scope_id, scope_mark));
                }
            }
        }

        None
    }

    /// Resolve one block or owned declaration scope through source parents.
    fn parent_block_scope(
        &self,
        enclosing: &[EnclosingSpan],
        offset: u32,
    ) -> Option<ScopeAtOffset> {
        let start_id = enclosing.first().map(|enc| enc.source_id)?;

        let view = self.view();
        let symbols = self.symbols();

        for parent_id in self.parents().walk_parents_by_id(start_id) {
            let Some(dir_node_id) = view.get_node_id_by_source_id(parent_id) else {
                if self.tree().get_node_type(parent_id) == dir::NodeType::Declaration {
                    if let Some(scope_id) = self.source_declaration_scope(parent_id) {
                        let scope_mark = self.scope_mark_at_offset(scope_id, offset, view, symbols);

                        return Some(ScopeAtOffset::new(scope_id, scope_mark));
                    }
                }

                continue;
            };

            // prefer the nearest enclosing block
            if dir_node_id.ty == dir::NodeType::Block {
                let Ok(block_id) = dir_node_id.try_into_typed::<dir::Block>() else {
                    continue;
                };

                let Some(scope) = self.node_scope(block_id.into_any()) else {
                    continue;
                };
                let scope_mark = self.scope_mark_at_offset(scope.id, offset, view, symbols);

                return Some(ScopeAtOffset::new(scope.id, scope_mark));
            }

            // otherwise accept owned declaration scopes
            if dir_node_id.ty == dir::NodeType::Declaration {
                if let Some(scope_id) = self.declaration_scope(dir_node_id) {
                    let scope_mark = self.scope_mark_at_offset(scope_id, offset, view, symbols);

                    return Some(ScopeAtOffset::new(scope_id, scope_mark));
                }
            }
        }

        None
    }

    /// Resolve the current scope mark at one offset.
    fn scope_mark_at_offset(
        &self,
        scope_id: dir::LocalScopeId,
        offset: u32,
        view: dir::View<'_>,
        symbols: &dir::BindingTable<'_>,
    ) -> dir::LocalScopeMark {
        let scope = symbols.get_scope_by_id(scope_id);
        if scope.bindings.is_empty() {
            return dir::LocalScopeMark(0);
        }

        // count symbols whose declaration begins before the cursor
        let mut mark_index = 0u32;

        for (index, binding) in scope.bindings.iter().enumerate() {
            let symbol = symbols.get_symbol(binding.symbol);
            let Some(declaration) = symbol.declaration else {
                mark_index = (index + 1) as u32;
                continue;
            };

            let span = self.get_main_span(view, declaration.local_id);

            if span.start <= offset {
                mark_index = (index + 1) as u32;
            }
        }

        dir::LocalScopeMark(mark_index)
    }

    /// Resolve the scope owned by one declaration node.
    fn declaration_scope(&self, declaration: dir::LocalNodeIdAny) -> Option<dir::LocalScopeId> {
        let symbol_id = self.node_symbol(declaration)?;
        let scope = self.symbols().scope_for_owner(symbol_id)?;

        Some(scope.id)
    }

    /// Resolve the scope owned by one source declaration node.
    fn source_declaration_scope(&self, source_id: u32) -> Option<dir::LocalScopeId> {
        for (declaration, symbol_id) in self.symbols().declaration_symbols() {
            if self.view().get_source_any(declaration.local_id) != source_id {
                continue;
            }

            let scope = self.symbols().scope_for_owner(symbol_id)?;

            return Some(scope.id);
        }

        None
    }
}
