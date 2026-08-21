use destack_core::FxIndexMap;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{
    Cause, CauseId, CauseKind, CheckState, Expectation, FlowSite, GenericTemplateId,
    ObligationEntry, Origin, PlaceUse, Relation, TypeSubstitution, Verdict,
};
use crate::{CompilerError, CompilerResult};

/// Component-global id of one collected check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) struct CheckId(u32);

impl CheckId {
    /// Return the check id at one index.
    pub(in crate::sema) fn at(index: usize) -> Self {
        Self(index as u32)
    }

    /// Return the check index.
    pub(in crate::sema) fn index(self) -> usize {
        self.0 as usize
    }
}

/// One unit of pending checking work: a claim to decide, or a blocked step to resume.
#[derive(Debug, Clone)]
pub(in crate::sema) enum Check {
    // claims deciding to verdicts
    /// A relation must hold between two types.
    Relation(RelationCheck),
    /// A declared obligation must hold once its assuming scope closes.
    Declared(ObligationEntry),

    // blocked steps resuming into the walker
    /// A node blocked by an inference barrier resumes its whole check.
    Node(NodeCheck),
    /// A checked value resumes its conversion once its expected type closes.
    Conversion(ConversionCheck),
    /// A flow narrowing resumes once its consulted operation decides.
    Narrowing(NarrowingCheck),
    /// A node's selection resumes once its blocking variable solves.
    Selection(SelectionCheck),
    /// A switch equality selection resumes once its open operand solves.
    Equality(EqualityCheck),
}

/// One relation checked between two types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) struct RelationCheck {
    /// The type evaluation site.
    pub(in crate::sema) origin: Origin,
    /// The relation to enforce.
    pub(in crate::sema) relation: Relation,
    /// The source operand.
    pub(in crate::sema) source: dir::GlobalTypeId,
    /// The target operand.
    pub(in crate::sema) target: dir::GlobalTypeId,
    /// Why this check exists.
    pub(in crate::sema) cause: CauseId,
    /// The generic application this bound guards, when any.
    pub(in crate::sema) application: Option<dir::GlobalTypeId>,
}

impl dir::TypeFold for RelationCheck {
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(dir::GlobalTypeId) -> Result<dir::GlobalTypeId, E>,
    ) -> Result<(), E> {
        self.source.map_types(map)?;
        self.target.map_types(map)?;
        self.application.map_types(map)
    }
}

impl RelationCheck {
    /// Create a pure relation between two types.
    pub(in crate::sema) fn new(
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        cause: CauseId,
    ) -> Self {
        Self {
            origin,
            relation,
            source,
            target,
            cause,
            application: None,
        }
    }

    /// Create a generic argument bound check.
    pub(in crate::sema) fn generic_bound(
        origin: Origin,
        argument: dir::GlobalTypeId,
        bound: dir::GlobalTypeId,
        application: dir::GlobalTypeId,
        cause: CauseId,
    ) -> Self {
        Self {
            origin,
            relation: Relation::Satisfies,
            source: argument,
            target: bound,
            cause,
            application: Some(application),
        }
    }
}

/// One node awaiting its whole check once an inference barrier opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) struct NodeCheck {
    /// The blocked site.
    pub(in crate::sema) site: FlowSite,
    /// The expectation the node re-checks under.
    pub(in crate::sema) expectation: Expectation,
}

/// One checked value awaiting its expected type to convert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) struct ConversionCheck {
    /// The converted site.
    pub(in crate::sema) site: FlowSite,
    /// The checked source value.
    pub(in crate::sema) source: Value,
    /// The conversion expectation.
    pub(in crate::sema) expectation: Expectation,
}

