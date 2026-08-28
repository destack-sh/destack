use destack_artifact::{MirElaborated, MirLowered};
use destack_core::StringPool;
use destack_mir as mir;
use destack_source::ProvenanceBuilder;

use crate::{CompilerError, CompilerResult};

use super::drop::{DestructorBuilder, DropInserter, DropPlan};

const INSERT_DROPS: &str = "insert-drops";

/// State for one MIR elaboration.
pub(crate) struct ElaborateState<'a> {
    /// The MIR tree being elaborated.
    pub(in crate::elaborate) tree: mir::Tree,
    /// The provenance transformations produced by elaboration.
    pub(in crate::elaborate) provenance: ProvenanceBuilder,
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
            provenance: lowered.provenance.extend(),
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
            .into_iter()
            .map(|id| DropPlan::build(id, self.tree.get(id), &self.tree, retention))
            .collect::<Vec<_>>();

        // build every storage destructor required by the complete drop plan
        let mut destructors = DestructorBuilder::new(
            &mut self.tree,
            &mut self.provenance,
            self.target,
            &mut self.drops,
            &mut self.effects,
            self.strings,
        );
        destructors.build(&plans);

        // insert verified destruction into each source function
        let mut provenance = self.provenance.record(INSERT_DROPS);
        DropInserter::new(&mut self.tree, &mut provenance, &self.drops).insert(plans);

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
            provenance: self.provenance.finish(),
            layouts: self.layouts,
            drops: self.drops,
            effects: self.effects,
        }
    }
}
