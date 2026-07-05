use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, Relation, ScalarFamily, answer};

impl CheckState<'_> {
    /// Decide assignability from one reduced source to one reduced target.
    pub(in crate::check) fn decide_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        if self.decide_equal(origin, source, target)?.is_ready_true() {
            return Ok(Answer::Ready(true));
        }
        if answer!(self.widens_to(origin, source, target)?) {
            return Ok(Answer::Ready(true));
        }

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
            (_, dir::Type::Any) | (_, dir::Type::Unknown) => Answer::Ready(true),
            (dir::Type::Any, _) => Answer::Ready(true),
            (dir::Type::Never, _) => Answer::Ready(true),

            // exact property keys flow into their primitive key domains
            (_, dir::Type::Primitive(primitive))
                if self
                    .static_key_from_type(source)?
                    .is_some_and(|key| primitive_accepts_key(primitive, key)) =>
            {
                Answer::Ready(true)
            }

            // memory forms own placement and readonly views
            _ if let Some(decision) = self.constrain_form_assignable(origin, source, target)? => {
                decision
            }

            // union sources need every element assignable
            (dir::Type::Union(union), _) => {
                let elements = self.type_ids(source.module_id, union.elements)?.to_vec();

                self.decide_all_assignable(origin, &elements, target)?
            }
            // parameters assign through their constraints, or sit inside a union target
            (dir::Type::Parameter(parameter), _) => {
                let decision = self.decide_parameter_assignable(origin, parameter, target)?;

                self.decide_union_membership(
                    origin,
                    Relation::Assignable,
                    decision,
                    source,
                    target,
                )?
            }
            // this assigns through its enclosing interface hypotheses, or sits inside a union target
            (dir::Type::This, _) => {
                let decision = self.decide_this_assignable(origin, target)?;

                self.decide_union_membership(
                    origin,
                    Relation::Assignable,
                    decision,
                    source,
                    target,
                )?
            }
            // intersection sources assign through any element
            (dir::Type::Intersection(intersection), _) => {
                let elements = self
                    .type_ids(source.module_id, intersection.elements)?
                    .to_vec();
                let mut decision = Answer::Ready(false);
                for element in elements {
                    decision = decision.or(self.decide_relation(
                        origin,
                        Relation::Assignable,
                        element,
                        target,
                    )?);
                    if decision.is_ready_true() {
                        break;
                    }
                }

                decision
            }
            // union targets need one viable element
            (_, dir::Type::Union(union)) => {
                let elements = self.type_ids(target.module_id, union.elements)?.to_vec();

                self.decide_any_assignable(origin, source, &elements)?
            }
            // intersection targets need every element
            (_, dir::Type::Intersection(intersection)) => {
                let elements = self
                    .type_ids(target.module_id, intersection.elements)?
                    .to_vec();
                let mut decision = Answer::Ready(true);
                for element in elements {
                    decision = decision.and(self.decide_relation(
                        origin,
                        Relation::Assignable,
                        source,
                        element,
                    )?);
                    if decision.is_ready_false() {
                        break;
                    }
                }

                decision
            }
            // erase compatible values into dynamic targets
            (_, dir::Type::Dynamic(dynamic)) => {
                let constraint = dynamic.constraint;

                self.decide_dynamic_assignable(origin, source, constraint)?
            }

            // comptime scalars prove against rigid parameters universally:
            // the value must fit every member of the parameter's families
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
                self.decide_relation(origin, Relation::Assignable, member.owner, target)?
            }

            // mutable collections alias their elements and stay invariant
            (dir::Type::Array(source), dir::Type::Array(target)) => {
                let (source, target) = (source.element, target.element);

                self.decide_relation(origin, Relation::Equal, source, target)?
            }
            (dir::Type::Array(source), dir::Type::Slice(target)) => {
                let (source, target) = (source.element, target.element);

                self.decide_relation(origin, Relation::Equal, source, target)?
            }
            (dir::Type::Array(_), dir::Type::FixedArray(_)) => Answer::Ready(false),
            (dir::Type::Slice(source), dir::Type::Slice(target)) => {
                let (source, target) = (source.element, target.element);

                self.decide_relation(origin, Relation::Equal, source, target)?
            }
            (dir::Type::FixedArray(source), dir::Type::FixedArray(target)) => {
                let (source_element, target_element) = (source.element, target.element);
                let (source_count, target_count) = (source.count, target.count);
                let element = self.decide_relation(
                    origin,
                    Relation::Assignable,
                    source_element,
                    target_element,
                )?;
                let count =
                    self.decide_relation(origin, Relation::Equal, source_count, target_count)?;

                element.and(count)
            }
            (dir::Type::FixedArray(source), dir::Type::Slice(target)) => {
                let (source, target) = (source.element, target.element);

                self.decide_relation(origin, Relation::Assignable, source, target)?
            }
            (dir::Type::Tuple(_), dir::Type::Tuple(_)) => {
                self.decide_tuple_assignable(origin, source, target)?
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

                self.decide_type_arguments(origin, symbol, &source_arguments, &target_arguments)?
            }
            (dir::Type::Instance(_), dir::Type::Instance(_)) => {
                self.decide_nominal_assignable(origin, source, target)?
            }

            // functions assign by signature variance
            (dir::Type::FunctionSignature(_), dir::Type::FunctionSignature(_)) => {
                self.decide_function_assignable(origin, source, target)?
            }
            (_, dir::Type::FunctionSignature(_)) if let Some(source) = source_signature => {
                self.decide_function_assignable(origin, source, target)?
            }
            (dir::Type::FunctionSignature(_), _) if let Some(target) = target_signature => {
                self.decide_function_assignable(origin, source, target)?
            }
            (_, _) if let (Some(source), Some(target)) = (source_signature, target_signature) => {
                self.decide_function_assignable(origin, source, target)?
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
                self.definition(instance.symbol),
                Some(dir::Definition::Struct(_))
            );
            if is_struct {
                return self.decide_struct_construction(origin, source, target, &instance);
            }
        }

        self.decide_assignable(origin, source, target)
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

    /// Decide assignability under one deep readonly form.
    ///
    /// Aliasing containers relax to covariant element relations because
    ///  no mutation can flow back through the form.
    pub(in crate::check) fn decide_readonly_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // readable collections relate elements covariantly
        let elements = match (self.ty(source)?, self.ty(target)?) {
            (dir::Type::Array(source), dir::Type::Array(target)) => {
                Some((source.element, target.element))
            }
            (dir::Type::Array(source), dir::Type::Slice(target)) => {
                Some((source.element, target.element))
            }
            (dir::Type::Slice(source), dir::Type::Slice(target)) => {
                Some((source.element, target.element))
            }
            _ => None,
        };

        match elements {
            Some((source, target)) => self.decide_readonly_assignable(origin, source, target),
            None => self.decide_assignable(origin, source, target),
        }
    }

    /// Decide whether one parameter's constraint carries the assignment.
    pub(in crate::check) fn decide_parameter_assignable(
        &mut self,
        origin: Origin,
        parameter: dir::GlobalGenericParameterId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // prove through any declared or assumed bound
        let mut decision = Answer::Ready(false);
        for bound in self.parameter_bounds(origin, parameter)? {
            decision =
                decision.or(self.decide_relation(origin, Relation::Assignable, bound, target)?);
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

/// Return whether one primitive key domain accepts one exact property key.
fn primitive_accepts_key(primitive: dir::PrimitiveType, key: dir::StaticKey) -> bool {
    match primitive {
        dir::PrimitiveType::String => key.is_string_like(),
        dir::PrimitiveType::Symbol | dir::PrimitiveType::UniqueSymbol => key.is_symbol_like(),
        dir::PrimitiveType::Integer(_) => key.is_number_like(),
        dir::PrimitiveType::Boolean
        | dir::PrimitiveType::Character
        | dir::PrimitiveType::Float(_)
        | dir::PrimitiveType::Bigint => false,
    }
}
