use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::{IndexMap, IndexSet};

use crate::check::{
    CheckState, Condition, Origin, StaticOperand, StaticTerm, TypeOperand, TypeRelation, TypeTerm,
    VariableId, VariableKind,
};

/// Check operands keyed by source identity.
#[derive(Debug)]
pub(in crate::check) struct OperandTable {
    /// Type operands keyed by checked source node.
    pub(in crate::check) node_types: IndexMap<dir::GlobalNodeIdAny, TypeOperand>,
    /// Type operands keyed by checked source symbol.
    pub(in crate::check) symbol_types: IndexMap<dir::GlobalSymbolId, TypeOperand>,
    /// Static operands keyed by checked source node.
    pub(in crate::check) node_statics: IndexMap<dir::GlobalNodeIdAny, StaticOperand>,
    /// Static operands keyed by checked source symbol.
    pub(in crate::check) symbol_statics: IndexMap<dir::GlobalSymbolId, StaticOperand>,
}

impl OperandTable {
    /// Create an empty operand table.
    pub(in crate::check) fn new() -> Self {
        Self {
            node_types: IndexMap::new(),
            symbol_types: IndexMap::new(),
            node_statics: IndexMap::new(),
            symbol_statics: IndexMap::new(),
        }
    }
}

/// Checked source identities that should be committed.
#[derive(Debug)]
pub(in crate::check) struct OutputTable {
    /// Nodes whose checked type should be committed.
    pub(in crate::check) node_types: IndexSet<dir::GlobalNodeIdAny>,
    /// Symbols whose checked type should be committed.
    pub(in crate::check) symbol_types: IndexSet<dir::GlobalSymbolId>,
    /// Nodes whose checked static should be committed.
    pub(in crate::check) node_statics: IndexSet<dir::GlobalNodeIdAny>,
    /// Symbols whose checked static should be committed.
    pub(in crate::check) symbol_statics: IndexSet<dir::GlobalSymbolId>,
}

impl OutputTable {
    /// Create an empty output table.
    pub(in crate::check) fn new() -> Self {
        Self {
            node_types: IndexSet::new(),
            symbol_types: IndexSet::new(),
            node_statics: IndexSet::new(),
            symbol_statics: IndexSet::new(),
        }
    }
}

