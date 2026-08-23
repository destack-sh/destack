use destack_dir as dir;
use destack_source::ModuleId;

use crate::sema::{CheckEvent, CheckState};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Commit one node decision into its module's decision segment.
    pub(in crate::sema) fn commit_decision(
        &mut self,
        node: dir::GlobalNodeIdAny,
        resolution: dir::Decision,
    ) -> CompilerResult<()> {
        let uses = resolution.binding_uses();

        // refuse refinements that would invalidate already committed uses
        if let Some(previous) = self.module(node.module_id).decisions.decision(node) {
            let previous_uses = previous.binding_uses();
            if !previous_uses.is_empty() && previous_uses != uses {
                return Err(CompilerError::Internal {
                    message: format!(
                        "check node {} refined its selected declaration uses: previous = {previous_uses:?}, new = {uses:?}",
                        self.node_label(node),
                    ),
                });
            }
        }

        // write the selected declaration uses
        for (symbol, binding_use) in uses {
            self.module_mut(node.module_id).flows.commit_binding_use(
                node.local_id,
                symbol,
                binding_use,
            );
        }

        // later derivations refine the same targets, the last committed payload stays
        self.module_mut(node.module_id)
            .decisions
            .set_decision(node, resolution);
        self.push_event(CheckEvent::NodeDecided { node });

        Ok(())
    }

    /// Commit one node's lexical name into its module's resolution segment.
    pub(in crate::sema) fn commit_name(
        &mut self,
        node: dir::GlobalNodeIdAny,
        resolution: dir::NameResolution,
    ) -> CompilerResult<()> {
        // write one unambiguous lexical reference
        if let Some(symbol) = resolution.single_symbol() {
            self.module_mut(node.module_id).flows.commit_binding_use(
                node.local_id,
                symbol,
                dir::BindingUse::READ,
            );
        }

        self.module_mut(node.module_id)
            .resolutions
            .set_name_resolution(node, resolution);
        self.push_event(CheckEvent::NodeDecided { node });

        Ok(())
    }

    /// Return one node's committed lexical name.
    pub(in crate::sema) fn name_decision(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> Option<&dir::NameResolution> {
        self.module(node.module_id)
            .resolutions
            .name_resolution(node)
    }

    /// Poison one node whose operand already reported an error.
    pub(in crate::sema) fn poison_node(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.commit_decision(node, dir::Decision::Poisoned)?;
        self.commit_error_node(node)
    }

    /// Return one node's committed resolution.
    pub(in crate::sema) fn decision(&self, node: dir::GlobalNodeIdAny) -> Option<&dir::Decision> {
        self.module(node.module_id).decisions.decision(node)
    }

    /// Return one module's committed resolutions.
    pub(in crate::sema) fn resolutions(&self, module: ModuleId) -> &dir::ResolutionSegment {
        &self.module(module).resolutions
    }

    /// Return one module's committed decisions.
    pub(in crate::sema) fn decisions(&self, module: ModuleId) -> &dir::DecisionSegment {
        &self.module(module).decisions
    }
}
