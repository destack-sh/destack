use crate::check::{
    CheckState, FunctionTerm, Layout, LayoutQuery, OperatorTermKind, Solution, TypeOperand,
};
use destack_dir as dir;

use super::{GenericApplication, VariableId};

impl CheckState<'_> {
    /// Insert one solution known before ordinary solver reduction.
    pub(in crate::check) fn insert_known_solution(
        &mut self,
        variable: VariableId,
        solution: Solution,
    ) {
        let previous = self.inference.variable_solutions.insert(variable, solution);

        assert!(
            previous.is_none(),
            "check variable {variable:?} already has a known solution"
        );
    }
}

/// Resolved contextual receiver selected by check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct ReceiverResolution {
    /// The receiver expression node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The receiver syntax kind.
    pub(in crate::check) kind: dir::ReceiverKind,
    /// The declaration that introduces the receiver, when known.
    pub(in crate::check) owner: Option<dir::GlobalSymbolId>,
    /// The receiver type variable.
    pub(in crate::check) ty: TypeOperand,
}

/// Solved runtime call resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct CallResolution {
    /// The source call expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The resolved call target.
    pub(in crate::check) target: CallResolutionTarget,
    /// The resolved function signature.
    pub(in crate::check) function: FunctionTerm,
}

/// Runtime call target resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum CallResolutionTarget {
    /// Callable value without a declaration symbol.
    Value,
    /// Symbol-backed callable selected at compile time.
    Symbol {
        /// The resolved callable symbol.
        symbol: dir::GlobalSymbolId,
        /// The resolved generic application.
        application: Option<GenericApplication>,
        /// The resolved receiver type for method calls.
        receiver: Option<TypeOperand>,
    },
    /// Symbol-backed callable variants selected from a union receiver.
    Select {
        /// The resolved callable candidates.
        candidates: Vec<SymbolCandidate>,
        /// The resolved receiver type for method calls.
        receiver: Option<TypeOperand>,
    },
    /// Constructor selected through call syntax.
    Constructor {
        /// The resolved constructor symbol.
        symbol: dir::GlobalSymbolId,
        /// The resolved generic application.
        application: Option<GenericApplication>,
    },
}

impl CallResolutionTarget {
    /// Return the constructor symbol and application represented by this target.
    pub(in crate::check) fn as_constructor_target(
        &self,
    ) -> (Option<dir::GlobalSymbolId>, Option<&GenericApplication>) {
        match self {
            Self::Constructor {
                symbol,
                application,
            } => (Some(*symbol), application.as_ref()),
            Self::Value
            | Self::Symbol {
                symbol: _,
                application: _,
                receiver: _,
            }
            | Self::Select {
                candidates: _,
                receiver: _,
            } => (None, None),
        }
    }
}

/// One symbol-backed candidate selected by check.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct SymbolCandidate {
    /// The selected declaration symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The selected generic application.
    pub(in crate::check) application: Option<GenericApplication>,
}

/// Runtime call failure resolved by the solver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) enum CallFailure {
    /// The callee value has no call signature.
    NotCallable,
    /// No callable overload accepts the arguments.
    NoMatch,
    /// One selected callable target rejected an argument type.
    ArgumentType {
        /// The incompatible argument type operand.
        argument: TypeOperand,
        /// The expected parameter type operand.
        parameter: TypeOperand,
    },
}

impl From<ConstructFailure> for CallFailure {
    /// Convert construct failure detail to call failure detail.
    fn from(failure: ConstructFailure) -> Self {
        match failure {
            ConstructFailure::NotConstructible => Self::NotCallable,
            ConstructFailure::NoMatch => Self::NoMatch,
        }
    }
}

/// Runtime call decision resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum CallDecision {
    /// One call target resolved.
    Resolved(CallResolution),
    /// Call resolution failed.
    Rejected(CallFailure),
}

/// Solved runtime construct expression resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct ConstructResolution {
    /// The source construct expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The resolved constructor symbol when construction is symbol backed.
    pub(in crate::check) symbol: Option<dir::GlobalSymbolId>,
    /// The resolved generic application.
    pub(in crate::check) application: Option<GenericApplication>,
    /// The resolved constructor signature.
    pub(in crate::check) function: FunctionTerm,
}

