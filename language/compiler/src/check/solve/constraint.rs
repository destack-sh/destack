use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Cause, CauseId, CauseKind, CheckState, GenericTemplateId, Origin, Relation, TypeSubstitution,
    Verdict,
};
use crate::{CompilerError, CompilerResult};

/// Component-global id of one collected constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct ConstraintId(u32);

impl ConstraintId {
    /// Return the constraint id at one index.
    pub(in crate::check) fn at(index: usize) -> Self {
        Self(index as u32)
    }

    /// Return the constraint index.
    pub(in crate::check) fn index(self) -> usize {
        self.0 as usize
    }
}

/// One solver constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct Constraint {
    /// The type evaluation site.
    pub(in crate::check) origin: Origin,
    /// The relation to enforce.
    pub(in crate::check) relation: Relation,
    /// The source operand.
    pub(in crate::check) source: dir::GlobalTypeId,
    /// The target operand.
    pub(in crate::check) target: dir::GlobalTypeId,
    /// Why this constraint exists.
    pub(in crate::check) cause: CauseId,
    /// The generic application this bound guards, when any.
    pub(in crate::check) application: Option<dir::GlobalTypeId>,
}

/// One contextual value role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum ValueUse {
    /// Value assigned into a storage or pattern target.
    Store,

    /// Value assigned into a call or subscript parameter.
    Argument,

    /// Value observed by a compiler-defined operator.
    Operand,

    /// Value evaluated as compile-time decorator data.
    Comptime,

    /// Function body value assigned into a return or yield result.
    Output,

    /// Control-flow condition assigned to boolean.
    Condition,

    /// Value related against a written type without taking it.
    Satisfies,
}

impl ValueUse {
    /// Return whether this use writes a value into a storage destination.
    pub(in crate::check) fn requires_storage(self) -> bool {
        matches!(self, Self::Store | Self::Argument | Self::Output)
    }

    /// Return whether this use requires a runtime coercion.
    pub(in crate::check) fn requires_runtime_coercion(self) -> bool {
        matches!(
            self,
            Self::Store | Self::Argument | Self::Operand | Self::Output
        )
    }
}

impl Constraint {
    /// Create a pure relation between two types.
    pub(in crate::check) fn r#type(
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

    /// Create a generic argument bound constraint.
    pub(in crate::check) fn generic_bound(
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

    /// Return the relation to enforce.
    pub(in crate::check) fn relation(&self) -> Relation {
        self.relation
    }

    /// Return why this constraint exists.
    pub(in crate::check) fn cause(&self) -> CauseId {
        self.cause
    }
}

/// Collected constraints and their results, in allocation order.
#[derive(Debug, Default)]
pub(in crate::check) struct ConstraintTable {
    /// The collected constraints indexed by constraint id.
    constraints: Vec<Constraint>,
    /// Completed results indexed by constraint id.
    results: Vec<Option<ConstraintResult>>,
    /// Ids of collected constraints by value, so one task collects once.
    interned: FxIndexMap<Constraint, ConstraintId>,
}

impl ConstraintTable {
    /// Create an empty constraint table.
    pub(in crate::check) fn new() -> Self {
        Self::default()
    }

    /// Return the id one constraint already collected under.
    pub(in crate::check) fn lookup(&self, constraint: &Constraint) -> Option<ConstraintId> {
        self.interned.get(constraint).copied()
    }

    /// Append one constraint at the next id.
    pub(in crate::check) fn insert(&mut self, id: ConstraintId, constraint: Constraint) {
        debug_assert_eq!(self.constraints.len(), id.index());
        self.constraints.push(constraint);
        self.results.push(None);
        self.interned.insert(constraint, id);
    }

    /// Truncate constraints undone by one probe rollback.
    pub(in crate::check) fn truncate(&mut self, count: usize) {
        self.constraints.truncate(count);
        self.results.truncate(count);

        // truncate the tables together, since every id interns one entry in order
        self.interned.truncate(count);
    }

