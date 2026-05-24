use destack_source::ModuleId;

use destack_dir as dir;
use indexmap::IndexMap;

use super::{
    CheckModuleState, ConstraintOrigin, GenericArgumentKey, GenericParameter, StaticTerm, TypeTerm,
};

/// Component-valid id for one check variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(in crate::check) struct VariableId {
    /// The module that owns the variable.
    pub(in crate::check) module: ModuleId,
    /// The variable index inside the owning module.
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
    pub(in crate::check) origin: VariableOrigin,
    /// The solved value, when known.
    pub(in crate::check) solution: Option<Solution>,
    /// Variables that must be assignable to this variable.
    pub(in crate::check) lower_bounds: Vec<VariableId>,
    /// Variables that this variable must be assignable to.
    pub(in crate::check) upper_bounds: Vec<VariableId>,
}

/// Variables and variable indexes for one module.
#[derive(Debug)]
pub(in crate::check) struct CheckVariableState {
    /// Check variables in allocation order.
    pub(in crate::check) all: Vec<Variable>,
    /// Type variables keyed by source node.
    pub(in crate::check) type_by_node: IndexMap<dir::GlobalNodeIdAny, VariableId>,
    /// Type variables keyed by source symbol.
    pub(in crate::check) type_by_symbol: IndexMap<dir::GlobalSymbolId, VariableId>,
    /// Static variables keyed by source node.
    pub(in crate::check) static_by_node: IndexMap<dir::GlobalNodeIdAny, VariableId>,
    /// Static variables keyed by source symbol.
    pub(in crate::check) static_by_symbol: IndexMap<dir::GlobalSymbolId, VariableId>,
    /// Inferred generic arguments keyed by source and slot.
    pub(in crate::check) generic_argument: IndexMap<GenericArgumentKey, VariableId>,
    /// Next generic slot index keyed by owner.
    pub(in crate::check) generic_slot_index: IndexMap<dir::GlobalSymbolId, dir::GenericSlotIndex>,
    /// Next induced generic name index keyed by owner.
    pub(in crate::check) induced_generic_index: IndexMap<dir::GlobalSymbolId, u32>,
    /// Generic parameter variables keyed by owning symbol.
    pub(in crate::check) generic_parameter_by_owner: IndexMap<dir::GlobalSymbolId, Vec<VariableId>>,
}

impl CheckVariableState {
    /// Create empty variable state.
    pub(in crate::check) fn new() -> Self {
        Self {
            all: Vec::new(),
            type_by_node: IndexMap::new(),
            type_by_symbol: IndexMap::new(),
            static_by_node: IndexMap::new(),
            static_by_symbol: IndexMap::new(),
            generic_argument: IndexMap::new(),
            generic_slot_index: IndexMap::new(),
            induced_generic_index: IndexMap::new(),
            generic_parameter_by_owner: IndexMap::new(),
        }
    }
}

impl Variable {
    /// Create one unsolved variable.
    pub(in crate::check) fn new(
        id: VariableId,
        kind: VariableKind,
        origin: VariableOrigin,
    ) -> Self {
        Self {
            id,
            kind,
            origin,
            solution: None,
            lower_bounds: Vec::new(),
            upper_bounds: Vec::new(),
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
    Type(TypeTerm),
    /// Solved static term.
    Static(StaticTerm),
}

/// Source that produced one variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum VariableOrigin {
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

    /// Variable generated by check or solver rules.
    ///
    /// ```ts
    /// WithLifetime<T, L>
    /// ```
    Generated {
        /// The source or constraint that caused generation.
        origin: ConstraintOrigin,
    },
}

impl VariableOrigin {
    /// Create a generated origin without a more specific source.
    pub(in crate::check) fn generated() -> Self {
        Self::Generated {
            origin: ConstraintOrigin::Synthetic,
        }
    }

    /// Create a generated origin from one constraint origin.
    pub(in crate::check) fn generated_from(origin: ConstraintOrigin) -> Self {
        Self::Generated { origin }
    }
}

impl CheckModuleState {
    /// Push one check variable.
    pub(in crate::check) fn push_variable(
        &mut self,
        kind: VariableKind,
        origin: VariableOrigin,
    ) -> VariableId {
        let id = VariableId::new(self.input.module, self.work.variables.all.len() as u32);
        let variable = Variable::new(id, kind, origin);
        self.work.variables.all.push(variable);

        id
    }

    /// Return one variable.
    pub(in crate::check) fn variable(&self, id: VariableId) -> &Variable {
        &self.work.variables.all[id.index as usize]
    }

    /// Return one variable mutably.
    pub(in crate::check) fn variable_mut(&mut self, id: VariableId) -> &mut Variable {
        &mut self.work.variables.all[id.index as usize]
    }

    /// Return the solved value for one variable.
    pub(in crate::check) fn variable_solution(&self, id: VariableId) -> Option<&Solution> {
        self.variable(id).solution.as_ref()
    }

    /// Solve one variable and return whether it changed.
    pub(in crate::check) fn solve_variable(&mut self, id: VariableId, value: Solution) -> bool {
        let variable = self.variable_mut(id);
        let Some(existing) = &variable.solution else {
            variable.solution = Some(value);

            return true;
        };

        // keep the first solution, relation constraints report conflicts
        if existing != &value {
            return false;
        }

        false
    }

