use destack_dir as dir;
use indexmap::IndexMap;

use crate::{CompilerError, CompilerResult};

/// Component-global id of one pending selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct SelectionId(u32);

impl SelectionId {
    /// Return the selection id at one index.
    pub(in crate::check) fn at(index: usize) -> Self {
        Self(index as u32)
    }

    /// Return the selection index.
    pub(in crate::check) fn index(self) -> usize {
        self.0 as usize
    }
}

/// One syntactic use of a place expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum PlaceUse {
    /// Place value is read.
    Read,
    /// Place value is replaced.
    Write,
    /// Place value is read, transformed, and replaced.
    Update,
}

/// Result shape produced by one construct expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum ConstructResult {
    /// Construct expression produces the constructed value directly.
    Direct,
    /// Construct expression produces the fallible construction carrier.
    Fallible,
}

/// Source-node operation waiting for enough type information to select.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Selection {
    /// Member access selection.
    Member {
        /// The selected expression node.
        node: dir::GlobalNodeId<dir::Expression>,
        /// The receiver expression.
        left: dir::LocalNodeId<dir::Expression>,
        /// The member name.
        name: Option<dir::StringId>,
    },
    /// Object literal spread merge.
    ObjectMerge {
        /// The selected expression node.
        node: dir::GlobalNodeId<dir::Expression>,
        /// The object properties.
        properties: Vec<dir::LocalNodeId<dir::Property>>,
    },
    /// Struct literal spread merge.
    StructMerge {
        /// The selected expression node.
        node: dir::GlobalNodeId<dir::Expression>,
        /// The struct properties.
        properties: Vec<dir::LocalNodeId<dir::Property>>,
    },
    /// Function call selection.
    Call {
        /// The selected expression node.
        node: dir::GlobalNodeId<dir::Expression>,
        /// The callee expression.
        callee: dir::LocalNodeId<dir::Expression>,
        /// The explicit generic arguments.
        generic_arguments: Vec<dir::LocalNodeId<dir::GenericArgument>>,
        /// The runtime arguments.
        arguments: Vec<dir::LocalNodeId<dir::Argument>>,
    },
    /// Membership predicate selection.
    MemberPredicate {
        /// The selected expression node.
        node: dir::GlobalNodeId<dir::Expression>,
        /// The key expression.
        left: dir::LocalNodeId<dir::Expression>,
        /// The receiver expression.
        right: dir::LocalNodeId<dir::Expression>,
    },
    /// Binary operator selection.
    BinaryOperator {
        /// The selected expression node.
        node: dir::GlobalNodeId<dir::Expression>,
        /// The operator.
        operator: dir::BinaryOperator,
        /// The left operand.
        left: dir::LocalNodeId<dir::Expression>,
        /// The right operand.
        right: dir::LocalNodeId<dir::Expression>,
    },
    /// Type predicate selection.
    TypePredicate {
        /// The selected expression node.
        node: dir::GlobalNodeId<dir::Expression>,
        /// The tested value.
        value: dir::LocalNodeId<dir::Expression>,
        /// The target type.
        target: dir::LocalNodeId<dir::TypeExpression>,
    },
    /// Class predicate selection.
    ClassPredicate {
        /// The selected expression node.
        node: dir::GlobalNodeId<dir::Expression>,
        /// The tested value.
        value: dir::LocalNodeId<dir::Expression>,
        /// The target expression.
        target: dir::LocalNodeId<dir::Expression>,
    },
    /// Unary operator selection.
    UnaryOperator {
        /// The selected expression node.
        node: dir::GlobalNodeId<dir::Expression>,
        /// The operator.
        operator: dir::UnaryOperator,
        /// The operand.
        operand: dir::LocalNodeId<dir::Expression>,
        /// The place use when this unary expression selects a place protocol.
        use_: PlaceUse,
    },
    /// Construct expression selection.
    Construct {
        /// The selected expression node.
        node: dir::GlobalNodeId<dir::Expression>,
        /// The constructed type expression.
        ty: dir::LocalNodeId<dir::TypeExpression>,
        /// The runtime arguments.
        arguments: Vec<dir::LocalNodeId<dir::Argument>>,
        /// How construction result failures are represented.
        result: ConstructResult,
    },
    /// Subscript selection.
    Subscript {
        /// The selected expression node.
        node: dir::GlobalNodeId<dir::Expression>,
        /// The indexed receiver expression.
        left: dir::LocalNodeId<dir::Expression>,
        /// The index expression.
        index: Option<dir::LocalNodeId<dir::Expression>>,
        /// The syntactic use of the subscript place.
        use_: PlaceUse,
    },
    /// Explicit generic instantiation selection.
    Instantiation {
        /// The selected expression node.
        node: dir::GlobalNodeId<dir::Expression>,
        /// The instantiated target expression.
        left: dir::LocalNodeId<dir::Expression>,
        /// The explicit generic arguments.
        arguments: Vec<dir::LocalNodeId<dir::GenericArgument>>,
    },
    /// Tagged template selection.
    TaggedTemplate {
        /// The selected expression node.
        node: dir::GlobalNodeId<dir::Expression>,
        /// The tag expression.
        tag: dir::LocalNodeId<dir::Expression>,
    },
    /// Tree expression selection.
    Tree {
        /// The selected expression node.
        node: dir::GlobalNodeId<dir::Expression>,
    },
    /// Pattern selection.
    Pattern {
        /// The selected pattern node.
        node: dir::GlobalNodeId<dir::Pattern>,
    },
}

