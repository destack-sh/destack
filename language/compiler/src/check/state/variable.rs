use destack_source::ModuleId;

use destack_dir as dir;
use indexmap::IndexMap;

use crate::check::{
    CheckState, Constraint, ConstraintOrigin, Definition, Obligation, StaticTerm, TermId, TypeTerm,
};

use super::{GenericArgumentKey, GenericParameter};

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

/// One check variable.
///
/// ```ts
/// let value = input.name;
/// ```
///
/// The checker creates variables for the binding symbol `value`, the expression node `input.name`,
/// and any static or type operations needed to solve the expression.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct Variable {
    /// The variable id.
    pub(in crate::check) id: VariableId,
    /// The variable kind.
    pub(in crate::check) kind: VariableKind,
    /// The source that produced the variable.
    pub(in crate::check) source: ConstraintOrigin,
    /// The committed output that receives this variable when solved.
    pub(in crate::check) output: Option<VariableOutput>,
}

/// Variable relation bounds collected by the solver.
#[derive(Debug)]
pub(in crate::check) struct VariableBounds {
    /// Variables that must be assignable to this variable.
    pub(in crate::check) lower: IndexMap<VariableId, Vec<VariableId>>,
    /// Variables that this variable must be assignable to.
    pub(in crate::check) upper: IndexMap<VariableId, Vec<VariableId>>,
}

/// Variables and variable indexes for one checked component.
#[derive(Debug)]
pub(in crate::check) struct VariableTable {
    /// Whether new variables may still be allocated.
    pub(in crate::check) is_open: bool,
    /// Check variables in allocation order.
    pub(in crate::check) variables: Vec<Variable>,
    /// Variable definitions in collection order.
    pub(in crate::check) definitions: Vec<Definition>,
    /// Variable constraints in collection order.
    pub(in crate::check) constraints: Vec<Constraint>,
    /// Check obligations in collection order.
    pub(in crate::check) obligations: Vec<Obligation>,

    /// Type variables keyed by source node.
    pub(in crate::check) type_by_node: IndexMap<dir::GlobalNodeIdAny, VariableId>,
    /// Type variables keyed by source symbol.
    pub(in crate::check) type_by_symbol: IndexMap<dir::GlobalSymbolId, VariableId>,
    /// Static variables keyed by source node.
    pub(in crate::check) static_by_node: IndexMap<dir::GlobalNodeIdAny, VariableId>,
    /// Static variables keyed by source symbol.
    pub(in crate::check) static_by_symbol: IndexMap<dir::GlobalSymbolId, VariableId>,
    /// Materialized type variables keyed by visible checked type id.
    pub(in crate::check) type_by_id: IndexMap<dir::GlobalTypeId, VariableId>,
    /// Materialized static variables keyed by visible checked static id.
    pub(in crate::check) static_by_id: IndexMap<dir::GlobalStaticId, VariableId>,
    /// Inferred generic arguments keyed by source and slot.
    pub(in crate::check) generic_argument: IndexMap<GenericArgumentKey, VariableId>,
    /// Next generic slot index keyed by owner.
    pub(in crate::check) generic_slot_index: IndexMap<dir::GlobalSymbolId, dir::GenericSlotIndex>,
    /// Generic parameter variables keyed by owning symbol.
    pub(in crate::check) generic_parameter_by_owner: IndexMap<dir::GlobalSymbolId, Vec<VariableId>>,
}

impl VariableTable {
    /// Create empty variable state.
    pub(in crate::check) fn new() -> Self {
        Self {
            is_open: true,
            variables: Vec::new(),
            definitions: Vec::new(),
            constraints: Vec::new(),
            obligations: Vec::new(),
            type_by_node: IndexMap::new(),
            type_by_symbol: IndexMap::new(),
            static_by_node: IndexMap::new(),
            static_by_symbol: IndexMap::new(),
            type_by_id: IndexMap::new(),
            static_by_id: IndexMap::new(),
            generic_argument: IndexMap::new(),
            generic_slot_index: IndexMap::new(),
            generic_parameter_by_owner: IndexMap::new(),
        }
    }

    /// Close the variable graph before solving.
    pub(in crate::check) fn close(&mut self) {
        self.is_open = false;
    }
}

impl VariableBounds {
    /// Create empty variable bounds.
    pub(in crate::check) fn new() -> Self {
        Self {
            lower: IndexMap::new(),
            upper: IndexMap::new(),
        }
    }
}

