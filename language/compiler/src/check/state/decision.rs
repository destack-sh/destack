use destack_dir as dir;
use indexmap::IndexMap;

use super::{CheckModuleState, FunctionTerm, GenericInstance, OperatorTermKind, VariableId};

/// Solver decisions recorded for one module.
#[derive(Debug)]
pub(in crate::check) struct CheckDecisionState {
    /// Runtime calls resolved or rejected by solve.
    pub(in crate::check) call: IndexMap<dir::GlobalNodeIdAny, CallOutcome>,
    /// Runtime constructs resolved or rejected by solve.
    pub(in crate::check) construct: IndexMap<dir::GlobalNodeIdAny, ConstructOutcome>,
    /// Runtime operators resolved or rejected by solve.
    pub(in crate::check) operator: IndexMap<dir::GlobalNodeIdAny, OperatorOutcome>,
    /// Runtime key membership checks resolved or rejected by solve.
    pub(in crate::check) key_membership: IndexMap<dir::GlobalNodeIdAny, KeyMembershipOutcome>,
    /// Runtime instance checks resolved or rejected by solve.
    pub(in crate::check) instance_check: IndexMap<dir::GlobalNodeIdAny, InstanceCheckOutcome>,
    /// Runtime identity checks resolved or rejected by solve.
    pub(in crate::check) identity: IndexMap<dir::GlobalNodeIdAny, IdentityOutcome>,
    /// Runtime tagged templates resolved or rejected by solve.
    pub(in crate::check) tagged_template: IndexMap<dir::GlobalNodeIdAny, TaggedTemplateOutcome>,
    /// Runtime members resolved or rejected by solve.
    pub(in crate::check) member: IndexMap<dir::GlobalNodeIdAny, MemberOutcome>,
}

impl CheckDecisionState {
    /// Create empty solver decisions.
    pub(in crate::check) fn new() -> Self {
        Self {
            call: IndexMap::new(),
            construct: IndexMap::new(),
            operator: IndexMap::new(),
            key_membership: IndexMap::new(),
            instance_check: IndexMap::new(),
            identity: IndexMap::new(),
            tagged_template: IndexMap::new(),
            member: IndexMap::new(),
        }
    }
}

/// Solved runtime call resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct CallResolution {
    /// The source call expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The member expression node when a member call was resolved.
    pub(in crate::check) member_source: Option<dir::GlobalNodeIdAny>,
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
    /// Constructible nominal selected through call syntax.
    Construct {
        /// The resolved constructible symbol.
        symbol: dir::GlobalSymbolId,
        /// The resolved generic instance.
        instance: Option<GenericInstance>,
    },
}

/// Runtime call failure resolved by the solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum CallFailure {
    /// The callee value has no call signature.
    NotCallable,
    /// No callable overload accepts the arguments.
    NoMatch,
}

/// Runtime call outcome resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum CallOutcome {
    /// One call target resolved.
    Resolved(CallResolution),
    /// Call resolution failed.
    Rejected(CallFailure),
}

/// Solved runtime construct resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct ConstructResolution {
    /// The source construct expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The resolved constructor symbol when construction is symbol-backed.
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

/// Runtime construct outcome resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum ConstructOutcome {
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

/// Runtime operator outcome resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum OperatorOutcome {
    /// One operator target resolved.
    Resolved(OperatorResolution),
    /// Operator resolution failed.
    Rejected(OperatorFailure),
}

/// Solved key membership check resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum KeyMembershipResolution {
    /// Structural member presence check.
    Structural {
        /// The source membership expression.
        source: dir::GlobalNodeIdAny,
        /// The receiver type.
        receiver: VariableId,
        /// The checked member key.
        key: dir::StaticKey,
    },
    /// Protocol-backed key membership check.
    Protocol {
        /// The source membership expression.
        source: dir::GlobalNodeIdAny,
        /// The resolved protocol method symbol.
        symbol: dir::GlobalSymbolId,
        /// The receiver type.
        receiver: VariableId,
        /// The resolved function signature.
        function: FunctionTerm,
    },
}

/// Runtime key membership failure resolved by the solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum KeyMembershipFailure {
    /// The receiver type has no matching key relation.
    NoMatch,
}

/// Runtime key membership outcome resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum KeyMembershipOutcome {
    /// One key membership resolution chosen.
    Resolved(KeyMembershipResolution),
    /// Key membership resolution failed.
    Rejected(KeyMembershipFailure),
}

