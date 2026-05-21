use destack_dir as dir;

use super::{CheckModuleState, TypeInferId};

impl CheckModuleState {
    /// Return the symbol introduced by a source declaration node.
    pub(in crate::check) fn symbol_for_declaration(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        self.binding_table()
            .symbol_for_declaration(node.into_global(self.module()))
            .map(|symbol| symbol.into_global(self.module()))
    }

    /// Return the symbol introduced by a simple binding pattern.
    pub(in crate::check) fn pattern_symbol(
        &self,
        pattern: dir::LocalNodeId<dir::Pattern>,
    ) -> Option<dir::GlobalSymbolId> {
        self.symbol_for_declaration(pattern.into_any())
    }

    /// Return the inference id for one argument expression.
    pub(in crate::check) fn argument_infer(
        &mut self,
        argument: dir::LocalNodeId<dir::Argument>,
    ) -> Option<TypeInferId> {
        let value = self.parsed().tree.get(argument).value()?;
        let node = value.into_global_any(self.module());

        Some(self.infer_node(node))
    }

    /// Return the inference id for a simple assignment target.
    pub(in crate::check) fn assignment_target_infer(
        &mut self,
        pattern: dir::LocalNodeId<dir::AssignPattern>,
    ) -> Option<TypeInferId> {
        match self.parsed().tree.get(pattern) {
            dir::AssignPattern::Expression { value } => {
                Some(self.infer_node(value.into_global_any(self.module())))
            }
            dir::AssignPattern::Assign { pattern, .. } => self.assignment_target_infer(*pattern),
            dir::AssignPattern::Sequence { .. } | dir::AssignPattern::Object { .. } => None,
        }
    }
}
