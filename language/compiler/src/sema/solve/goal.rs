use std::sync::Arc;

use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    CheckFailure, MemberLookup, NewtypeInstance, NewtypeOverload, OperatorExpressionResult,
    ProtocolCall, Relation, Response, SignatureInstance, ValueUse, VariableKind, VariableRole,
    Verdict,
};

/// One goal the solver decides, in canonical form.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(in crate::sema) struct CanonicalGoal {
    /// The decided subject.
    pub(in crate::sema) goal: Goal,
    /// The canonical operands, in goal order.
    pub(in crate::sema) operands: dir::TypeListId,
    /// The assumptions the operands decide under.
    pub(in crate::sema) premise: Premise,
}

/// The subject one goal decides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum Goal {
    /// One decided relation over [source, target].
    Relation(Relation),
    /// One implementation decision over [source, target].
    Implementation(Relation),
    /// The extensions one head exposes over [receiver, subject].
    Sources {
        /// The head whose extensions the goal reaches.
        root: dir::TypeRoot,
        /// The module whose visibility reaches the extensions.
        module: ModuleId,
    },
    /// One extension's deduced arguments over [receiver, subject].
    Extension {
        /// The extension declaration the match decides.
        extension: dir::GlobalSymbolId,
    },
    /// The member one key exposes over [receiver, target].
    Member {
        /// The module whose visibility reaches the members.
        module: ModuleId,
        /// The searched member space.
        space: dir::MemberSpace,
        /// The looked-up member key.
        key: dir::StaticKey,
    },
    /// The callable selected over [expected.., callee, arguments..].
    Selection {
        /// The callable identity canonicalized.
        callee: Callee,
        /// Whether an expectation leads the canonical operands.
        expected: bool,
    },
    /// The union arm selected over [source, targets..].
    Arm {
        /// The expected value use.
        value_use: ValueUse,
        /// Whether the converted value owns an addressable place.
        is_placed: bool,
    },
}

/// The callable identity one selection decides for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum Callee {
    /// A declared callable symbol.
    Symbol(dir::GlobalSymbolId),
    /// A builtin binary operator.
    Operator(dir::BinaryOperator),
    /// A newtype construction under one backing selection rule.
    Newtype(dir::GlobalSymbolId, NewtypeOverload),
}

/// One decided answer, applied without re-derivation.
#[derive(Debug, Clone)]
pub(in crate::sema) enum Answer {
    /// The relation holds.
    Holds,
    /// The relation fails.
    Fails,
    /// The implementation decision with its winner and hole solutions.
    Implement(Arc<Response<Implementation>>),
    /// The matching extensions with their deduced canonical arguments.
    Sources(Arc<Response<Vec<ExtensionSource>>>),
    /// The extension match with its deduced canonical arguments, absent on refusal.
    Extension(Option<Arc<Response<SmallVec<[dir::GlobalTypeId; 4]>>>>),
    /// The member lookup the canonical receiver decided.
    Member(Arc<Response<MemberLookup>>),
    /// The selected union arm by target position, exact or converted, or its failure.
    Arm(Result<(u16, bool), CheckFailure>),
    /// The evaluation stayed undecided, so sites decide in place.
    Undecided,
    /// The callable decision one canonical operand list selected.
    Selection(Arc<Response<Dispatch>>),
}

/// One decided extension-implementation verdict with its winner's match.
#[derive(Debug, Clone, Copy)]
pub(in crate::sema) struct Implementation {
    /// The decided verdict.
    pub(in crate::sema) verdict: Verdict,
    /// The winning implementation, absent on disproof.
    pub(in crate::sema) winner: Option<dir::GlobalSymbolId>,
    /// The winner's substituted target, constrained by each goal site's receiver.
    pub(in crate::sema) target: Option<dir::GlobalTypeId>,
    /// The winner's matched interface application, related to each goal site's request.
    pub(in crate::sema) interface: Option<dir::GlobalTypeId>,
}