/// One flow narrowing awaiting its consulted operation's decision.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(in crate::sema) struct NarrowingCheck {
    /// The narrowed site.
    pub(in crate::sema) site: FlowSite,
    /// The narrowed access path.
    pub(in crate::sema) path: dir::AccessPath,
    /// The equality operation the narrowing consults.
    pub(in crate::sema) operation: dir::GlobalNodeIdAny,
    /// The unnarrowed base type.
    pub(in crate::sema) source: dir::GlobalTypeId,
    /// The hole standing for the narrowed result until this check completes.
    pub(in crate::sema) hole: dir::TypeVariableId,
}

/// One node selection awaiting a blocking variable to solve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) struct SelectionCheck {
    /// The reselected site.
    pub(in crate::sema) site: FlowSite,
    /// The value use reattempted at the site.
    pub(in crate::sema) use_: PlaceUse,
    /// The open variable the selection stalled on, when known.
    pub(in crate::sema) stalled_on: Option<dir::TypeVariableId>,
}

/// One switch equality selection stalled on an open operand.
#[derive(Debug, Clone)]
pub(in crate::sema) struct EqualityCheck {
    /// The switch value node the selection reports through.
    pub(in crate::sema) value: dir::GlobalNodeIdAny,
    /// The scrutinee type.
    pub(in crate::sema) scrutinee: dir::GlobalTypeId,
    /// The admitted cases with their selector nodes and types.
    pub(in crate::sema) cases: Vec<(
        dir::GlobalNodeIdAny,
        dir::GlobalNodeIdAny,
        dir::GlobalTypeId,
    )>,
    /// The open variable the selection waits for.
    pub(in crate::sema) stalled_on: dir::TypeVariableId,
}

/// One contextual value role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum ValueUse {
    /// Value assigned into a storage or pattern target.
    Store,
    /// Value assigned into a call or subscript parameter.
    Argument,
    /// Value observed by a compiler-defined operator.
    Operand,
    /// Value evaluated as compile-time decorator data.
    Const,
    /// Function body value assigned into a return or yield result.
    Output,
    /// Control-flow condition assigned to boolean.
    Condition,
    /// Value related against a written type without taking it.
    Satisfies,
}

impl ValueUse {
    /// Return whether this use writes a value into a storage destination.
    pub(in crate::sema) fn requires_storage(self) -> bool {
        match self {
            Self::Store | Self::Argument | Self::Output => true,
            Self::Operand | Self::Const | Self::Condition | Self::Satisfies => false,
        }
    }

    /// Return whether this use requires a runtime coercion.
    pub(in crate::sema) fn requires_runtime_coercion(self) -> bool {
        match self {
            Self::Store | Self::Argument | Self::Operand | Self::Output => true,
            Self::Const | Self::Condition | Self::Satisfies => false,
        }
    }
}

/// The stepping state of one collected check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum WorkState {
    /// Queued to step.
    Ready,
    /// Awaiting one of its wake events.
    Waiting,
    /// Completed or cancelled.
    Done,
}

/// One collected check with its outcome and scheduling state.
#[derive(Debug, Clone)]
pub(in crate::sema) struct CheckEntry {
    /// The collected check.
    pub(in crate::sema) check: Check,
    /// The decided outcome, empty while the check stays open.
    pub(in crate::sema) result: Option<CheckOutcome>,
    /// The stepping state.
    pub(in crate::sema) state: WorkState,
    /// The open variables this check's completion can still bound.
    pub(in crate::sema) produces: SmallVec<[dir::TypeVariableId; 2]>,
}

/// Collected checks with their outcomes and scheduling state, in allocation order.
#[derive(Debug, Default)]
pub(in crate::sema) struct CheckTable {
    /// The collected rows indexed by check id.
    pub(in crate::sema) entries: Vec<CheckEntry>,
    /// Ids of collected relation checks by value, so one task collects once.
    relations: FxIndexMap<RelationCheck, CheckId>,
    /// Ids of collected node checks by value, so one task collects once.
    nodes: FxIndexMap<NodeCheck, CheckId>,
    /// Ids of collected conversion checks by value, so one task collects once.
    conversions: FxIndexMap<ConversionCheck, CheckId>,
    /// Ids of collected narrowing checks by value, so one task collects once.
    narrowings: FxIndexMap<NarrowingCheck, CheckId>,
    /// Ids of collected selection checks by value, so one task collects once.
    selections: FxIndexMap<SelectionCheck, CheckId>,
}

