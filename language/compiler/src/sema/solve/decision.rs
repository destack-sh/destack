use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::sema::{CheckEvent, CheckState};

impl CheckState<'_> {
    /// Commit one node decision into its module's decision segment.
    pub(in crate::sema) fn commit_decision(
        &mut self,
        node: dir::GlobalNodeIdAny,
        resolution: dir::Decision,
    ) -> CompilerResult<()> {
        // later derivations refine the same answer, the last settled one stays
        self.module_mut(node.module_id)
            .decisions
            .set_decision(node, resolution);
        self.record_event(CheckEvent::NodeDecided { node });

        Ok(())
    }

    /// Commit one node's lexical name into its module's resolution segment.
    pub(in crate::sema) fn commit_name(
        &mut self,
        node: dir::GlobalNodeIdAny,
        resolution: dir::NameResolution,
    ) -> CompilerResult<()> {
        self.module_mut(node.module_id)
            .resolutions
            .set_name_resolution(node, resolution);
        self.record_event(CheckEvent::NodeDecided { node });

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
