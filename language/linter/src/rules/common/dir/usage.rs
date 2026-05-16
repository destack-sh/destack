use std::collections::HashSet;

use destack_dir as dir;
use destack_dir::{NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_source::ModuleId;

use super::{
    assign_pattern_contains_expression, expression_assignment_target, expression_candidate_symbols,
    expression_is_standalone_statement, resolution_target_symbols,
};

/// Collected symbol usage for one DIR module.
#[derive(Debug, Clone, Default)]
pub struct ModuleSymbolUsage {
    /// Symbols referenced directly by expression target symbols.
    pub direct_symbols: HashSet<dir::GlobalSymbolId>,
    /// Symbols referenced through DIR resolution candidates.
    pub candidate_symbols: HashSet<dir::GlobalSymbolId>,
}

impl ModuleSymbolUsage {
    /// Return true when a global symbol appears in direct or candidate usage.
    pub fn references_symbol(&self, symbol_id: dir::GlobalSymbolId) -> bool {
        self.direct_symbols.contains(&symbol_id) || self.candidate_symbols.contains(&symbol_id)
    }

    /// Return true when a local symbol appears in direct or candidate usage.
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

        // candidate local references
        for symbol_id in &self.candidate_symbols {
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
    tree: &dir::Tree,
    types: &dir::TypeTable<'_>,
) -> ModuleSymbolUsage {
    let mut usage = ModuleSymbolUsage::default();

    // direct symbol references and resolution candidates
    for (expression_id, _) in tree.iter_nodes_of_type::<dir::Expression>() {
        let global_expression_id = expression_id.into_global_any(module_id);

        if let Some(symbol_id) = types.symbol_resolution(global_expression_id) {
            usage.direct_symbols.insert(symbol_id);
        }

        if let Some(resolution) = types.resolution(global_expression_id) {
            for symbol_id in resolution_target_symbols(resolution) {
                usage.candidate_symbols.insert(symbol_id);
            }
        }
    }

    usage
}

/// Resolve the single lexical target symbol for one expression.
fn expression_target_symbol(
    module_id: ModuleId,
    types: &dir::TypeTable<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::GlobalSymbolId> {
    types.symbol_resolution(expression_id.into_global_any(module_id))
}

/// Collect direct reference expression ids for one local symbol in one module.
pub fn collect_local_symbol_direct_reference_expression_ids(
    module_id: ModuleId,
    tree: &dir::Tree,
    types: &dir::TypeTable<'_>,
    symbol_id: dir::LocalSymbolId,
) -> Vec<dir::LocalNodeId<dir::Expression>> {
    let mut references = Vec::new();
    let global_symbol_id = symbol_id.into_global(module_id);

    // collect direct target symbol references in deterministic tree order
    for (expression_id, _) in tree.iter_nodes_of_type::<dir::Expression>() {
        if expression_target_symbol(module_id, types, expression_id) == Some(global_symbol_id) {
            references.push(expression_id);
        }
    }

    references
}

/// Return true when one local symbol has direct references in one module.
pub fn local_symbol_has_direct_references(
    module_id: ModuleId,
    tree: &dir::Tree,
    types: &dir::TypeTable<'_>,
    symbol_id: dir::LocalSymbolId,
) -> bool {
    tree.iter_nodes_of_type::<dir::Expression>()
        .any(|(expression_id, _)| {
            expression_target_symbol(module_id, types, expression_id)
                == Some(symbol_id.into_global(module_id))
        })
}

/// Collect symbols read by one expression subtree.
pub fn collect_expression_read_symbol_usage(
    module_id: ModuleId,
    tree: &dir::Tree,
    types: &dir::TypeTable<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> HashSet<dir::GlobalSymbolId> {
    let mut collector = ReadSymbolCollector {
        module_id,
        types,
        reads: HashSet::new(),
        options: NodeVisitorOptions::default(),
    };
    let expression = tree.get(expression_id);
    collector.visit_expression(tree, expression_id, expression);
    collector.reads
}

/// Collect read symbols for one module.
pub fn collect_module_read_symbol_usage(
    module_id: ModuleId,
    tree: &dir::Tree,
    types: &dir::TypeTable<'_>,
) -> HashSet<dir::GlobalSymbolId> {
    let mut reads = HashSet::new();

    // collect read usages from expression nodes
    for (expression_id, _) in tree.iter_nodes_of_type::<dir::Expression>() {
        if !expression_reference_is_read(tree, expression_id) {
            continue;
        }

        let global_expression_id = expression_id.into_global_any(module_id);

        if let Some(symbol_id) = types.symbol_resolution(global_expression_id) {
            reads.insert(symbol_id);
        }

        if let Some(resolution) = types.resolution(global_expression_id) {
            for symbol_id in resolution_target_symbols(resolution) {
                reads.insert(symbol_id);
            }
        }
    }

    reads
}

/// Collect assigned symbols for assignment-like expressions in one module.
#[allow(clippy::too_many_arguments)]
pub fn collect_assigned_symbol_usage(
    module_id: ModuleId,
    tree: &dir::Tree,
    types: &dir::TypeTable<'_>,
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
        let Some(assigned_expression_id) =
            expression_assignment_target(tree, assignment_expression)
        else {
            continue;
        };
        if !include_assignment(assignment_expression_id, assigned_expression_id) {
            continue;
        }

        let candidate_symbols =
            expression_candidate_symbols(module_id, types, assigned_expression_id);
        assigned_symbols.extend(candidate_symbols);
    }

    assigned_symbols
}

/// Collect symbol reads while skipping pure write positions.
struct ReadSymbolCollector<'a> {
    /// The module being scanned.
    module_id: ModuleId,
    /// The type table carrying semantic resolutions.
    types: &'a dir::TypeTable<'a>,
    /// Collected symbols read from one expression.
    reads: HashSet<dir::GlobalSymbolId>,
    /// Visitor options.
    options: NodeVisitorOptions,
}

impl NodeVisitor for ReadSymbolCollector<'_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // assignment left side is write only here, only visit the right side
        if let dir::Expression::Assign {
            left: _,
            operator: _,
            right,
        } = expression
        {
            let right_expression = tree.get(*right);
            self.visit_expression(tree, *right, right_expression);
            return;
        }

        // let declarator patterns are writes, only visit initializers
        if let dir::Expression::Let {
            kind: _,
            export: _,
            is_ambient: _,
            mutability: _,
            declarators,
            is_shared: _,
        } = expression
        {
            for declarator_id in declarators {
                let declarator = tree.get(*declarator_id);
                if let Some(value_expression_id) = declarator.value {
                    let value_expression = tree.get(value_expression_id);
                    self.visit_expression(tree, value_expression_id, value_expression);
                }
            }
            return;
        }

        if let Some(symbol_id) = expression_target_symbol(self.module_id, self.types, id) {
            self.reads.insert(symbol_id);
        }

        walk_expression(self, tree, id, expression);
    }
}