impl CheckTable {
    /// Create an empty check table.
    pub(in crate::sema) fn new() -> Self {
        Self::default()
    }

    /// Allocate one check, interning it when its kind collects by value.
    pub(in crate::sema) fn allocate(&mut self, check: Check) -> CheckId {
        match check {
            Check::Relation(relation) => self.allocate_relation(relation),
            Check::Declared(entry) => self.push(Check::Declared(entry)),
            Check::Node(node) => self.allocate_node(node),
            Check::Conversion(conversion) => self.allocate_conversion(conversion),
            Check::Narrowing(narrowing) => self.allocate_narrowing(narrowing),
            Check::Selection(selection) => self.allocate_selection(selection),
            Check::Equality(equality) => self.push(Check::Equality(equality)),
        }
    }

    /// Intern one relation check, returning its existing id when already collected.
    pub(in crate::sema) fn allocate_relation(&mut self, relation: RelationCheck) -> CheckId {
        if let Some(id) = self.relations.get(&relation) {
            return *id;
        }

        let id = self.push(Check::Relation(relation));
        self.relations.insert(relation, id);

        id
    }

    /// Intern one node check, returning its existing id when already collected.
    fn allocate_node(&mut self, node: NodeCheck) -> CheckId {
        if let Some(id) = self.nodes.get(&node) {
            return *id;
        }

        let id = self.push(Check::Node(node));
        self.nodes.insert(node, id);

        id
    }

    /// Intern one conversion check, returning its existing id when already collected.
    fn allocate_conversion(&mut self, conversion: ConversionCheck) -> CheckId {
        if let Some(id) = self.conversions.get(&conversion) {
            return *id;
        }

        let id = self.push(Check::Conversion(conversion));
        self.conversions.insert(conversion, id);

        id
    }

    /// Intern one narrowing check, returning its existing id when already collected.
    fn allocate_narrowing(&mut self, narrowing: NarrowingCheck) -> CheckId {
        if let Some(id) = self.narrowings.get(&narrowing) {
            return *id;
        }

        let id = self.push(Check::Narrowing(narrowing.clone()));
        self.narrowings.insert(narrowing, id);

        id
    }

    /// Intern one selection check, returning its existing id when already collected.
    fn allocate_selection(&mut self, selection: SelectionCheck) -> CheckId {
        if let Some(id) = self.selections.get(&selection) {
            return *id;
        }

        let id = self.push(Check::Selection(selection));
        self.selections.insert(selection, id);

        id
    }

    /// Append one check at the next id, leaving it unscheduled.
    fn push(&mut self, check: Check) -> CheckId {
        let id = CheckId::at(self.entries.len());
        self.entries.push(CheckEntry {
            check,
            result: None,
            state: WorkState::Done,
            produces: SmallVec::new(),
        });

        id
    }

    /// Truncate checks undone by one probe rollback.
    pub(in crate::sema) fn truncate(&mut self, count: usize) {
        self.entries.truncate(count);

        // drop the interned entries naming truncated checks
        self.relations.retain(|_, id| id.index() < count);
        self.nodes.retain(|_, id| id.index() < count);
        self.conversions.retain(|_, id| id.index() < count);
        self.narrowings.retain(|_, id| id.index() < count);
        self.selections.retain(|_, id| id.index() < count);
    }

