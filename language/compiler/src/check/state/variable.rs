use destack_source::ModuleId;

use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{CheckState, Condition, Origin, StaticOperand, StaticTerm, TypeOperand};

/// Component-valid id for one check variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(in crate::check) struct VariableId {
    /// The module that produced the variable.
    pub(in crate::check) module: ModuleId,
    /// The variable index inside the checked component.
    pub(in crate::check) index: u32,
}

impl VariableId {
    /// Create one variable id.
    pub(in crate::check) fn new(module: ModuleId, index: u32) -> Self {
        Self { module, index }
    }
}

/// One solver variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct Variable {
    /// The variable id.
    pub(in crate::check) id: VariableId,
    /// The variable kind.
    pub(in crate::check) kind: VariableKind,
    /// The source that produced the variable.
    pub(in crate::check) source: Origin,
}

impl Variable {
    /// Create one unsolved variable.
    pub(in crate::check) fn new(id: VariableId, kind: VariableKind, source: Origin) -> Self {
        Self { id, kind, source }
    }
}

/// The value space of one check variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum VariableKind {
    /// Type variable.
    Type,
    /// Static value variable.
    Static,
}

impl CheckState<'_> {
    /// Allocate one solver variable.
    pub(in crate::check) fn allocate_variable(
        &mut self,
        module: ModuleId,
        kind: VariableKind,
        source: Origin,
    ) -> VariableId {
        assert_eq!(
            source.module(),
            module,
            "check inference variable source must be local"
        );

        let id = VariableId::new(module, self.inference.variable_count() as u32);
        let variable = Variable::new(id, kind, source);
        self.inference.push_variable(variable);

        id
    }

    /// Allocate one static expression variable without committing a checked node operand.
    pub(in crate::check) fn allocate_static_expression_variable(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::Expression>,
        condition: Condition,
    ) -> VariableId {
        let source = id.into_global_any(module);
        let origin = Origin::Node(source);
        let variable = self.allocate_variable(module, VariableKind::Static, origin);
        let term = StaticTerm::Expression(id.into_global(module));
        let term = self.push_term(term);

        self.equate_static(origin, variable, term, condition);

        variable
    }

    /// Return the number of allocated variables.
    pub(in crate::check) fn variable_count(&self) -> usize {
        self.inference.variable_count()
    }

    /// Return one variable.
    pub(in crate::check) fn variable(&self, id: VariableId) -> &Variable {
        self.inference.variable_at(id.index as usize)
    }

    /// Return one variable by allocation index.
    pub(in crate::check) fn variable_at(&self, index: usize) -> &Variable {
        self.inference.variable_at(index)
    }

    /// Return the source symbol for one variable.
    pub(in crate::check) fn variable_source_symbol(
        &self,
        id: VariableId,
    ) -> Option<dir::GlobalSymbolId> {
        match self.variable(id).source {
            Origin::Symbol(symbol) => Some(symbol),
            Origin::Node(_) => None,
        }
    }

    /// Insert one lower type bound for a variable.
    pub(in crate::check) fn insert_lower_type_bound(
        &mut self,
        variable: VariableId,
        lower_bound: TypeOperand,
    ) -> CompilerResult<bool> {
        Ok(self
            .inference
            .insert_lower_type_bound(variable, lower_bound))
    }

    /// Insert one upper type bound for a variable.
    pub(in crate::check) fn insert_upper_type_bound(
        &mut self,
        variable: VariableId,
        upper_bound: TypeOperand,
    ) -> CompilerResult<bool> {
        Ok(self
            .inference
            .insert_upper_type_bound(variable, upper_bound))
    }

    /// Insert one lower static bound for a variable.
    pub(in crate::check) fn insert_lower_static_bound(
        &mut self,
        variable: VariableId,
        lower_bound: StaticOperand,
    ) -> CompilerResult<bool> {
        Ok(self
            .inference
            .insert_lower_static_bound(variable, lower_bound))
    }

    /// Insert one upper static bound for a variable.
    pub(in crate::check) fn insert_upper_static_bound(
        &mut self,
        variable: VariableId,
        upper_bound: StaticOperand,
    ) -> CompilerResult<bool> {
        Ok(self
            .inference
            .insert_upper_static_bound(variable, upper_bound))
    }

    /// Return lower type bounds for one variable.
    pub(in crate::check) fn lower_type_bounds(&self, variable: VariableId) -> Vec<TypeOperand> {
        self.inference.lower_type_bounds(variable)
    }

    /// Return upper type bounds for one variable.
    pub(in crate::check) fn upper_type_bounds(&self, variable: VariableId) -> Vec<TypeOperand> {
        self.inference.upper_type_bounds(variable)
    }

    /// Return lower static bounds for one variable.
    pub(in crate::check) fn lower_static_bounds(&self, variable: VariableId) -> Vec<StaticOperand> {
        self.inference.lower_static_bounds(variable)
    }

    /// Return upper static bounds for one variable.
    pub(in crate::check) fn upper_static_bounds(&self, variable: VariableId) -> Vec<StaticOperand> {
        self.inference.upper_static_bounds(variable)
    }

    /// Return one variable's source node for diagnostics.
    pub(in crate::check) fn variable_source_node(&self, id: VariableId) -> dir::LocalNodeIdAny {
        match self.variable(id).source {
            Origin::Node(node) => {
                assert_eq!(
                    node.module_id, id.module,
                    "check variable source node must be local"
                );

                node.local_id
            }
            Origin::Symbol(symbol) => {
                assert_eq!(
                    symbol.module_id, id.module,
                    "check variable source symbol must be local"
                );

                self.symbol_source_node(symbol)
            }
        }
    }
}
