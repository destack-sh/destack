use destack_core::Arena;

use crate::{Fragment, FragmentId, MetavariableTable, Predicate, PredicateId, Tree};

/// A compiled pattern.
#[derive(Debug)]
pub struct Pattern {
    /// The operations evaluated for each candidate node.
    pub(crate) tree: Tree,
    /// The parsed structural fragments.
    pub(crate) fragments: Arena<Fragment>,
    /// The parsed predicate expressions.
    pub(crate) predicates: Arena<Predicate>,
    /// The shared metavariable declarations.
    pub(crate) metavariables: MetavariableTable,
}

impl Pattern {
    /// Return the pattern operations.
    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    /// Return the parsed structural fragments.
    pub fn fragments(&self) -> &[Fragment] {
        self.fragments.as_slice()
    }

    /// Return a parsed structural fragment.
    pub fn fragment(&self, fragment: FragmentId) -> &Fragment {
        self.fragments.get(fragment.0)
    }

    /// Return the parsed predicate expressions.
    pub fn predicates(&self) -> &[Predicate] {
        self.predicates.as_slice()
    }

    /// Return a parsed predicate expression.
    pub fn predicate(&self, predicate: PredicateId) -> &Predicate {
        self.predicates.get(predicate.0)
    }

    /// Return the shared metavariable declarations.
    pub fn metavariables(&self) -> &MetavariableTable {
        &self.metavariables
    }
}