impl Variable {
    /// Create one unsolved variable.
    pub(in crate::check) fn new(
        id: VariableId,
        kind: VariableKind,
        source: ConstraintOrigin,
        output: Option<VariableOutput>,
    ) -> Self {
        Self {
            id,
            kind,
            source,
            output,
        }
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

/// Solved value for one check variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Solution {
    /// Solved type term.
    Type(TermId<TypeTerm>),
    /// Solved static term.
    Static(TermId<StaticTerm>),
}

/// Output target attached to one variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum VariableOutput {
    /// Variable recorded as one generic slot.
    ///
    /// ```ts
    /// function first<T>(value: &T): &T {
    ///     return value;
    /// }
    /// ```
    ///
    /// Explicit `T` and induced lifetime slots are represented as generic parameters.
    Generic(GenericParameter),

    /// Variable attached to a source node.
    ///
    /// ```ts
    /// value.name
    /// ```
    ///
    /// The member expression node has its own type variable.
    Node(dir::GlobalNodeIdAny),

    /// Variable attached to a source symbol.
    ///
    /// ```ts
    /// let value = 1;
    /// ```
    ///
    /// The binding symbol `value` has its own type variable.
    Symbol(dir::GlobalSymbolId),
}

impl VariableOutput {
    /// Return the diagnostic source implied by this output.
    pub(in crate::check) fn source(&self) -> ConstraintOrigin {
        match self {
            Self::Generic(generic) => ConstraintOrigin::Symbol(generic.slot().owner),
            Self::Node(node) => ConstraintOrigin::Node(*node),
            Self::Symbol(symbol) => ConstraintOrigin::Symbol(*symbol),
        }
    }
}