    /// Return one constraint.
    pub(in crate::check) fn get(&self, id: ConstraintId) -> CompilerResult<&Constraint> {
        self.constraints
            .get(id.index())
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check constraint {id:?} is not allocated"),
            })
    }

    /// Iterate over the collected constraints with their ids.
    pub(in crate::check) fn iter(&self) -> impl Iterator<Item = (ConstraintId, &Constraint)> {
        self.constraints
            .iter()
            .enumerate()
            .map(|(index, constraint)| (ConstraintId::at(index), constraint))
    }

    /// Iterate over failed constraints at or after one allocation index.
    pub(in crate::check) fn failures_from(
        &self,
        start: usize,
    ) -> impl Iterator<Item = ConstraintId> + '_ {
        self.results
            .iter()
            .enumerate()
            .skip(start)
            .filter_map(|(index, result)| {
                result
                    .as_ref()
                    .is_some_and(|result| matches!(result.outcome, CheckOutcome::Fails(_)))
                    .then_some(ConstraintId::at(index))
            })
    }

    /// Return one completed constraint result.
    pub(in crate::check) fn result(
        &self,
        id: ConstraintId,
    ) -> CompilerResult<Option<&ConstraintResult>> {
        self.results
            .get(id.index())
            .map(Option::as_ref)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check constraint {id:?} has no result slot"),
            })
    }

    /// Replace one constraint result.
    pub(in crate::check) fn set_result(
        &mut self,
        id: ConstraintId,
        result: Option<ConstraintResult>,
    ) -> CompilerResult<()> {
        let slot = self
            .results
            .get_mut(id.index())
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check constraint {id:?} has no result slot"),
            })?;
        *slot = result;

        Ok(())
    }

    /// Return whether one constraint finished solving.
    pub(in crate::check) fn is_complete(&self, id: ConstraintId) -> bool {
        self.result(id).is_ok_and(|result| result.is_some())
    }

    /// Return the number of collected constraints.
    pub(in crate::check) fn count(&self) -> usize {
        self.constraints.len()
    }
}

/// Completed result of one constraint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct ConstraintResult {
    /// The reduced source type.
    pub(in crate::check) source: dir::GlobalTypeId,
    /// The reduced target type.
    pub(in crate::check) target: dir::GlobalTypeId,
    /// The completed check outcome.
    pub(in crate::check) outcome: CheckOutcome,
}

/// One failed check retained until its cause tree is complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct FailedCheck {
    /// The cause that produced the check.
    pub(in crate::check) cause: CauseId,
    /// The relation that failed.
    pub(in crate::check) relation: Relation,
    /// The checked value use, when the check consumed a value.
    pub(in crate::check) use_: Option<ValueUse>,
    /// The checked source type.
    pub(in crate::check) source: dir::GlobalTypeId,
    /// The expected target type.
    pub(in crate::check) target: dir::GlobalTypeId,
    /// The failure reason.
    pub(in crate::check) failure: CheckFailure,
    /// Whether an open variable was load-bearing when the check judged.
    pub(in crate::check) is_provisional: bool,
}

