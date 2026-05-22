use destack_dir as dir;

use crate::DiagnosticAnchor;

use super::{Predicate, TypeRelation, VariableId};

/// Rule validated after solving.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct Obligation {
    /// The guard that controls this obligation.
    pub(in crate::check) guard: Predicate,
    /// The diagnostic context.
    pub(in crate::check) context: ObligationContext,
    /// The obligation payload.
    pub(in crate::check) kind: ObligationKind,
}

impl Obligation {
    /// Create one type relation obligation.
    pub(in crate::check) fn type_relation(
        guard: Predicate,
        relation: TypeRelation,
        left: VariableId,
        right: VariableId,
        site: ObligationSite,
        anchor: DiagnosticAnchor,
    ) -> Self {
        Self {
            guard,
            context: ObligationContext { anchor, site },
            kind: ObligationKind::Type {
                relation,
                left,
                right,
            },
        }
    }

    /// Create one required variable obligation.
    pub(in crate::check) fn required(
        guard: Predicate,
        variable: VariableId,
        site: ObligationSite,
        anchor: DiagnosticAnchor,
    ) -> Self {
        Self {
            guard,
            context: ObligationContext { anchor, site },
            kind: ObligationKind::Required { variable },
        }
    }
}

/// Obligation payload.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum ObligationKind {
    /// Type relation that must hold after solving.
    Type {
        /// The required relation.
        relation: TypeRelation,
        /// The left type.
        left: VariableId,
        /// The right type.
        right: VariableId,
    },
    /// Variable must be solved.
    Required {
        /// The required variable.
        variable: VariableId,
    },
}

/// Diagnostic context for an obligation.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct ObligationContext {
    /// The diagnostic anchor.
    pub(in crate::check) anchor: DiagnosticAnchor,
    /// The obligation source context.
    pub(in crate::check) site: ObligationSite,
}

/// Source of an obligation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) enum ObligationSite {
    /// Declaration type annotation.
    DeclarationType {
        /// The declaration node.
        declaration: dir::GlobalNodeIdAny,
    },
    /// Variable initializer assignment.
    VariableInitializer {
        /// The variable symbol.
        symbol: dir::GlobalSymbolId,
    },
    /// Parameter default assignment.
    ParameterDefault {
        /// The parameter symbol.
        parameter: dir::GlobalSymbolId,
    },
    /// Function return assignment.
    FunctionReturn {
        /// The function symbol.
        function: dir::GlobalSymbolId,
        /// The returned node.
        return_node: dir::GlobalNodeIdAny,
    },
    /// Call argument assignment.
    CallArgument {
        /// The call node.
        call: dir::GlobalNodeIdAny,
        /// The parameter symbol, when known.
        parameter: Option<dir::GlobalSymbolId>,
        /// The argument index.
        index: usize,
    },
    /// Generic or static argument bound check.
    GenericArgument {
        /// The generic application node.
        application: dir::GlobalNodeIdAny,
        /// The parameter symbol, when known.
        parameter: Option<dir::GlobalSymbolId>,
        /// The argument index.
        index: usize,
    },
    /// Static condition.
    StaticCondition {
        /// The static condition node.
        condition: dir::GlobalNodeIdAny,
    },
    /// Dynamic condition.
    DynamicCondition {
        /// The dynamic condition node.
        condition: dir::GlobalNodeIdAny,
    },
    /// Layout intrinsic.
    LayoutIntrinsic {
        /// The intrinsic expression node.
        intrinsic: dir::GlobalNodeIdAny,
    },
}
