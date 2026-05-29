use destack_source::ModuleId;

use destack_dir as dir;
use indexmap::{IndexMap, IndexSet};

use crate::CompilerResult;
use crate::check::{
    CheckState, Condition, Constraint, Decision, Definition, Origin, StaticOperand, StaticRelation,
    StaticTerm, TermId, TypeOperand, TypeRelation, TypeTerm,
};

use super::{CallInstantiationArgumentKey, GenericParameter};

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
    pub(in crate::check) source: Origin,
    /// The committed output that receives this variable when solved.
    pub(in crate::check) output: Option<VariableOutput>,
}

/// Variables and variable indexes for one checked component.
#[derive(Debug)]
pub(in crate::check) struct VariableTable {
    /// Check variables in allocation order.
    pub(in crate::check) variables: Vec<Variable>,
    /// Variable definitions in collection order.
    pub(in crate::check) definitions: Vec<Definition>,
    /// Variable constraints in collection order.
    pub(in crate::check) constraints: Vec<Constraint>,
    /// Variables with an explicit definition term.
    pub(in crate::check) defined: IndexSet<VariableId>,

    /// Type variables keyed by source node.
    pub(in crate::check) type_by_node: IndexMap<dir::GlobalNodeIdAny, VariableId>,
    /// Type variables keyed by local source symbol.
    pub(in crate::check) type_by_symbol: IndexMap<dir::GlobalSymbolId, VariableId>,
    /// Static variables keyed by source node.
    pub(in crate::check) static_by_node: IndexMap<dir::GlobalNodeIdAny, VariableId>,
    /// Static variables keyed by local source symbol.
    pub(in crate::check) static_by_symbol: IndexMap<dir::GlobalSymbolId, VariableId>,
    /// Materialized type variables keyed by visible checked type id.
    pub(in crate::check) type_by_id: IndexMap<dir::GlobalTypeId, VariableId>,
    /// Materialized static variables keyed by visible checked static id.
    pub(in crate::check) static_by_id: IndexMap<dir::GlobalStaticId, VariableId>,

    /// Omitted call instantiation arguments keyed by source and slot.
    pub(in crate::check) call_instantiation_argument:
        IndexMap<CallInstantiationArgumentKey, VariableId>,
    /// Next generic slot index keyed by owner.
    pub(in crate::check) generic_slot_index: IndexMap<dir::GlobalSymbolId, dir::GenericSlotIndex>,
    /// Generic parameter variables keyed by owning symbol.
    pub(in crate::check) generic_parameter_by_owner: IndexMap<dir::GlobalSymbolId, Vec<VariableId>>,
    /// Generic parameter variables keyed by parameter symbol.
    pub(in crate::check) generic_parameter_by_symbol: IndexMap<dir::GlobalSymbolId, VariableId>,
}

impl VariableTable {
    /// Create empty variable state.
    pub(in crate::check) fn new() -> Self {
        Self {
            variables: Vec::new(),
            definitions: Vec::new(),
            constraints: Vec::new(),
            defined: IndexSet::new(),

            type_by_node: IndexMap::new(),
            type_by_symbol: IndexMap::new(),
            static_by_node: IndexMap::new(),
            static_by_symbol: IndexMap::new(),
            type_by_id: IndexMap::new(),
            static_by_id: IndexMap::new(),

            call_instantiation_argument: IndexMap::new(),
            generic_slot_index: IndexMap::new(),
            generic_parameter_by_owner: IndexMap::new(),
            generic_parameter_by_symbol: IndexMap::new(),
        }
    }
}

impl Variable {
    /// Create one unsolved variable.
    pub(in crate::check) fn new(
        id: VariableId,
        kind: VariableKind,
        source: Origin,
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
    /// Variable attached to one generic slot.
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
    pub(in crate::check) fn source(&self) -> Origin {
        match self {
            Self::Generic(generic) => Origin::Symbol(generic.slot().owner),
            Self::Node(node) => Origin::Node(*node),
            Self::Symbol(symbol) => Origin::Symbol(*symbol),
        }
    }
}

impl CheckState<'_> {
    /// Allocate one output backed check variable.
    pub(in crate::check) fn allocate_output_variable(
        &mut self,
        module: ModuleId,
        kind: VariableKind,
        output: VariableOutput,
    ) -> VariableId {
        assert_eq!(
            output.source().module(),
            module,
            "check output variable source must be local"
        );

        let source = output.source();
        let id = VariableId::new(module, self.variables.variables.len() as u32);
        let variable = Variable::new(id, kind, source, Some(output));
        self.variables.variables.push(variable);

        id
    }

