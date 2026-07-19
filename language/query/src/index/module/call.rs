use destack_dir as dir;

use crate::ModuleQueryContext;

/// Builder for one call index from checked DIR.
pub(super) struct CallIndexer<'context, 'query> {
    /// The indexed module context.
    module: &'context ModuleQueryContext<'query>,
    /// The collected index entries.
    entries: Vec<dir::CallEntry>,
}

impl<'context, 'query> CallIndexer<'context, 'query> {
    /// Build the call index.
    pub(super) fn build(module: &'context ModuleQueryContext<'query>) -> dir::CallIndex {
        let mut indexer = Self {
            module,
            entries: Vec::new(),
        };

        // collect checked call families
        indexer.collect_calls();
        indexer.collect_constructs();

        dir::CallIndex::new(indexer.entries)
    }

    /// Collect checked call target edges.
    fn collect_calls(&mut self) {
        for (node_id, resolution) in self.module.resolutions().call_entries() {
            // resolve call source metadata
            let source = node_id;
            let target = self.call_target_node(node_id.local_id);
            let span = self.module.get_span(self.module.view(), node_id.local_id);
            let caller_symbol = self.containing_symbol(node_id.local_id);

            // emit one edge per resolved callee
            for callee_symbol in self.call_resolution_symbols(resolution) {
                self.entries.push(dir::CallEntry {
                    source,
                    target,
                    kind: dir::CallKind::Call,
                    caller: caller_symbol,
                    callee: callee_symbol,
                    span,
                });
            }
        }
    }

    /// Collect checked construct target edges.
    fn collect_constructs(&mut self) {
        for (node_id, resolution) in self.module.resolutions().construct_entries() {
            // resolve construct source metadata
            let source = node_id;
            let target = self.construct_target_node(node_id.local_id);
            let span = self.module.get_span(self.module.view(), node_id.local_id);
            let caller_symbol = self.containing_symbol(node_id.local_id);

            // emit one edge per resolved construct target
            for callee_symbol in self.construct_resolution_symbols(resolution) {
                self.entries.push(dir::CallEntry {
                    source,
                    target,
                    kind: dir::CallKind::Construct,
                    caller: caller_symbol,
                    callee: callee_symbol,
                    span,
                });
            }
        }
    }

    /// Return the callee expression for one indexed call.
    fn call_target_node(&self, source: dir::LocalNodeIdAny) -> dir::GlobalNodeIdAny {
        // verify the checked resolution source shape
        let expression_id = source
            .try_into_typed::<dir::Expression>()
            .unwrap_or_else(|_| panic!("call resolution source is not an expression: {source:?}"));
        let expression = self.module.view().get(expression_id);
        let dir::Expression::Call { left, .. } = expression else {
            panic!("call resolution source is not a call expression: {source:?}");
        };

        (*left).into_global_any(self.module.module_id())
    }

    /// Return the constructed type expression for one indexed construct.
    fn construct_target_node(&self, source: dir::LocalNodeIdAny) -> dir::GlobalNodeIdAny {
        // verify the checked resolution source shape
        let expression_id = source
            .try_into_typed::<dir::Expression>()
            .unwrap_or_else(|_| {
                panic!("construct resolution source is not an expression: {source:?}")
            });
        let expression = self.module.view().get(expression_id);

        // read the constructed type expression
        let ty = match expression {
            dir::Expression::New { ty, .. } | dir::Expression::NewMaybe { ty, .. } => *ty,
            _ => panic!("construct resolution source is not a construct expression: {source:?}"),
        };

        ty.into_global_any(self.module.module_id())
    }

    /// Return symbols selected by one call resolution.
    fn call_resolution_symbols(
        &self,
        resolution: &dir::CallResolution,
    ) -> Vec<dir::GlobalSymbolId> {
        let mut targets = Vec::new();

        // collect checked call targets
        match &resolution.target {
            dir::CallTarget::Symbol(candidate) => {
                targets.push(candidate.symbol);
            }
            dir::CallTarget::Universal(candidates) => {
                for candidate in candidates {
                    targets.push(candidate.symbol);
                }
            }
            dir::CallTarget::Expression { .. } => {}
        }

        // normalize duplicate overload targets
        targets.sort();
        targets.dedup();

        targets
    }

    /// Return symbols selected by one construct resolution.
    fn construct_resolution_symbols(
        &self,
        resolution: &dir::ConstructResolution,
    ) -> Vec<dir::GlobalSymbolId> {
        let mut targets = Vec::new();

        // collect checked construct target
        let symbol = resolution.target.symbol();

        targets.push(symbol);

        // normalize duplicate construct targets
        targets.sort();
        targets.dedup();

        targets
    }

    /// Find the containing declaration or member symbol for one node.
    fn containing_symbol(&self, node_id: dir::LocalNodeIdAny) -> Option<dir::GlobalSymbolId> {
        let mut current = Some(node_id);

        // walk parents until a callable declaration owner is found
        while let Some(node_id) = current {
            if let Some(symbol_id) = self.containing_function_symbol(node_id) {
                return Some(symbol_id);
            }

            // use member symbols as callable owners
            if let Some(symbol_id) = self.containing_member_symbol(node_id) {
                return Some(symbol_id);
            }

            // continue with the parent node
            current = self.module.view().get_parent_any(node_id);
        }

        None
    }

    /// Return the containing function declaration symbol for one node.
    fn containing_function_symbol(
        &self,
        node_id: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        // skip non-declaration nodes
        if node_id.ty != dir::NodeType::Declaration {
            return None;
        }

        // verify the parent declaration id shape
        let declaration_id: dir::LocalNodeId<dir::Declaration> = node_id
            .try_into()
            .unwrap_or_else(|_| panic!("declaration node id has incompatible type: {node_id:?}"));

        // keep only function declarations as call owners
        let declaration = self.module.view().get::<dir::Declaration>(declaration_id);
        if !matches!(declaration, dir::Declaration::Function(_)) {
            return None;
        }

        // read the checked symbol for the function declaration
        let symbol_id = self
            .module
            .node_symbol(declaration_id.into())
            .unwrap_or_else(|| {
                panic!("missing symbol for function declaration {declaration_id:?}")
            });

        Some(dir::GlobalSymbolId::new(self.module.module_id(), symbol_id))
    }

    /// Return the containing member symbol for one node.
    fn containing_member_symbol(
        &self,
        node_id: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        // skip non-member nodes
        if node_id.ty != dir::NodeType::Member {
            return None;
        }

        // read the checked symbol attached to this member
        self.module
            .node_symbol(node_id)
            .map(|symbol_id| dir::GlobalSymbolId::new(self.module.module_id(), symbol_id))
    }
}
