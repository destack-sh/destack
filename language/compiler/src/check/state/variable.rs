use destack_source::ModuleId;

use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    BoundSide, CheckEvent, CheckState, Condition, Origin, StaticOperand, StaticTerm, TraceOperand,
    TypeOperand,
};

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
    /// Open one type inference variable.
    pub(in crate::check) fn create_type_variable(
        &mut self,
        module: ModuleId,
        source: Origin,
    ) -> VariableId {
        self.push_variable(module, VariableKind::Type, source)
    }

    /// Open one static inference variable.
    pub(in crate::check) fn create_static_variable(
        &mut self,
        module: ModuleId,
        source: Origin,
    ) -> VariableId {
        self.push_variable(module, VariableKind::Static, source)
    }

    /// Push one solver variable.
    pub(in crate::check::state) fn push_variable(
        &mut self,
        module: ModuleId,
        kind: VariableKind,
        source: Origin,
    ) -> VariableId {
        if source.module() != module {
            unreachable!("inference variable source must be local");
        }

        let id = VariableId::new(module, self.inference.variable_count() as u32);
        let variable = Variable::new(id, kind, source);
        self.inference.push_variable(variable);

        id
    }

    /// Create one static expression variable without binding a node operand.
    pub(in crate::check) fn create_static_expression_variable(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::Expression>,
        condition: Condition,
    ) -> VariableId {
        let source = id.into_global_any(module);
        let origin = Origin::Node(source);
        let variable = self.create_static_variable(module, origin);
        let term = StaticTerm::Expression(id.into_global(module));
        let term = self.inference.push_term(term);

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
        let inserted = self
            .inference
            .insert_lower_type_bound(variable, lower_bound);
        if inserted {
            self.trace.record(CheckEvent::BoundInsert {
                variable,
                kind: VariableKind::Type,
                side: BoundSide::Lower,
                value: TraceOperand::Type(lower_bound),
            });
        }

        Ok(inserted)
    }

    /// Insert one upper type bound for a variable.
    pub(in crate::check) fn insert_upper_type_bound(
        &mut self,
        variable: VariableId,
        upper_bound: TypeOperand,
    ) -> CompilerResult<bool> {
        let inserted = self
            .inference
            .insert_upper_type_bound(variable, upper_bound);
        if inserted {
            self.trace.record(CheckEvent::BoundInsert {
                variable,
                kind: VariableKind::Type,
                side: BoundSide::Upper,
                value: TraceOperand::Type(upper_bound),
            });
        }

        Ok(inserted)
    }

    /// Insert one lower static bound for a variable.
    pub(in crate::check) fn insert_lower_static_bound(
        &mut self,
        variable: VariableId,
        lower_bound: StaticOperand,
    ) -> CompilerResult<bool> {
        let inserted = self
            .inference
            .insert_lower_static_bound(variable, lower_bound);
        if inserted {
            self.trace.record(CheckEvent::BoundInsert {
                variable,
                kind: VariableKind::Static,
                side: BoundSide::Lower,
                value: TraceOperand::Static(lower_bound),
            });
        }

        Ok(inserted)
    }

    /// Insert one upper static bound for a variable.
    pub(in crate::check) fn insert_upper_static_bound(
        &mut self,
        variable: VariableId,
        upper_bound: StaticOperand,
    ) -> CompilerResult<bool> {
        let inserted = self
            .inference
            .insert_upper_static_bound(variable, upper_bound);
        if inserted {
            self.trace.record(CheckEvent::BoundInsert {
                variable,
                kind: VariableKind::Static,
                side: BoundSide::Upper,
                value: TraceOperand::Static(upper_bound),
            });
        }

        Ok(inserted)
    }

    /// Return one variable's source node for diagnostics.
    pub(in crate::check) fn variable_source_node(&self, id: VariableId) -> dir::LocalNodeIdAny {
        match self.variable(id).source {
            Origin::Node(node) => {
                if node.module_id != id.module {
                    unreachable!("variable source node must be local");
                }

                node.local_id
            }
            Origin::Symbol(symbol) => {
                if symbol.module_id != id.module {
                    unreachable!("variable source symbol must be local");
                }

                self.module(symbol.module_id)
                    .symbol_declaration_node(symbol.local_id)
            }
        }
    }
}