/// Runtime construct failure resolved by the solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum ConstructFailure {
    /// The constructed value has no construct signature.
    NotConstructible,
    /// No construct overload accepts the arguments.
    NoMatch,
}

impl From<CallFailure> for ConstructFailure {
    /// Convert call failure detail to construct failure detail.
    fn from(failure: CallFailure) -> Self {
        match failure {
            CallFailure::NotCallable => Self::NotConstructible,
            CallFailure::NoMatch | CallFailure::ArgumentType { .. } => Self::NoMatch,
        }
    }
}

/// Runtime construct decision resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum ConstructDecision {
    /// One construct target resolved.
    Resolved(ConstructResolution),
    /// Construct resolution failed.
    Rejected(ConstructFailure),
}

/// Solved runtime operator resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum OperatorResolution {
    /// Builtin operator behavior.
    Builtin {
        /// The source operator expression.
        source: dir::GlobalNodeIdAny,
        /// The source operator.
        kind: OperatorTermKind,
        /// The receiver operand type.
        receiver: TypeOperand,
        /// The remaining operand type.
        argument: Option<TypeOperand>,
        /// The result type.
        result: VariableId,
    },
    /// Symbol-backed operator method.
    Method {
        /// The source binary expression.
        source: dir::GlobalNodeIdAny,
        /// The resolved operator method symbol.
        symbol: dir::GlobalSymbolId,
        /// The receiver type.
        receiver: TypeOperand,
        /// The resolved function signature.
        function: FunctionTerm,
    },
}

/// Runtime operator failure resolved by the solver.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct OperatorFailure {
    /// The source operator expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The source operator.
    pub(in crate::check) kind: OperatorTermKind,
    /// The reason operator resolution failed.
    pub(in crate::check) reason: OperatorFailureReason,
}

/// Runtime operator failure reason resolved by the solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum OperatorFailureReason {
    /// No builtin or protocol operator accepted the operands.
    NoMatch,
    /// Strict equality was used with non identity-compatible operands.
    InvalidStrictEquality,
}

/// Runtime operator decision resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum OperatorDecision {
    /// One operator target resolved.
    Resolved(OperatorResolution),
    /// Operator resolution failed.
    Rejected(OperatorFailure),
}

/// Solved identity equality check resolved by the solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct IdentityResolution {
    /// The source identity expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The left operand type.
    pub(in crate::check) left: TypeOperand,
    /// The right operand type.
    pub(in crate::check) right: TypeOperand,
}

/// Runtime identity equality failure resolved by the solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum IdentityFailure {
    /// The operand types do not have a shared identity domain.
    Incompatible,
}

/// Runtime identity equality decision resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum IdentityDecision {
    /// One identity comparison resolved.
    Resolved(IdentityResolution),
    /// Identity comparison resolution failed.
    Rejected(IdentityFailure),
}

/// Solved layout query resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct LayoutResolution {
    /// The source layout query expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The queried type.
    pub(in crate::check) target: TypeOperand,
    /// The requested layout property.
    pub(in crate::check) query: LayoutQuery,
    /// The resolved layout.
    pub(in crate::check) layout: Layout,
}

/// Layout query failure resolved by the solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct LayoutFailure {
    /// The source layout query expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The queried type.
    pub(in crate::check) target: TypeOperand,
    /// The requested layout property.
    pub(in crate::check) query: LayoutQuery,
}

/// Layout query decision resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum LayoutDecision {
    /// One layout resolved.
    Resolved(LayoutResolution),
    /// Layout resolution failed.
    Rejected(LayoutFailure),
}

/// Solved member projection resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct MemberResolution {
    /// The source member expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The receiver type.
    pub(in crate::check) receiver: TypeOperand,
    /// The resolved member target.
    pub(in crate::check) target: MemberResolutionTarget,
}

/// Runtime member failure resolved by the solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum MemberFailure {
    /// The receiver has no such member.
    Missing,
}