/// Solved nominal instance check resolved by the solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct InstanceCheckResolution {
    /// The source instance check expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The checked value type.
    pub(in crate::check) value: VariableId,
    /// The nominal target symbol.
    pub(in crate::check) target: dir::GlobalSymbolId,
}

/// Runtime instance check failure resolved by the solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum InstanceCheckFailure {
    /// The target expression is not a runtime nominal type.
    InvalidTarget,
    /// The value cannot be checked against the target.
    NoMatch,
}

/// Runtime instance check outcome resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum InstanceCheckOutcome {
    /// One instance check resolved.
    Resolved(InstanceCheckResolution),
    /// Instance check resolution failed.
    Rejected(InstanceCheckFailure),
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

/// Runtime identity equality outcome resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum IdentityOutcome {
    /// One identity comparison resolved.
    Resolved(IdentityResolution),
    /// Identity comparison resolution failed.
    Rejected(IdentityFailure),
}

/// Solved tagged template call resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct TaggedTemplateResolution {
    /// The source tagged template expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The resolved tag function symbol.
    pub(in crate::check) symbol: Option<dir::GlobalSymbolId>,
    /// The resolved generic instance.
    pub(in crate::check) instance: Option<GenericInstance>,
    /// The resolved function signature.
    pub(in crate::check) function: FunctionTerm,
}

/// Runtime tagged template failure resolved by the solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum TaggedTemplateFailure {
    /// The tag value has no compatible call signature.
    NoMatch,
}

/// Runtime tagged template outcome resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum TaggedTemplateOutcome {
    /// One tag call target resolved.
    Resolved(TaggedTemplateResolution),
    /// Tagged template resolution failed.
    Rejected(TaggedTemplateFailure),
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

/// Runtime member outcome resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum MemberOutcome {
    /// One member target resolved.
    Resolved(MemberResolution),
    /// Member resolution failed.
    Rejected(MemberFailure),
}

/// Solved member target resolved by the solver.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum MemberResolutionTarget {
    /// Structural field resolved from a shape type.
    Field(dir::StaticKey),
    /// Symbol-backed member resolved from a nominal type.
    Symbol {
        /// The resolved member symbol.
        symbol: dir::GlobalSymbolId,
        /// The resolved generic instance.
        instance: Option<GenericInstance>,
    },
}

impl CheckModuleState {
    /// Record one call outcome.
    pub(in crate::check) fn record_call_outcome(
        &mut self,
        source: dir::GlobalNodeIdAny,
        outcome: CallOutcome,
    ) {
        self.decisions.call.insert(source, outcome);
    }

    /// Record one construct outcome.
    pub(in crate::check) fn record_construct_outcome(
        &mut self,
        source: dir::GlobalNodeIdAny,
        outcome: ConstructOutcome,
    ) {
        self.decisions.construct.insert(source, outcome);
    }

    /// Record one operator outcome.
    pub(in crate::check) fn record_operator_outcome(&mut self, outcome: OperatorOutcome) {
        let source = match &outcome {
            OperatorOutcome::Resolved(operator) => match operator {
                OperatorResolution::Builtin { source, .. }
                | OperatorResolution::Method { source, .. } => *source,
            },
            OperatorOutcome::Rejected(failure) => failure.source,
        };

        self.decisions.operator.insert(source, outcome);
    }

    /// Record one key membership outcome.
    pub(in crate::check) fn record_key_membership_outcome(
        &mut self,
        source: dir::GlobalNodeIdAny,
        outcome: KeyMembershipOutcome,
    ) {
        self.decisions.key_membership.insert(source, outcome);
    }

    /// Record one instance check outcome.
    pub(in crate::check) fn record_instance_check_outcome(
        &mut self,
        source: dir::GlobalNodeIdAny,
        outcome: InstanceCheckOutcome,
    ) {
        self.decisions.instance_check.insert(source, outcome);
    }

    /// Record one identity equality outcome.
    pub(in crate::check) fn record_identity_outcome(
        &mut self,
        source: dir::GlobalNodeIdAny,
        outcome: IdentityOutcome,
    ) {
        self.decisions.identity.insert(source, outcome);
    }

    /// Record one tagged template outcome.
    pub(in crate::check) fn record_tagged_template_outcome(
        &mut self,
        source: dir::GlobalNodeIdAny,
        outcome: TaggedTemplateOutcome,
    ) {
        self.decisions.tagged_template.insert(source, outcome);
    }

    /// Record one member outcome.
    pub(in crate::check) fn record_member_outcome(
        &mut self,
        source: dir::GlobalNodeIdAny,
        outcome: MemberOutcome,
    ) {
        self.decisions.member.insert(source, outcome);
    }
}