    /// Allocate one inferred check variable.
    pub(in crate::check) fn allocate_inference_variable(
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

        let id = VariableId::new(module, self.variables.variables.len() as u32);
        let variable = Variable::new(id, kind, source, None);
        self.variables.variables.push(variable);

        id
    }

    /// Return the number of allocated variables.
    pub(in crate::check) fn variable_count(&self) -> usize {
        self.variables.variables.len()
    }

    /// Return one variable.
    pub(in crate::check) fn variable(&self, id: VariableId) -> &Variable {
        &self.variables.variables[id.index as usize]
    }

    /// Return one variable by allocation index.
    pub(in crate::check) fn variable_at(&self, index: usize) -> &Variable {
        &self.variables.variables[index]
    }

    /// Add one lower type bound to a variable.
    pub(in crate::check) fn add_lower_type_bound(
        &mut self,
        variable: VariableId,
        lower_bound: TypeOperand,
    ) -> CompilerResult<bool> {
        let has_bound = match self.solutions.type_lower.get(&variable) {
            Some(bounds) => self.type_bound_contains(bounds, lower_bound)?,
            None => false,
        };
        if has_bound {
            return Ok(false);
        }
        let bounds = self.solutions.type_lower.entry(variable).or_default();

        bounds.push(lower_bound);

        Ok(true)
    }

    /// Add one upper type bound to a variable.
    pub(in crate::check) fn add_upper_type_bound(
        &mut self,
        variable: VariableId,
        upper_bound: TypeOperand,
    ) -> CompilerResult<bool> {
        let has_bound = match self.solutions.type_upper.get(&variable) {
            Some(bounds) => self.type_bound_contains(bounds, upper_bound)?,
            None => false,
        };
        if has_bound {
            return Ok(false);
        }
        let bounds = self.solutions.type_upper.entry(variable).or_default();

        bounds.push(upper_bound);

        Ok(true)
    }

    /// Add one lower static bound to a variable.
    pub(in crate::check) fn add_lower_static_bound(
        &mut self,
        variable: VariableId,
        lower_bound: StaticOperand,
    ) -> CompilerResult<bool> {
        let has_bound = match self.solutions.static_lower.get(&variable) {
            Some(bounds) => self.static_bound_contains(bounds, lower_bound)?,
            None => false,
        };
        if has_bound {
            return Ok(false);
        }
        let bounds = self.solutions.static_lower.entry(variable).or_default();

        bounds.push(lower_bound);

        Ok(true)
    }

    /// Add one upper static bound to a variable.
    pub(in crate::check) fn add_upper_static_bound(
        &mut self,
        variable: VariableId,
        upper_bound: StaticOperand,
    ) -> CompilerResult<bool> {
        let has_bound = match self.solutions.static_upper.get(&variable) {
            Some(bounds) => self.static_bound_contains(bounds, upper_bound)?,
            None => false,
        };
        if has_bound {
            return Ok(false);
        }
        let bounds = self.solutions.static_upper.entry(variable).or_default();

        bounds.push(upper_bound);

        Ok(true)
    }

