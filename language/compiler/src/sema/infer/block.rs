use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::CompilerResult;
use crate::sema::{CheckState, Expectation, FlowSite, PlaceUse, ValueCheck};

impl CheckState<'_> {
    /// Infer one block from its tail expression.
    pub(in crate::sema) fn infer_block(
        &mut self,
        site: FlowSite,
        block: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<()> {
        // infer the leading statements
        let node = site.node;
        let module = node.module_id;
        self.infer_block_statements(module, block)?;

        // type the block at its tail expression
        let tail = self.visit_block_value(module, block)?;
        let ty = match tail {
            Some(tail) => self.infer_node_type(tail, PlaceUse::Read)?,
            None => self.end_type(module, block.into_any())?,
        };
        self.commit_node_type(node, ty)?;

        Ok(())
    }

    /// Check one block under an expected result type.
    pub(in crate::sema) fn check_block(
        &mut self,
        site: FlowSite,
        block: dir::LocalNodeId<dir::Block>,
        expectation: Expectation,
    ) -> CompilerResult<ValueCheck> {
        // infer the leading statements once
        let module = site.node.module_id;
        if self.committed_node_type(site.node).is_none() {
            self.infer_block_statements(module, block)?;
        }

        // check the block's value against the expectation
        let value = self.visit_block_value(module, block)?;
        let check = match value {
            Some(value) => {
                let check = self.check_node(value, expectation)?;
                let value_type = check.source;
                if self.committed_node_type(site.node).is_none() {
                    self.commit_node_type(site.node, value_type)?;
                }

                ValueCheck {
                    source: value_type,
                    outcome: check.outcome,
                    target: check.target,
                }
            }
            None => {
                let value = self.end_type(module, block.into_any())?;
                self.commit_node_type(site.node, value)?;

                self.check_value(site, value, expectation)?
            }
        };

        Ok(check)
    }

    /// Infer every leading statement of one block in source order.
    pub(in crate::sema) fn infer_block_statements(
        &mut self,
        module: ModuleId,
        block: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<()> {
        // infer each leading statement in source order
        let statements = self
            .module(module)
            .view()
            .get(block)
            .leading_expressions
            .clone();
        let mut is_end_reachable = true;
        for statement in statements {
            let node = statement.into_global_any(module);
            let Some(site) = self.visit_block_expression(module, statement)? else {
                continue;
            };

            // mark a statement past the diverging end unreachable
            if !is_end_reachable {
                self.module_mut(module).flows.set_unreachable(node.local_id);
            }

            // scope the inference each statement opens to that statement
            self.attempt_node(site, PlaceUse::Read, None)?;

            // end the block unreachable on a never-typed statement, excluding unbound jumps
            if let Some(ty) = self.committed_node_type(node) {
                let ty = self.shallow_resolve(ty)?;
                if matches!(self.ty(ty)?, dir::Type::Never)
                    && !self.flow.is_unbound_jump(node.local_id)
                {
                    is_end_reachable = false;
                    self.module_mut(module).flows.set_diverging(node.local_id);
                    self.flow.insert_diverge();
                }
            }
        }

        // record the unreachable end for the block's completion type
        if !is_end_reachable {
            self.module_mut(module)
                .unreachable_ends
                .insert(block.into_any());
        }

        Ok(())
    }

    /// Return the implicit value type at one source node's end.
    pub(in crate::sema) fn end_type(
        &mut self,
        module: ModuleId,
        node: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // end an unreachable block at never
        let ty = match self.module(module).unreachable_ends.contains(&node) {
            true => dir::Type::Never,
            false => dir::Type::Void,
        };

        self.intern_type(ty)
    }

    /// Visit one statically present block expression after walking its decorators.
    fn visit_block_expression(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<FlowSite>> {
        // skip a statement its decorators removed
        if !self.walk_body_decorators(module, expression.into_any())? {
            return Ok(None);
        }

        // visit the statement at its site
        let site = self.visit_site(expression.into_global_any(module))?;

        Ok(Some(site))
    }

    /// Visit the expression that produces one block's value.
    fn visit_block_value(
        &mut self,
        module: ModuleId,
        block: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<Option<FlowSite>> {
        // require a value expression
        let Some(value) = self.module(module).view().get(block).value_expression() else {
            return Ok(None);
        };

        self.visit_block_expression(module, value)
    }
}