impl CheckState<'_> {
    /// Output one checked type variable for a node.
    pub(in crate::check) fn output_node_type_variable(
        &mut self,
        module: ModuleId,
        node: dir::GlobalNodeIdAny,
    ) -> VariableId {
        if self.operands.node_types.contains_key(&node) {
            panic!("check node {node:?} already has checked type");
        }

        let variable = self.allocate_variable(module, VariableKind::Type, Origin::Node(node));
        self.operands
            .node_types
            .insert(node, TypeOperand::Variable(variable));
        self.outputs.node_types.insert(node);

        variable
    }

    /// Output one checked static variable for a node.
    pub(in crate::check) fn output_node_static_variable(
        &mut self,
        module: ModuleId,
        node: dir::GlobalNodeIdAny,
    ) -> VariableId {
        if self.operands.node_statics.contains_key(&node) {
            panic!("check node {node:?} already has checked static");
        }

        let variable = self.allocate_variable(module, VariableKind::Static, Origin::Node(node));
        self.operands
            .node_statics
            .insert(node, StaticOperand::Variable(variable));
        self.outputs.node_statics.insert(node);

        variable
    }

    /// Output one checked type variable for a symbol.
    pub(in crate::check) fn output_symbol_type_variable(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> VariableId {
        if self.operands.symbol_types.contains_key(&symbol) {
            panic!("check symbol {symbol:?} already has checked type");
        }

        let variable = self.allocate_variable(module, VariableKind::Type, Origin::Symbol(symbol));
        self.operands
            .symbol_types
            .insert(symbol, TypeOperand::Variable(variable));
        self.outputs.symbol_types.insert(symbol);

        variable
    }

    /// Return one output symbol type variable, creating it when missing.
    pub(in crate::check) fn ensure_symbol_type_output_variable(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> VariableId {
        match self.operands.symbol_types.get(&symbol).copied() {
            Some(TypeOperand::Variable(variable)) => variable,
            Some(TypeOperand::Term(_)) => {
                panic!("check symbol {symbol:?} already has checked type term")
            }
            None => self.output_symbol_type_variable(module, symbol),
        }
    }

    /// Output one checked static variable for a symbol.
    pub(in crate::check) fn output_symbol_static_variable(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> VariableId {
        if self.operands.symbol_statics.contains_key(&symbol) {
            panic!("check symbol {symbol:?} already has checked static");
        }

        let variable = self.allocate_variable(module, VariableKind::Static, Origin::Symbol(symbol));
        self.operands
            .symbol_statics
            .insert(symbol, StaticOperand::Variable(variable));
        self.outputs.symbol_statics.insert(symbol);

        variable
    }

    /// Output one node checked type under a guard.
    pub(in crate::check) fn output_node_type_guarded<T: dir::Node + Clone>(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<T>,
        term: TypeTerm,
        condition: Condition,
    ) -> TypeOperand {
        let node = id.into_global_any(module);
        if self.operands.node_types.contains_key(&node) {
            panic!("check node {node:?} already has checked type");
        }
        let operand = self.guard_type_term(module, Origin::Node(node), term, condition);

        self.operands.node_types.insert(node, operand);
        self.outputs.node_types.insert(node);

        operand
    }

    /// Output one node checked type operand under a guard.
    pub(in crate::check) fn output_node_type_operand_guarded<T: dir::Node + Clone>(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<T>,
        operand: TypeOperand,
        condition: Condition,
    ) -> TypeOperand {
        let node = id.into_global_any(module);
        if self.operands.node_types.contains_key(&node) {
            panic!("check node {node:?} already has checked type");
        }
        let operand = self.guard_type_operand(module, Origin::Node(node), operand, condition);

        self.operands.node_types.insert(node, operand);
        self.outputs.node_types.insert(node);

        operand
    }

    /// Output one symbol checked type under a guard.
    pub(in crate::check) fn output_symbol_type(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        term: TypeTerm,
        condition: Condition,
    ) -> TypeOperand {
        if let Some(operand) = self.operands.symbol_types.get(&symbol).copied() {
            return match operand {
                TypeOperand::Variable(variable) => {
                    self.equate_type(variable, term, condition);

                    operand
                }
                TypeOperand::Term(_) => {
                    panic!("check symbol {symbol:?} already has checked type term")
                }
            };
        }
        let operand = self.guard_type_term(module, Origin::Symbol(symbol), term, condition);

        self.operands.symbol_types.insert(symbol, operand);
        self.outputs.symbol_types.insert(symbol);

        operand
    }

    /// Output one symbol checked type operand under a guard.
    pub(in crate::check) fn output_symbol_type_operand(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        operand: TypeOperand,
        condition: Condition,
    ) -> TypeOperand {
        if let Some(existing) = self.operands.symbol_types.get(&symbol).copied() {
            return match existing {
                TypeOperand::Variable(variable) => {
                    let origin = Origin::Symbol(symbol);

                    self.relate_type(origin, TypeRelation::Equal, variable, operand, condition);

                    existing
                }
                TypeOperand::Term(_) => {
                    panic!("check symbol {symbol:?} already has checked type term")
                }
            };
        }
        let operand = self.guard_type_operand(module, Origin::Symbol(symbol), operand, condition);

        self.operands.symbol_types.insert(symbol, operand);
        self.outputs.symbol_types.insert(symbol);

        operand
    }

    /// Output one checked static variable for an expression.
    pub(in crate::check) fn output_static_expression_variable(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::Expression>,
        condition: Condition,
    ) -> VariableId {
        let node = id.into_global_any(module);
        let variable = self.output_node_static_variable(module, node);
        let term = StaticTerm::Expression(id.into_global(module));

        self.equate_static(variable, term, condition);

        variable
    }

    /// Return one checked local node type operand.
    pub(in crate::check) fn require_local_node_type<T: dir::Node + Clone>(
        &self,
        module: ModuleId,
        id: dir::LocalNodeId<T>,
    ) -> TypeOperand {
        let node = id.into_global_any(module);
        self.require_node_type(node)
    }

    /// Return one checked global node type operand.
    pub(in crate::check) fn require_node_type(&self, node: dir::GlobalNodeIdAny) -> TypeOperand {
        self.operands
            .node_types
            .get(&node)
            .copied()
            .unwrap_or_else(|| panic!("check node {node:?} has no checked type"))
    }

    /// Return one checked symbol type operand.
    pub(in crate::check) fn require_symbol_type(&self, symbol: dir::GlobalSymbolId) -> TypeOperand {
        *self
            .operands
            .symbol_types
            .get(&symbol)
            .unwrap_or_else(|| panic!("check symbol {symbol:?} has no checked type"))
    }

    /// Return one type operand as a type variable.
    pub(in crate::check) fn ensure_type_operand_variable(
        &mut self,
        module: ModuleId,
        origin: Origin,
        operand: TypeOperand,
        condition: Condition,
    ) -> VariableId {
        match operand {
            TypeOperand::Variable(variable) => variable,
            TypeOperand::Term(term) => {
                let variable = self.allocate_variable(module, VariableKind::Type, origin);
                let term = self.inference.terms.get(term).clone();

                self.equate_type(variable, term, condition);

                variable
            }
        }
    }

    /// Return one checked symbol static variable.
    pub(in crate::check) fn require_local_symbol_static_variable(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> VariableId {
        match self.operands.symbol_statics.get(&symbol).copied() {
            Some(StaticOperand::Variable(variable)) => variable,
            Some(StaticOperand::Term(_)) => {
                panic!("check symbol {symbol:?} has a static operand, not a solver slot")
            }
            None => panic!("check symbol {symbol:?} has no checked static"),
        }
    }

    /// Return one type term as a guarded operand.
    fn guard_type_term(
        &mut self,
        module: ModuleId,
        origin: Origin,
        term: TypeTerm,
        condition: Condition,
    ) -> TypeOperand {
        if condition == Condition::Always && term.is_stable(self) {
            let term = self.inference.terms.push(term);

            return TypeOperand::Term(term);
        }

        let variable = self.allocate_variable(module, VariableKind::Type, origin);

        self.equate_type(variable, term, condition);

        TypeOperand::Variable(variable)
    }

    /// Return one type operand under a guard.
    fn guard_type_operand(
        &mut self,
        module: ModuleId,
        origin: Origin,
        operand: TypeOperand,
        condition: Condition,
    ) -> TypeOperand {
        if condition == Condition::Always {
            return operand;
        }

        let variable = self.allocate_variable(module, VariableKind::Type, origin);

        self.relate_type(origin, TypeRelation::Equal, variable, operand, condition);

        TypeOperand::Variable(variable)
    }
}