/// Return true when one expression reference is consumed in a read context.
pub fn expression_reference_is_read(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let mut current_id = expression_id;

    loop {
        let Some(parent) = tree.get_parent(current_id.id) else {
            return true;
        };
        if parent.ty != dir::NodeType::Expression {
            return true;
        }

        let parent_id = parent.into_typed::<dir::Expression>();
        let parent_expression = tree.get(parent_id);

        match parent_expression {
            // unwrap transparent wrappers and continue
            dir::Expression::Parenthesized { expression } if *expression == current_id => {
                current_id = parent_id;
            }
            dir::Expression::As {
                expression: value,
                target_type: _,
            }
            | dir::Expression::Satisfies {
                expression: value,
                target_type: _,
            } if *value == current_id => {
                current_id = parent_id;
            }
            dir::Expression::Maybe { left, .. } | dir::Expression::Must { left, .. }
                if *left == current_id =>
            {
                current_id = parent_id;
            }

            // plain assignment left side is write only
            dir::Expression::Assign {
                left,
                operator: dir::AssignOperator::Assign,
                right: _,
            } if assign_pattern_contains_expression(tree, *left, current_id) => {
                return false;
            }

            // update assignments read previous value only when the result is consumed
            dir::Expression::Assign {
                left,
                operator,
                right: _,
            } if *operator != dir::AssignOperator::Assign
                && assign_pattern_contains_expression(tree, *left, current_id) =>
            {
                return !expression_is_standalone_statement(tree, parent_id);
            }

            // standalone increments and decrements are treated as write only
            dir::Expression::Unary {
                operator:
                    dir::UnaryOperator::PreIncrement
                    | dir::UnaryOperator::PostIncrement
                    | dir::UnaryOperator::PreDecrement
                    | dir::UnaryOperator::PostDecrement,
                right,
            } if *right == current_id => {
                return !expression_is_standalone_statement(tree, parent_id);
            }

            // all other parent contexts consume this value
            _ => {
                return true;
            }
        }
    }
}