/// One matched extension with the arguments its template deduces.
#[derive(Debug, Clone)]
pub(in crate::sema) struct ExtensionSource {
    /// The matched extension declaration.
    pub(in crate::sema) extension: dir::GlobalSymbolId,
    /// The deduced template arguments, in declaration order.
    pub(in crate::sema) arguments: SmallVec<[dir::GlobalTypeId; 4]>,
}

/// One decided dispatch, instantiated per goal site.
#[derive(Debug, Clone)]
pub(in crate::sema) enum Dispatch {
    /// The selected declaration with its instantiated signature.
    Callable(SignatureInstance),
    /// The selected newtype backing without per-site coercions.
    Newtype(NewtypeInstance),
    /// The selected operator protocol call.
    Protocol(ProtocolCall, OperatorExpressionResult),
    /// A builtin binary operator application.
    Builtin {
        /// The applied result type.
        result: dir::GlobalTypeId,
        /// The coerced operand types.
        operands: [dir::GlobalTypeId; 2],
    },
    /// No candidate applies to these operands.
    Rejected,
}

/// One interned bound set inside the solver's premise storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) struct BoundSetId(pub(in crate::sema) u32);

/// The assumptions one canonical goal decides under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum Premise {
    /// The operands are closed and decide once per pass.
    Free,
    /// The renamed parameters assume one interned bound set.
    Bounds(BoundSetId),
    /// The operands stay scoped to their assuming template.
    Scope(Option<dir::GlobalGenericTemplateId>),
}

/// The declared content one renamed parameter assumes, in canonical form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) struct PremiseParameter {
    /// The parameter representation.
    pub(in crate::sema) kind: dir::GenericParameterKind,
    /// Whether the parameter captures remaining arguments.
    pub(in crate::sema) is_variadic: bool,
    /// The canonical declared constraint.
    pub(in crate::sema) constraint: Option<dir::GlobalTypeId>,
    /// The canonical declared default.
    pub(in crate::sema) default: Option<dir::GlobalTypeId>,
}

/// The bound content one goal's renamed parameters assume.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub(in crate::sema) struct BoundSet {
    /// The renamed parameters' declared content, in rigid order.
    pub(in crate::sema) parameters: SmallVec<[PremiseParameter; 4]>,
    /// The assumed where predicates reaching the renamed parameters.
    pub(in crate::sema) predicates:
        SmallVec<[(dir::WhereRelation, dir::GlobalTypeId, dir::GlobalTypeId); 2]>,
    /// The renamed holes' carried content, in hole order.
    pub(in crate::sema) holes: SmallVec<[Hole; 2]>,
}

/// One numbered open root's carried content, shared by goals and answers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(in crate::sema) struct Hole {
    /// What the root ranges over.
    pub(in crate::sema) kind: VariableKind,
    /// The special role the root carries.
    pub(in crate::sema) role: VariableRole,
}

impl dir::TypeFold for Implementation {
    /// Map every type this verdict carries.
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(dir::GlobalTypeId) -> Result<dir::GlobalTypeId, E>,
    ) -> Result<(), E> {
        if let Some(target) = &mut self.target {
            *target = map(*target)?;
        }
        if let Some(interface) = &mut self.interface {
            *interface = map(*interface)?;
        }

        Ok(())
    }
}

impl dir::TypeFold for ExtensionSource {
    /// Map every type this match carries.
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(dir::GlobalTypeId) -> Result<dir::GlobalTypeId, E>,
    ) -> Result<(), E> {
        self.arguments.map_types(map)
    }
}

impl dir::TypeFold for Dispatch {
    /// Map every type this selection carries.
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(dir::GlobalTypeId) -> Result<dir::GlobalTypeId, E>,
    ) -> Result<(), E> {
        match self {
            Self::Callable(instance) => instance.map_types(map),
            Self::Newtype(instance) => instance.map_types(map),
            Self::Protocol(call, _) => call.map_types(map),
            Self::Builtin { result, operands } => {
                result.map_types(map)?;
                operands.map_types(map)
            }
            Self::Rejected => Ok(()),
        }
    }
}
