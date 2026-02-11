use std::collections::HashSet;

use destack_dir as dir;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, Program};

use super::{expression_candidate_symbols, resolution_target_symbols};

/// Collected symbol usage for one DIR module.
#[derive(Debug, Clone, Default)]
pub struct ModuleSymbolUsage {
    /// Symbols referenced directly by expression target symbols.
    pub direct_symbols: HashSet<dir::GlobalSymbolId>,
    /// Symbols referenced through DIR resolution candidates.
    pub resolved_symbols: HashSet<dir::GlobalSymbolId>,
}

impl ModuleSymbolUsage {
    /// Return true when a global symbol appears in direct or resolved usage.
    pub fn references_symbol(&self, symbol_id: dir::GlobalSymbolId) -> bool {
        self.direct_symbols.contains(&symbol_id) || self.resolved_symbols.contains(&symbol_id)
    }

    /// Return true when a local symbol appears in direct or resolved usage.
    pub fn references_local_symbol(
        &self,
        module_id: ModuleId,
        symbol_id: dir::LocalSymbolId,
    ) -> bool {
        self.references_symbol(symbol_id.into_global(module_id))
    }

    /// Return all used local symbols for this module.
    pub fn used_local_symbols(&self, module_id: ModuleId) -> HashSet<dir::LocalSymbolId> {
        let mut symbols = HashSet::new();

        // direct local references
        for symbol_id in &self.direct_symbols {
            if symbol_id.module_id == module_id {
                symbols.insert(symbol_id.local_id);
            }
        }

        // resolved local references
        for symbol_id in &self.resolved_symbols {
            if symbol_id.module_id == module_id {
                symbols.insert(symbol_id.local_id);
            }
        }

        symbols
    }
}

/// Collect symbol usage for all expression nodes in one module.
pub fn collect_module_symbol_usage(
    module_id: ModuleId,
    tree: &dir::NodeTree,
    types: &dir::TypeTable,
) -> ModuleSymbolUsage {
    let mut usage = ModuleSymbolUsage::default();

    // direct symbol references and resolver candidates
    for (expression_id, expression) in tree.iter_nodes_of_type::<dir::Expression>() {
        if let Some(symbol_id) = expression.target_symbol() {
            usage.direct_symbols.insert(symbol_id);
        }

        let global_expression_id = expression_id.into_global_any(module_id);
        let Some(resolution_id) = types.get_resolution_for_node(global_expression_id) else {
            continue;
        };

        let resolution = types.get_resolution(resolution_id);
        for symbol_id in resolution_target_symbols(resolution) {
            usage.resolved_symbols.insert(symbol_id);
        }
    }

    usage
}

/// Collect assigned symbols for assignment-like expressions in one module.
#[allow(clippy::too_many_arguments)]
pub fn collect_assigned_symbol_usage(
    program: &Program,
    profile_id: ProfileId,
    module_id: ModuleId,
    tree: &dir::NodeTree,
    symbols: &dir::SymbolTable,
    types: &dir::TypeTable,
    mut include_assignment: impl FnMut(
        dir::LocalNodeId<dir::Expression>,
        dir::LocalNodeId<dir::Expression>,
    ) -> bool,
) -> HashSet<dir::GlobalSymbolId> {
    let mut assigned_symbols = HashSet::new();

    // collect assignment targets from assignment-like expressions
    for (assignment_expression_id, assignment_expression) in
        tree.iter_nodes_of_type::<dir::Expression>()
    {
        let Some(assigned_expression_id) = assignment_target_expression_id(assignment_expression)
        else {
            continue;
        };
        if !include_assignment(assignment_expression_id, assigned_expression_id) {
            continue;
        }

        let assigned_expression = tree.get(assigned_expression_id);
        let candidate_symbols = expression_candidate_symbols(
            program,
            profile_id,
            module_id,
            symbols,
            types,
            assigned_expression_id,
            assigned_expression,
        );
        assigned_symbols.extend(candidate_symbols);
    }

    assigned_symbols
}

/// Return one assigned target expression for assignment-like expressions.
fn assignment_target_expression_id(
    expression: &dir::Expression,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    match expression {
        dir::Expression::Assign { left, .. } | dir::Expression::AssignBinary { left, .. } => {
            Some(*left)
        }
        dir::Expression::Unary {
            operator:
                dir::UnaryOperator::PreIncrement
                | dir::UnaryOperator::PostIncrement
                | dir::UnaryOperator::PreDecrement
                | dir::UnaryOperator::PostDecrement,
            right,
        } => Some(*right),
        _ => None,
    }
}
