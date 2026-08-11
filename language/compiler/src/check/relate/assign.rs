use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{CheckState, Origin, Relation};

impl CheckState<'_> {
    /// Decide assignability from one reduced source to one reduced target.
    pub(in crate::check) fn decide_assignable(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let widens = relation == Relation::Widens;

        let source_signature = self.callable_signature(source)?;
        let target_signature = self.callable_signature(target)?;

        let decision = match (self.ty(source)?, self.ty(target)?) {
            // top and error types absorb everything
            (dir::Type::Error, _) | (_, dir::Type::Error) => true,
            // collect lifetimes without deciding, leaving outlives to Verify on MIR
            (_, _)
                if self.is_lifetime_slot_type(source)? && self.is_lifetime_slot_type(target)? =>
            {
                true
            }
            // box values into an existential target, which never widens
            (_, dir::Type::Any) | (_, dir::Type::Unknown) => !widens,
            (dir::Type::Any, _) => !widens,
            (dir::Type::Never, _) => true,

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
            // accept every template literal instance as a string
            (dir::Type::Operation(operation), dir::Type::Primitive(dir::PrimitiveType::String))
                if matches!(
                    self.type_operation(source.module_id, operation)?,
                    dir::TypeOperation::TemplateLiteral(_)
                ) =>
            {
                true
            }
            // unconstraining patterns absorb the whole string domain
            (dir::Type::Primitive(dir::PrimitiveType::String), dir::Type::Operation(operation))
                if let dir::TypeOperation::TemplateLiteral(template) =
                    self.type_operation(target.module_id, operation)? =>
            {
                self.decide_string_inhabits_template(target.module_id, &template)?
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
                true
            }

            // decide memory forms through their placement and readonly views
            _ if let Some(decision) =
                self.constrain_form_assignable_rooted(origin, relation, source, target)? =>
            {
                decision
            }

            // widen literal and constructed values into a union target by membership
            (dir::Type::Literal(_) | dir::Type::Object(_), dir::Type::Union(union)) if widens => {
                let elements = self.type_ids(target.module_id, union.elements)?.to_vec();

                self.decide_any_target(origin, relation, source, &elements)?
            }
            // reject every other widening into or out of a union
            (dir::Type::Union(_), _) | (_, dir::Type::Union(_)) if widens => false,

            // require every element of a union source to assign
            (dir::Type::Union(union), _) => {
                let elements = self.type_ids(source.module_id, union.elements)?.to_vec();

                self.decide_all_sources(origin, relation, &elements, target)?
            }
            // assign parameters and erased arguments through their constraints or a union target
            (dir::Type::Parameter(parameter) | dir::Type::Erased(parameter), _) => {
                let decision =
                    self.decide_parameter_relation(origin, relation, parameter, target)?;

                self.decide_union_membership(origin, relation, decision, source, target)?
            }
            // reject concrete writes into an existential erased target
            (_, dir::Type::Erased(_)) => false,
            // assign this through its enclosing interface hypotheses or a union target
            (dir::Type::This, _) => {
                let decision = self.decide_this_relation(origin, Relation::Assignable, target)?;

                self.decide_union_membership(origin, relation, decision, source, target)?
            }
            // rigid projections assign through their declared constraint
            (dir::Type::Member(member), _) => {
                let member = self.type_member(source.module_id, member)?;
                let constraint = self.body().projection_constraint(origin, &member)?;
                let decision = match constraint {
                    Some(constraint) => {
                        self.decide_relation(origin, relation, constraint, target)?
                    }
                    None => false,
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
            // accept a union target when one element is viable
            (_, dir::Type::Union(union)) => {
                let elements = self.type_ids(target.module_id, union.elements)?.to_vec();

                self.decide_any_target(origin, relation, source, &elements)?
            }
            // require every element of an intersection target
            (_, dir::Type::Intersection(intersection)) => {
                let elements = self
                    .type_ids(target.module_id, intersection.elements)?
                    .to_vec();

                self.decide_all_targets(origin, relation, source, &elements)?
            }
            // box values into an existential target, which never widens
            (_, dir::Type::Dynamic(_)) | (dir::Type::Dynamic(_), _) if widens => false,
            // erase compatible values into dynamic targets
            (_, dir::Type::Dynamic(dynamic)) => {
                self.decide_dynamic_assignable(origin, source, dynamic.constraint)?
            }
            // relate a dynamic source through its constraint
            (dir::Type::Dynamic(dynamic), _) => {
                self.decide_relation(origin, Relation::Assignable, dynamic.constraint, target)?
            }

            // reject widening for literals without a uniform carrier
            (dir::Type::Literal(literal), _) if widens && !literal.has_uniform_carrier() => false,
            (dir::Type::Range(_), _) if widens => false,

            // relate literal and interval sources to interface targets
            (dir::Type::Literal(_) | dir::Type::Range(_), dir::Type::Application(instance))
                if self
                    .symbol_kind_maybe(instance.symbol)?
                    .is_some_and(|kind| kind.is_interface()) =>
            {
                self.decide_interface_relation(origin, Relation::Assignable, source, target)?
                    .holds()
            }

            // literals and intervals widen by value
            (dir::Type::Literal(literal), target) => literal.widens_to(&target),
            (dir::Type::Range(range), target) => range.widens_to(&target),
            (dir::Type::Variant(member), _) => {
                self.decide_relation(origin, relation, member.owner, target)?
            }

            // keep mutable collection elements invariant, since they alias
            (dir::Type::Array(source), dir::Type::Array(target)) => {
                self.decide_relation(origin, Relation::Equal, source.element, target.element)?
            }
            (dir::Type::Array(source), dir::Type::Slice(target)) => {
                self.decide_relation(origin, Relation::Equal, source.element, target.element)?
            }
            (dir::Type::Array(_), dir::Type::FixedArray(_)) => false,
            (dir::Type::Slice(source), dir::Type::Slice(target)) => {
                self.decide_relation(origin, Relation::Equal, source.element, target.element)?
            }
            // widen value container elements, since the container copies whole
            (dir::Type::FixedArray(source), dir::Type::FixedArray(target)) => {
                let element =
                    self.decide_relation(origin, Relation::Widens, source.element, target.element)?;
                let count =
                    self.decide_relation(origin, Relation::Equal, source.count, target.count)?;

                element && count
            }
            // view a fixed array through a slice of the same element
            (dir::Type::FixedArray(source), dir::Type::Slice(target)) if !widens => {
                self.decide_relation(origin, Relation::Equal, source.element, target.element)?
            }
            // reject growing into a managed array, which allocates and copies
            (dir::Type::FixedArray(_), dir::Type::Array(_)) => false,
            (dir::Type::Tuple(_), dir::Type::Tuple(_)) => {
                self.decide_tuple_assignable(origin, relation, source, target)?
            }

            // structural shapes and nominal boundaries
            (dir::Type::Object(_), dir::Type::Object(_)) => {
                self.decide_shape_equal(origin, source, target)?
            }
            (dir::Type::Shape(_) | dir::Type::Object(_), dir::Type::Shape(_)) => {
                self.decide_shape_assignable(origin, source, target)?
            }
            (dir::Type::Reference(_), dir::Type::Shape(_)) => {
                self.decide_reference_shape_assignable(origin, source, target)?
            }
            // relate structural, callable, and scalar values to interface targets
            (
                dir::Type::Shape(_)
                | dir::Type::Object(_)
                | dir::Type::FunctionSignature(_)
                | dir::Type::Function(_)
                | dir::Type::FunctionPointer(_)
                | dir::Type::Primitive(_)
                | dir::Type::Array(_)
                | dir::Type::Slice(_)
                | dir::Type::FixedArray(_),
                dir::Type::Application(instance),
            ) if self
                .symbol_kind_maybe(instance.symbol)?
                .is_some_and(|kind| kind.is_interface()) =>
            {
                self.decide_interface_relation(origin, Relation::Assignable, source, target)?
                    .holds()
            }
            // relate callable applications to interface targets
            (dir::Type::Application(callable), dir::Type::Application(instance))
                if self
                    .symbol_kind_maybe(instance.symbol)?
                    .is_some_and(|kind| kind.is_interface())
                    && self.is_function_language_item(callable.symbol)? =>
            {
                self.decide_interface_relation(origin, Relation::Assignable, source, target)?
                    .holds()
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
                let form = self.default_variance_form(symbol)?;

                self.decide_type_arguments(
                    origin,
                    symbol,
                    form,
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

            _ => false,
        };

        Ok(decision)
    }

    /// Decide whether one source value can erase into `Dynamic<constraint>`.
    pub(in crate::check) fn decide_dynamic_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        constraint: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let source = match self.ty(source)? {
            dir::Type::Dynamic(dynamic) => dynamic.constraint,
            _ => source,
        };

        // box erasable values only behind a dynamic constraint
        if !self.satisfies_auto_interface(origin, source, dir::AutoInterface::DynamicSafe)? {
            return Ok(false);
        }

        self.decide_relation(origin, Relation::Assignable, source, constraint)
    }

    /// Decide whether one parameter's bounds prove one relation.
    pub(in crate::check) fn decide_parameter_relation(
        &mut self,
        origin: Origin,
        relation: Relation,
        parameter: dir::GlobalGenericParameterId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // prove through any declared or assumed bound
        for bound in self.parameter_bounds(origin, parameter)? {
            if self.decide_relation(origin, relation, bound, target)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Decide whether one assumed `this` bound proves one relation.
    pub(in crate::check) fn decide_this_relation(
        &mut self,
        origin: Origin,
        relation: Relation,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // prove through any assumed this bound
        for bound in self.this_bounds(origin)? {
            if self.decide_relation(origin, relation, bound, target)? {
                return Ok(true);
            }
        }

        Ok(false)
    }
}
