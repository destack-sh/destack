use destack_artifact::{MirElaborated, MirLowered};
use destack_core::StringPool;
use destack_mir as mir;

/// State for one MIR elaboration.
pub(crate) struct ElaborateState<'a> {
    /// The MIR tree being elaborated.
    pub(in crate::elaborate) tree: mir::Tree,
    /// Target ABI layout.
    pub(in crate::elaborate) target: mir::TargetLayout,
    /// Canonical MIR type table.
    pub(in crate::elaborate) types: mir::TypeTable,
    /// Canonical MIR layout table.
    pub(in crate::elaborate) layouts: mir::LayoutTable,
    /// Canonical MIR dispatch table.
    pub(in crate::elaborate) dispatch: mir::DispatchTable,
    /// Canonical MIR drop table.
    pub(in crate::elaborate) drops: mir::DropTable,
    /// Explicit MIR memory access table.
    pub(in crate::elaborate) memory: mir::MemoryTable,
    /// Function and call effect table.
    pub(in crate::elaborate) effects: mir::EffectTable,
    /// Static profile counter table.
    pub(in crate::elaborate) profile: mir::ProfileTable,
    /// Strings needed by generated MIR names.
    pub(in crate::elaborate) strings: &'a StringPool,
}

impl<'a> ElaborateState<'a> {
    /// Create one elaboration from lowered MIR.
    pub(in crate::elaborate) fn new(lowered: MirLowered, strings: &'a StringPool) -> Self {
        Self {
            tree: lowered.tree,
            target: lowered.target,
            types: lowered.types,
            layouts: lowered.layouts,
            dispatch: lowered.dispatch,
            drops: lowered.drops,
            memory: lowered.memory,
            effects: lowered.effects,
            profile: lowered.profile,
            strings,
        }
    }

    /// Finish elaborated MIR.
    pub(in crate::elaborate) fn finish(self) -> MirElaborated {
        MirElaborated {
            tree: self.tree,
            target: self.target,
            types: self.types,
            layouts: self.layouts,
            dispatch: self.dispatch,
            drops: self.drops,
            memory: self.memory,
            effects: self.effects,
            profile: self.profile,
        }
    }
}
