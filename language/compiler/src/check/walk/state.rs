use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    CheckState, Condition, FlowState, Origin, StaticOperand, StaticTerm, TypeOperand, TypeRelation,
    TypeTerm, VariableId,
};

/// Source node whose type can be read or allocated during walk.
pub(in crate::check) trait NodeTypeSource {
    /// Return this source node as a global DIR node id.
    fn into_global_node(self, module: ModuleId) -> dir::GlobalNodeIdAny;
}

impl<T: dir::Node + Clone> NodeTypeSource for dir::LocalNodeId<T> {
    /// Return this local source node as a global DIR node id.
    fn into_global_node(self, module: ModuleId) -> dir::GlobalNodeIdAny {
        self.into_global_any(module)
    }
}

impl NodeTypeSource for dir::GlobalNodeIdAny {
    /// Return this global source node unchanged.
    fn into_global_node(self, _module: ModuleId) -> dir::GlobalNodeIdAny {
        self
    }
}

/// State used only while walking one module.
pub(in crate::check) struct WalkState<'check, 'state> {
    /// The component check state being populated.
    pub(in crate::check) check: &'check mut CheckState<'state>,
    /// The module being walked.
    pub(in crate::check) module: ModuleId,
    /// Flow state for the current module walk.
    flow: FlowState,
}

impl<'check, 'state> WalkState<'check, 'state> {
    /// Create walk state for one module.
    pub(in crate::check) fn new(module: ModuleId, check: &'check mut CheckState<'state>) -> Self {
        Self {
            check,
            module,
            flow: FlowState::default(),
        }
    }

    /// Return flow state for the active module.
    pub(in crate::check) fn flow(&self) -> &FlowState {
        &self.flow
    }

    /// Return mutable flow state for the active module.
    pub(in crate::check) fn flow_mut(&mut self) -> &mut FlowState {
        &mut self.flow
    }

    /// Allocate one node type operand when missing.
    pub(in crate::check) fn allocate_node_type_operand<S: NodeTypeSource>(
        &mut self,
        source: S,
    ) -> TypeOperand {
        let node = source.into_global_node(self.module);
        if let Some(operand) = self.check.inputs.node_type(node) {
            return operand;
        }

        let variable = self
            .check
            .create_type_variable(self.module, Origin::Node(node));
        let operand = TypeOperand::Variable(variable);

        self.check.inputs.insert_node_type(node, operand)
    }

    /// Allocate one node type variable when missing.
    pub(in crate::check) fn allocate_node_type_variable<S: NodeTypeSource>(
        &mut self,
        source: S,
    ) -> VariableId {
        let node = source.into_global_node(self.module);

        match self.allocate_node_type_operand(node) {
            TypeOperand::Variable(variable) => variable,
            TypeOperand::Term(_) | TypeOperand::Type(_) => {
                panic!("check node {node:?} has a type operand, not an inference variable")
            }
        }
    }

    /// Bind one node type term.
    pub(in crate::check) fn bind_node_type<T: dir::Node + Clone>(
        &mut self,
        id: dir::LocalNodeId<T>,
        term: TypeTerm,
    ) -> TypeOperand {
        let node = id.into_global_any(self.module);
        let operand = self.type_term_operand(Origin::Node(node), term);

        self.check.inputs.insert_node_type(node, operand)
    }

    /// Bind one node type operand.
    pub(in crate::check) fn bind_node_type_operand<T: dir::Node + Clone>(
        &mut self,
        id: dir::LocalNodeId<T>,
        operand: TypeOperand,
    ) -> TypeOperand {
        let node = id.into_global_any(self.module);

        self.check.inputs.insert_node_type(node, operand)
    }

