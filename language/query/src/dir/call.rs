use destack_dir as dir;

use crate::core::{CallEntry, DirQueryContext, ModuleQueryContext};

/// Information about a call target.
#[derive(Debug, Clone)]
pub(crate) struct CallTarget {
    /// The name of the target.
    pub name: Option<String>,
    /// The symbol id of the target.
    pub symbol: Option<dir::GlobalSymbolId>,
}

impl CallTarget {
    /// Create a call target with the given name and symbol.
    pub(crate) fn new(name: Option<String>, symbol: Option<dir::GlobalSymbolId>) -> Self {
        Self { name, symbol }
    }
}

impl ModuleQueryContext<'_> {
    /// Build call index entries for this module.
    pub(crate) fn build_call_candidates(&self) -> Vec<CallEntry> {
        let mut entries = Vec::new();
        let dir_tree = self.dir().view();
        let module_id = self.module_id();

        // collect checked call target edges
        for (expression_id, expression) in dir_tree.iter_nodes_of_type::<dir::Expression>() {
            let left_expression = match expression {
                dir::Expression::Call { left, .. } => Some(*left),
                dir::Expression::New { .. } => None,
                _ => continue,
            };

            let call_span = self
                .dir()
                .get_node_tree_span(self.dir().view(), expression_id.into());
            let caller_symbol = self.find_containing_function_symbol(expression_id.into());
            let callee_symbols = self
                .dir()
                .call_target_symbols(expression_id, left_expression);
            for callee_symbol in callee_symbols {
                entries.push(CallEntry {
                    module_id,
                    caller_symbol,
                    callee_symbol,
                    span: call_span,
                });
            }
        }

        entries
    }
}

impl DirQueryContext<'_> {
    /// Return the call target name and symbol for a call expression.
    pub(crate) fn call_target(
        self,
        left_expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CallTarget {
        let dir_tree = self.view();
        let left_expression = dir_tree.get::<dir::Expression>(left_expression_id);

        // inspect the target expression shape
        match left_expression {
            dir::Expression::QualifiedReference { path, .. } => {
                let symbol = self.expression_symbol_target(left_expression_id);
                let name = symbol
                    .and_then(|symbol| self.symbol_name(symbol))
                    .or_else(|| {
                        path.last_segment()
                            .map(|name_id| self.strings().get(name_id).to_string())
                    });

                // prefer the canonical function symbol when possible
                let function_symbol = symbol.and_then(|symbol| {
                    let canonical_symbol = self.canonical_symbol(symbol);

                    self.symbol_is_function(canonical_symbol)
                        .then_some(canonical_symbol)
                        .or_else(|| self.symbol_is_function(symbol).then_some(symbol))
                });

                CallTarget::new(name, function_symbol)
            }
            dir::Expression::Member { name, .. } => {
                let Some(name) = *name else {
                    return CallTarget::new(None, None);
                };

                let member_name = self.strings().get(name).to_string();
                let member_symbol = self.member_access_symbol_target(left_expression_id);

                CallTarget::new(Some(member_name), member_symbol)
            }
            _ => CallTarget::new(None, None),
        }
    }

    /// Check whether a symbol id refers to a function declaration.
    fn symbol_is_function(self, symbol_id: dir::GlobalSymbolId) -> bool {
        let _ctx = self;
        let Some(ctx) = self.module_context(symbol_id.module_id) else {
            return false;
        };

        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);

        symbol.kind == dir::SymbolKind::Function
    }

    /// Return the canonical function symbols targeted by one call.
    fn call_target_symbols(
        self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left_expression_id: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> Vec<dir::GlobalSymbolId> {
        let ctx = self;
        let mut targets = Vec::new();

        if let Some(target_symbol) = left_expression_id
            .and_then(|left_expression_id| ctx.expression_symbol_target(left_expression_id))
        {
            targets.push(target_symbol);
            targets.push(ctx.canonical_symbol(target_symbol));
        }

        let node_id = dir::GlobalNodeIdAny {
            module_id: ctx.module_id(),
            local_id: expression_id.into(),
        };
        let Some(resolution) = ctx.resolutions().call_resolution(node_id) else {
            return targets;
        };

        match &resolution.target {
            dir::CallTarget::Symbol(candidate) => {
                targets.push(candidate.symbol);
                targets.push(ctx.canonical_symbol(candidate.symbol));
            }
            dir::CallTarget::Union(candidates) => {
                for candidate in candidates {
                    targets.push(candidate.symbol);
                    targets.push(ctx.canonical_symbol(candidate.symbol));
                }
            }
            dir::CallTarget::Builtin(_) | dir::CallTarget::Expression { .. } => {}
        };

        if let Some(resolution) = ctx.resolutions().construct_resolution(node_id) {
            let symbol = resolution.target.symbol();

            targets.push(symbol);
            targets.push(ctx.canonical_symbol(symbol));
        }

        targets.sort();
        targets.dedup();
        targets
    }
}

impl ModuleQueryContext<'_> {
    /// Find the containing function symbol for one node.
    fn find_containing_function_symbol(
        &self,
        node_id: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        let ctx = self;
        let mut current = Some(node_id);

        while let Some(node_id) = current {
            if node_id.ty == dir::NodeType::Declaration {
                let declaration_id: dir::LocalNodeId<dir::Declaration> = node_id.try_into().ok()?;
                let declaration = ctx.dir().view().get::<dir::Declaration>(declaration_id);
                if matches!(declaration, dir::Declaration::Function(_)) {
                    return ctx
                        .dir()
                        .symbol_for_node(declaration_id.into())
                        .map(|symbol_id| dir::GlobalSymbolId::new(ctx.module_id(), symbol_id));
                }
            }

            current = ctx.dir().view().get_parent_any(node_id);
        }

        None
    }
}
