use destack_dir as dir;
use indexmap::IndexMap;

use crate::check::{
    CallInstantiation, CallInstantiationKey, CheckState, FunctionTerm, Layout, LayoutQuery,
    OperatorTermKind, Solution, StaticOperand, TypeOperand,
};

use super::{GenericInstance, VariableId};

/// Solver solutions for one check component.
#[derive(Debug)]
pub(in crate::check) struct SolutionTable {
    /// Solved variable values.
    pub(in crate::check) variable: IndexMap<VariableId, Solution>,

    /// Type operands that must be assignable to each variable.
    pub(in crate::check) type_lower: IndexMap<VariableId, Vec<TypeOperand>>,
    /// Type operands that each variable must be assignable to.
    pub(in crate::check) type_upper: IndexMap<VariableId, Vec<TypeOperand>>,
    /// Static operands that must be assignable to each variable.
    pub(in crate::check) static_lower: IndexMap<VariableId, Vec<StaticOperand>>,
    /// Static operands that each variable must be assignable to.
    pub(in crate::check) static_upper: IndexMap<VariableId, Vec<StaticOperand>>,

    /// Runtime calls resolved or rejected by solve.
    pub(in crate::check) call: IndexMap<dir::GlobalNodeIdAny, CallSelection>,
    /// Call generic instantiations keyed by use site and selected target.
    pub(in crate::check) call_instantiation: IndexMap<CallInstantiationKey, CallInstantiation>,
    /// Runtime construct expressions resolved or rejected by solve.
    pub(in crate::check) construct: IndexMap<dir::GlobalNodeIdAny, ConstructSelection>,
    /// Runtime operators resolved or rejected by solve.
    pub(in crate::check) operator: IndexMap<dir::GlobalNodeIdAny, OperatorSelection>,
    /// Runtime identity checks resolved or rejected by solve.
    pub(in crate::check) identity: IndexMap<dir::GlobalNodeIdAny, IdentitySelection>,
    /// Layout queries resolved or rejected by solve.
    pub(in crate::check) layout: IndexMap<dir::GlobalNodeIdAny, LayoutSelection>,
    /// Runtime members resolved or rejected by solve.
    pub(in crate::check) member: IndexMap<dir::GlobalNodeIdAny, MemberSelection>,
    /// Contextual receivers resolved by check.
    pub(in crate::check) receiver: IndexMap<dir::GlobalNodeIdAny, ReceiverSelection>,
    /// Lexical names resolved by check.
    pub(in crate::check) name: IndexMap<dir::GlobalNodeIdAny, NameSelection>,
}

impl SolutionTable {
    /// Create empty solver solutions.
    pub(in crate::check) fn new() -> Self {
        Self {
            variable: IndexMap::new(),
            call_instantiation: IndexMap::new(),
            type_lower: IndexMap::new(),
            type_upper: IndexMap::new(),
            static_lower: IndexMap::new(),
            static_upper: IndexMap::new(),
            call: IndexMap::new(),
            construct: IndexMap::new(),
            operator: IndexMap::new(),
            identity: IndexMap::new(),
            layout: IndexMap::new(),
            member: IndexMap::new(),
            receiver: IndexMap::new(),
            name: IndexMap::new(),
        }
    }
}

impl CheckState<'_> {
    /// Insert one solution known before ordinary solver reduction.
    pub(in crate::check) fn insert_known_solution(
        &mut self,
        variable: VariableId,
        solution: Solution,
    ) {
        let previous = self.solutions.variable.insert(variable, solution);

        assert!(
            previous.is_none(),
            "check variable {variable:?} already has a known solution"
        );
    }
}

/// Resolved lexical name selected by check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct NameResolution {
    /// The selected symbols in declaration order.
    pub(in crate::check) symbols: Vec<dir::GlobalSymbolId>,
}

impl NameResolution {
    /// Create a single-symbol name resolution.
    pub(in crate::check) fn new(symbol: dir::GlobalSymbolId) -> Self {
        Self {
            symbols: vec![symbol],
        }
    }
}

/// Lexical name decision selected by check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) enum NameSelection {
    /// One lexical name was resolved.
    Resolved(NameResolution),
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
    pub(in crate::check) ty: VariableId,
}

