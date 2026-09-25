use std::sync::Arc;

use tspp_core::{Arena, StringPool};

use crate::{
    Evaluator, Fragment, FragmentId, MatchError, MetavariableTable, ModuleContext, PatternMatch,
    Predicate, ProgramContext, Tree,
};

/// A compiled pattern.
#[derive(Debug)]
pub struct Pattern {
    /// The strings referenced by parsed DIR and metavariable names.
    pub(crate) strings: Arc<StringPool>,
    /// The operations evaluated for each candidate node.
    pub(crate) tree: Tree,
    /// The parsed structural fragments.
    pub(crate) fragments: Arena<Fragment>,
    /// The parsed predicate expressions.
    pub(crate) predicates: Vec<Predicate>,
    /// The shared metavariable declarations.
    pub(crate) metavariables: MetavariableTable,
}

impl Pattern {
    /// Return the strings referenced by this pattern.
    pub fn strings(&self) -> &Arc<StringPool> {
        &self.strings
    }

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
        &self.predicates
    }

    /// Evaluate every predicate against one checked structural match.
    pub fn matches_predicates(
        &self,
        pattern_match: &PatternMatch,
        module: &ModuleContext,
        program: &ProgramContext,
    ) -> Result<bool, MatchError> {
        for predicate in &self.predicates {
            let evaluator = Evaluator::new(
                predicate,
                &pattern_match.bindings,
                pattern_match.root,
                module,
                program,
            );
            if !evaluator.evaluate()? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Return the shared metavariable declarations.
    pub fn metavariables(&self) -> &MetavariableTable {
        &self.metavariables
    }
}
