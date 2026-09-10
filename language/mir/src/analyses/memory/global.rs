use crate::{Analysis, CallTable, EffectTable, Mutation, Tree};

/// Global reads, writes, address escapes, and initializer dependencies.
#[derive(Debug)]
pub struct GlobalAccessTable {}

impl GlobalAccessTable {
    /// Collect global accesses across the module's functions and initializers.
    pub fn analyse(_calls: &CallTable, _effects: &EffectTable, _tree: &Tree) -> Self {
        todo!("TODO #Incomplete: implement GlobalAccessTable")
    }
}

impl Analysis for GlobalAccessTable {
    const INVALIDATED_BY: Mutation = CallTable::INVALIDATED_BY
        .union(EffectTable::INVALIDATED_BY)
        .union(Mutation::SYMBOL)
        .union(Mutation::DROP);
}