/// Contextual receiver decision selected by check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum ReceiverSelection {
    /// One contextual receiver was resolved.
    Resolved(ReceiverResolution),
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
        /// The resolved generic instance.
        instance: Option<GenericInstance>,
        /// The resolved receiver type for method calls.
        receiver: Option<VariableId>,
    },
    /// Symbol-backed callable variants selected from a union receiver.
    Select {
        /// The resolved callable candidates.
        candidates: Vec<SymbolCandidate>,
        /// The resolved receiver type for method calls.
        receiver: Option<VariableId>,
    },
    /// Constructor selected through call syntax.
    Constructor {
        /// The resolved constructor symbol.
        symbol: dir::GlobalSymbolId,
        /// The resolved generic instance.
        instance: Option<GenericInstance>,
    },
}

impl CallResolutionTarget {
    /// Return the constructor symbol and instance represented by this target.
    pub(in crate::check) fn as_constructor_target(
        &self,
    ) -> (Option<dir::GlobalSymbolId>, Option<&GenericInstance>) {
        match self {
            Self::Constructor { symbol, instance } => (Some(*symbol), instance.as_ref()),
            Self::Value
            | Self::Symbol {
                symbol: _,
                instance: _,
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
    /// The selected generic instance.
    pub(in crate::check) instance: Option<GenericInstance>,
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
pub(in crate::check) enum CallSelection {
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
    /// The resolved generic instance.
    pub(in crate::check) instance: Option<GenericInstance>,
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
pub(in crate::check) enum ConstructSelection {
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
        receiver: VariableId,
        /// The remaining operand type.
        argument: Option<VariableId>,
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
        receiver: VariableId,
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
pub(in crate::check) enum OperatorSelection {
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
    pub(in crate::check) left: VariableId,
    /// The right operand type.
    pub(in crate::check) right: VariableId,
}

/// Runtime identity equality failure resolved by the solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum IdentityFailure {
    /// The operand types do not have a shared identity domain.
    Incompatible,
}

/// Runtime identity equality decision resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum IdentitySelection {
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
    pub(in crate::check) target: VariableId,
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
    pub(in crate::check) target: VariableId,
    /// The requested layout property.
    pub(in crate::check) query: LayoutQuery,
}

/// Layout query decision resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum LayoutSelection {
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
    pub(in crate::check) receiver: VariableId,
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
pub(in crate::check) enum MemberSelection {
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
        /// The resolved generic instance.
        instance: Option<GenericInstance>,
    },
    /// Symbol-backed members selected from a union receiver.
    Select(Vec<SymbolCandidate>),
}

impl CheckState<'_> {
    /// Select one call decision.
    pub(in crate::check) fn select_call(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: CallSelection,
    ) {
        self.solutions.call.insert(source, decision);
    }

    /// Select one construct decision.
    pub(in crate::check) fn select_construct(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: ConstructSelection,
    ) {
        self.solutions.construct.insert(source, decision);
    }

    /// Select one operator decision.
    pub(in crate::check) fn select_operator(&mut self, decision: OperatorSelection) {
        let source = match &decision {
            OperatorSelection::Resolved(operator) => match operator {
                OperatorResolution::Builtin { source, .. }
                | OperatorResolution::Method { source, .. } => *source,
            },
            OperatorSelection::Rejected(failure) => failure.source,
        };

        self.solutions.operator.insert(source, decision);
    }

    /// Select one identity equality decision.
    pub(in crate::check) fn select_identity(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: IdentitySelection,
    ) {
        self.solutions.identity.insert(source, decision);
    }

    /// Select one concrete layout decision.
    pub(in crate::check) fn select_layout(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: LayoutSelection,
    ) {
        self.solutions.layout.insert(source, decision);
    }

    /// Select one member decision.
    pub(in crate::check) fn select_member(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: MemberSelection,
    ) {
        self.solutions.member.insert(source, decision);
    }

    /// Select one receiver resolution.
    pub(in crate::check) fn select_receiver(&mut self, receiver: ReceiverResolution) {
        self.solutions
            .receiver
            .insert(receiver.source, ReceiverSelection::Resolved(receiver));
    }

    /// Select one lexical name resolution.
    pub(in crate::check) fn select_name(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) {
        let resolution = NameResolution::new(symbol);

        self.solutions
            .name
            .insert(source, NameSelection::Resolved(resolution));
    }
}