    /// Return whether a type bound list already contains one equivalent bound.
    fn type_bound_contains(
        &self,
        bounds: &[TypeOperand],
        candidate: TypeOperand,
    ) -> CompilerResult<bool> {
        for bound in bounds {
            if bound == &candidate {
                return Ok(true);
            }
            if let (Some(bound), Some(candidate)) = (
                self.solved_type_operand(*bound)?,
                self.solved_type_operand(candidate)?,
            ) && bound == candidate
            {
                return Ok(true);
            }
            if self.decide_type_relation(TypeRelation::Equal, *bound, candidate)? == Decision::Yes {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return whether a static bound list already contains one equivalent bound.
    fn static_bound_contains(
        &self,
        bounds: &[StaticOperand],
        candidate: StaticOperand,
    ) -> CompilerResult<bool> {
        for bound in bounds {
            if bound == &candidate {
                return Ok(true);
            }
            if self.decide_static_relation(StaticRelation::Equal, *bound, candidate)?
                == Decision::Yes
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return lower type bounds for one variable.
    pub(in crate::check) fn lower_type_bounds(&self, variable: VariableId) -> &[TypeOperand] {
        self.solutions
            .type_lower
            .get(&variable)
            .map_or(&[], Vec::as_slice)
    }

    /// Return upper type bounds for one variable.
    pub(in crate::check) fn upper_type_bounds(&self, variable: VariableId) -> &[TypeOperand] {
        self.solutions
            .type_upper
            .get(&variable)
            .map_or(&[], Vec::as_slice)
    }

    /// Return lower static bounds for one variable.
    pub(in crate::check) fn lower_static_bounds(&self, variable: VariableId) -> &[StaticOperand] {
        self.solutions
            .static_lower
            .get(&variable)
            .map_or(&[], Vec::as_slice)
    }

    /// Return upper static bounds for one variable.
    pub(in crate::check) fn upper_static_bounds(&self, variable: VariableId) -> &[StaticOperand] {
        self.solutions
            .static_upper
            .get(&variable)
            .map_or(&[], Vec::as_slice)
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

    /// Return the solved type for one variable.
    pub(in crate::check) fn variable_type_solution(&self, id: VariableId) -> Option<TypeTerm> {
        match self.solutions.variable.get(&id).cloned() {
            Some(Solution::Type(term)) => Some(self.terms.get(term).clone()),
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
            self.allocate_output_variable(module, VariableKind::Type, VariableOutput::Node(node));
        self.variables.type_by_node.insert(node, variable);

        variable
    }

    /// Return or create a type variable for one symbol.
    pub(in crate::check) fn intern_local_symbol_type_variable(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> VariableId {
        assert_eq!(
            symbol.module_id, module,
            "check symbol type variable must be local"
        );

        if let Some(variable) = self.variables.type_by_symbol.get(&symbol).copied() {
            return variable;
        }

        let variable = self.allocate_output_variable(
            module,
            VariableKind::Type,
            VariableOutput::Symbol(symbol),
        );

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
            self.allocate_output_variable(module, VariableKind::Static, VariableOutput::Node(node));
        self.variables.static_by_node.insert(node, variable);

        variable
    }

    /// Return or create a type variable for one local node.
    pub(in crate::check) fn intern_local_node_type_variable<T: dir::Node + Clone>(
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
        condition: Condition,
    ) -> VariableId {
        if self.static_expression_is_inference_hole(module, id) {
            return self.intern_node_static_variable(module, id.into_global_any(module));
        }

        let expression = id.into_global(module);
        let variable = self.intern_node_static_variable(module, expression.clone().into_any());
        let term = StaticTerm::Expression(expression);

        self.add_static_definition(variable, term, condition);

        variable
    }

    /// Return whether one static expression is an inference hole.
    pub(in crate::check) fn static_expression_is_inference_hole(
        &self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let dir::Expression::Type { value } = self.module(module).view().get(expression) else {
            return false;
        };
        let dir::TypeExpression::Infer {
            form: dir::InferForm::Hole,
            ..
        } = self.module(module).view().get(*value)
        else {
            return false;
        };

        true
    }

    /// Return or create a static variable for one symbol.
    pub(in crate::check) fn intern_symbol_static_variable(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> VariableId {
        assert_eq!(
            symbol.module_id, module,
            "check symbol static variable must be local"
        );

        if let Some(variable) = self.variables.static_by_symbol.get(&symbol).copied() {
            return variable;
        }

        let variable = self.allocate_output_variable(
            module,
            VariableKind::Static,
            VariableOutput::Symbol(symbol),
        );

        self.variables.static_by_symbol.insert(symbol, variable);

        variable
    }

    /// Return the solved static value for one variable.
    pub(in crate::check) fn variable_static_solution(&self, id: VariableId) -> Option<StaticTerm> {
        match self.solutions.variable.get(&id).cloned() {
            Some(Solution::Static(term)) => Some(self.terms.get(term).clone()),
            _ => None,
        }
    }
}
