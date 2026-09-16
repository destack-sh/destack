use destack_artifact::{MirElaborated, MirInstantiated};
use destack_core::StringPool;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::{CompilerError, CompilerResult};

use super::drop::{DestructorBuilder, DropPlan};
use super::function::DropInserter;

/// State for one MIR elaboration.
pub(crate) struct ElaborateState<'a> {
    /// The module elaborated.
    pub(in crate::elaborate) module: ModuleId,
    /// The MIR tree being elaborated.
    pub(in crate::elaborate) tree: mir::Tree,
    /// Target ABI layout.
    pub(in crate::elaborate) target: mir::TargetLayout,
    /// Canonical MIR layout table.
    pub(in crate::elaborate) layouts: mir::LayoutTable,
    /// Dispatch shapes and tables.
    pub(in crate::elaborate) dispatch: mir::DispatchTable,
    /// Canonical MIR drop table.
    pub(in crate::elaborate) drops: mir::DropTable,
    /// Function and call effect table.
    pub(in crate::elaborate) effects: mir::EffectTable,

    /// The module initializer, when one exists.
    initializer: Option<mir::FunctionId>,
    /// Static profile counters.
    profile: mir::ProfileTable,

    /// Strings needed by generated MIR names.
    pub(in crate::elaborate) strings: &'a StringPool,
}

impl<'a> ElaborateState<'a> {
    /// Create one elaboration over instantiated MIR.
    pub(in crate::elaborate) fn new(
        module: ModuleId,
        instantiated: &MirInstantiated,
        strings: &'a StringPool,
    ) -> Self {
        Self {
            module,
            initializer: instantiated.initializer,
            tree: mir::Tree::clone(&instantiated.tree),
            target: instantiated.target,
            layouts: instantiated.layouts.clone(),
            drops: instantiated.drops.clone(),
            effects: instantiated.effects.clone(),
            dispatch: instantiated.dispatch.clone(),
            profile: instantiated.profile.clone(),
            strings,
        }
    }

    /// Elaborate implicit destruction.
    pub(in crate::elaborate) fn elaborate(
        &mut self,
        retention: &mir::RetentionTable,
    ) -> CompilerResult<()> {
        // collect the source functions with bodies
        let functions = self
            .tree
            .iter_nodes::<mir::Function>()
            .filter_map(|(id, function)| {
                (function.entry().is_some() && !self.drops.is_destructor(id)).then_some(id)
            })
            .collect::<Vec<_>>();

        // plan destruction over the explicit functions
        let plans = functions
            .iter()
            .map(|&id| DropPlan::build(id, self.tree.get(id), &self.tree, retention, &self.drops))
            .collect::<Vec<_>>();
        let frame_roots = plans
            .iter()
            .flat_map(DropPlan::roots)
            .chain(
                plans
                    .iter()
                    .flat_map(|plan| plan.overwrite_types(&self.tree)),
            )
            .collect::<Vec<_>>();

        // build storage destructors required by managed allocations
        let mut destructors = DestructorBuilder::new(
            self.module,
            &mut self.tree,
            self.target,
            &mut self.drops,
            &mut self.effects,
            self.strings,
        );
        destructors.build_allocations();
        destructors.build_frames(frame_roots);

        // insert verified destruction
        DropInserter::new(&mut self.tree, &self.drops).insert(plans);

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
            target: self.target,
            initializer: self.initializer,
            layouts: self.layouts,
            dispatch: self.dispatch,
            drops: self.drops,
            effects: self.effects,
            profile: self.profile,
        }
    }
}