impl Selection {
    /// Return the source node selected by this operation.
    pub(in crate::check) fn node(&self) -> dir::GlobalNodeIdAny {
        match self {
            Self::Member { node, .. }
            | Self::ObjectMerge { node, .. }
            | Self::StructMerge { node, .. }
            | Self::Call { node, .. }
            | Self::MemberPredicate { node, .. }
            | Self::BinaryOperator { node, .. }
            | Self::TypePredicate { node, .. }
            | Self::ClassPredicate { node, .. }
            | Self::UnaryOperator { node, .. }
            | Self::Construct { node, .. }
            | Self::Subscript { node, .. }
            | Self::Instantiation { node, .. }
            | Self::TaggedTemplate { node, .. }
            | Self::Tree { node, .. } => node.into_any(),
            Self::Pattern { node } => node.into_any(),
        }
    }
}

/// Solved state of one selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum SelectionState {
    /// The selection has not finished.
    Pending,
    /// The selection finished.
    Done,
}

impl SelectionState {
    /// Return whether the selection is done.
    pub(in crate::check) fn is_done(self) -> bool {
        matches!(self, Self::Done)
    }
}

/// Collected selections with solver state.
#[derive(Debug)]
pub(in crate::check) struct SelectionTable {
    /// The collected selections keyed by absolute selection id.
    selections: IndexMap<SelectionId, Selection>,
    /// Selection states keyed by absolute selection id.
    states: IndexMap<SelectionId, SelectionState>,
}

impl SelectionTable {
    /// Create an empty selection table.
    pub(in crate::check) fn new() -> Self {
        Self {
            selections: IndexMap::new(),
            states: IndexMap::new(),
        }
    }

    /// Insert one exact selection id.
    pub(in crate::check) fn insert(&mut self, id: SelectionId, selection: Selection) {
        self.selections.insert(id, selection);
        self.states.insert(id, SelectionState::Pending);
    }

    /// Remove one exact selection id.
    pub(in crate::check) fn remove(
        &mut self,
        id: SelectionId,
    ) -> (Option<Selection>, Option<SelectionState>) {
        let selection = self.selections.swap_remove(&id);
        let state = self.states.swap_remove(&id);

        (selection, state)
    }

    /// Return one selection.
    pub(in crate::check) fn get(&self, id: SelectionId) -> CompilerResult<&Selection> {
        self.selections
            .get(&id)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check selection {id:?} does not exist"),
            })
    }

    /// Return one selection state.
    pub(in crate::check) fn state(&self, id: SelectionId) -> CompilerResult<SelectionState> {
        self.states
            .get(&id)
            .copied()
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check selection {id:?} has no solver state"),
            })
    }

    /// Set one selection state.
    pub(in crate::check) fn set_state(&mut self, id: SelectionId, state: SelectionState) {
        self.states.insert(id, state);
    }

    /// Return whether one selection finished.
    pub(in crate::check) fn is_complete(&self, id: SelectionId) -> bool {
        self.state(id).is_ok_and(SelectionState::is_done)
    }
}