/// Runtime member decision resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum MemberDecision {
    /// One member target resolved.
    Resolved(MemberResolution),
    /// Member resolution failed.
    Rejected(MemberFailure),
}

/// Solved member target resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum MemberResolutionTarget {
    /// Compiler builtin member behavior.
    Builtin(dir::BuiltinMember),
    /// Structural field resolved from a shape type.
    Field(dir::StaticKey),
    /// Symbol-backed member resolved from a nominal type.
    Symbol {
        /// The resolved member symbol.
        symbol: dir::GlobalSymbolId,
        /// The resolved generic application.
        application: Option<GenericApplication>,
    },
    /// Symbol-backed members selected from a union receiver.
    Select(Vec<SymbolCandidate>),
}

impl CheckState<'_> {
    /// Select one call decision.
    pub(in crate::check) fn select_call(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: CallDecision,
    ) {
        if let Some(previous) = self.inference.calls.get(&source) {
            assert_eq!(
                previous, &decision,
                "check call {source:?} already has a different decision"
            );

            return;
        }

        self.inference.calls.insert(source, decision);
    }

    /// Select one construct decision.
    pub(in crate::check) fn select_construct(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: ConstructDecision,
    ) {
        if let Some(previous) = self.inference.constructs.get(&source) {
            assert_eq!(
                previous, &decision,
                "check construct {source:?} already has a different decision"
            );

            return;
        }

        self.inference.constructs.insert(source, decision);
    }

    /// Select one operator decision.
    pub(in crate::check) fn select_operator(&mut self, decision: OperatorDecision) {
        let source = match &decision {
            OperatorDecision::Resolved(operator) => match operator {
                OperatorResolution::Builtin { source, .. }
                | OperatorResolution::Method { source, .. } => *source,
            },
            OperatorDecision::Rejected(failure) => failure.source,
        };

        if let Some(previous) = self.inference.operators.get(&source) {
            assert_eq!(
                previous, &decision,
                "check operator {source:?} already has a different decision"
            );

            return;
        }

        self.inference.operators.insert(source, decision);
    }

    /// Select one identity equality decision.
    pub(in crate::check) fn select_identity(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: IdentityDecision,
    ) {
        if let Some(previous) = self.inference.identities.get(&source) {
            assert_eq!(
                previous, &decision,
                "check identity {source:?} already has a different decision"
            );

            return;
        }

        self.inference.identities.insert(source, decision);
    }

    /// Select one concrete layout decision.
    pub(in crate::check) fn select_layout(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: LayoutDecision,
    ) {
        if let Some(previous) = self.inference.layouts.get(&source) {
            assert_eq!(
                previous, &decision,
                "check layout {source:?} already has a different decision"
            );

            return;
        }

        self.inference.layouts.insert(source, decision);
    }

    /// Select one member decision.
    pub(in crate::check) fn select_member(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: MemberDecision,
    ) {
        if let Some(previous) = self.inference.members.get(&source) {
            assert_eq!(
                previous, &decision,
                "check member {source:?} already has a different decision"
            );

            return;
        }

        self.inference.members.insert(source, decision);
    }

    /// Select one receiver resolution.
    pub(in crate::check) fn select_receiver(&mut self, receiver: ReceiverResolution) {
        if let Some(previous) = self.inference.receivers.get(&receiver.source) {
            assert_eq!(
                previous, &receiver,
                "check receiver {:?} already has a different decision",
                receiver.source
            );

            return;
        }

        self.inference.receivers.insert(receiver.source, receiver);
    }

    /// Select one lexical name resolution.
    pub(in crate::check) fn select_name(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) {
        let resolution = dir::NameResolution::new(symbol);
        if let Some(previous) = self.inference.names.get(&source) {
            assert_eq!(
                previous, &resolution,
                "check name {source:?} already has a different decision"
            );

            return;
        }

        self.inference.names.insert(source, resolution);
    }

    /// Return the selected symbol for one resolved lexical name.
    pub(in crate::check) fn selected_name(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        self.inference.names.get(&source)?.symbol()
    }
}