    /// Return one check.
    pub(in crate::sema) fn get(&self, id: CheckId) -> CompilerResult<&Check> {
        self.entries
            .get(id.index())
            .map(|row| &row.check)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check {id:?} is not allocated"),
            })
    }

    /// Iterate over the collected checks with their ids.
    pub(in crate::sema) fn iter(&self) -> impl Iterator<Item = (CheckId, &Check)> {
        self.entries
            .iter()
            .enumerate()
            .map(|(index, row)| (CheckId::at(index), &row.check))
    }

    /// Iterate over failed relation checks at or after one allocation index.
    pub(in crate::sema) fn relation_failures_from(
        &self,
        start: usize,
    ) -> impl Iterator<Item = CheckId> + '_ {
        self.entries
            .iter()
            .enumerate()
            .skip(start)
            .filter_map(|(index, row)| {
                let is_failed_relation = matches!(row.check, Check::Relation(_))
                    && row
                        .result
                        .as_ref()
                        .is_some_and(|result| matches!(result, CheckOutcome::Fails(_)));

                is_failed_relation.then(|| CheckId::at(index))
            })
    }

    /// Return one completed check outcome.
    pub(in crate::sema) fn result(&self, id: CheckId) -> CompilerResult<Option<&CheckOutcome>> {
        self.entries
            .get(id.index())
            .map(|row| row.result.as_ref())
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check {id:?} has no result slot"),
            })
    }

    /// Replace one check's completed outcome.
    ///
    /// The table stores decided outcomes only; an undecided check keeps its
    /// empty slot and its queued work.
    pub(in crate::sema) fn set_result(
        &mut self,
        id: CheckId,
        result: Option<CheckOutcome>,
    ) -> CompilerResult<()> {
        // reject completing a check as undecided
        if result == Some(CheckOutcome::Pending) {
            return Err(CompilerError::Internal {
                message: format!("check {id:?} cannot complete as pending"),
            });
        }

        let row = self
            .entries
            .get_mut(id.index())
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check {id:?} has no result slot"),
            })?;

        // reject a resumption completing with a verdict
        if matches!(
            row.check,
            Check::Node(_)
                | Check::Conversion(_)
                | Check::Narrowing(_)
                | Check::Selection(_)
                | Check::Equality(_)
        ) {
            return Err(CompilerError::Internal {
                message: format!("resumption {id:?} cannot complete with a result"),
            });
        }
        row.result = result;

        Ok(())
    }

    /// Return one check's stepping state.
    pub(in crate::sema) fn state(&self, id: CheckId) -> WorkState {
        self.entries[id.index()].state
    }

    /// Return whether one check finished solving.
    pub(in crate::sema) fn is_complete(&self, id: CheckId) -> bool {
        self.result(id).is_ok_and(|result| result.is_some())
    }

    /// Return the number of collected checks.
    pub(in crate::sema) fn count(&self) -> usize {
        self.entries.len()
    }
}

/// One failed check retained until its cause tree is complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) struct FailedCheck {
    /// The cause that produced the check.
    pub(in crate::sema) cause: CauseId,
    /// The relation that failed.
    pub(in crate::sema) relation: Relation,
    /// The checked value use, when the check consumed a value.
    pub(in crate::sema) use_: Option<ValueUse>,
    /// The checked source type.
    pub(in crate::sema) source: dir::GlobalTypeId,
    /// The expected target type.
    pub(in crate::sema) target: dir::GlobalTypeId,
    /// The failure reason.
    pub(in crate::sema) failure: CheckFailure,
    /// Whether the check judged over open variables, so its failure is re-judged once they close.
    pub(in crate::sema) is_provisional: bool,
}

/// Reason one closed check did not hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum CheckFailure {
    /// The relation itself did not hold.
    Relation,
    /// The check stayed ambiguous once inference closed: an annotation decides it.
    Undecided,
    /// A value converts to more than one represented union case.
    AmbiguousUnionCoercion,
    /// The source cannot erase behind an erased carrier target.
    NotErasable,
    /// Direct property literal missed one required key.
    MissingRequiredProperty {
        /// The missing key.
        key: dir::StaticKey,
    },
    /// Direct property literal supplied one unknown key.
    ExcessProperty {
        /// The excess key.
        key: dir::StaticKey,
    },
    /// Source type cannot satisfy one writable index signature target.
    WritableIndexRequiresIndexSet {
        /// The required writable index signature.
        signature: dir::TypeIndexSignature,
    },
    /// An inner site reported the failure, so the outer check stays silent.
    Reported,
}

