use destack_core::ensure_sufficient_stack;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::sema::{
    Answer, CandidateOutcome, Cause, CauseId, CauseKind, CheckState, Cycle, Goal, Origin, Relation,
};

/// The outcome of deciding one relation, keeping ambiguity apart from failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum Verdict {
    /// The relation holds.
    Holds,
    /// The relation fails on closed operands.
    Fails,
    /// An open variable decided the outcome: retry once it solves.
    Ambiguous,
}

impl Verdict {
    /// Classify one decided outcome.
    pub(in crate::sema) fn decided(is_holds: bool) -> Self {
        match is_holds {
            true => Self::Holds,
            false => Self::Fails,
        }
    }

    /// Return whether the relation decidedly holds.
    pub(in crate::sema) fn holds(self) -> bool {
        self == Self::Holds
    }

    /// Keep a failure ambiguous while another undecided path may still hold.
    pub(in crate::sema) fn join_undecided(self, other: Self) -> Self {
        match (self, other) {
            (Self::Fails, Self::Ambiguous) => Self::Ambiguous,
            _ => self,
        }
    }

    /// Join one conjunct, where failure dominates and ambiguity taints.
    pub(in crate::sema) fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::Fails, _) | (_, Self::Fails) => Self::Fails,
            (Self::Ambiguous, _) | (_, Self::Ambiguous) => Self::Ambiguous,
            _ => Self::Holds,
        }
    }

    /// Join one disjunct, where success dominates and ambiguity taints.
    pub(in crate::sema) fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::Holds, _) | (_, Self::Holds) => Self::Holds,
            (Self::Ambiguous, _) | (_, Self::Ambiguous) => Self::Ambiguous,
            _ => Self::Fails,
        }
    }

    /// Join one disjunct reached only while this one leaves the outcome open.
    pub(in crate::sema) fn or_else(
        self,
        other: impl FnOnce() -> CompilerResult<Self>,
    ) -> CompilerResult<Self> {
        match self {
            Self::Holds => Ok(Self::Holds),
            verdict => Ok(verdict.or(other()?)),
        }
    }
}

