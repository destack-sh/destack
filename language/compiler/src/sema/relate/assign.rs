use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::sema::{CauseId, CheckState, Origin, Relation, Verdict};

impl CheckState<'_> {
    /// Relate assignability from one reduced source to one reduced target.
    pub(in crate::sema) fn relate_assignable(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        let widens = relation == Relation::Widens;

        let source_signature = self.callable_signature(source)?;
        let target_signature = self.callable_signature(target)?;

        let decision = match (self.ty(source)?, self.ty(target)?) {
            // error types absorb everything
            (dir::Type::Error, _) | (_, dir::Type::Error) => Verdict::Holds,
            // box erasable values into an erased top target
            (_, dir::Type::Any) | (_, dir::Type::Unknown) => match widens {
                true => Verdict::Fails,
                false => self.erasable_source(origin, source)?,
            },
            (dir::Type::Any, _) => Verdict::decided(!widens),
            (dir::Type::Never, _) => Verdict::Holds,

            // string literals inhabit matching template literal patterns
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
            // accept every template literal instance as a string
            (dir::Type::Operation(operation), dir::Type::Primitive(dir::PrimitiveType::String))
                if matches!(
                    self.type_operation(source.module_id, operation)?,
                    dir::TypeOperation::TemplateLiteral(_)
                ) =>
            {
                Verdict::Holds
            }
            // patterns with only empty segments absorb the whole string domain
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
            // template patterns compare span-wise through their pieces
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

            // exact property keys flow into their primitive key domains
            (_, dir::Type::Primitive(primitive))
                if !widens
                    && self
                        .static_key_from_type(source)?
                        .is_some_and(|key| key.widens_to_primitive(primitive)) =>
            {
                Verdict::Holds
            }

            // require every element of an intersection target
            (_, dir::Type::Intersection(intersection)) => {
                let elements: SmallVec<[_; 8]> = self
                    .type_ids(target.module_id, intersection.elements)?
                    .into();

                self.relate_all_targets(origin, cause, relation, source, &elements)?
            }
            // intersection sources assign through any element
            (dir::Type::Intersection(intersection), _) => {
                let elements: SmallVec<[_; 8]> = self
                    .type_ids(source.module_id, intersection.elements)?
                    .into();

                self.relate_any_source(origin, cause, relation, &elements, target)?
            }
            // assign parameters and erased arguments through their bounds, then their form
            (dir::Type::Parameter(parameter) | dir::Type::Erased(parameter), _) => {
                let decision = self
                    .relate_parameter_bounds(origin, cause, relation, parameter, target)?
                    .or_else(|| self.relate_into_union(origin, cause, relation, source, target))?;
                match decision {
                    Verdict::Holds => Verdict::Holds,
                    decision => self
                        .constrain_form_assignable(origin, cause, relation, source, target)?
                        .unwrap_or(decision),
                }
            }
            // decide memory forms through their placement and readonly views
            _ if let Some(decision) =
                self.constrain_form_assignable(origin, cause, relation, source, target)? =>
            {
                decision
            }

            // widen union representations only through exact set equality
            (dir::Type::Union(source_union), dir::Type::Union(target_union)) if widens => {
                let source_elements: SmallVec<[_; 8]> = self
                    .type_ids(source.module_id, source_union.elements)?
                    .into();
                let target_elements: SmallVec<[_; 8]> = self
                    .type_ids(target.module_id, target_union.elements)?
                    .into();

                self.relate_type_sets_equal(origin, cause, &source_elements, &target_elements)?
            }
            // distribute a union source over its elements, which widen one by one
            (dir::Type::Union(union), _) if widens => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(source.module_id, union.elements)?.into();

                self.relate_all_sources(origin, cause, relation, &elements, target)?
            }
            // widen literal, constructed, and scalar values into a union target by membership
            (
                dir::Type::Literal(_) | dir::Type::Object(_) | dir::Type::Primitive(_),
                dir::Type::Union(union),
            ) if widens => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(target.module_id, union.elements)?.into();

                self.relate_any_target(origin, cause, relation, source, &elements)?
            }
            // try union membership for a nominal value while the target still solves a parameter
            (dir::Type::Application(_), dir::Type::Union(union))
                if widens && self.type_flags(target)?.has_variable() =>
            {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(target.module_id, union.elements)?.into();

                self.relate_any_target(origin, cause, relation, source, &elements)?
            }
            // reject every other widening into a union
            (_, dir::Type::Union(_)) if widens => Verdict::Fails,

            // require every element of a union source to assign
            (dir::Type::Union(union), _) => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(source.module_id, union.elements)?.into();

                self.relate_all_sources(origin, cause, relation, &elements, target)?
            }
            // reject concrete writes into an erased parameter target
            (_, dir::Type::Erased(_)) => Verdict::Fails,
            // assign this through its enclosing interface hypotheses or a union target
            (dir::Type::This, _) => self
                .relate_this_bounds(origin, cause, relation, target)?
                .or_else(|| self.relate_into_union(origin, cause, relation, source, target))?,
            // rigid projections assign through their declared constraint
            (dir::Type::Member(member), _) => {
                let member = self.type_member(source.module_id, member)?;
                let constraint = self.body().projection_constraint(origin, &member)?;
                let decision = match constraint {
                    Some(constraint) => {
                        self.constrain_type(origin, cause, relation, constraint, target)?
                    }
                    None => Verdict::Fails,
                };

                decision
                    .or_else(|| self.relate_into_union(origin, cause, relation, source, target))?
            }
            // accept a union target when one element is viable
            (_, dir::Type::Union(union)) => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(target.module_id, union.elements)?.into();

                self.relate_any_target(origin, cause, relation, source, &elements)?
            }
            // relate two erased carriers through their constraints, which rebuild the fat pointer
            (dir::Type::Dynamic(source_dynamic), dir::Type::Dynamic(target_dynamic)) => {
                let constraint_relation = match widens {
                    true => Relation::Equal,
                    false => Relation::Assignable,
                };

                self.constrain_type(
                    origin,
                    cause,
                    constraint_relation,
                    source_dynamic.constraint,
                    target_dynamic.constraint,
                )?
            }
            // widen between erased values only through equal constraints
            _ if widens
                && let Some(source_constraint) = self.erased_constraint(source)?
                && let Some(target_constraint) = self.erased_constraint(target)? =>
            {
                self.constrain_type(
                    origin,
                    cause,
                    Relation::Equal,
                    source_constraint,
                    target_constraint,
                )?
            }
            // erased carriers refuse widening, the literal materializes first
            _ if widens && (self.is_erased_value(source)? || self.is_erased_value(target)?) => {
                Verdict::Fails
            }
            // erase compatible values into dynamic targets
            (_, dir::Type::Dynamic(dynamic)) => {
                self.relate_dynamic_assignable(origin, cause, source, dynamic.constraint)?
            }
            // relate a dynamic source through its constraint
            (dir::Type::Dynamic(dynamic), _) => self.constrain_type(
                origin,
                cause,
                Relation::Assignable,
                dynamic.constraint,
                target,
            )?,

            // reject widening for literals without a uniform carrier
            (dir::Type::Literal(literal), _) if widens && !literal.has_uniform_carrier() => {
                Verdict::Fails
            }
            (dir::Type::Range(_), _) if widens => Verdict::Fails,

            // adapt a const literal to a parameter its scalar-family admits
            (dir::Type::Literal(_), dir::Type::Parameter(_)) => {
                Verdict::decided(self.builtin_scalar_accepts_literal(origin, source, target)?)
            }

            // relate literal and interval sources to interface targets
            (dir::Type::Literal(_) | dir::Type::Range(_), dir::Type::Application(instance))
                if self
                    .symbol_kind_maybe(instance.symbol)?
                    .is_some_and(|kind| kind.is_interface()) =>
            {
                self.relate_erased_assignable(origin, cause, source, target)?
            }

            // literals and intervals widen by value
            (dir::Type::Literal(literal), target) => Verdict::decided(literal.widens_to(&target)),
            (dir::Type::Range(range), target) => Verdict::decided(range.widens_to(&target)),

            // precise variants assign through their declared owner
            (dir::Type::Variant(member), _)
                if !matches!(self.ty(target)?, dir::Type::Variant(_)) =>
            {
                self.constrain_type(origin, cause, relation, member.owner, target)?
            }

            // keep array views over slices and fixed arrays element-invariant
            (dir::Type::Application(_), dir::Type::Slice(target))
                if let Some(element) = self.array_element(source)? =>
            {
                self.constrain_type(origin, cause, Relation::Equal, element, target.element)?
            }
            (dir::Type::Application(_), dir::Type::FixedArray(_))
                if self.array_element(source)?.is_some() =>
            {
                Verdict::Fails
            }
            (dir::Type::Slice(source), dir::Type::Slice(target)) => self.constrain_type(
                origin,
                cause,
                Relation::Equal,
                source.element,
                target.element,
            )?,
            // widen value container elements, which the container copies whole
            (dir::Type::FixedArray(source), dir::Type::FixedArray(target)) => {
                let element = self.constrain_type(
                    origin,
                    cause,
                    relation.interior(),
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
            // view a fixed array through a slice of the same element
            (dir::Type::FixedArray(source), dir::Type::Slice(target)) if !widens => self
                .constrain_type(
                    origin,
                    cause,
                    Relation::Equal,
                    source.element,
                    target.element,
                )?,
            // reject growing into a managed array, which allocates and copies
            (dir::Type::FixedArray(_), dir::Type::Application(_))
                if self.array_element(target)?.is_some() =>
            {
                Verdict::Fails
            }
            (dir::Type::Tuple(_), dir::Type::Tuple(_)) => {
                self.relate_tuple_assignable(origin, cause, relation, source, target)?
            }
            // an array flows into the tuple writing its rest, `T[]` into `[...T[]]`
            (dir::Type::Application(_), dir::Type::Tuple(tuple))
                if let Some(rest) = self.sole_rest_container(target, tuple)? =>
            {
                self.constrain_type(origin, cause, relation, source, rest)?
            }

            // anonymous classes and nominal declarations
            (dir::Type::Object(_), dir::Type::Object(target_shape)) => {
                match target_shape.declares_signatures() {
                    // an object type declaring signatures reads its members structurally
                    true => {
                        self.relate_shape(origin, cause, Relation::Assignable, source, target)?
                    }
                    // a written field set stores at its exact member set
                    false => self.relate_shape_equal(origin, cause, source, target)?,
                }
            }
            // satisfy an object type declaring signatures from a static declaration reference
            (dir::Type::Reference(_), dir::Type::Object(target_shape))
                if target_shape.declares_signatures() =>
            {
                self.relate_reference_shape_assignable(origin, cause, source, target)?
            }
            // satisfy a bare construct signature from the declared constructors
            (dir::Type::Reference(_), dir::Type::FunctionSignature(_))
                if self
                    .signature_head(target)?
                    .is_some_and(|head| head.is_construct) =>
            {
                self.relate_reference_construct_assignable(origin, cause, relation, source, target)?
            }
            // satisfy a target from a struct static's carrier type
            (dir::Type::Static(value), _)
                if !matches!(self.ty(target)?, dir::Type::Static(_))
                    && let dir::StaticTerm::Struct { ty, .. } = self.r#static(value).clone() =>
            {
                self.constrain_type(origin, cause, relation, ty, target)?
            }
            // satisfy a bare construct signature from a class value's static side
            (dir::Type::Static(value), dir::Type::FunctionSignature(_))
                if self
                    .signature_head(target)?
                    .is_some_and(|head| head.is_construct)
                    && let dir::StaticTerm::Type { ty } = self.r#static(value).clone()
                    && matches!(self.ty(ty)?, dir::Type::Reference(_)) =>
            {
                self.relate_reference_construct_assignable(origin, cause, relation, ty, target)?
            }
            // satisfy a keyed object type from a nominal instance
            (dir::Type::Application(reference), dir::Type::Object(target_shape))
                if target_shape.declares_signatures() =>
            {
                self.relate_reference_against_target(
                    origin,
                    cause,
                    Relation::Assignable,
                    source.module_id,
                    &reference,
                    target,
                )?
            }
            // relate structural, callable, and scalar values to interface targets
            (
                dir::Type::Object(_)
                | dir::Type::FunctionSignature(_)
                | dir::Type::Function(_)
                | dir::Type::FunctionPointer(_)
                | dir::Type::Primitive(_)
                | dir::Type::Slice(_)
                | dir::Type::FixedArray(_),
                dir::Type::Application(instance),
            ) if self
                .symbol_kind_maybe(instance.symbol)?
                .is_some_and(|kind| kind.is_interface()) =>
            {
                self.relate_erased_assignable(origin, cause, source, target)?
            }
            // relate callable applications to interface targets
            (dir::Type::Application(callable), dir::Type::Application(instance))
                if self
                    .symbol_kind_maybe(instance.symbol)?
                    .is_some_and(|kind| kind.is_interface())
                    && self.is_function_language_item(callable.symbol)? =>
            {
                self.relate_erased_assignable(origin, cause, source, target)?
            }
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

                self.relate_application_arguments(
                    origin,
                    cause,
                    symbol,
                    form,
                    relation.interior(),
                    &source_arguments,
                    &target_arguments,
                )?
            }
            (dir::Type::Application(_), dir::Type::Application(instance))
                if self
                    .symbol_kind_maybe(instance.symbol)?
                    .is_some_and(|kind| kind.is_interface()) =>
            {
                // box erasable values only behind an erased interface target
                match self.erasable_source(origin, source)? {
                    Verdict::Holds => {
                        self.relate_application_assignable(origin, cause, source, target)?
                    }
                    verdict @ (Verdict::Fails | Verdict::Ambiguous) => verdict,
                }
            }
            (dir::Type::Application(_), dir::Type::Application(_)) => {
                self.relate_application_assignable(origin, cause, source, target)?
            }

            // functions assign by signature variance
            (dir::Type::FunctionSignature(_), dir::Type::FunctionSignature(_)) => {
                self.relate_function_assignable(origin, cause, relation, source, target)?
            }
            (_, dir::Type::FunctionSignature(_)) if let Some(source) = source_signature => {
                self.relate_function_assignable(origin, cause, relation, source, target)?
            }
            (dir::Type::FunctionSignature(_), _) if let Some(target) = target_signature => {
                self.relate_function_assignable(origin, cause, relation, source, target)?
            }
            (_, _) if let (Some(source), Some(target)) = (source_signature, target_signature) => {
                self.relate_function_assignable(origin, cause, relation, source, target)?
            }

            // a keyof stuck on a parameter proves through the property key domain
            (dir::Type::Operation(operation), _)
                if !widens
                    && self.type_flags(source)?.has_parameter()
                    && matches!(
                        self.type_operation(source.module_id, operation)?,
                        dir::TypeOperation::KeyOf(_)
                    ) =>
            {
                let keys = self.language_type(dir::LanguageItem::PropertyKey, &[])?;

                self.constrain_type(origin, cause, relation, keys, target)?
            }
            _ => Verdict::Fails,
        };

        Ok(decision)
    }

    /// Relate one erasable source value into an erased interface target.
    pub(in crate::sema) fn relate_erased_assignable(
        &mut self,
        origin: Origin,
        cause: CauseId,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        match self.erasable_source(origin, source)? {
            Verdict::Holds => {}
            verdict @ (Verdict::Fails | Verdict::Ambiguous) => return Ok(verdict),
        }

        self.relate_interface(origin, cause, Relation::Assignable, source, target)
    }

    /// Judge whether one source value erases behind a dynamic payload.
    pub(in crate::sema) fn erasable_source(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        self.satisfies_auto_interface(origin, source, dir::AutoInterface::DynamicSafe)
    }

    /// Relate one source value erasing into `Dynamic<constraint>`.
    pub(in crate::sema) fn relate_dynamic_assignable(
        &mut self,
        origin: Origin,
        cause: CauseId,
        source: dir::GlobalTypeId,
        constraint: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        let source = match self.ty(source)? {
            dir::Type::Dynamic(dynamic) => dynamic.constraint,
            _ => source,
        };

        // box erasable values only behind a dynamic constraint
        match self.satisfies_auto_interface(origin, source, dir::AutoInterface::DynamicSafe)? {
            Verdict::Holds => {}
            verdict @ (Verdict::Fails | Verdict::Ambiguous) => return Ok(verdict),
        }

        self.constrain_type(origin, cause, Relation::Assignable, source, constraint)
    }

    /// Return the rest container of a tuple written as `...T[]`, if it is.
    fn sole_rest_container(
        &mut self,
        tuple_id: dir::GlobalTypeId,
        tuple: dir::TupleType,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let elements = self.tuple_elements(tuple_id.module_id, tuple.elements)?;
        let rest = match elements {
            [element] if element.is_rest => Some(element.ty),
            _ => None,
        };

        Ok(rest)
    }

    /// Relate one parameter's bounds against a target.
    pub(in crate::sema) fn relate_parameter_bounds(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        parameter: dir::GlobalGenericParameterId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        let bounds = self.parameter_bounds(origin, parameter)?;

        // choose one open bound by constraint over the whole candidate set
        let mut is_open = self.type_flags(target)?.has_variable();
        for bound in &bounds {
            is_open = is_open || self.type_flags(*bound)?.has_variable();
        }
        if is_open {
            let candidates = bounds
                .iter()
                .map(|bound| (*bound, target))
                .collect::<SmallVec<[_; 4]>>();

            return self.constrain_any_relation(origin, cause, relation, &candidates);
        }

        // prove through any declared or assumed bound
        let mut verdict = Verdict::Fails;
        for bound in bounds {
            verdict = verdict.or(self.constrain_type(origin, cause, relation, bound, target)?);
            if verdict == Verdict::Holds {
                return Ok(Verdict::Holds);
            }
        }

        Ok(verdict)
    }

    /// Relate the assumed `this` bounds against a target.
    pub(in crate::sema) fn relate_this_bounds(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        let bounds = self.this_bounds(origin)?;

        // choose one open bound by constraint over the whole candidate set
        let mut is_open = self.type_flags(target)?.has_variable();
        for bound in &bounds {
            is_open = is_open || self.type_flags(*bound)?.has_variable();
        }
        if is_open {
            let candidates = bounds
                .iter()
                .map(|bound| (*bound, target))
                .collect::<SmallVec<[_; 4]>>();

            return self.constrain_any_relation(origin, cause, relation, &candidates);
        }

        // prove through any assumed this bound
        let mut verdict = Verdict::Fails;
        for bound in bounds {
            verdict = verdict.or(self.constrain_type(origin, cause, relation, bound, target)?);
            if verdict == Verdict::Holds {
                return Ok(Verdict::Holds);
            }
        }

        Ok(verdict)
    }
}
