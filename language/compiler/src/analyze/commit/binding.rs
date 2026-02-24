use crate::analyze::common::CommitContext;
use crate::{Compiler, InferContext};
use destack_dir::{GlobalSymbolId, InferTable, LocalTypeId};

/// One direct-binding value-type commit action collected after solve convergence.
#[derive(Debug, Clone, Copy)]
pub(in crate::analyze::commit) struct DirectBindingValueTypeCommitAction {
    /// The direct binding symbol to commit.
    pub symbol_id: GlobalSymbolId,
    /// The value type id in the collection snapshot.
    pub type_id: LocalTypeId,
}

impl Compiler {
    /// Collect direct-binding value-type commit actions from solved initializer intents.
    pub(super) fn collect_binding_value_commit_actions(
        &self,
        ctx: &mut CommitContext<'_>,
        infer: &InferTable,
    ) -> Vec<DirectBindingValueTypeCommitAction> {
        let module = ctx.module.module;
        let profile = ctx.module.profile;
        let tree = ctx.module.tree;
        let symbols = ctx.module.symbols;
        let types = &mut *ctx.types;
        let options = *ctx.module.options;

        // consume infer-recorded write intents in deterministic symbol or node order
        let mut intents = infer
            .iter_direct_binding_value_commit_intents()
            .filter(|(symbol_id, _, _)| symbol_id.module_id == module.id)
            .collect::<Vec<_>>();
        intents.sort_by_key(|(symbol_id, declarator_id, value_id)| {
            (symbol_id.local_id.id, declarator_id.id, value_id.id)
        });
        let mut actions = Vec::with_capacity(intents.len());

        // commit one deterministic binding value type per direct initializer binding
        for (symbol_id, declarator_id, value_id) in intents {
            if !self.is_node_active(tree, symbols, declarator_id.into_any()) {
                continue;
            }

            let declarator_node_id = declarator_id.into_global_any(module.id);
            if types.get_declared_type_id(declarator_node_id).is_some() {
                continue;
            }

            let inferred_node_id = value_id.into_global_any(module.id);
            let Some(inferred_type_id) = types.get_inferred_type_id(inferred_node_id) else {
                continue;
            };

            let binding_mutability = symbols.get_symbol(symbol_id.local_id).binding_mutability;
            let materialize_ctx =
                InferContext::new(profile, options).with_binding_initializer(binding_mutability);
            let committed_type_id = self.materialize_declarator_initializer_type(
                module,
                declarator_id,
                value_id,
                inferred_type_id,
                tree,
                &materialize_ctx,
                types,
            );
            actions.push(DirectBindingValueTypeCommitAction {
                symbol_id,
                type_id: committed_type_id,
            });
        }

        actions
    }

    /// Apply direct-binding value-type commit actions to committed type tables.
    pub(super) fn apply_binding_value_commit_actions(
        &self,
        actions: Vec<DirectBindingValueTypeCommitAction>,
        ctx: &mut CommitContext<'_>,
    ) {
        for action in actions {
            ctx.types.set_value_type(action.symbol_id, action.type_id);
        }
    }
}
