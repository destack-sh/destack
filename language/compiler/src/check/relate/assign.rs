use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, Relation, ScalarFamily, answer};

impl CheckState<'_> {
    /// Decide assignability from one reduced source to one reduced target.
    ///
    /// `Widens` decides the same judgment restricted to identity-witnessed
    /// edges: conversions that would reify as coercions never widen.
    pub(in crate::check) fn decide_assignable(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        if self.decide_equal(origin, source, target)?.is_ready_true() {
            return Ok(Answer::Ready(true));
        }
        let widens = relation == Relation::Widens;

        let source_signature = self.callable_signature(source)?;
        let target_signature = self.callable_signature(target)?;
        let decision = match (self.ty(source)?, self.ty(target)?) {
            // top and error types absorb everything
            (dir::Type::Error, _) | (_, dir::Type::Error) => Answer::Ready(true),
            // collect lifetime components, Verify checks outlives on MIR
            (
                dir::Type::Memory(dir::MemoryLiteral::Lifetime(_)),
                dir::Type::Memory(dir::MemoryLiteral::Lifetime(_)),
            ) => Answer::Ready(true),
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
                self.constrain_form_assignable(origin, relation, source, target)? =>
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
                let decision = self.decide_this_assignable(origin, target)?;

                self.decide_union_membership(origin, relation, decision, source, target)?
            }
            // rigid projections assign through their declared constraint
            (dir::Type::Member(member), _) => {
                let member = self.type_member(source.module_id, member)?;
                let constraint = self
                    .body(origin.module())
                    .projection_constraint(origin, &member)?;
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
            // interface-typed values already store as their own dynamic
            //  carrier, so wrapping and unwrapping the written type is identity
            (dir::Type::Instance(instance), dir::Type::Dynamic(dynamic))
                if self.is_interface_instance(Some(&instance))? =>
            {
                self.decide_relation(origin, relation, source, dynamic.constraint)?
            }
            (dir::Type::Dynamic(dynamic), dir::Type::Instance(instance))
                if self.is_interface_instance(Some(&instance))? =>
            {
                self.decide_relation(origin, relation, dynamic.constraint, target)?
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

            // comptime scalars prove against rigid parameters universally:
            //  the value must fit every member of the parameter's families
            (dir::Type::Literal(literal), dir::Type::Parameter(_)) => {
                let families = answer!(self.scalar_families(origin, target)?);
                let holds = families.is_some_and(|families| {
                    !families.is_empty()
                        && families.iter().all(|family| {
                            matches!(
                                family,
                                ScalarFamily::Domain(domain) if literal.widens_to_domain(*domain)
                            )
                        })
                });

                Answer::Ready(holds)
            }

            // literals and intervals widen by value
            (dir::Type::Literal(literal), target) => Answer::Ready(literal.widens_to(&target)),
            (dir::Type::Range(range), target) => Answer::Ready(range.widens_to(&target)),
            (dir::Type::EnumMember(member), _) => {
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
            (dir::Type::Shape(_), dir::Type::Instance(reference)) => {
                self.decide_source_against_reference(origin, source, target.module_id, &reference)?
            }
            (dir::Type::Instance(reference), dir::Type::Shape(_)) => {
                self.decide_reference_against_target(origin, source.module_id, &reference, target)?
            }
            (dir::Type::Instance(source_instance), dir::Type::Instance(target_instance))
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

                let context = self.default_symbol_context(symbol);

                self.decide_type_arguments(
                    origin,
                    symbol,
                    context,
                    Relation::Widens,
                    &source_arguments,
                    &target_arguments,
                )?
            }
            (dir::Type::Instance(_), dir::Type::Instance(_)) => {
                self.decide_nominal_assignable(origin, source, target)?
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

    /// Decide whether one value may be written into one place type.
    pub(in crate::check) fn decide_writable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // shape literals construct structs member-wise
        if let (dir::Type::Shape(_), dir::Type::Instance(instance)) =
            (self.ty(source)?, self.ty(target)?)
        {
            let is_struct = matches!(
                self.definition(instance.symbol)?,
                Some(dir::Definition::Struct(_))
            );
            if is_struct {
                return self.decide_struct_construction(origin, source, target, &instance);
            }
        }

        // union sources distribute as ordinary assignability
        if matches!(self.ty(source)?, dir::Type::Union(_)) {
            return self.decide_assignable(origin, Relation::Assignable, source, target);
        }

        // fresh values keep their freshness through target arms
        if let dir::Type::Union(union) = self.ty(target)? {
            let elements = self.type_ids(target.module_id, union.elements)?.to_vec();

            return self.decide_any_target(origin, Relation::Writable, source, &elements);
        }
        if let dir::Type::Intersection(intersection) = self.ty(target)? {
            let elements = self
                .type_ids(target.module_id, intersection.elements)?
                .to_vec();

            return self.decide_all_targets(origin, Relation::Writable, source, &elements);
        }

        // fresh shapes conform covariantly with strict excess keys
        if let (dir::Type::Shape(_), dir::Type::Shape(_)) = (self.ty(source)?, self.ty(target)?) {
            return self.decide_fresh_shape_writable(origin, source, target);
        }

        // fresh collections write their elements covariantly
        match (self.ty(source)?, self.ty(target)?) {
            (dir::Type::Array(source_array), dir::Type::Array(target_array)) => {
                return self.decide_relation(
                    origin,
                    Relation::Writable,
                    source_array.element,
                    target_array.element,
                );
            }
            (dir::Type::FixedArray(source_array), dir::Type::FixedArray(target_array)) => {
                let elements = self.decide_relation(
                    origin,
                    Relation::Writable,
                    source_array.element,
                    target_array.element,
                )?;
                let counts = self.decide_relation(
                    origin,
                    Relation::Equal,
                    source_array.count,
                    target_array.count,
                )?;

                return Ok(elements.and(counts));
            }
            (dir::Type::Tuple(source_tuple), dir::Type::Tuple(target_tuple)) => {
                let source_elements = self
                    .tuple_elements(source.module_id, source_tuple.elements)?
                    .to_vec();
                let target_elements = self
                    .tuple_elements(target.module_id, target_tuple.elements)?
                    .to_vec();
                if source_elements.len() == target_elements.len() {
                    let mut decision = Answer::Ready(true);
                    for (source_element, target_element) in
                        source_elements.iter().zip(target_elements.iter())
                    {
                        decision = decision.and(self.decide_relation(
                            origin,
                            Relation::Writable,
                            source_element.ty,
                            target_element.ty,
                        )?);
                        if decision.is_ready_false() {
                            break;
                        }
                    }

                    return Ok(decision);
                }
            }
            _ => {}
        }

        self.decide_assignable(origin, Relation::Assignable, source, target)
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
    pub(in crate::check) fn decide_this_assignable(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // prove through any assumed bound on this
        let mut decision = Answer::Ready(false);
        for bound in self.assumed_bounds(origin, |ty| matches!(ty, dir::Type::This))? {
            decision =
                decision.or(self.decide_relation(origin, Relation::Assignable, bound, target)?);
            if decision.is_ready_true() {
                break;
            }
        }

        Ok(decision)
    }
}
