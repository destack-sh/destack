use destack_artifact::{MirElaborated, MirLowered};
use destack_core::StringPool;
use destack_mir as mir;

use crate::{CompilerError, CompilerResult};

use super::drop::{DestructorBuilder, DropInserter, DropPlan};
use super::safepoint::SafepointInserter;

/// State for one MIR elaboration.
pub(crate) struct ElaborateState<'a> {
    /// The MIR tree being elaborated.
    pub(in crate::elaborate) tree: mir::Tree,
    /// Target ABI layout.
    pub(in crate::elaborate) target: mir::TargetLayout,
    /// Canonical MIR layout table.
    pub(in crate::elaborate) layouts: mir::LayoutTable,
    /// Canonical MIR drop table.
    pub(in crate::elaborate) drops: mir::DropTable,
    /// Function and call effect table.
    pub(in crate::elaborate) effects: mir::EffectTable,

    /// Strings needed by generated MIR names.
    pub(in crate::elaborate) strings: &'a StringPool,
}

impl<'a> ElaborateState<'a> {
    /// Create one elaboration from lowered MIR.
    pub(in crate::elaborate) fn new(lowered: &MirLowered, strings: &'a StringPool) -> Self {
        Self {
            tree: lowered.tree.clone(),
            target: lowered.target,
            layouts: lowered.layouts.clone(),
            drops: lowered.drops.clone(),
            effects: lowered.effects.clone(),
            strings,
        }
    }

    /// Elaborate implicit destruction.
    pub(in crate::elaborate) fn elaborate(
        &mut self,
        retention: &mir::RetentionTable,
        safepoints: &mir::SafepointTable,
    ) -> CompilerResult<()> {
        // plan destruction against the verified source functions
        let functions = self
            .tree
            .iter_nodes::<mir::Function>()
            .filter_map(|(id, function)| {
                (function.entry().is_some() && !self.drops.is_destructor(id)).then_some(id)
            })
            .collect::<Vec<_>>();
        let plans = functions
            .iter()
            .map(|&id| DropPlan::build(id, self.tree.get(id), &self.tree, retention, &self.drops))
            .collect::<Vec<_>>();
        let frame_roots = plans.iter().flat_map(DropPlan::roots).collect::<Vec<_>>();
        let deferred_roots = plans
            .iter()
            .flat_map(|plan| plan.deferred_types(&self.tree))
            .collect::<Vec<_>>();

        // build storage destructors required by managed allocations
        let mut destructors = DestructorBuilder::new(
            &mut self.tree,
            self.target,
            &mut self.drops,
            &mut self.effects,
            self.strings,
        );
        destructors.build_allocations();
        destructors.build_frames(frame_roots);
        destructors.build_deferred(deferred_roots);

        // insert verified destruction into each source function
        DropInserter::new(&mut self.tree, &self.drops).insert(plans);

        // pin over parking safepoints and poll at the others
        SafepointInserter::new(&mut self.tree, &functions).insert(safepoints);

        // complete layouts for types introduced by elaboration
        let mut layouts = mir::LayoutBuilder::new(&self.tree, &mut self.layouts, self.target);
        layouts
            .layout_reachable_types()
            .map_err(|error| CompilerError::Internal {
                message: format!("elaborated MIR contains an invalid physical layout: {error}"),
            })?;

        Ok(())
    }

    /// Finish elaborated MIR.
    pub(in crate::elaborate) fn finish(self) -> MirElaborated {
        MirElaborated {
            tree: self.tree,
            layouts: self.layouts,
            drops: self.drops,
            effects: self.effects,
        }
    }
}
