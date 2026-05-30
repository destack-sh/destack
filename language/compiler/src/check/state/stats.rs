use crate::check::{CheckState, VariableKind};

/// Derived size counters for one check component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct CheckStats {
    /// The number of allocated solver variables.
    pub(in crate::check) variables: usize,
    /// The number of allocated type variables.
    pub(in crate::check) type_variables: usize,
    /// The number of allocated static variables.
    pub(in crate::check) static_variables: usize,
    /// The number of collected definitions.
    pub(in crate::check) definitions: usize,
    /// The number of collected constraints.
    pub(in crate::check) constraints: usize,
    /// The number of collected obligations.
    pub(in crate::check) obligations: usize,
    /// The number of stored terms.
    pub(in crate::check) terms: usize,
    /// The number of solved variables.
    pub(in crate::check) solutions: usize,
    /// The total number of bounds.
    pub(in crate::check) bounds: usize,
    /// The number of lower type bounds.
    pub(in crate::check) type_lower_bounds: usize,
    /// The number of upper type bounds.
    pub(in crate::check) type_upper_bounds: usize,
    /// The number of lower static bounds.
    pub(in crate::check) static_lower_bounds: usize,
    /// The number of upper static bounds.
    pub(in crate::check) static_upper_bounds: usize,
    /// The total number of source outputs.
    pub(in crate::check) outputs: usize,
    /// The number of type outputs attached to nodes.
    pub(in crate::check) type_node_outputs: usize,
    /// The number of type outputs attached to symbols.
    pub(in crate::check) type_symbol_outputs: usize,
    /// The number of static outputs attached to nodes.
    pub(in crate::check) static_node_outputs: usize,
    /// The number of static outputs attached to symbols.
    pub(in crate::check) static_symbol_outputs: usize,
    /// The number of solver decisions.
    pub(in crate::check) decisions: usize,
}

impl CheckStats {
    /// Render these stats as stable metadata lines.
    pub(in crate::check) fn render_metadata(self) -> String {
        format!(
            "\
check.stats.solve.variables={}
check.stats.solve.definitions={}
check.stats.solve.constraints={}
check.stats.solve.obligations={}
check.stats.solve.solutions={}
check.stats.solve.bounds={}
check.stats.solve.decisions={}
check.stats.output.total={}
check.stats.output.type_nodes={}
check.stats.output.type_symbols={}
check.stats.output.static_nodes={}
check.stats.output.static_symbols={}",
            self.variables,
            self.definitions,
            self.constraints,
            self.obligations,
            self.solutions,
            self.bounds,
            self.decisions,
            self.outputs,
            self.type_node_outputs,
            self.type_symbol_outputs,
            self.static_node_outputs,
            self.static_symbol_outputs,
        )
    }
}

impl CheckState<'_> {
    /// Return derived size counters for this component.
    pub(in crate::check) fn stats(&self) -> CheckStats {
        let type_variables = self
            .variables
            .variables
            .iter()
            .filter(|variable| variable.kind == VariableKind::Type)
            .count();
        let static_variables = self.variables.variables.len() - type_variables;
        let type_lower_bounds = self
            .solutions
            .type_lower
            .values()
            .map(Vec::len)
            .sum::<usize>();
        let type_upper_bounds = self
            .solutions
            .type_upper
            .values()
            .map(Vec::len)
            .sum::<usize>();
        let static_lower_bounds = self
            .solutions
            .static_lower
            .values()
            .map(Vec::len)
            .sum::<usize>();
        let static_upper_bounds = self
            .solutions
            .static_upper
            .values()
            .map(Vec::len)
            .sum::<usize>();
        let bounds =
            type_lower_bounds + type_upper_bounds + static_lower_bounds + static_upper_bounds;
        let type_node_outputs = self.node_types.len();
        let type_symbol_outputs = self.symbol_types.len();
        let static_node_outputs = self.node_statics.len();
        let static_symbol_outputs = self.symbol_statics.len();
        let outputs =
            type_node_outputs + type_symbol_outputs + static_node_outputs + static_symbol_outputs;
        let decisions = self.solutions.call.len()
            + self.solutions.construct.len()
            + self.solutions.operator.len()
            + self.solutions.identity.len()
            + self.solutions.layout.len()
            + self.solutions.member.len()
            + self.solutions.receiver.len()
            + self.solutions.name.len();

        CheckStats {
            variables: self.variables.variables.len(),
            type_variables,
            static_variables,
            definitions: self.definitions.len(),
            constraints: self.constraints.len(),
            obligations: self.obligations.len(),
            terms: self.terms.len(),
            solutions: self.solutions.variable.len(),
            bounds,
            type_lower_bounds,
            type_upper_bounds,
            static_lower_bounds,
            static_upper_bounds,
            outputs,
            type_node_outputs,
            type_symbol_outputs,
            static_node_outputs,
            static_symbol_outputs,
            decisions,
        }
    }
}
