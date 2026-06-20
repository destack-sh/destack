use std::collections::HashMap;

use destack_mir as mir;

use super::{FunctionId, TypeId};
use crate::StaticId;

/// Dense program ids assigned to one MIR tree.
#[derive(Debug, Clone)]
pub struct ProgramIndex {
    /// Program function ids keyed by MIR function id.
    function_ids: HashMap<mir::LocalNodeId<mir::Function>, FunctionId>,
    /// Program type ids keyed by MIR type id.
    type_ids: HashMap<mir::LocalNodeId<mir::Type>, TypeId>,
    /// Program static ids keyed by MIR global id.
    static_ids: HashMap<mir::LocalNodeId<mir::Global>, StaticId>,
}

impl ProgramIndex {
    /// Assign dense program ids for one MIR tree.
    pub fn new(tree: &mir::Tree) -> Self {
        let function_ids = tree
            .iter_nodes::<mir::Function>()
            .enumerate()
            .map(|(index, (function, _))| (function, FunctionId::from(index as u32)))
            .collect();
        let type_ids = tree
            .iter_nodes::<mir::Type>()
            .enumerate()
            .map(|(index, (ty, _))| (ty, TypeId::from(index as u32)))
            .collect();
        let static_ids = tree
            .iter_nodes::<mir::Global>()
            .enumerate()
            .map(|(index, (global, _))| (global, StaticId::from(index as u32)))
            .collect();

        Self {
            function_ids,
            type_ids,
            static_ids,
        }
    }

    /// Return the program function id for one MIR function.
    pub fn function_id(&self, function: mir::LocalNodeId<mir::Function>) -> FunctionId {
        self.function_ids[&function]
    }

    /// Return the program type id for one MIR type.
    pub fn type_id(&self, ty: mir::LocalNodeId<mir::Type>) -> TypeId {
        self.type_ids[&ty]
    }

    /// Return the program static id for one MIR global.
    pub fn static_id(&self, global: mir::LocalNodeId<mir::Global>) -> StaticId {
        self.static_ids[&global]
    }
}