impl CheckState<'_> {
    /// Allocate one output backed check variable.
    pub(in crate::check) fn allocate_variable(
        &mut self,
        module: ModuleId,
        kind: VariableKind,
        output: VariableOutput,
    ) -> VariableId {
        let source = output.source();
        assert!(
            self.variables.is_open,
            "check variable graph is closed before solve"
        );
        let id = VariableId::new(module, self.variables.variables.len() as u32);
        let variable = Variable::new(id, kind, source, Some(output));
        self.variables.variables.push(variable);

        id
    }

    /// Allocate one anonymous check variable.
    pub(in crate::check) fn allocate_anonymous_variable(
        &mut self,
        module: ModuleId,
        kind: VariableKind,
        source: ConstraintOrigin,
    ) -> VariableId {
        assert!(
            self.variables.is_open,
            "check variable graph is closed before solve"
        );
        let id = VariableId::new(module, self.variables.variables.len() as u32);
        let variable = Variable::new(id, kind, source, None);
        self.variables.variables.push(variable);

        id
    }

    /// Return one variable.
    pub(in crate::check) fn variable(&self, id: VariableId) -> Variable {
        self.variables.variables[id.index as usize].clone()
    }

    /// Solve one variable and return whether it changed.
    pub(in crate::check) fn solve_variable(&mut self, id: VariableId, value: Solution) -> bool {
        let Some(existing) = self.solutions.solution.get(&id) else {
            self.solutions.solution.insert(id, value);

            return true;
        };

        // keep the first solution, relation constraints report conflicts
        if existing != &value {
            return false;
        }

        false
    }

    /// Add one lower bound to a variable.
    pub(in crate::check) fn add_lower_bound(
        &mut self,
        variable: VariableId,
        lower_bound: VariableId,
    ) -> bool {
        let bounds = self.solutions.bound.lower.entry(variable).or_default();
        if bounds.contains(&lower_bound) {
            return false;
        }

        bounds.push(lower_bound);

        true
    }

    /// Add one upper bound to a variable.
    pub(in crate::check) fn add_upper_bound(
        &mut self,
        variable: VariableId,
        upper_bound: VariableId,
    ) -> bool {
        let bounds = self.solutions.bound.upper.entry(variable).or_default();
        if bounds.contains(&upper_bound) {
            return false;
        }

        bounds.push(upper_bound);

        true
    }

    /// Return lower bounds for one variable.
    pub(in crate::check) fn lower_bounds(&self, variable: VariableId) -> Vec<VariableId> {
        self.solutions
            .bound
            .lower
            .get(&variable)
            .cloned()
            .unwrap_or_default()
    }

    /// Return upper bounds for one variable.
    pub(in crate::check) fn upper_bounds(&self, variable: VariableId) -> Vec<VariableId> {
        self.solutions
            .bound
            .upper
            .get(&variable)
            .cloned()
            .unwrap_or_default()
    }

    /// Return one variable's source node for diagnostics.
    pub(in crate::check) fn variable_source_node(&self, id: VariableId) -> dir::LocalNodeIdAny {
        let module_node = self.input(id.module).bound.module_node;

        match self.variable(id).source {
            ConstraintOrigin::Node(node) if node.module_id == id.module => node.local_id,
            ConstraintOrigin::Symbol(symbol) if symbol.module_id == id.module => {
                match self.symbol_source_node(id.module, symbol) {
                    Some(node) => node,
                    None => module_node,
                }
            }
            ConstraintOrigin::Node(_) | ConstraintOrigin::Symbol(_) => module_node,
        }
    }

    /// Return the solved type for one variable.
    pub(in crate::check) fn variable_type_solution(&self, id: VariableId) -> Option<TypeTerm> {
        match self.solutions.solution.get(&id).cloned() {
            Some(Solution::Type(term)) => Some(self.term(term)),
            _ => None,
        }
    }

    /// Return or create a type variable for one node.
    pub(in crate::check) fn intern_node_type_variable(
        &mut self,
        module: ModuleId,
        node: dir::GlobalNodeIdAny,
    ) -> VariableId {
        if let Some(variable) = self.variables.type_by_node.get(&node).copied() {
            return variable;
        }

        let variable =
            self.allocate_variable(module, VariableKind::Type, VariableOutput::Node(node));
        self.variables.type_by_node.insert(node, variable);

        variable
    }

    /// Return or create a type variable for one symbol.
    pub(in crate::check) fn intern_symbol_type_variable(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> VariableId {
        if let Some(variable) = self.variables.type_by_symbol.get(&symbol).copied() {
            return variable;
        }

        let variable =
            self.allocate_variable(module, VariableKind::Type, VariableOutput::Symbol(symbol));

        self.variables.type_by_symbol.insert(symbol, variable);

        variable
    }

    /// Return or create a static variable for one node.
    pub(in crate::check) fn intern_node_static_variable(
        &mut self,
        module: ModuleId,
        node: dir::GlobalNodeIdAny,
    ) -> VariableId {
        if let Some(variable) = self.variables.static_by_node.get(&node).copied() {
            return variable;
        }

        let variable =
            self.allocate_variable(module, VariableKind::Static, VariableOutput::Node(node));
        self.variables.static_by_node.insert(node, variable);

        variable
    }

    /// Return or create a type variable for one local node.
    pub(in crate::check) fn intern_local_type_variable<T: dir::Node + Clone>(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<T>,
    ) -> VariableId {
        self.intern_node_type_variable(module, id.into_global_any(module))
    }

    /// Return or create a static variable for one expression node.
    pub(in crate::check) fn define_static_expression_variable(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> VariableId {
        let expression = id.into_global(module);
        let variable = self.intern_node_static_variable(module, expression.clone().into_any());
        let term = StaticTerm::Expression(expression);

        self.define_static(module, variable, term);

        variable
    }

    /// Return or create a static variable for one symbol.
    pub(in crate::check) fn intern_symbol_static_variable(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> VariableId {
        if let Some(variable) = self.variables.static_by_symbol.get(&symbol).copied() {
            return variable;
        }

        let variable =
            self.allocate_variable(module, VariableKind::Static, VariableOutput::Symbol(symbol));

        self.variables.static_by_symbol.insert(symbol, variable);

        variable
    }

    /// Define one type variable from one term.
    pub(in crate::check) fn define_anonymous_type(
        &mut self,
        module: ModuleId,
        origin: ConstraintOrigin,
        term: TypeTerm,
    ) -> VariableId {
        let variable = self.allocate_anonymous_variable(module, VariableKind::Type, origin);

        self.define_type(module, variable, term);

        variable
    }

    /// Define one static literal variable.
    pub(in crate::check) fn define_static_literal(
        &mut self,
        module: ModuleId,
        origin: ConstraintOrigin,
        value: dir::StaticTerm,
    ) -> VariableId {
        let variable = self.allocate_anonymous_variable(module, VariableKind::Static, origin);

        self.define_static(module, variable, StaticTerm::Literal(value));

        variable
    }

    /// Return the solved static value for one variable.
    pub(in crate::check) fn variable_static_solution(&self, id: VariableId) -> Option<StaticTerm> {
        match self.solutions.solution.get(&id).cloned() {
            Some(Solution::Static(term)) => Some(self.term(term)),
            _ => None,
        }
    }
}
