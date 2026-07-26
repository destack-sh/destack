use destack_dir as dir;
use destack_repository::{ProviderError, ProviderResult};

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
    pub(super) fn build(
        module: &'context ModuleQueryContext<'query>,
    ) -> ProviderResult<dir::CallIndex> {
        let mut indexer = Self {
            module,
            entries: Vec::new(),
        };

        // collect checked call families
        indexer.collect_calls()?;
        indexer.collect_constructs()?;

        Ok(dir::CallIndex::new(indexer.entries))
    }

    /// Collect checked call target edges.
    fn collect_calls(&mut self) -> ProviderResult<()> {
        for (node_id, resolution) in self.module.resolutions().call_entries() {
            // resolve call source metadata
            let source = node_id
                .try_into_typed::<dir::Expression>()
                .map_err(|error| {
                    ProviderError::internal(format!("call source is not an expression: {error}"))
                })?;
            let Some(span) = self.module.view().get_span_by_id(node_id.local_id.id) else {
                continue;
            };
            let caller = self.containing_symbol(node_id.local_id)?;

            // emit one edge per resolved callee
            match &resolution.target {
                dir::CallTarget::Symbol(candidate) => {
                    self.entries.push(dir::CallEntry {
                        source,
                        kind: dir::CallKind::Call,
                        caller,
                        callee: candidate.symbol,
                        span,
                    });
                }
                dir::CallTarget::Universal(candidates) => {
                    for candidate in candidates {
                        self.entries.push(dir::CallEntry {
                            source,
                            kind: dir::CallKind::Call,
                            caller,
                            callee: candidate.symbol,
                            span,
                        });
                    }
                }
                dir::CallTarget::Expression { .. } => {}
            }
        }

        Ok(())
    }

    /// Collect checked construct target edges.
    fn collect_constructs(&mut self) -> ProviderResult<()> {
        for (node_id, resolution) in self.module.resolutions().construct_entries() {
            // resolve construct source metadata
            let source = node_id
                .try_into_typed::<dir::Expression>()
                .map_err(|error| {
                    ProviderError::internal(format!(
                        "construct source is not an expression: {error}"
                    ))
                })?;
            let Some(span) = self.module.view().get_span_by_id(node_id.local_id.id) else {
                continue;
            };
            let caller = self.containing_symbol(node_id.local_id)?;
            let callee = resolution.target.call_symbol();

            // emit the resolved construct edge
            self.entries.push(dir::CallEntry {
                source,
                kind: dir::CallKind::Construct,
                caller,
                callee,
                span,
            });
        }

        Ok(())
    }

    /// Find the containing declaration or member symbol for one node.
    fn containing_symbol(
        &self,
        node_id: dir::LocalNodeIdAny,
    ) -> ProviderResult<Option<dir::GlobalSymbolId>> {
        let mut current = Some(node_id);

        // stop at the nearest callable declaration
        while let Some(node_id) = current {
            if node_id.ty == dir::NodeType::Declaration {
                let declaration_id =
                    node_id.try_into_typed::<dir::Declaration>().map_err(|_| {
                        ProviderError::internal(format!(
                            "declaration node id has incompatible type: {node_id:?}"
                        ))
                    })?;
                let declaration = self.module.view().get(declaration_id);

                // anonymous functions have no call hierarchy item
                if let dir::Declaration::Function(function) = declaration {
                    if function.name.is_none() {
                        return Ok(None);
                    }
                    let symbol_id = self.module.node_symbol(node_id).ok_or_else(|| {
                        ProviderError::internal(format!(
                            "missing symbol for function declaration {declaration_id:?}"
                        ))
                    })?;

                    return Ok(Some(dir::GlobalSymbolId::new(
                        self.module.module_id(),
                        symbol_id,
                    )));
                }
            }

            // stop at the nearest method declaration
            if node_id.ty == dir::NodeType::Member {
                let member_id = node_id.try_into_typed::<dir::Member>().map_err(|_| {
                    ProviderError::internal(format!(
                        "member node id has incompatible type: {node_id:?}"
                    ))
                })?;
                if matches!(
                    self.module.view().get(member_id),
                    dir::Member::Method { .. }
                ) {
                    let symbol_id = self.module.node_symbol(node_id).ok_or_else(|| {
                        ProviderError::internal(format!(
                            "missing symbol for method member {member_id:?}"
                        ))
                    })?;

                    return Ok(Some(dir::GlobalSymbolId::new(
                        self.module.module_id(),
                        symbol_id,
                    )));
                }
            }

            // continue with the parent node
            current = self.module.view().get_parent_any(node_id);
        }

        Ok(None)
    }
}