/// Result of checking one source against one target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum CheckOutcome {
    /// The check holds.
    Holds,
    /// The check failed for one known reason.
    Fails(CheckFailure),
    /// The check is undecided over open variables: the queue re-checks it.
    Pending,
}

/// Result of checking one value against its contextual target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) struct ValueCheck {
    /// The checked source type at its flow site.
    pub(in crate::sema) source: dir::GlobalTypeId,
    /// The type a slot stores for the value: the source as converted.
    pub(in crate::sema) stored: dir::GlobalTypeId,
    /// Whether value checking held.
    pub(in crate::sema) outcome: CheckOutcome,
    /// The concrete contextual target.
    pub(in crate::sema) target: dir::GlobalTypeId,
}

/// One checked runtime value and its addressable storage, when present.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) struct Value {
    /// The checked value type.
    pub(in crate::sema) ty: dir::GlobalTypeId,
    /// The authored node producing the value, when any.
    pub(in crate::sema) node: Option<dir::GlobalNodeIdAny>,
    /// The storage designated by the source expression.
    pub(in crate::sema) place: Option<dir::PlaceResolution>,
    /// Whether the value is a literal fresh from its expression, still open to widening.
    pub(in crate::sema) is_fresh: bool,
}

/// Result of converting one checked value to its expected type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::sema) struct ValueConversion {
    /// Whether the conversion held.
    pub(in crate::sema) outcome: CheckOutcome,
    /// The source type as converted, a widened literal taking its inference variable.
    pub(in crate::sema) source: dir::GlobalTypeId,
    /// The concrete target used after inference completed.
    pub(in crate::sema) target: dir::GlobalTypeId,
    /// The required runtime coercion.
    pub(in crate::sema) coercion: Option<Box<dir::Coercion>>,
}

impl CheckOutcome {
    /// Return this check followed by another check.
    pub(in crate::sema) fn and(self, next: Self) -> Self {
        match self {
            // yield to the next check
            Self::Holds => next,
            // keep a decided failure
            Self::Fails(_) => self,
            // outrank a pending check with a decided failure
            Self::Pending => match next {
                Self::Fails(_) => next,
                _ => Self::Pending,
            },
        }
    }
}

/// Applicability of one target-sensitive check path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum CheckAttempt {
    /// The expression form does not use this target directly.
    NotApplicable,
    /// The expression form checked against this target.
    Checked(ValueCheck),
}

