use destack_dir as dir;

use super::{CheckModuleState, TypeRelation, VariableId};

/// User-facing check that requires solved terms or whole-expression context.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Obligation {
    /// A source construct needs a concrete type after inference.
    ///
    /// ```ts
    /// let value;
    /// ```
    RequireType {
        /// The source node that requires a type.
        source: dir::GlobalNodeIdAny,
        /// The type variable that must be solved.
        variable: VariableId,
    },
    /// Match cases must cover every possible selector value.
    ///
    /// ```ts
    /// match value {
    ///     true => 1,
    ///     false => 0,
    /// }
    /// ```
    RequireExhaustiveMatch {
        /// The match expression.
        source: dir::GlobalNodeIdAny,
        /// The matched value type.
        value: VariableId,
        /// The match cases in source order.
        cases: Vec<dir::LocalNodeId<dir::MatchCase>>,
    },
    /// Pattern must be valid for the matched value type.
    ///
    /// ```ts
    /// const Point { x, y } = value;
    /// ```
    RequirePattern {
        /// The pattern node.
        source: dir::GlobalNodeIdAny,
        /// The matched value type.
        value: VariableId,
        /// The source pattern.
        pattern: dir::LocalNodeId<dir::Pattern>,
    },
    /// Try propagation must fit the enclosing return type.
    ///
    /// ```ts
    /// value?
    /// ```
    RequireTryPropagation {
        /// The try expression.
        source: dir::GlobalNodeIdAny,
        /// The tried value type.
        value: VariableId,
        /// The enclosing function return type.
        return_type: Option<VariableId>,
    },
    /// A resource binding must implement the required disposal protocol.
    ///
    /// ```ts
    /// using file = open(path);
    /// await using lock = acquire();
    /// ```
    RequireDispose {
        /// The using declaration or declarator.
        source: dir::GlobalNodeIdAny,
        /// The resource value type.
        value: VariableId,
        /// The required disposal mode.
        mode: DisposeMode,
    },
    /// A where clause must hold after generic substitution.
    ///
    /// ```ts
    /// where T: Iterator
    /// ```
    RequireWhereClause {
        /// The where clause node.
        source: dir::GlobalNodeIdAny,
        /// The left type.
        left: VariableId,
        /// The required relation.
        relation: TypeRelation,
        /// The right type.
        right: VariableId,
    },
    /// A static source condition must evaluate to true.
    ///
    /// ```ts
    /// @if(import.meta.platform == "linux")
    /// ```
    RequireStaticCondition {
        /// The condition node.
        source: dir::GlobalNodeIdAny,
        /// The solved condition static value.
        condition: VariableId,
    },
    /// A flow-sensitive condition must be applied to a branch.
    ///
    /// ```ts
    /// if (value is User) {
    ///     value.name
    /// }
    /// ```
    RequireFlowCondition {
        /// The condition expression.
        source: dir::GlobalNodeIdAny,
        /// The condition value type.
        condition: VariableId,
    },
}

/// Disposal mode required by a resource binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum DisposeMode {
    /// Require synchronous disposal.
    Sync,
    /// Require asynchronous disposal.
    Async,
}

impl CheckModuleState {
    /// Add one solved check obligation.
    pub(in crate::check) fn push_obligation(&mut self, obligation: Obligation) {
        self.work.obligations.push(obligation);
    }

    /// Require one type variable to solve before diagnostics are committed.
    pub(in crate::check) fn require_type(
        &mut self,
        source: dir::LocalNodeIdAny,
        variable: VariableId,
    ) {
        self.push_obligation(Obligation::RequireType {
            source: source.into_global(self.input.module),
            variable,
        });
    }
}