impl CheckState<'_> {
    /// Relate the bounds and predicates of one matched substitution.
    pub(in crate::check) fn relate_substitution_constraints(
        &mut self,
        origin: Origin,
        template: GenericTemplateId,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<bool> {
        let mut constraints = self.substitute_argument_bounds(origin, substitution)?;
        constraints.extend(self.substitute_application_predicates(
            origin,
            template,
            substitution,
        )?);

        // require every substituted declaration constraint
        for constraint in constraints {
            let holds = self.evaluate_relation(
                constraint.origin,
                constraint.relation,
                constraint.source,
                constraint.target,
            )?;
            let verdict = self.verdict(
                holds,
                constraint.origin,
                constraint.relation,
                constraint.source,
                constraint.target,
            )?;
            let mut satisfied = verdict != Verdict::Fails;

            // try rigid arguments through their declared bounds for transitive relations
            if !satisfied
                && constraint.relation == Relation::Satisfies
                && let dir::Type::Parameter(parameter) = self.ty(constraint.source)?
                && let Some(declared) = self
                    .generic_parameter(parameter)
                    .and_then(|binding| binding.constraint)
            {
                let declared = self.substitute_type(declared, substitution)?;
                satisfied = self.evaluate_relation(
                    constraint.origin,
                    constraint.relation,
                    declared,
                    constraint.target,
                )?;
            }

            if !satisfied {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Substitute the constraints enforced by one complete generic application.
    pub(in crate::check) fn substitute_application_constraints(
        &mut self,
        origin: Origin,
        template: GenericTemplateId,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<SmallVec<[Constraint; 4]>> {
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

        let mut constraints = self.substitute_argument_bounds(origin, substitution)?;
        constraints.extend(self.substitute_application_predicates(
            origin,
            template,
            substitution,
        )?);

        Ok(constraints)
    }

    /// Substitute the declared bounds of every applied argument.
    fn substitute_argument_bounds(
        &mut self,
        origin: Origin,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<SmallVec<[Constraint; 4]>> {
        let mut constraints = SmallVec::new();

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
            constraints.push(Constraint {
                origin,
                relation: Relation::Satisfies,
                source: argument,
                target: bound,
                cause,
                application: None,
            });
        }

        Ok(constraints)
    }

    /// Substitute predicates enforced by one generic application.
    pub(in crate::check) fn substitute_application_predicates(
        &mut self,
        origin: Origin,
        template: GenericTemplateId,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<SmallVec<[Constraint; 2]>> {
        let mut constraints = SmallVec::new();

        // receiver predicates govern declarations, not their type applications
        for predicate in self.template_predicates(Some(template)) {
            let requires_receiver = self.type_flags(predicate.left)?.has_this()
                || self.type_flags(predicate.right)?.has_this();
            if requires_receiver {
                continue;
            }
            let source = self.substitute_type(predicate.left, substitution)?;
            let target = self.substitute_type(predicate.right, substitution)?;
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
            constraints.push(Constraint {
                origin,
                relation: predicate.relation.into(),
                source,
                target,
                cause,
                application: None,
            });
        }

        Ok(constraints)
    }
}

/// Reason one closed check did not hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum CheckFailure {
    /// The relation itself did not hold.
    Relation,
    /// A value converts to more than one represented union case.
    AmbiguousUnionCoercion,
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
}

/// Result of checking one source against one target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum CheckOutcome {
    /// The check holds.
    Holds,
    /// The check failed for one known reason.
    Fails(CheckFailure),
}

/// Result of checking one value against its contextual target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct ValueCheck {
    /// The checked source type at its flow site.
    pub(in crate::check) source: dir::GlobalTypeId,
    /// Whether value checking held.
    pub(in crate::check) outcome: CheckOutcome,
    /// The concrete contextual target.
    pub(in crate::check) target: dir::GlobalTypeId,
}

/// One checked runtime value and its addressable storage, when present.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct Value {
    /// The checked value type.
    pub(in crate::check) ty: dir::GlobalTypeId,
    /// The storage designated by the source expression.
    pub(in crate::check) place: Option<dir::PlaceResolution>,
}

/// Result of converting one checked value to its expected type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct ValueConversion {
    /// Whether the conversion held.
    pub(in crate::check) outcome: CheckOutcome,
    /// The concrete target used after inference completed.
    pub(in crate::check) target: dir::GlobalTypeId,
    /// The required runtime coercion.
    pub(in crate::check) coercion: Option<Box<dir::Coercion>>,
}

impl CheckOutcome {
    /// Return this check followed by another check.
    pub(in crate::check) fn and(self, next: Self) -> Self {
        match self {
            Self::Holds => next,
            Self::Fails(_) => self,
        }
    }

    /// Return whether this check holds.
    pub(in crate::check) fn is_holds(self) -> bool {
        matches!(self, Self::Holds)
    }
}

/// Applicability of one target-sensitive check path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum CheckAttempt {
    /// The expression form does not use this target directly.
    NotApplicable,
    /// The expression form checked against this target.
    Checked(ValueCheck),
}

impl CheckState<'_> {
    /// Collect generic applications whose declared argument bounds failed.
    pub(in crate::check) fn failed_generic_applications(
        &self,
    ) -> CompilerResult<FxIndexSet<dir::GlobalTypeId>> {
        let mut applications = FxIndexSet::default();
        for id in self.fulfill.constraints.failures_from(0) {
            let constraint = self.fulfill.constraints.get(id)?;
            if let Some(application) = constraint.application {
                applications.insert(application);
            }
        }

        Ok(applications)
    }
}
