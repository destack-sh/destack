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
    /// The number of checked type operands attached to nodes.
    pub(in crate::check) node_types: usize,
    /// The number of checked type operands attached to symbols.
    pub(in crate::check) symbol_types: usize,
    /// The number of checked static operands attached to nodes.
    pub(in crate::check) node_statics: usize,
    /// The number of checked static operands attached to symbols.
    pub(in crate::check) symbol_statics: usize,
    /// The number of solver decisions.
    pub(in crate::check) decisions: usize,
}

impl CheckStats {
    /// Render these stats as stable metadata lines.
    pub(in crate::check) fn render_metadata(self) -> String {
        format!(
            "\
check.stats.solve.variables={}
check.stats.solve.terms={}
check.stats.solve.constraints={}
check.stats.solve.obligations={}
check.stats.solve.solutions={}
check.stats.solve.bounds={}
check.stats.solve.decisions={}
check.stats.output.total={}
check.stats.output.node_types={}
check.stats.output.symbol_types={}
check.stats.output.node_statics={}
check.stats.output.symbol_statics={}",
            self.variables,
            self.terms,
            self.constraints,
            self.obligations,
            self.solutions,
            self.bounds,
            self.decisions,
            self.outputs,
            self.node_types,
            self.symbol_types,
            self.node_statics,
            self.symbol_statics,
        )
    }
}

impl CheckState<'_> {
    /// Return derived size counters for this component.
    pub(in crate::check) fn stats(&self) -> CheckStats {
        let type_variables = self
            .inference
            .variables
            .variables
            .iter()
            .filter(|variable| variable.kind == VariableKind::Type)
            .count();
        let static_variables = self.inference.variables.variables.len() - type_variables;
        let type_lower_bounds = self
            .inference
            .type_lower_bounds
            .values()
            .map(Vec::len)
            .sum::<usize>();
        let type_upper_bounds = self
            .inference
            .type_upper_bounds
            .values()
            .map(Vec::len)
            .sum::<usize>();
        let static_lower_bounds = self
            .inference
            .static_lower_bounds
            .values()
            .map(Vec::len)
            .sum::<usize>();
        let static_upper_bounds = self
            .inference
            .static_upper_bounds
            .values()
            .map(Vec::len)
            .sum::<usize>();
        let bounds =
            type_lower_bounds + type_upper_bounds + static_lower_bounds + static_upper_bounds;
        let node_types = self.outputs.node_types.len();
        let symbol_types = self.outputs.symbol_types.len();
        let node_statics = self.outputs.node_statics.len();
        let symbol_statics = self.outputs.symbol_statics.len();
        let outputs = node_types + symbol_types + node_statics + symbol_statics;
        let decisions = self.inference.calls.len()
            + self.inference.constructs.len()
            + self.inference.operators.len()
            + self.inference.identities.len()
            + self.inference.layouts.len()
            + self.inference.members.len()
            + self.inference.receivers.len()
            + self.inference.names.len();

        CheckStats {
            variables: self.inference.variables.variables.len(),
            type_variables,
            static_variables,
            constraints: self.inference.constraints.len(),
            obligations: self.inference.obligations.len(),
            terms: self.inference.terms.len(),
            solutions: self.inference.variable_solutions.len(),
            bounds,
            type_lower_bounds,
            type_upper_bounds,
            static_lower_bounds,
            static_upper_bounds,
            outputs,
            node_types,
            symbol_types,
            node_statics,
            symbol_statics,
            decisions,
        }
    }
}
