use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    BodyState, CheckOutcome, Expectation, FlowSite, InferenceScope, PlaceUse, ValueCheck,
};

impl BodyState<'_, '_> {
    /// Infer one block from its tail expression.
    pub(in crate::check) fn infer_block(
        &mut self,
        site: FlowSite,
        block: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<()> {
        let node = site.node;
        let module = node.module_id;
        self.check_block_statements(module, block)?;
        let tail = self.block_value(module, block)?;
        let ty = match tail {
            Some(tail) => {
                let tail_site = self.visit_site(tail.into_global_any(module))?;
                self.infer_node_type(tail_site, PlaceUse::Read)?
            }
            None => self.end_type(module, block.into_any())?,
        };
        self.commit_node_type(node, ty)?;

        Ok(())
    }

    /// Check every leading statement of one block in source order.
    pub(in crate::check) fn check_block_statements(
        &mut self,
        module: ModuleId,
        block: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<()> {
        let statements = self
            .module(module)
            .view()
            .get(block)
            .leading_expressions
            .clone();
        let mut is_end_reachable = true;
        for statement in statements {
            let node = statement.into_global_any(module);
            if !self.walk_body_decorators(module, statement.into_any())? {
                continue;
            }
            let site = self.check.visit_site(node)?;

            // let each statement own the inference it opens
            let scope = InferenceScope::open(
                self.check.infer.variable_count(),
                self.check.infer.trail.len(),
            );
            self.attempt_node(site, PlaceUse::Read, None)?;

            // inference closes at the statement that opened it
            self.check.close_statement(scope)?;

            // end the block unreachable on a never-typed statement, excluding unbound jumps
            if let Some(ty) = self.check.committed_node_type(node) {
                let ty = self.check.shallow_resolve(ty)?;
                if matches!(self.check.ty(ty)?, dir::Type::Never)
                    && !self.check.flow.is_unbound_jump(node.local_id)
                {
                    is_end_reachable = false;
                }
            }
        }

        // record the unreachable end for the block's completion type
        if !is_end_reachable {
            self.check
                .module_mut(module)
                .unreachable_ends
                .insert(block.into_any());
        }

        Ok(())
    }

    /// Return the implicit value type at one source node's end.
    pub(in crate::check) fn end_type(
        &mut self,
        module: ModuleId,
        node: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let ty = match self.check.module(module).unreachable_ends.contains(&node) {
            true => dir::Type::Never,
            false => dir::Type::Void,
        };

        self.intern_type(ty)
    }

    /// Check one block under an expected result type.
    pub(in crate::check) fn check_block(
        &mut self,
        site: FlowSite,
        block: dir::LocalNodeId<dir::Block>,
        expectation: Expectation,
    ) -> CompilerResult<ValueCheck> {
        let module = site.node.module_id;
        self.check_block_statements(module, block)?;
        let value = self.block_value(module, block)?;
        let check = match value {
            Some(value) => {
                let value_site = self.visit_site(value.into_global_any(module))?;
                let check = self.check_node(value_site, expectation)?;
                let value_type = check.source;
                self.commit_node_type(site.node, value_type)?;

                ValueCheck {
                    source: value_type,
                    outcome: check.outcome,
                    target: check.target,
                }
            }
            None => {
                let value = self.end_type(module, block.into_any())?;
                self.commit_node_type(site.node, value)?;
                ValueCheck {
                    source: value,
                    outcome: CheckOutcome::Holds,
                    target: expectation.target,
                }
            }
        };

        Ok(check)
    }

    /// Return the statically present value expression of one block.
    fn block_value(
        &mut self,
        module: ModuleId,
        block: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<Option<dir::LocalNodeId<dir::Expression>>> {
        let value = self.module(module).view().get(block).value_expression();
        let value = match value {
            Some(value) if self.check.decide_static_presence(value.into_any())? => Some(value),
            Some(_) | None => None,
        };

        Ok(value)
    }
}