impl CheckState<'_> {
    /// Evaluate one relation between type roots as a pure query, growing the stack.
    pub(in crate::sema) fn decide_relation(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // read solved variables through their committed entries
        let source = self.shallow_resolve(source)?;
        let target = self.shallow_resolve(target)?;

        // retry an open head once it solves
        if self.root_variable(source)?.is_some() || self.root_variable(target)?.is_some() {
            return Ok(Verdict::Ambiguous);
        }

        // relate the written heads first, rolling every evaluation effect back
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
        let mut related = Verdict::Fails;
        let probe = self.probe_relation(|state| {
            related = ensure_sufficient_stack(|| {
                state.relate_pair(origin, cause, relation, source, target)
            })?;

            Ok(related.holds())
        })?;
        if probe == Verdict::Holds {
            return Ok(Verdict::Holds);
        }

        // reduce stuck heads once and decide the reduced pair
        let reduced_source = self.structurally_normalize(origin, source)?;
        let reduced_target = self.structurally_normalize(origin, target)?;
        if reduced_source == source && reduced_target == target {
            // leave a relation over open heads undecided
            if probe == Verdict::Ambiguous || related == Verdict::Ambiguous {
                return Ok(Verdict::Ambiguous);
            }

            return self.decide_stuck_relation(source, target);
        }

        self.decide_relation(origin, relation, reduced_source, reduced_target)
    }

    /// Decide one failed stuck relation, undecided while either head stays open.
    pub(in crate::sema) fn decide_stuck_relation(
        &mut self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        let is_undecided = self.is_open_head(source)? || self.is_open_head(target)?;

        Ok(match is_undecided {
            true => Verdict::Ambiguous,
            false => Verdict::Fails,
        })
    }

    /// Return whether one head stays open: a variable, or a non-template projection over one.
    pub(in crate::sema) fn is_open_head(&mut self, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        if self.root_variable(ty)?.is_some() {
            return Ok(true);
        }

        // template literal verdicts decide through matching
        let projects = match self.ty(ty)? {
            dir::Type::Member(_) => true,
            dir::Type::Operation(operation) => !matches!(
                self.type_operation(ty.module_id, operation)?,
                dir::TypeOperation::TemplateLiteral(_)
            ),
            _ => false,
        };
        if !projects {
            return Ok(false);
        }

        Ok(!self.type_variables(ty)?.is_empty())
    }

    /// Classify one relation outcome, where failure over an open head is ambiguity.
    pub(in crate::sema) fn decide_outcome(
        &mut self,
        is_holds: bool,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        if is_holds {
            return Ok(Verdict::Holds);
        }

        // retry the decision once an open head solves
        let source = self.shallow_resolve(source)?;
        let target = self.shallow_resolve(target)?;
        if self.is_open_head(source)? || self.is_open_head(target)? {
            return Ok(Verdict::Ambiguous);
        }

        // predicates carry their own ambiguity, decided without committing
        if relation == Relation::Satisfies {
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
            let mut decision = Verdict::Fails;
            self.probe_candidate(|state| {
                decision = state.relate_satisfies(origin, cause, relation, source, target)?;

                Ok(CandidateOutcome::<(), ()>::Rejected(()))
            })?;

            return Ok(decision);
        }

        Ok(Verdict::Fails)
    }

    /// Relate one resolved pair through the structural matrix, binding the open children.
    pub(in crate::sema) fn relate_pair(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // identity mapped targets over an open variable bind it whole
        if let Some(variable) = self.reverse_mapped_variable(target)? {
            return self.constrain_type(origin, cause, relation, source, variable);
        }

        // identical roots relate under every relation
        if source == target {
            return Ok(Verdict::Holds);
        }

        // accept inference barrier targets outright
        if self.no_infer_target(target)?.is_some() {
            return Ok(Verdict::Holds);
        }

        // refined targets require the base and the refined member equality
        if let Some(refined) = self.refined_head(target)? {
            let base = self.constrain_type(origin, cause, relation, source, refined.base)?;
            if base == Verdict::Fails {
                return Ok(Verdict::Fails);
            }

            // project the refined key out of the source
            let arguments = self.intern_type_ids(&[])?;
            let projected = self.intern_member(dir::MemberType {
                owner: source,
                key: refined.key,
                arguments,
                qualifier: None,
            })?;
            let value =
                self.constrain_type(origin, cause, Relation::Equal, projected, refined.value)?;

            return Ok(base.and(value));
        }

        // refined sources imply their base application
        if let Some(refined) = self.refined_head(source)? {
            return self.constrain_type(origin, cause, relation, refined.base, target);
        }

        // leave the relation undecided while a conditional blocks on open variables
        for side in [source, target] {
            if let Some(dir::TypeOperation::Conditional(conditional)) = self.operation_head(side)?
                && !self
                    .collect_open_variables([conditional.left, conditional.right])?
                    .is_empty()
            {
                return Ok(Verdict::Ambiguous);
            }
        }

        // stuck narrows decide relations through their source, positive ones
        //  through the narrowing target as well
        if let Some(dir::TypeOperation::Narrow(narrow)) = self.operation_head(source)? {
            let through_source =
                self.constrain_type(origin, cause, relation, narrow.source, target)?;
            if through_source == Verdict::Holds {
                return Ok(Verdict::Holds);
            }

            if narrow.is_positive {
                let through_target =
                    self.constrain_type(origin, cause, relation, narrow.target, target)?;
                if through_target == Verdict::Holds {
                    return Ok(Verdict::Holds);
                }
            }
        }

        // irreducible conditionals relate through both branches
        if matches!(relation, Relation::Assignable | Relation::Widens)
            && let Some(dir::TypeOperation::Conditional(conditional)) =
                self.operation_head(source)?
        {
            let then_branch =
                self.constrain_type(origin, cause, relation, conditional.then_type, target)?;
            if then_branch == Verdict::Fails {
                return Ok(Verdict::Fails);
            }

            let else_branch =
                self.constrain_type(origin, cause, relation, conditional.else_type, target)?;

            return Ok(then_branch.and(else_branch));
        }

        // exact property keys compare by key identity
        if let (Some(source_key), Some(target_key)) = (
            self.static_key_from_type(source)?,
            self.static_key_from_type(target)?,
        ) {
            return Ok(Verdict::decided(source_key == target_key));
        }

        // relate region terms by their own rule
        if let Some(verdict) = self.relate_region_terms(origin, cause, relation, source, target)? {
            return Ok(verdict);
        }

        // decide region kind inhabitants before canonicalization erases parameter kinds
        if let Some(symbol) = self.type_symbol(target)?
            && matches!(
                self.language_item(symbol)?,
                Some(
                    dir::LanguageItem::Lifetime
                        | dir::LanguageItem::Region
                        | dir::LanguageItem::Place
                        | dir::LanguageItem::Space
                )
            )
            && self.memory_kind(source)? == Some(dir::MemoryParameter::Region)
        {
            return Ok(Verdict::Holds);
        }

        // canonicalize the pair
        let Some((question, canonical)) =
            self.canonicalize_goal(origin, Goal::Relation(relation), &[source, target], true)?
        else {
            return self.evaluate_relation(origin, cause, relation, source, target);
        };
        let flags =
            self.type_flags(canonical.operands[0])? | self.type_flags(canonical.operands[1])?;
        let has_holes = flags.has_hole();

        // reuse a decided answer
        if let Some(answer) = self.answers.get(&question) {
            let is_holds = matches!(answer, Answer::Holds);
            if !has_holes || !is_holds {
                self.counters.relation_reuses += 1;

                return Ok(Verdict::decided(is_holds));
            }
        }

        // decide holed pairs outside the in-flight stack
        if has_holes {
            let decision = self.evaluate_relation(origin, cause, relation, source, target)?;
            if decision == Verdict::Fails && self.infer.relations.is_idle() {
                self.answers.insert(question, Answer::Fails);
            }

            return Ok(decision);
        }

        // reuse decided relations
        let cycle = if self.is_conformance_target(target)? {
            Cycle::Inductive
        } else {
            Cycle::Coinductive
        };
        let key = (relation, source, target);
        if let Some(is_holds) = self.infer.relations.lookup(&key, cycle) {
            self.counters.relation_reuses += 1;

            return Ok(Verdict::decided(is_holds));
        }

        // enter this pair as an active decision
        self.counters.relation_decisions += 1;
        let attempt = self.infer.relations.enter(key);

        // decide the pair through its own rule
        let decision = self.evaluate_relation(origin, cause, relation, source, target);

        // memoize resolved decisions, forget failed and undecided attempts
        match &decision {
            Ok(verdict) if *verdict != Verdict::Ambiguous => {
                // decided resolved pairs are durable for the whole module
                let is_holds = verdict.holds();
                if let Some(is_holds) = self.infer.relations.finish(attempt, is_holds) {
                    let answer = if is_holds {
                        Answer::Holds
                    } else {
                        Answer::Fails
                    };
                    self.answers.insert(question, answer);
                }
            }
            _ => self.infer.relations.cancel(attempt),
        }

        decision
    }

    /// Return whether one relation target asks interface conformance.
    fn is_conformance_target(&mut self, target: dir::GlobalTypeId) -> CompilerResult<bool> {
        // read the nominal symbol the target names
        let symbol = match self.ty(target)? {
            dir::Type::Application(application) => application.symbol,
            dir::Type::Reference(reference) => reference.symbol,
            _ => return Ok(false),
        };

        Ok(matches!(
            self.definition(symbol)?,
            Some(dir::Definition::Interface(_))
        ))
    }

    /// Evaluate one relation through its own rule.
    fn evaluate_relation(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // decide equality on closed pairs before the broader relation
        let closed = !(self.type_flags(source)? | self.type_flags(target)?).has_variable();
        let equal = match relation {
            Relation::Equal => false,
            _ => closed && self.relate_equal(origin, cause, source, target)?.holds(),
        };
        if equal {
            return Ok(Verdict::Holds);
        }

        // decide the relation itself when equality left it open
        let decision = match relation {
            Relation::Equal => self.relate_equal(origin, cause, source, target)?,
            Relation::Subtype => self.relate_subtype(origin, cause, source, target)?,
            Relation::Assignable | Relation::Widens => {
                self.relate_assignable(origin, cause, relation, source, target)?
            }
            Relation::Castable => self.relate_castable(origin, cause, source, target)?,
            Relation::Satisfies | Relation::Extends => {
                self.relate_satisfies(origin, cause, relation, source, target)?
            }
        };

        Ok(decision)
    }

    /// Return the open variable behind one identity mapped target.
    pub(in crate::sema) fn reverse_mapped_variable(
        &mut self,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let dir::Type::Operation(operation) = self.ty(target)? else {
            return Ok(None);
        };
        let dir::TypeOperation::Mapped(mapped) =
            self.type_operation(target.module_id, operation)?
        else {
            return Ok(None);
        };

        // reject remaps and modifiers that reshape the source
        let is_plain = mapped.parameter.key_remap.is_none()
            && mapped.modifiers.readonly == dir::MappedTypeModifier::None
            && mapped.modifiers.optional == dir::MappedTypeModifier::None;
        if !is_plain {
            return Ok(None);
        }

        // iterate an open variable's keys
        let constraint = self.shallow_resolve(mapped.parameter.constraint)?;
        let dir::Type::Operation(constraint) = self.ty(constraint)? else {
            return Ok(None);
        };
        let dir::TypeOperation::KeyOf(keys) = self.type_operation(target.module_id, constraint)?
        else {
            return Ok(None);
        };
        let variable = self.shallow_resolve(keys.target)?;
        if !matches!(self.ty(variable)?, dir::Type::Variable(_)) {
            return Ok(None);
        }

        // project the iterated key back out of the variable
        let value = self.shallow_resolve(mapped.value)?;
        let dir::Type::Operation(value) = self.ty(value)? else {
            return Ok(None);
        };
        let dir::TypeOperation::Index(index) = self.type_operation(target.module_id, value)? else {
            return Ok(None);
        };
        let is_identity = self.shallow_resolve(index.left)? == variable
            && matches!(
                self.ty(self.shallow_resolve(index.index)?)?,
                dir::Type::Parameter(parameter) if parameter == mapped.parameter.parameter
            );

        Ok(is_identity.then_some(variable))
    }

    /// Relate every pair in one pair list.
    pub(in crate::sema) fn relate_each(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        pairs: &[(dir::GlobalTypeId, dir::GlobalTypeId)],
    ) -> CompilerResult<Verdict> {
        // stop at the first decided failure
        let mut verdict = Verdict::Holds;
        for (source, target) in pairs.iter().copied() {
            verdict = verdict.and(self.constrain_type(origin, cause, relation, source, target)?);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        Ok(verdict)
    }

    /// Relate one pair under subtype inclusion, where every source inhabitant inhabits the target.
    fn relate_subtype(
        &mut self,
        origin: Origin,
        cause: CauseId,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // read both callable signatures for the callable fallbacks below
        let source_signature = self.callable_signature(source)?;
        let target_signature = self.callable_signature(target)?;

        // decide inclusion by constructor pair
        let decision = match (self.ty(source)?, self.ty(target)?) {
            // empty and indeterminate domains
            (dir::Type::Error, _) | (_, dir::Type::Error) => Verdict::Holds,
            (dir::Type::Never, _) => Verdict::Holds,
            (_, dir::Type::Unknown) => Verdict::Holds,
            (dir::Type::Unknown, _) => Verdict::Fails,

            // union and intersection inclusion
            (dir::Type::Union(union), _) => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(source.module_id, union.elements)?.into();

                self.relate_all_sources(origin, cause, Relation::Subtype, &elements, target)?
            }
            (_, dir::Type::Union(union)) => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(target.module_id, union.elements)?.into();

                self.relate_any_target(origin, cause, Relation::Subtype, source, &elements)?
            }
            (dir::Type::Intersection(intersection), _) => {
                let elements: SmallVec<[_; 8]> = self
                    .type_ids(source.module_id, intersection.elements)?
                    .into();

                self.relate_any_source(origin, cause, Relation::Subtype, &elements, target)?
            }
            (_, dir::Type::Intersection(intersection)) => {
                let elements: SmallVec<[_; 8]> = self
                    .type_ids(target.module_id, intersection.elements)?
                    .into();

                self.relate_all_targets(origin, cause, Relation::Subtype, source, &elements)?
            }

            // rigid generic domains prove inclusion through their bounds
            (dir::Type::Parameter(parameter) | dir::Type::Erased(parameter), _) => {
                self.relate_parameter_bounds(origin, cause, Relation::Subtype, parameter, target)?
            }
            (dir::Type::This, _) => {
                self.relate_this_bounds(origin, cause, Relation::Subtype, target)?
            }
            (dir::Type::Member(member), _) => {
                let member = self.type_member(source.module_id, member)?;
                match self.body().projection_constraint(origin, &member)? {
                    Some(constraint) => {
                        self.constrain_type(origin, cause, Relation::Subtype, constraint, target)?
                    }
                    None => Verdict::Fails,
                }
            }

            // scalar singleton and interval inclusion
            (
                dir::Type::Literal(dir::Literal::String(text))
                | dir::Type::Key(dir::StaticKey::Name(text)),
                dir::Type::Operation(operation),
            ) if let dir::TypeOperation::TemplateLiteral(template) =
                self.type_operation(target.module_id, operation)? =>
            {
                let text = self.strings().get(text).to_string();

                // bind open spans to the text they capture
                let mut is_open = false;
                for &span in self.type_ids(target.module_id, template.spans)? {
                    is_open = is_open || self.type_flags(span)?.has_variable();
                }
                if is_open {
                    self.relate_template_captures(
                        origin,
                        cause,
                        &text,
                        target.module_id,
                        &template,
                    )?
                } else {
                    self.relate_template_string(origin, &text, target.module_id, &template)?
                }
            }
            (dir::Type::Operation(operation), dir::Type::Primitive(dir::PrimitiveType::String))
                if matches!(
                    self.type_operation(source.module_id, operation)?,
                    dir::TypeOperation::TemplateLiteral(_)
                ) =>
            {
                Verdict::Holds
            }
            (dir::Type::Primitive(dir::PrimitiveType::String), dir::Type::Operation(operation))
                if let dir::TypeOperation::TemplateLiteral(template) =
                    self.type_operation(target.module_id, operation)? =>
            {
                self.relate_string_into_template(
                    origin,
                    cause,
                    source,
                    target.module_id,
                    &template,
                )?
            }
            (dir::Type::Operation(source_operation), dir::Type::Operation(target_operation))
                if let dir::TypeOperation::TemplateLiteral(source_template) =
                    self.type_operation(source.module_id, source_operation)?
                    && let dir::TypeOperation::TemplateLiteral(target_template) =
                        self.type_operation(target.module_id, target_operation)? =>
            {
                self.relate_template_template(
                    origin,
                    source.module_id,
                    &source_template,
                    target.module_id,
                    &target_template,
                )?
            }
            (_, dir::Type::Primitive(primitive))
                if self
                    .static_key_from_type(source)?
                    .is_some_and(|key| key.widens_to_primitive(primitive)) =>
            {
                Verdict::Holds
            }

            // scalar sources decide interface targets before literal widening
            (dir::Type::Literal(_) | dir::Type::Range(_), dir::Type::Application(instance))
                if self.symbol_kind(instance.symbol)?.is_interface() =>
            {
                self.relate_interface(origin, cause, Relation::Subtype, source, target)?
            }
            (dir::Type::Literal(literal), target) => Verdict::decided(literal.widens_to(&target)),
            (dir::Type::Range(range), target) => Verdict::decided(range.widens_to(&target)),

            // precise variants inhabit their declared owner
            (dir::Type::Variant(variant), _)
                if !matches!(self.ty(target)?, dir::Type::Variant(_)) =>
            {
                self.constrain_type(origin, cause, Relation::Subtype, variant.owner, target)?
            }

            // relate dynamic values only against other dynamic values
            (dir::Type::Dynamic(source), dir::Type::Dynamic(target)) => self.constrain_type(
                origin,
                cause,
                Relation::Subtype,
                source.constraint,
                target.constraint,
            )?,
            (dir::Type::Dynamic(_), _) | (_, dir::Type::Dynamic(_)) => Verdict::Fails,

            // subtype inclusion observes collection elements covariantly
            (dir::Type::Slice(source), dir::Type::Slice(target)) => self.constrain_type(
                origin,
                cause,
                Relation::Subtype,
                source.element,
                target.element,
            )?,
            (dir::Type::FixedArray(source), dir::Type::FixedArray(target)) => {
                let element = self.constrain_type(
                    origin,
                    cause,
                    Relation::Subtype,
                    source.element,
                    target.element,
                )?;
                let count = self.constrain_type(
                    origin,
                    cause,
                    Relation::Equal,
                    source.count,
                    target.count,
                )?;

                element.and(count)
            }
            (dir::Type::Tuple(_), dir::Type::Tuple(_)) => {
                self.relate_tuple_assignable(origin, cause, Relation::Subtype, source, target)?
            }

            // structural and nominal inclusion
            (dir::Type::Object(_), dir::Type::Object(_)) => {
                self.relate_shape(origin, cause, Relation::Subtype, source, target)?
            }
            (
                dir::Type::Object(_)
                | dir::Type::Primitive(_)
                | dir::Type::Slice(_)
                | dir::Type::FixedArray(_),
                dir::Type::Application(instance),
            ) if self.symbol_kind(instance.symbol)?.is_interface() => {
                self.relate_interface(origin, cause, Relation::Subtype, source, target)?
            }
            (dir::Type::Application(instance), dir::Type::Object(_)) => self
                .relate_reference_against_target(
                    origin,
                    cause,
                    Relation::Subtype,
                    source.module_id,
                    &instance,
                    target,
                )?,
            (dir::Type::Application(source_instance), dir::Type::Application(target_instance)) => {
                self.relate_application(
                    origin,
                    cause,
                    Relation::Subtype,
                    source,
                    &source_instance,
                    target,
                    &target_instance,
                )?
            }

            // callable values include according to signature variance
            (dir::Type::FunctionSignature(_), dir::Type::FunctionSignature(_)) => {
                self.relate_function_assignable(origin, cause, Relation::Subtype, source, target)?
            }
            (_, dir::Type::FunctionSignature(_)) if let Some(source) = source_signature => {
                self.relate_function_assignable(origin, cause, Relation::Subtype, source, target)?
            }
            (dir::Type::FunctionSignature(_), _) if let Some(target) = target_signature => {
                self.relate_function_assignable(origin, cause, Relation::Subtype, source, target)?
            }
            (_, _) if let (Some(source), Some(target)) = (source_signature, target_signature) => {
                self.relate_function_assignable(origin, cause, Relation::Subtype, source, target)?
            }

            // forms and all unmatched constructors require equality
            _ => Verdict::Fails,
        };

        Ok(decision)
    }
}