impl CheckState<'_> {
    /// Relate the bounds and predicates of one matched substitution.
    pub(in crate::sema) fn relate_substitution_constraints(
        &mut self,
        origin: Origin,
        template: GenericTemplateId,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<bool> {
        let Some(ambiguous) =
            self.evaluate_substitution_constraints(origin, template, substitution)?
        else {
            return Ok(false);
        };

        // queue the undecided checks behind their blocking variables
        for check in ambiguous {
            self.push_relation(check)?;
        }

        Ok(true)
    }

    /// Return whether one application's substituted constraints may still hold.
    pub(in crate::sema) fn substitution_constraints_may_hold(
        &mut self,
        origin: Origin,
        template: GenericTemplateId,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<bool> {
        let ambiguous = self.evaluate_substitution_constraints(origin, template, substitution)?;

        Ok(ambiguous.is_some())
    }

    /// Evaluate one application's constraints, returning its undecided checks.
    fn evaluate_substitution_constraints(
        &mut self,
        origin: Origin,
        template: GenericTemplateId,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<Option<SmallVec<[RelationCheck; 4]>>> {
        // substitute the bounds and predicates this application declares
        let checks = self.substitute_constraint_checks(origin, template, substitution)?;

        // require every substituted declaration check
        let mut ambiguous = SmallVec::new();
        for check in checks {
            let verdict =
                self.evaluate_relation(check.origin, check.relation, check.source, check.target)?;
            let mut satisfied = verdict != Verdict::Fails;
            if verdict == Verdict::Ambiguous {
                ambiguous.push(check);
            }

            // try rigid arguments through their declared bounds for transitive relations
            if !satisfied
                && check.relation == Relation::Satisfies
                && let dir::Type::Parameter(parameter) = self.ty(check.source)?
                && let Some(declared) = self
                    .generic_parameter(parameter)
                    .and_then(|binding| binding.constraint)
            {
                let declared = self.substitute_type(declared, substitution)?;
                satisfied = self
                    .evaluate_relation(check.origin, check.relation, declared, check.target)?
                    .holds();
            }

            if !satisfied {
                return Ok(None);
            }
        }

        Ok(Some(ambiguous))
    }

    /// Substitute the checks enforced by one complete generic application.
    pub(in crate::sema) fn substitute_application_constraints(
        &mut self,
        origin: Origin,
        template: GenericTemplateId,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<SmallVec<[RelationCheck; 4]>> {
        // require the application to bind every declared parameter
        let parameters = self.generic_template_parameters(template)?;
        if let Some(parameter) = parameters
            .iter()
            .find(|parameter| substitution.argument(**parameter).is_none())
        {
            let template = self.require_generic_template(template)?;
            let declaration = template
                .symbol
                .map(|symbol| self.format_symbol(symbol))
                .unwrap_or_else(|| self.node_label(template.source));
            let parameter_type = self.generic_parameter_type(*parameter)?;
            let parameter = self.format_type(parameter_type);

            return Err(CompilerError::Internal {
                message: format!(
                    "generic application for {declaration} does not bind parameter {parameter}"
                ),
            });
        }

        self.substitute_constraint_checks(origin, template, substitution)
    }

    /// Substitute the bounds and predicates one generic application declares.
    fn substitute_constraint_checks(
        &mut self,
        origin: Origin,
        template: GenericTemplateId,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<SmallVec<[RelationCheck; 4]>> {
        let mut checks = self.substitute_argument_bounds(origin, substitution)?;
        checks.extend(self.substitute_application_predicates(origin, template, substitution)?);

        Ok(checks)
    }

    /// Substitute the declared bounds of every applied argument.
    fn substitute_argument_bounds(
        &mut self,
        origin: Origin,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<SmallVec<[RelationCheck; 4]>> {
        let mut checks = SmallVec::new();

        // substitute bounds for every applied parameter
        for applied in &substitution.bindings {
            let parameter = applied.parameter;
            let argument = applied.argument;
            let Some(bound) = self
                .generic_parameter(parameter)
                .and_then(|binding| binding.constraint)
            else {
                continue;
            };
            let bound = self.substitute_type(bound, substitution)?;
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Bound { parameter }));
            checks.push(RelationCheck::new(
                origin,
                Relation::Satisfies,
                argument,
                bound,
                cause,
            ));
        }

        Ok(checks)
    }

    /// Substitute predicates enforced by one generic application.
    pub(in crate::sema) fn substitute_application_predicates(
        &mut self,
        origin: Origin,
        template: GenericTemplateId,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<SmallVec<[RelationCheck; 2]>> {
        let mut checks = SmallVec::new();

        // substitute every predicate free of the receiver type
        for predicate in self.template_predicates(Some(template)) {
            let requires_receiver = self.type_flags(predicate.left)?.has_this()
                || self.type_flags(predicate.right)?.has_this();
            if requires_receiver {
                continue;
            }
            let source = self.substitute_type(predicate.left, substitution)?;
            let target = self.substitute_type(predicate.right, substitution)?;
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
            checks.push(RelationCheck::new(
                origin,
                predicate.relation.into(),
                source,
                target,
                cause,
            ));
        }

        Ok(checks)
    }
}
