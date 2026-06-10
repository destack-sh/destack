use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    CheckState, Condition, FlowState, GenericArgument, Origin, StaticOperand, StaticTerm,
    TypeLiteralTerm, TypeOperand, TypeRelation, TypeTerm,
};
use crate::{CompilerError, CompilerResult};

/// State used only while walking one module.
pub(in crate::check) struct WalkState<'check, 'state> {
    /// The component check state being populated.
    pub(in crate::check) check: &'check mut CheckState<'state>,
    /// The visible DIR tree being walked.
    pub(in crate::check) tree: dir::View<'check>,
    /// The module being walked.
    pub(in crate::check) module: ModuleId,
    /// Flow state for the current module walk.
    flow: FlowState,
}

impl<'check, 'state> WalkState<'check, 'state> {
    /// Create walk state for one module.
    pub(in crate::check) fn new(
        module: ModuleId,
        tree: dir::View<'check>,
        check: &'check mut CheckState<'state>,
    ) -> Self {
        Self {
            check,
            tree,
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

    /// Return one node type operand, creating it when missing.
    pub(in crate::check) fn node_type_operand<T: dir::Node + Clone>(
        &mut self,
        id: dir::LocalNodeId<T>,
    ) -> CompilerResult<TypeOperand> {
        let node = id.into_global_any(self.module);
        if let Some(operand) = self.check.inputs.node_type(node) {
            return Ok(operand);
        }

        let variable = self
            .check
            .push_type_variable(self.module, Origin::Node(node));
        let operand = TypeOperand::Variable(variable);
        self.check.inputs.insert_node_type(node, operand)
    }

    /// Return the effective type operand for one value expression.
    pub(in crate::check) fn expression_type_operand(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<TypeOperand> {
        if let Some(narrowed) = self.flow_path_narrowing(id) {
            return Ok(narrowed);
        }

        self.node_type_operand(id)
    }

    /// Constrain one node to equal a type term.
    pub(in crate::check) fn constrain_node_type_term<T: dir::Node + Clone>(
        &mut self,
        id: dir::LocalNodeId<T>,
        term: TypeTerm,
    ) -> CompilerResult<TypeOperand> {
        let operand = self.type_term_operand(term);
        self.constrain_node_type(id, operand)
    }

    /// Set one node's own type term.
    pub(in crate::check) fn set_node_type_term<T: dir::Node + Clone>(
        &mut self,
        id: dir::LocalNodeId<T>,
        term: TypeTerm,
    ) -> CompilerResult<TypeOperand> {
        let operand = self.type_term_operand(term);
        self.set_node_type(id, operand)
    }

    /// Set one node's own type operand.
    pub(in crate::check) fn set_node_type<T: dir::Node + Clone>(
        &mut self,
        id: dir::LocalNodeId<T>,
        operand: TypeOperand,
    ) -> CompilerResult<TypeOperand> {
        let node = id.into_global_any(self.module);
        let Some(target) = self.check.inputs.node_type(node) else {
            return self.check.inputs.insert_node_type(node, operand);
        };

        match target {
            TypeOperand::Variable(_) => {
                let origin = Origin::Node(node);
                let condition = self.active_static_guard();
                self.check
                    .constrain_type(origin, TypeRelation::Equal, target, operand, condition);

                Ok(target)
            }
            TypeOperand::Term(_) | TypeOperand::Type(_) if target == operand => Ok(target),
            TypeOperand::Term(_) | TypeOperand::Type(_) => Err(CompilerError::Internal {
                message: format!("check node {node:?} already has a different type operand"),
            }),
        }
    }

    /// Constrain one node to equal a type operand.
    pub(in crate::check) fn constrain_node_type<T: dir::Node + Clone>(
        &mut self,
        id: dir::LocalNodeId<T>,
        operand: TypeOperand,
    ) -> CompilerResult<TypeOperand> {
        let node = id.into_global_any(self.module);
        let origin = Origin::Node(node);
        let condition = self.active_static_guard();
        let target = self.node_type_operand(id)?;
        self.check
            .constrain_type(origin, TypeRelation::Equal, target, operand, condition);

        Ok(target)
    }

    /// Return a reference to one well-known library type.
    pub(in crate::check) fn language_type_reference(
        &mut self,
        source: dir::GlobalNodeIdAny,
        item: dir::LanguageItem,
        arguments: Vec<GenericArgument>,
    ) -> TypeTerm {
        let symbol = self.check.language_symbol(item);

        TypeTerm::Reference {
            origin: Origin::Node(source),
            symbol,
            arguments: arguments.into_iter().collect(),
        }
    }

    /// Return a value type with `undefined` included.
    pub(in crate::check) fn optional_value_type(&mut self, operand: TypeOperand) -> TypeOperand {
        let undefined = self
            .check
            .inference
            .push_term(TypeTerm::Literal(TypeLiteralTerm::Undefined));
        let term = TypeTerm::Union {
            elements: vec![operand, undefined.into()],
        };
        let term = self.check.inference.push_term(term);

        term.into()
    }

    /// Return one symbol type operand, creating it when missing.
    pub(in crate::check) fn symbol_type_operand(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<TypeOperand> {
        if let Some(operand) = self.check.inputs.symbol_type(symbol) {
            return Ok(operand);
        }

        // require imported operands to be available at the boundary
        if !self.check.is_component_module(symbol.module_id) {
            return Err(CompilerError::Internal {
                message: format!("external symbol {symbol:?} has no imported type operand"),
            });
        }

        if self
            .check
            .module(symbol.module_id)
            .is_import_alias(symbol.local_id)
        {
            return Err(CompilerError::Internal {
                message: format!("import alias {symbol:?} reached type operand creation"),
            });
        }

        // create source declaration operands on first use
        let variable = self
            .check
            .push_type_variable(symbol.module_id, Origin::Symbol(symbol));
        let operand = TypeOperand::Variable(variable);
        self.check.inputs.insert_symbol_type(symbol, operand)
    }

    /// Constrain one symbol to equal a type term.
    pub(in crate::check) fn constrain_symbol_type_term(
        &mut self,
        symbol: dir::GlobalSymbolId,
        term: TypeTerm,
        condition: Condition,
    ) -> CompilerResult<TypeOperand> {
        if let Some(existing) = self.check.inputs.symbol_type(symbol) {
            let origin = Origin::Symbol(symbol);
            let term = self.type_term_operand(term);
            self.check
                .constrain_type(origin, TypeRelation::Equal, existing, term, condition);

            return Ok(existing);
        }

        let operand = self.type_term_operand(term);
        self.check.inputs.insert_symbol_type(symbol, operand)
    }

    /// Constrain one symbol to equal a type operand.
    pub(in crate::check) fn constrain_symbol_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        operand: TypeOperand,
        condition: Condition,
    ) -> CompilerResult<TypeOperand> {
        if let Some(existing) = self.check.inputs.symbol_type(symbol) {
            let origin = Origin::Symbol(symbol);
            self.check
                .constrain_type(origin, TypeRelation::Equal, existing, operand, condition);

            return Ok(existing);
        }

        self.check.inputs.insert_symbol_type(symbol, operand)
    }

    /// Return one node static operand, creating it when missing.
    pub(in crate::check) fn node_static_operand<T: dir::Node + Clone>(
        &mut self,
        id: dir::LocalNodeId<T>,
    ) -> CompilerResult<StaticOperand> {
        let node = id.into_global_any(self.module);
        if let Some(operand) = self.check.inputs.node_static(node) {
            return Ok(operand);
        }

        let variable = self
            .check
            .push_static_variable(self.module, Origin::Node(node));
        let operand = StaticOperand::Variable(variable);
        self.check.inputs.insert_node_static(node, operand)
    }

    /// Constrain one node to equal a static term.
    pub(in crate::check) fn constrain_node_static<T: dir::Node + Clone>(
        &mut self,
        id: dir::LocalNodeId<T>,
        term: StaticTerm,
        condition: Condition,
    ) -> CompilerResult<StaticOperand> {
        let node = id.into_global_any(self.module);
        let origin = Origin::Node(node);
        let target = self.node_static_operand(id)?;
        let term = self.check.inference.push_term(term);
        self.check.equate_static(origin, target, term, condition);

        Ok(target)
    }

    /// Set one node's own static operand.
    pub(in crate::check) fn set_node_static<T: dir::Node + Clone>(
        &mut self,
        id: dir::LocalNodeId<T>,
        operand: StaticOperand,
    ) -> CompilerResult<StaticOperand> {
        let node = id.into_global_any(self.module);
        self.check.inputs.insert_node_static(node, operand)
    }

    /// Return one symbol static operand, creating it when missing.
    pub(in crate::check) fn symbol_static_operand(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<StaticOperand> {
        if let Some(operand) = self.check.inputs.symbol_static(symbol) {
            return Ok(operand);
        }

        if !self.check.is_component_module(symbol.module_id) {
            return Err(CompilerError::Internal {
                message:
                    "check static variable creation requires a source symbol in the checked component"
                        .to_owned(),
            });
        }

        if self
            .check
            .module(symbol.module_id)
            .is_import_alias(symbol.local_id)
        {
            return Err(CompilerError::Internal {
                message: format!("import alias {symbol:?} reached static operand creation"),
            });
        }

        // create source declaration operands on first use
        let variable = self
            .check
            .push_static_variable(symbol.module_id, Origin::Symbol(symbol));
        let operand = StaticOperand::Variable(variable);
        self.check.inputs.insert_symbol_static(symbol, operand)
    }

    /// Constrain one symbol to equal a static term.
    pub(in crate::check) fn constrain_symbol_static(
        &mut self,
        symbol: dir::GlobalSymbolId,
        term: StaticTerm,
        condition: Condition,
    ) -> CompilerResult<StaticOperand> {
        if let Some(existing) = self.check.inputs.symbol_static(symbol) {
            let origin = Origin::Symbol(symbol);
            let term = self.check.inference.push_term(term);
            self.check.equate_static(origin, existing, term, condition);

            return Ok(existing);
        }

        let operand = StaticOperand::Term(self.check.inference.push_term(term));
        self.check.inputs.insert_symbol_static(symbol, operand)
    }

    /// Return one static expression operand, creating it when missing.
    pub(in crate::check) fn static_expression_operand(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        condition: Condition,
    ) -> CompilerResult<StaticOperand> {
        let term = StaticTerm::Expression(id.into_global(self.module));
        self.constrain_node_static(id, term, condition)
    }

    /// Intern one type term as an operand.
    fn type_term_operand(&mut self, term: TypeTerm) -> TypeOperand {
        self.check.type_term_operand(term)
    }
}