    /// Add one lower bound to a variable.
    pub(in crate::check) fn bound_variable_below(
        &mut self,
        variable: VariableId,
        lower_bound: VariableId,
    ) -> bool {
        let variable = self.variable_mut(variable);
        if variable.lower_bounds.contains(&lower_bound) {
            return false;
        }

        variable.lower_bounds.push(lower_bound);

        true
    }

    /// Add one upper bound to a variable.
    pub(in crate::check) fn bound_variable_above(
        &mut self,
        variable: VariableId,
        upper_bound: VariableId,
    ) -> bool {
        let variable = self.variable_mut(variable);
        if variable.upper_bounds.contains(&upper_bound) {
            return false;
        }

        variable.upper_bounds.push(upper_bound);

        true
    }

    /// Return one variable's source node for diagnostics.
    pub(in crate::check) fn variable_source_node(&self, id: VariableId) -> dir::LocalNodeIdAny {
        let module_node = self.input.bound.module_node;

        match &self.variable(id).origin {
            VariableOrigin::Node(node) if node.module_id == self.input.module => node.local_id,
            VariableOrigin::Symbol(symbol) if symbol.module_id == self.input.module => {
                match self.symbol_source_node(*symbol) {
                    Some(node) => node,
                    None => module_node,
                }
            }
            VariableOrigin::Generic(generic)
                if generic.slot().owner.module_id == self.input.module =>
            {
                match self.symbol_source_node(generic.slot().owner) {
                    Some(node) => node,
                    None => module_node,
                }
            }
            VariableOrigin::Generated { origin } => match origin {
                ConstraintOrigin::Node(node) if node.module_id == self.input.module => {
                    node.local_id
                }
                ConstraintOrigin::Symbol(symbol) if symbol.module_id == self.input.module => {
                    match self.symbol_source_node(*symbol) {
                        Some(node) => node,
                        None => module_node,
                    }
                }
                ConstraintOrigin::Synthetic
                | ConstraintOrigin::Node(_)
                | ConstraintOrigin::Symbol(_) => module_node,
            },
            VariableOrigin::Symbol(_) | VariableOrigin::Node(_) | VariableOrigin::Generic(_) => {
                module_node
            }
        }
    }

    /// Return the solved type for one variable.
    pub(in crate::check) fn variable_type_solution(&self, id: VariableId) -> Option<&TypeTerm> {
        match self.variable_solution(id) {
            Some(Solution::Type(term)) => Some(term),
            _ => None,
        }
    }

    /// Return or create a type variable for one node.
    pub(in crate::check) fn node_type_variable(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> VariableId {
        if let Some(variable) = self.work.variables.type_by_node.get(&node).copied() {
            return variable;
        }

        let variable = self.push_variable(VariableKind::Type, VariableOrigin::Node(node));
        self.work.variables.type_by_node.insert(node, variable);

        variable
    }

    /// Return or create a type variable for one symbol.
    pub(in crate::check) fn symbol_type_variable(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> VariableId {
        if let Some(variable) = self.work.variables.type_by_symbol.get(&symbol).copied() {
            return variable;
        }

        let variable = self.push_variable(VariableKind::Type, VariableOrigin::Symbol(symbol));

        // seed declarations that already have checked input types
        if let Some(type_id) = self.visible_symbol_type_id(symbol) {
            let ty = self.get_type(type_id);
            let _ = self.solve_variable(variable, Solution::Type(TypeTerm::Literal(ty)));
        }

        self.work.variables.type_by_symbol.insert(symbol, variable);

        variable
    }

    /// Return or create a static variable for one node.
    pub(in crate::check) fn node_static_variable(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> VariableId {
        if let Some(variable) = self.work.variables.static_by_node.get(&node).copied() {
            return variable;
        }

        let variable = self.push_variable(VariableKind::Static, VariableOrigin::Node(node));
        self.work.variables.static_by_node.insert(node, variable);

        variable
    }

    /// Return or create a static variable for one symbol.
    pub(in crate::check) fn symbol_static_variable(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> VariableId {
        if let Some(variable) = self.work.variables.static_by_symbol.get(&symbol).copied() {
            return variable;
        }

        let variable = self.push_variable(VariableKind::Static, VariableOrigin::Symbol(symbol));

        // seed declarations that already have checked input statics
        if let Some(static_id) = self.visible_symbol_static_id(symbol) {
            let term = self.get_static(static_id);
            let _ = self.solve_variable(variable, Solution::Static(StaticTerm::Literal(term)));
        }

        self.work
            .variables
            .static_by_symbol
            .insert(symbol, variable);

        variable
    }

    /// Return one synthetic static value variable.
    pub(in crate::check) fn static_value_variable(&mut self, value: dir::StaticTerm) -> VariableId {
        let variable = self.push_variable(VariableKind::Static, VariableOrigin::generated());

        self.define_static_term(variable, StaticTerm::Literal(value));

        variable
    }

    /// Return the solved static value for one variable.
    pub(in crate::check) fn variable_static_solution(&self, id: VariableId) -> Option<&StaticTerm> {
        match self.variable_solution(id) {
            Some(Solution::Static(term)) => Some(term),
            _ => None,
        }
    }
}
