use destack_core::{FxIndexSet, ensure_sufficient_stack};
use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{CheckState, Origin, Relation};

/// The outcome of deciding one relation, keeping ambiguity apart from failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum Verdict {
    /// The relation holds.
    Holds,
    /// The relation fails on closed operands.
    Fails,
    /// An open variable decided the outcome: retry once it solves.
    Ambiguous,
}

impl Verdict {
    /// Classify one decided outcome.
    pub(in crate::check) fn decided(holds: bool) -> Self {
        match holds {
            true => Self::Holds,
            false => Self::Fails,
        }
    }

    /// Return whether the relation decidedly holds.
    pub(in crate::check) fn holds(self) -> bool {
        self == Self::Holds
    }

    /// Keep a failure ambiguous while another undecided path may still hold.
    pub(in crate::check) fn join_undecided(self, other: Self) -> Self {
        match (self, other) {
            (Self::Fails, Self::Ambiguous) => Self::Ambiguous,
            _ => self,
        }
    }
}

impl CheckState<'_> {
    /// Decide one relation between closed type roots, growing the stack.
    pub(in crate::check) fn decide_relation(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // read solved variables through their committed rows
        let mut source = self.resolve_head(source)?;
        if self.type_flags(source)?.has_variable() {
            source = self.resolve_committed_type(source, &FxIndexSet::default())?;
        }
        let mut target = self.resolve_head(target)?;
        if self.type_flags(target)?.has_variable() {
            target = self.resolve_committed_type(target, &FxIndexSet::default())?;
        }

        // decide the written heads first
        let holds = ensure_sufficient_stack(|| {
            self.decide_relation_recursive(origin, relation, source, target)
        })?;
        if holds {
            return Ok(true);
        }

        // retry a stuck decision once over reduced heads
        let reduced_source = self.normalize_stuck(origin, source)?;
        let reduced_target = self.normalize_stuck(origin, target)?;
        if reduced_source == source && reduced_target == target {
            return Ok(false);
        }

        self.decide_relation(origin, relation, reduced_source, reduced_target)
    }

    /// Classify one judged outcome, where failure over an open head is ambiguity.
    pub(in crate::check) fn verdict(
        &mut self,
        holds: bool,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        if holds {
            return Ok(Verdict::Holds);
        }

        // retry the judgment once an open head solves
        let source = self.resolve_head(source)?;
        let target = self.resolve_head(target)?;
        if self.root_variable(source)?.is_some() || self.root_variable(target)?.is_some() {
            return Ok(Verdict::Ambiguous);
        }

        // predicates carry their own ambiguity
        if relation == Relation::Satisfies {
            return self.decide_satisfies(origin, relation, source, target);
        }

        Ok(Verdict::Fails)
    }

    /// Decide one relation recursively on the grown stack.
    fn decide_relation_recursive(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // identity mapped targets over an open variable bind it whole
        if let Some(variable) = self.reverse_mapped_variable(target)? {
            return self.decide_relation(origin, relation, source, variable);
        }

        // identical roots relate under every relation
        if source == target {
            return Ok(true);
        }

        // refined targets require the base and the refined member equality
        if let Some(refined) = self.refined_head(target)? {
            let base = self.decide_relation(origin, relation, source, refined.base)?;
            if !base {
                return Ok(false);
            }

            // project the refined key out of the source
            let arguments = self.intern_type_ids(&[])?;
            let projected = self.intern_member(dir::MemberType {
                owner: source,
                key: refined.key,
                arguments,
                qualifier: None,
            })?;

            return self.decide_relation(origin, Relation::Equal, projected, refined.value);
        }

        // refined sources imply their base application
        if let Some(refined) = self.refined_head(source)? {
            return self.decide_relation(origin, relation, refined.base, target);
        }

        // irreducible conditionals relate through both branches
        if matches!(relation, Relation::Assignable | Relation::Widens)
            && let Some(dir::TypeOperation::Conditional(conditional)) =
                self.operation_head(source)?
        {
            let then_branch =
                self.decide_relation(origin, relation, conditional.then_type, target)?;
            if !then_branch {
                return Ok(false);
            }

            return self.decide_relation(origin, relation, conditional.else_type, target);
        }

        // exact property keys compare by key identity
        if let (Some(source_key), Some(target_key)) = (
            self.static_key_from_type(source)?,
            self.static_key_from_type(target)?,
        ) {
            return Ok(source_key == target_key);
        }

        // key parameter and this queries by their assuming scope
        let flags = self.type_flags(source)? | self.type_flags(target)?;
        let durable = !flags.has_variable();
        let scope = if flags.has_parameter() || flags.has_this() {
            self.assuming_scope(origin)?
        } else {
            None
        };

        // reuse decided relations, treating in-flight pairs as recursive cycles
        let key = (relation, source, target, scope);
        if let Some(holds) = self.relates.get(&key) {
            self.counters.judge_hits += 1;

            return Ok(*holds);
        }
        if let Some(holds) = self.infer.relations.lookup(&key) {
            self.counters.judge_hits += 1;

            return Ok(holds);
        }

        // enter this pair as an active decision
        self.counters.judges += 1;
        let attempt = self.infer.relations.enter(key);

        // prove type equality before the broader relation
        let equal = match relation {
            Relation::Equal => false,
            _ => self.decide_equal(origin, source, target)?,
        };

        // decide the relation itself when equality left it open
        let decision = if equal {
            Ok(true)
        } else {
            let decision = match relation {
                Relation::Equal => self.decide_equal(origin, source, target)?,
                Relation::Subtype => self.decide_subtype(origin, source, target)?,
                Relation::Assignable | Relation::Widens => {
                    self.decide_assignable(origin, relation, source, target)?
                }
                Relation::Castable => self.decide_castable(origin, source, target)?,
                Relation::Satisfies | Relation::Extends => {
                    let verdict = self.decide_satisfies(origin, relation, source, target)?;

                    // degrade an ambiguous predicate to false without memoizing
                    if verdict == Verdict::Ambiguous {
                        self.infer.relations.cancel(attempt);

                        return Ok(false);
                    }

                    verdict.holds()
                }
            };

            Ok(decision)
        };

        // memoize settled decisions, forget failed attempts
        match &decision {
            Ok(holds) => {
                // decided closed pairs are durable for the whole module
                if let Some((key, holds)) = self.infer.relations.finish(attempt, *holds, durable) {
                    self.relates.insert(key, holds);
                }
            }
            _ => self.infer.relations.cancel(attempt),
        }

        decision
    }

    /// Return the open variable behind one identity mapped target.
    pub(in crate::check) fn reverse_mapped_variable(
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
        let constraint = self.resolve_head(mapped.parameter.constraint)?;
        let dir::Type::Operation(constraint) = self.ty(constraint)? else {
            return Ok(None);
        };
        let dir::TypeOperation::KeyOf(keys) = self.type_operation(target.module_id, constraint)?
        else {
            return Ok(None);
        };
        let variable = self.resolve_head(keys.target)?;
        if !matches!(self.ty(variable)?, dir::Type::Variable(_)) {
            return Ok(None);
        }

        // project the iterated key back out of the variable
        let value = self.resolve_head(mapped.value)?;
        let dir::Type::Operation(value) = self.ty(value)? else {
            return Ok(None);
        };
        let dir::TypeOperation::Index(index) = self.type_operation(target.module_id, value)? else {
            return Ok(None);
        };
        let is_identity = self.resolve_head(index.left)? == variable
            && matches!(
                self.ty(self.resolve_head(index.index)?)?,
                dir::Type::Parameter(parameter) if parameter == mapped.parameter.parameter
            );

        Ok(is_identity.then_some(variable))
    }

    /// Decide every relation in one pair list.
    pub(in crate::check) fn decide_each(
        &mut self,
        origin: Origin,
        relation: Relation,
        pairs: &[(dir::GlobalTypeId, dir::GlobalTypeId)],
    ) -> CompilerResult<bool> {
        for (source, target) in pairs.iter().copied() {
            if !self.decide_relation(origin, relation, source, target)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Decide whether every source inhabitant also inhabits the target type.
    fn decide_subtype(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // read both callable signatures for the callable fallbacks below
        let source_signature = self.callable_signature(source)?;
        let target_signature = self.callable_signature(target)?;

        // decide inclusion by constructor pair
        let decision = match (self.ty(source)?, self.ty(target)?) {
            // empty and indeterminate domains
            (dir::Type::Error, _) | (_, dir::Type::Error) => true,
            (dir::Type::Never, _) => true,
            (_, dir::Type::Any | dir::Type::Unknown) => true,
            (dir::Type::Any | dir::Type::Unknown, _) => false,

            // union and intersection inclusion
            (dir::Type::Union(union), _) => {
                let elements = self.type_ids(source.module_id, union.elements)?.to_vec();

                self.decide_all_sources(origin, Relation::Subtype, &elements, target)?
            }
            (_, dir::Type::Union(union)) => {
                let elements = self.type_ids(target.module_id, union.elements)?.to_vec();

                self.decide_any_target(origin, Relation::Subtype, source, &elements)?
            }
            (dir::Type::Intersection(intersection), _) => {
                let elements = self
                    .type_ids(source.module_id, intersection.elements)?
                    .to_vec();

                self.decide_any_source(origin, Relation::Subtype, &elements, target)?
            }
            (_, dir::Type::Intersection(intersection)) => {
                let elements = self
                    .type_ids(target.module_id, intersection.elements)?
                    .to_vec();

                self.decide_all_targets(origin, Relation::Subtype, source, &elements)?
            }

            // rigid generic domains prove inclusion through their bounds
            (dir::Type::Parameter(parameter) | dir::Type::Erased(parameter), _) => {
                self.decide_parameter_relation(origin, Relation::Subtype, parameter, target)?
            }
            (dir::Type::This, _) => self.decide_this_relation(origin, Relation::Subtype, target)?,
            (dir::Type::Member(member), _) => {
                let member = self.type_member(source.module_id, member)?;
                match self.body().projection_constraint(origin, &member)? {
                    Some(constraint) => {
                        self.decide_relation(origin, Relation::Subtype, constraint, target)?
                    }
                    None => false,
                }
            }

            // scalar singleton and interval inclusion
            (
                dir::Type::Literal(dir::ScalarLiteral::String(text)),
                dir::Type::Operation(operation),
            ) if let dir::TypeOperation::TemplateLiteral(template) =
                self.type_operation(target.module_id, operation)? =>
            {
                let text = self.strings().get(text).to_string();

                self.decide_template_string(origin, &text, target.module_id, &template)?
            }
            (dir::Type::Operation(operation), dir::Type::Primitive(dir::PrimitiveType::String))
                if matches!(
                    self.type_operation(source.module_id, operation)?,
                    dir::TypeOperation::TemplateLiteral(_)
                ) =>
            {
                true
            }
            (dir::Type::Primitive(dir::PrimitiveType::String), dir::Type::Operation(operation))
                if let dir::TypeOperation::TemplateLiteral(template) =
                    self.type_operation(target.module_id, operation)? =>
            {
                self.decide_string_inhabits_template(target.module_id, &template)?
            }
            (dir::Type::Operation(source_operation), dir::Type::Operation(target_operation))
                if let dir::TypeOperation::TemplateLiteral(source_template) =
                    self.type_operation(source.module_id, source_operation)?
                    && let dir::TypeOperation::TemplateLiteral(target_template) =
                        self.type_operation(target.module_id, target_operation)? =>
            {
                self.decide_template_template(
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
                true
            }
            // scalar sources decide interface targets before literal widening
            (dir::Type::Literal(_) | dir::Type::Range(_), dir::Type::Application(instance))
                if self.symbol_kind(instance.symbol)?.is_interface() =>
            {
                self.decide_interface_relation(origin, Relation::Subtype, source, target)?
                    .holds()
            }
            (dir::Type::Literal(literal), target) => literal.widens_to(&target),
            (dir::Type::Range(range), target) => range.widens_to(&target),

            // precise variants inhabit their declared owner
            (dir::Type::Variant(variant), _) => {
                self.decide_relation(origin, Relation::Subtype, variant.owner, target)?
            }

            // relate dynamic values only against other dynamic values
            (dir::Type::Dynamic(source), dir::Type::Dynamic(target)) => self.decide_relation(
                origin,
                Relation::Subtype,
                source.constraint,
                target.constraint,
            )?,
            (dir::Type::Dynamic(_), _) | (_, dir::Type::Dynamic(_)) => false,

            // subtype inclusion observes collection elements covariantly
            (dir::Type::Array(source), dir::Type::Array(target)) => {
                self.decide_relation(origin, Relation::Subtype, source.element, target.element)?
            }
            (dir::Type::Slice(source), dir::Type::Slice(target)) => {
                self.decide_relation(origin, Relation::Subtype, source.element, target.element)?
            }
            (dir::Type::FixedArray(source), dir::Type::FixedArray(target)) => {
                let element = self.decide_relation(
                    origin,
                    Relation::Subtype,
                    source.element,
                    target.element,
                )?;
                let count =
                    self.decide_relation(origin, Relation::Equal, source.count, target.count)?;

                element && count
            }
            (dir::Type::Tuple(_), dir::Type::Tuple(_)) => {
                self.decide_tuple_assignable(origin, Relation::Subtype, source, target)?
            }

            // structural and nominal inclusion
            (dir::Type::Object(_), dir::Type::Object(_)) => {
                self.decide_shape_equal(origin, source, target)?
            }
            (dir::Type::Shape(_) | dir::Type::Object(_), dir::Type::Shape(_)) => {
                self.decide_shape_relation(origin, Relation::Subtype, source, target)?
            }
            (
                dir::Type::Shape(_)
                | dir::Type::Object(_)
                | dir::Type::Primitive(_)
                | dir::Type::Array(_)
                | dir::Type::Slice(_)
                | dir::Type::FixedArray(_),
                dir::Type::Application(instance),
            ) if self.symbol_kind(instance.symbol)?.is_interface() => self
                .decide_interface_relation(origin, Relation::Subtype, source, target)?
                .holds(),
            (dir::Type::Application(instance), dir::Type::Shape(_) | dir::Type::Object(_)) => self
                .decide_reference_against_target(
                    origin,
                    Relation::Subtype,
                    source.module_id,
                    &instance,
                    target,
                )?,
            (dir::Type::Application(source_instance), dir::Type::Application(target_instance)) => {
                self.decide_application_relation(
                    origin,
                    Relation::Subtype,
                    source,
                    &source_instance,
                    target,
                    &target_instance,
                )?
            }

            // callable values include according to signature variance
            (dir::Type::FunctionSignature(_), dir::Type::FunctionSignature(_)) => {
                self.decide_function_assignable(origin, Relation::Subtype, source, target)?
            }
            (_, dir::Type::FunctionSignature(_)) if let Some(source) = source_signature => {
                self.decide_function_assignable(origin, Relation::Subtype, source, target)?
            }
            (dir::Type::FunctionSignature(_), _) if let Some(target) = target_signature => {
                self.decide_function_assignable(origin, Relation::Subtype, source, target)?
            }
            (_, _) if let (Some(source), Some(target)) = (source_signature, target_signature) => {
                self.decide_function_assignable(origin, Relation::Subtype, source, target)?
            }

            // forms and all unmatched constructors require equality
            _ => false,
        };

        Ok(decision)
    }
}
