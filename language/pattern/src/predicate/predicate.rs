use tspp_dir as dir;

use crate::MetavariableId;

/// A parsed TS++ expression evaluated for a candidate match.
#[derive(Debug)]
pub struct Predicate {
    /// The parsed expression nodes.
    pub(crate) tree: dir::Tree,
    /// The expression evaluated as a boolean condition.
    pub(crate) root: dir::LocalNodeId<dir::Expression>,
    /// The metavariable reads in source order.
    pub(crate) uses: PredicateUses,
}

/// A metavariable reference in a predicate expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PredicateUse {
    /// The referenced declaration.
    pub variable: MetavariableId,
    /// The expression reading the bound value.
    pub expression: dir::LocalNodeId<dir::Expression>,
}

/// The metavariable reads in one predicate tree.
#[derive(Debug)]
pub struct PredicateUses {
    /// The reads in source order.
    uses: Vec<PredicateUse>,
    /// The declaration read by each expression node.
    variable_by_expression: Vec<Option<MetavariableId>>,
}

impl PredicateUses {
    /// Create an empty read index for a DIR tree.
    pub(crate) fn new(node_count: usize) -> Self {
        Self {
            uses: Vec::new(),
            variable_by_expression: vec![None; node_count],
        }
    }

    /// Insert one metavariable read.
    pub(crate) fn insert(&mut self, use_entry: PredicateUse) {
        let slot = &mut self.variable_by_expression[use_entry.expression.id as usize];
        assert!(
            slot.is_none(),
            "predicate expression has multiple metavariable reads"
        );
        *slot = Some(use_entry.variable);
        self.uses.push(use_entry);
    }

    /// Return the declaration read by an expression.
    pub fn get(&self, expression: dir::LocalNodeId<dir::Expression>) -> Option<MetavariableId> {
        self.variable_by_expression
            .get(expression.id as usize)
            .copied()
            .flatten()
    }

    /// Iterate over reads in source order.
    pub fn iter(&self) -> impl Iterator<Item = &PredicateUse> {
        self.uses.iter()
    }

    /// Return the read count.
    pub fn len(&self) -> usize {
        self.uses.len()
    }

    /// Return whether the predicate has no metavariable reads.
    pub fn is_empty(&self) -> bool {
        self.uses.is_empty()
    }
}

impl Predicate {
    /// Return the parsed expression nodes.
    pub fn tree(&self) -> &dir::Tree {
        &self.tree
    }

    /// Return the expression evaluated as a boolean condition.
    pub fn root(&self) -> dir::LocalNodeId<dir::Expression> {
        self.root
    }

    /// Return the metavariable reads in source order.
    pub fn uses(&self) -> &PredicateUses {
        &self.uses
    }
}
