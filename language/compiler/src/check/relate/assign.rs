use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, Relation, answer};

impl CheckState<'_> {
    /// Decide assignability from one reduced source to one reduced target.
    ///
    /// `Widens` decides the same relation restricted to identity-witnessed
    /// edges: conversions that would reify as coercions never widen.
    pub(in crate::check) fn decide_assignable(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let widens = relation == Relation::Widens;

        let source_signature = self.callable_signature(source)?;
        let target_signature = self.callable_signature(target)?;

        let decision = match (self.ty(source)?, self.ty(target)?) {
            // top and error types absorb everything
            (dir::Type::Error, _) | (_, dir::Type::Error) => Answer::Ready(true),
            // collect lifetimes without deciding, leaving outlives to Verify on MIR
            (source_head, target_head)
                if self.is_lifetime_slot(&source_head)?
                    && self.is_lifetime_slot(&target_head)? =>
            {
                Answer::Ready(true)
            }
            // existential carriers box their values and never widen
            (_, dir::Type::Any) | (_, dir::Type::Unknown) => Answer::Ready(!widens),
            (dir::Type::Any, _) => Answer::Ready(!widens),
            (dir::Type::Never, _) => Answer::Ready(true),

            // string literals inhabit matching template literal patterns
            (
                dir::Type::Literal(dir::ScalarLiteral::String(text)),
                dir::Type::Operation(operation),
            ) if let dir::TypeOperation::TemplateLiteral(template) =
                self.type_operation(target.module_id, operation)? =>
            {
                let text = self.strings().get(text).to_string();

                self.decide_template_string(origin, &text, target.module_id, &template)?
            }
            // every template literal instance is a string
            (dir::Type::Operation(operation), dir::Type::Primitive(dir::PrimitiveType::String))
                if matches!(
                    self.type_operation(source.module_id, operation)?,
                    dir::TypeOperation::TemplateLiteral(_)
                ) =>
            {
                Answer::Ready(true)
            }
            // unconstraining patterns absorb the whole string domain
            (dir::Type::Primitive(dir::PrimitiveType::String), dir::Type::Operation(operation))
                if let dir::TypeOperation::TemplateLiteral(template) =
                    self.type_operation(target.module_id, operation)? =>
            {
                self.decide_string_inhabits_template(origin, target.module_id, &template)?
            }
            // template patterns compare span-wise through their pieces
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

            // exact property keys flow into their primitive key domains
            (_, dir::Type::Primitive(primitive))
                if !widens
                    && self
                        .static_key_from_type(source)?
                        .is_some_and(|key| key.widens_to_primitive(primitive)) =>
            {
                Answer::Ready(true)
            }

            // memory forms own placement and readonly views
            _ if let Some(decision) =
                self.constrain_form_assignable_rooted(origin, relation, source, target)? =>
            {
                decision
            }

            // union carriers tag their values and never widen
            (dir::Type::Union(_), _) | (_, dir::Type::Union(_)) if widens => Answer::Ready(false),

            // union sources need every element assignable
            (dir::Type::Union(union), _) => {
                let elements = self.type_ids(source.module_id, union.elements)?.to_vec();

                self.decide_all_sources(origin, relation, &elements, target)?
            }
            // parameters and erased arguments assign through their
            //  constraints, or sit inside a union target
            (dir::Type::Parameter(parameter) | dir::Type::Erased(parameter), _) => {
                let decision =
                    self.decide_parameter_relation(origin, relation, parameter, target)?;

                self.decide_union_membership(origin, relation, decision, source, target)?
            }
            // erased arguments are existential and do not accept concrete writes
            (_, dir::Type::Erased(_)) => Answer::Ready(false),
            // this assigns through its enclosing interface hypotheses, or sits inside a union target
            (dir::Type::This, _) => {
                let decision = self.decide_this_relation(origin, Relation::Assignable, target)?;

                self.decide_union_membership(origin, relation, decision, source, target)?
            }
            // rigid projections assign through their declared constraint
            (dir::Type::Member(member), _) => {
                let member = self.type_member(source.module_id, member)?;
                let constraint = self.body().projection_constraint(origin, &member)?;
                let decision = match constraint {
                    Answer::Ready(Some(constraint)) => {
                        self.decide_relation(origin, relation, constraint, target)?
                    }
                    Answer::Ready(None) => Answer::Ready(false),
                    Answer::Pending(blockers) => Answer::Pending(blockers),
                };

                self.decide_union_membership(origin, relation, decision, source, target)?
            }
            // intersection sources assign through any element
            (dir::Type::Intersection(intersection), _) => {
                let elements = self
                    .type_ids(source.module_id, intersection.elements)?
                    .to_vec();

                self.decide_any_source(origin, relation, &elements, target)?
            }
            // union targets need one viable element
            (_, dir::Type::Union(union)) => {
                let elements = self.type_ids(target.module_id, union.elements)?.to_vec();

                self.decide_any_target(origin, relation, source, &elements)?
            }
            // intersection targets need every element
            (_, dir::Type::Intersection(intersection)) => {
                let elements = self
                    .type_ids(target.module_id, intersection.elements)?
                    .to_vec();

                self.decide_all_targets(origin, relation, source, &elements)?
            }
            // existential carriers box their values and never widen
            (_, dir::Type::Dynamic(_)) | (dir::Type::Dynamic(_), _) if widens => {
                Answer::Ready(false)
            }
            // erase compatible values into dynamic targets
            (_, dir::Type::Dynamic(dynamic)) => {
                self.decide_dynamic_assignable(origin, source, dynamic.constraint)?
            }
            // dynamic values carry their constraint's proof by construction
            (dir::Type::Dynamic(dynamic), _) => {
                self.decide_relation(origin, Relation::Assignable, dynamic.constraint, target)?
            }

            // numeric literal storage has no single carrier and never widens;
            //  uniform-carrier families like strings store identically
            (dir::Type::Literal(literal), _) if widens && !literal.has_uniform_carrier() => {
                Answer::Ready(false)
            }
            (dir::Type::Range(_), _) if widens => Answer::Ready(false),

            // literals and intervals widen by value
            (dir::Type::Literal(literal), target) => Answer::Ready(literal.widens_to(&target)),
            (dir::Type::Range(range), target) => Answer::Ready(range.widens_to(&target)),
            (dir::Type::Variant(member), _) => {
                self.decide_relation(origin, relation, member.owner, target)?
            }

            // mutable collections alias their elements and stay invariant
            (dir::Type::Array(source), dir::Type::Array(target)) => {
                self.decide_relation(origin, Relation::Equal, source.element, target.element)?
            }
            (dir::Type::Array(source), dir::Type::Slice(target)) => {
                self.decide_relation(origin, Relation::Equal, source.element, target.element)?
            }
            (dir::Type::Array(_), dir::Type::FixedArray(_)) => Answer::Ready(false),
            (dir::Type::Slice(source), dir::Type::Slice(target)) => {
                self.decide_relation(origin, Relation::Equal, source.element, target.element)?
            }
            // value containers copy without a rebuild: elements only widen
            (dir::Type::FixedArray(source), dir::Type::FixedArray(target)) => {
                let element =
                    self.decide_relation(origin, Relation::Widens, source.element, target.element)?;
                let count =
                    self.decide_relation(origin, Relation::Equal, source.count, target.count)?;

                element.and(count)
            }
            // sized sequences view through their fat slice carrier
            (dir::Type::FixedArray(source), dir::Type::Slice(target)) if !widens => {
                self.decide_relation(origin, Relation::Equal, source.element, target.element)?
            }
            // growing into a managed array allocates and copies: explicit only
            (dir::Type::FixedArray(_), dir::Type::Array(_)) => Answer::Ready(false),
            (dir::Type::Tuple(_), dir::Type::Tuple(_)) => {
                self.decide_tuple_assignable(origin, relation, source, target)?
            }

            // structural shapes and nominal boundaries
            (dir::Type::Shape(_), dir::Type::Shape(_)) => {
                self.decide_shape_assignable(origin, source, target)?
            }
            (dir::Type::Reference(_), dir::Type::Shape(_)) => {
                self.decide_reference_shape_assignable(origin, source, target)?
            }
            (dir::Type::Shape(_), dir::Type::Application(instance))
                if self.symbol_kind(instance.symbol).is_interface() =>
            {
                self.decide_interface_relation(origin, Relation::Assignable, source, target)?
            }
            (dir::Type::Application(reference), dir::Type::Shape(_)) => self
                .decide_reference_against_target(
                    origin,
                    Relation::Assignable,
                    source.module_id,
                    &reference,
                    target,
                )?,
            (dir::Type::Application(source_instance), dir::Type::Application(target_instance))
                if source_instance.symbol == target_instance.symbol =>
            {
                // relate argument pairs by their parameter variances
                let symbol = source_instance.symbol;
                let source_arguments = SmallVec::<[_; 4]>::from_slice(
                    self.type_ids(source.module_id, source_instance.arguments)?,
                );
                let target_arguments = SmallVec::<[_; 4]>::from_slice(
                    self.type_ids(target.module_id, target_instance.arguments)?,
                );

                self.decide_type_arguments(
                    origin,
                    symbol,
                    self.default_variance_form(symbol),
                    Relation::Widens,
                    &source_arguments,
                    &target_arguments,
                )?
            }
            (dir::Type::Application(_), dir::Type::Application(_)) => {
                self.decide_application_assignable(origin, source, target)?
            }

            // functions assign by signature variance
            (dir::Type::FunctionSignature(_), dir::Type::FunctionSignature(_)) => {
                self.decide_function_assignable(origin, relation, source, target)?
            }
            (_, dir::Type::FunctionSignature(_)) if let Some(source) = source_signature => {
                self.decide_function_assignable(origin, relation, source, target)?
            }
            (dir::Type::FunctionSignature(_), _) if let Some(target) = target_signature => {
                self.decide_function_assignable(origin, relation, source, target)?
            }
            (_, _) if let (Some(source), Some(target)) = (source_signature, target_signature) => {
                self.decide_function_assignable(origin, relation, source, target)?
            }

            _ => Answer::Ready(false),
        };

        Ok(decision)
    }

    /// Decide whether one source value can erase into `Dynamic<constraint>`.
    pub(in crate::check) fn decide_dynamic_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        constraint: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let source = match self.ty(source)? {
            dir::Type::Dynamic(dynamic) => dynamic.constraint,
            _ => source,
        };

        if !answer!(self.satisfies_auto_interface(
            origin,
            source,
            dir::AutoInterface::DynamicSafe,
        )?) {
            return Ok(Answer::Ready(false));
        }

        self.decide_relation(origin, Relation::Assignable, source, constraint)
    }

    /// Decide whether one parameter's bounds carry one relation.
    pub(in crate::check) fn decide_parameter_relation(
        &mut self,
        origin: Origin,
        relation: Relation,
        parameter: dir::GlobalGenericParameterId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // prove through any declared or assumed bound
        let mut decision = Answer::Ready(false);
        for bound in self.parameter_bounds(origin, parameter)? {
            decision = decision.or(self.decide_relation(origin, relation, bound, target)?);
            if decision.is_ready_true() {
                break;
            }
        }

        Ok(decision)
    }

    /// Decide whether one assumed bound carries `this`.
    pub(in crate::check) fn decide_this_relation(
        &mut self,
        origin: Origin,
        relation: Relation,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // prove through any bound carried by this
        let mut decision = Answer::Ready(false);
        for bound in self.this_bounds(origin)? {
            decision = decision.or(self.decide_relation(origin, relation, bound, target)?);
            if decision.is_ready_true() {
                break;
            }
        }

        Ok(decision)
    }
}