    /// Allocate one symbol type operand when missing.
    pub(in crate::check) fn allocate_symbol_type_operand(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> TypeOperand {
        if let Some(operand) = self.check.inputs.symbol_type(symbol) {
            return operand;
        }

        // delegate dependency symbols to component state
        if symbol.module_id != self.module {
            return self.check.import_symbol_type_operand(self.module, symbol);
        }

        if self
            .check
            .module(self.module)
            .is_import_alias(symbol.local_id)
        {
            panic!("import alias {symbol:?} reached type operand allocation");
        }

        // allocate walked local declarations on first use
        let variable = self
            .check
            .create_type_variable(self.module, Origin::Symbol(symbol));
        let operand = TypeOperand::Variable(variable);

        self.check.inputs.insert_symbol_type(symbol, operand)
    }

    /// Allocate one symbol type variable when missing.
    pub(in crate::check) fn allocate_symbol_type_variable(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> VariableId {
        if symbol.module_id != self.module {
            panic!("check type variable allocation requires a source symbol in the active module");
        }

        match self.allocate_symbol_type_operand(symbol) {
            TypeOperand::Variable(variable) => variable,
            TypeOperand::Term(_) | TypeOperand::Type(_) => {
                panic!("check symbol {symbol:?} has a type operand, not an inference variable")
            }
        }
    }

    /// Bind one symbol type term.
    pub(in crate::check) fn bind_symbol_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        term: TypeTerm,
        condition: Condition,
    ) -> TypeOperand {
        if let Some(existing) = self.check.inputs.symbol_type(symbol) {
            return match existing {
                TypeOperand::Variable(variable) => {
                    self.check.equate_type(variable, term, condition);

                    existing
                }
                TypeOperand::Term(_) | TypeOperand::Type(_) => {
                    panic!("check symbol {symbol:?} already has a type operand")
                }
            };
        }

        let operand = self.type_term_operand(Origin::Symbol(symbol), term);

        self.check.inputs.insert_symbol_type(symbol, operand)
    }

    /// Bind one symbol type operand.
    pub(in crate::check) fn bind_symbol_type_operand(
        &mut self,
        symbol: dir::GlobalSymbolId,
        operand: TypeOperand,
        condition: Condition,
    ) -> TypeOperand {
        if let Some(existing) = self.check.inputs.symbol_type(symbol) {
            return match existing {
                TypeOperand::Variable(variable) => {
                    let origin = Origin::Symbol(symbol);

                    self.check.relate_type(
                        origin,
                        TypeRelation::Equal,
                        variable,
                        operand,
                        condition,
                    );

                    existing
                }
                TypeOperand::Term(_) | TypeOperand::Type(_) => {
                    panic!("check symbol {symbol:?} already has a type operand")
                }
            };
        }

        self.check.inputs.insert_symbol_type(symbol, operand)
    }

    /// Allocate one node static operand when missing.
    pub(in crate::check) fn allocate_node_static_operand<T: dir::Node + Clone>(
        &mut self,
        id: dir::LocalNodeId<T>,
    ) -> StaticOperand {
        let node = id.into_global_any(self.module);
        if let Some(operand) = self.check.inputs.node_static(node) {
            return operand;
        }

        let variable = self
            .check
            .create_static_variable(self.module, Origin::Node(node));
        let operand = StaticOperand::Variable(variable);

        self.check.inputs.insert_node_static(node, operand)
    }

    /// Allocate one symbol static variable when missing.
    pub(in crate::check) fn allocate_symbol_static_variable(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> VariableId {
        if let Some(operand) = self.check.inputs.symbol_static(symbol) {
            return match operand {
                StaticOperand::Variable(variable) => variable,
                StaticOperand::Term(_) | StaticOperand::Static(_) => {
                    panic!(
                        "check symbol {symbol:?} has a static operand, not an inference variable"
                    )
                }
            };
        }

        if symbol.module_id != self.module {
            panic!(
                "check static variable allocation requires a source symbol in the active module"
            );
        }

        if self
            .check
            .module(self.module)
            .is_import_alias(symbol.local_id)
        {
            panic!("import alias {symbol:?} reached static operand allocation");
        }

        // allocate walked local declarations on first use
        let variable = self
            .check
            .create_static_variable(self.module, Origin::Symbol(symbol));
        let operand = StaticOperand::Variable(variable);

        self.check.inputs.insert_symbol_static(symbol, operand);

        variable
    }

    /// Bind one symbol static term.
    pub(in crate::check) fn bind_symbol_static(
        &mut self,
        symbol: dir::GlobalSymbolId,
        term: StaticTerm,
        condition: Condition,
    ) -> StaticOperand {
        if let Some(existing) = self.check.inputs.symbol_static(symbol) {
            return match existing {
                StaticOperand::Variable(variable) => {
                    let origin = Origin::Symbol(symbol);
                    let term = self.check.inference.push_term(term);

                    self.check.equate_static(origin, variable, term, condition);

                    existing
                }
                StaticOperand::Term(_) | StaticOperand::Static(_) => {
                    panic!("check symbol {symbol:?} already has a static operand")
                }
            };
        }

        let operand = StaticOperand::Term(self.check.inference.push_term(term));

        self.check.inputs.insert_symbol_static(symbol, operand)
    }

    /// Allocate one static expression variable.
    pub(in crate::check) fn allocate_static_expression_variable(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        condition: Condition,
    ) -> VariableId {
        let node = id.into_global_any(self.module);
        let origin = Origin::Node(node);
        let variable = match self.allocate_node_static_operand(id) {
            StaticOperand::Variable(variable) => variable,
            StaticOperand::Term(_) | StaticOperand::Static(_) => {
                panic!("check node {node:?} has a static operand, not an inference variable")
            }
        };
        let term = StaticTerm::Expression(id.into_global(self.module));
        let term = self.check.inference.push_term(term);

        self.check.equate_static(origin, variable, term, condition);

        variable
    }

    /// Return a type variable constrained by one operand.
    pub(in crate::check) fn type_variable_for_operand(
        &mut self,
        origin: Origin,
        operand: TypeOperand,
        condition: Condition,
    ) -> VariableId {
        self.check
            .type_variable_for_operand(self.module, origin, operand, condition)
    }

    /// Create one operand for a type term.
    fn type_term_operand(&mut self, origin: Origin, term: TypeTerm) -> TypeOperand {
        if term.is_stable(self.check) {
            let term = self.check.inference.push_term(term);

            return TypeOperand::Term(term);
        }

        let variable = self.check.create_type_variable(self.module, origin);

        self.check.equate_type(variable, term, Condition::Always);

        TypeOperand::Variable(variable)
    }
}
