use destack_core::ensure_sufficient_stack;
use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, Relation, answer};

impl CheckState<'_> {
    /// Decide one relation between closed type roots, growing the stack.
    pub(in crate::check) fn decide_relation(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        ensure_sufficient_stack(|| self.decide_relation_recursive(origin, relation, source, target))
    }

    /// Decide one relation recursively on the grown stack.
    fn decide_relation_recursive(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let source = answer!(self.reduce_type_head(origin, source)?);
        let target = answer!(self.reduce_type_head(origin, target)?);
        if source == target {
            return Ok(Answer::Ready(true));
        }

        // refined targets require the base and the refined member equality
        if let Some(refined) = self.refined_head(target)? {
            let base = answer!(self.decide_relation(origin, relation, source, refined.base)?);
            if !base {
                return Ok(Answer::Ready(false));
            }
            let arguments = self.intern_type_ids(origin.module(), &[])?;
            let projected = self.intern_member(
                origin.module(),
                dir::MemberType {
                    owner: source,
                    key: refined.key,
                    arguments,
                    qualifier: None,
                },
            )?;

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
                answer!(self.decide_relation(origin, relation, conditional.then_type, target)?);
            if !then_branch {
                return Ok(Answer::Ready(false));
            }

            return self.decide_relation(origin, relation, conditional.else_type, target);
        }

        // exact property keys compare by key identity
        if let (Some(source_key), Some(target_key)) = (
            self.static_key_from_type(source)?,
            self.static_key_from_type(target)?,
        ) {
            return Ok(Answer::Ready(source_key == target_key));
        }

        // key parameter and this queries by their assuming scope
        let flags = self.type_flags(source)? | self.type_flags(target)?;
        let scope = match flags.has_parameter() || flags.has_this() {
            true => self.origin_scope(origin)?,
            false => None,
        };

        // reuse memoized answers, treating active pairs as recursive cycles
        if let Some(holds) = self
            .solver
            .relations
            .lookup(relation, source, target, scope)
        {
            return Ok(Answer::Ready(holds));
        }
        let attempt = self.solver.relations.enter(relation, source, target, scope);

        // prove type equality before the broader relation
        let equal = match relation {
            Relation::Equal => Answer::Ready(false),
            _ => self.decide_equal(origin, source, target)?,
        };
        let decision = if equal.is_ready_true() {
            Ok(equal)
        } else {
            let decision = match relation {
                Relation::Equal => self.decide_equal(origin, source, target)?,
                Relation::Subtype => self.decide_subtype(origin, source, target)?,
                Relation::Assignable | Relation::Widens => {
                    self.decide_assignable(origin, relation, source, target)?
                }
                Relation::Castable => self.decide_castable(origin, source, target)?,
                Relation::Satisfies | Relation::Extends | Relation::Implements => {
                    self.decide_satisfies(origin, relation, source, target)?
                }
            };

            Ok(equal.or(decision))
        };

        // memoize settled decisions, forget pending or failed attempts
        match &decision {
            Ok(Answer::Ready(holds)) => {
                self.solver.relations.finish(attempt, *holds);
            }
            _ => self.solver.relations.cancel(attempt),
        }

        decision
    }

    /// Decide every relation in one pair list.
    pub(in crate::check) fn decide_each(
        &mut self,
        origin: Origin,
        relation: Relation,
        pairs: &[(dir::GlobalTypeId, dir::GlobalTypeId)],
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(true);
        for (source, target) in pairs.iter().copied() {
            decision = decision.and(self.decide_relation(origin, relation, source, target)?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide whether every source inhabitant also inhabits the target type.
    fn decide_subtype(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let source_signature = self.callable_signature(source)?;
        let target_signature = self.callable_signature(target)?;
        let decision = match (self.ty(source)?, self.ty(target)?) {
            // empty and indeterminate domains
            (dir::Type::Error, _) | (_, dir::Type::Error) => Answer::Ready(true),
            (dir::Type::Never, _) => Answer::Ready(true),
            (_, dir::Type::Any | dir::Type::Unknown) => Answer::Ready(true),
            (dir::Type::Any | dir::Type::Unknown, _) => Answer::Ready(false),

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
                    Answer::Ready(Some(constraint)) => {
                        self.decide_relation(origin, Relation::Subtype, constraint, target)?
                    }
                    Answer::Ready(None) => Answer::Ready(false),
                    Answer::Pending(dependencies) => Answer::Pending(dependencies),
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
                Answer::Ready(true)
            }
            (dir::Type::Primitive(dir::PrimitiveType::String), dir::Type::Operation(operation))
                if let dir::TypeOperation::TemplateLiteral(template) =
                    self.type_operation(target.module_id, operation)? =>
            {
                self.decide_string_inhabits_template(origin, target.module_id, &template)?
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
                Answer::Ready(true)
            }
            (dir::Type::Literal(literal), target) => Answer::Ready(literal.widens_to(&target)),
            (dir::Type::Range(range), target) => Answer::Ready(range.widens_to(&target)),

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
            (dir::Type::Dynamic(_), _) | (_, dir::Type::Dynamic(_)) => Answer::Ready(false),

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

                element.and(count)
            }
            (dir::Type::Tuple(_), dir::Type::Tuple(_)) => {
                self.decide_tuple_assignable(origin, Relation::Subtype, source, target)?
            }

            // structural and nominal inclusion
            (dir::Type::Shape(_), dir::Type::Shape(_)) => {
                self.decide_shape_relation(origin, Relation::Subtype, source, target)?
            }
            (dir::Type::Shape(_), dir::Type::Application(instance))
                if self.symbol_kind(instance.symbol).is_interface() =>
            {
                self.decide_interface_relation(origin, Relation::Subtype, source, target)?
            }
            (dir::Type::Application(instance), dir::Type::Shape(_)) => self
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
            _ => Answer::Ready(false),
        };

        Ok(decision)
    }
}
