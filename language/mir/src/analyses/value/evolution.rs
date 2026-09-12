use std::sync::Arc;

use crate::{
    Analysis, DefinitionTable, DominatorTable, FunctionId, LoopTable, Mutation, TargetLayout, Tree,
};

/// Symbolic scalar recurrences and loop exit counts.
#[derive(Debug)]
pub struct ScalarEvolutionTable {}

impl ScalarEvolutionTable {
    /// Construct scalar evolution for one function.
    pub fn analyse(
        _function: FunctionId,
        _definitions: Arc<DefinitionTable>,
        _dominators: Arc<DominatorTable>,
        _loops: Arc<LoopTable>,
        _target_layout: TargetLayout,
        _tree: &Tree,
    ) -> Self {
        todo!("TODO #Incomplete: implement ScalarEvolutionTable")
    }
}

impl Analysis for ScalarEvolutionTable {
    const INVALIDATED_BY: Mutation = DefinitionTable::INVALIDATED_BY
        .union(DominatorTable::INVALIDATED_BY)
        .union(LoopTable::INVALIDATED_BY)
        .union(Mutation::LAYOUT);
}
