use smallvec::SmallVec;
use tspp_dir as dir;

use crate::CompilerResult;
use crate::sema::{CauseId, CheckState, Origin, PropertySource, Relation, Verdict};

impl CheckState<'_> {
    /// Relate one directed pair under subtype inclusion or under storage.
    pub(in crate::sema) fn relate_directed(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // distinguish inclusion from storage
        let is_inclusion = relation == Relation::Subtype;

        // read the callable signature behind each side, when one stands there
        let source_signature = self.callable_signature(source)?;
        let target_signature = self.callable_signature(target)?;

        // decide the pair by the heads standing on both sides
        let decision = match (self.ty(source)?, self.ty(target)?) {
            // empty and indeterminate domains
            (dir::Type::Error, _) | (_, dir::Type::Error) => Verdict::Holds,
            (dir::Type::Never, _) => Verdict::Holds,
            (_, dir::Type::Unknown) => Verdict::decided(is_inclusion),
            (dir::Type::Unknown, _) if !self.is_conformance_target(target)? => Verdict::Fails,

            // string literals inhabit matching template literal patterns
            (
                dir::Type::Literal(dir::Literal::String(text))
                | dir::Type::Key(dir::StaticKey::Name(text)),
                dir::Type::Operation(operation),
            ) if let dir::TypeOperation::TemplateLiteral(template) =
                self.type_operation(target.module_id, operation)? =>
            {
                let text = self.strings().get(text).to_string();

                // capture the text into open spans, else match the closed pattern
                let mut is_open = false;
                for span in self.type_ids(target.module_id, template.spans)? {
                    is_open = is_open || self.type_flags(*span)?.has_variable();
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
            // every template literal instance is a string
            (dir::Type::Operation(operation), dir::Type::Primitive(dir::PrimitiveType::String))
                if matches!(
                    self.type_operation(source.module_id, operation)?,
                    dir::TypeOperation::TemplateLiteral(_)
                ) =>
            {
                Verdict::Holds
            }
            // patterns with only empty segments absorb the whole string domain
            (_, dir::Type::Operation(operation))
                if let dir::TypeOperation::TemplateLiteral(template) =
                    self.type_operation(target.module_id, operation)?
                    && self.ownership_payload(origin, source)?
                        == self
                            .intern_type(dir::Type::Primitive(dir::PrimitiveType::String))? =>
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
            // a primitive value inhabits its representation class
            (source_head @ dir::Type::Primitive(_), dir::Type::Application(instance))
                if is_inclusion
                    && let Some(item) = source_head.representation_item()
                    && self.language_symbol(item)? == instance.symbol =>
            {
                Verdict::Holds
            }

            // exact property keys inhabit their primitive key domains
            (_, dir::Type::Primitive(primitive))
                if is_inclusion
                    && self
                        .static_key_from_type(source)?
                        .is_some_and(|key| key.widens_to_primitive(primitive)) =>
            {
                Verdict::Holds
            }

            // intersections
            (_, dir::Type::Intersection(intersection)) => {
                let elements: SmallVec<[_; 8]> = self
                    .type_ids(target.module_id, intersection.elements)?
                    .into();

                self.relate_all_targets(origin, cause, relation, source, &elements)?
            }
            // prove the target from one member of an intersection source
            (dir::Type::Intersection(intersection), _) => {
                let elements: SmallVec<[_; 8]> = self
                    .type_ids(source.module_id, intersection.elements)?
                    .into();
                let by_member =
                    self.relate_any_source(origin, cause, relation, &elements, target)?;
                if by_member.holds() || !matches!(self.ty(target)?, dir::Type::Form(_)) {
                    by_member
                } else {
                    self.relate_form(origin, cause, relation, source, target)?
                        .unwrap_or(by_member)
                }
            }

            // conform a rigid parameter to an interface target
            (dir::Type::Parameter(parameter) | dir::Type::Erased(parameter), _) => {
                let decision = match self.is_conformance_target(target)? {
                    true => self.relate_interface(origin, cause, relation, source, target)?,
                    false => self
                        .relate_into_union(origin, cause, relation, source, target)?
                        .or_else(|| {
                            self.relate_parameter_bounds(origin, cause, relation, parameter, target)
                        })?,
                };
                match decision {
                    Verdict::Holds => Verdict::Holds,
                    decision => self
                        .relate_form(origin, cause, relation, source, target)?
                        .unwrap_or(decision),
                }
            }
            // relate contextual this through its assumed bounds
            (dir::Type::This, _) => self
                .relate_this_bounds(origin, cause, relation, target)?
                .or_else(|| self.relate_into_union(origin, cause, relation, source, target))?,
            // relate a stuck projection through its declared constraint
            (dir::Type::Member(member), _) => {
                let member = self.type_member(source.module_id, member)?;
                let decision = match self.projection_constraint(origin, &member)? {
                    Some(constraint) => {
                        self.constrain_type(origin, cause, relation, constraint, target)?
                    }
                    None => Verdict::Fails,
                };

                decision
                    .or_else(|| self.relate_into_union(origin, cause, relation, source, target))?
            }
            // relate a stuck key domain through the property key domain
            (dir::Type::Operation(operation), _)
                if self.type_flags(source)?.has_parameter()
                    && matches!(
                        self.type_operation(source.module_id, operation)?,
                        dir::TypeOperation::KeyOf(_)
                    ) =>
            {
                let keys = self.language_type(dir::LanguageItem::PropertyKey, &[])?;

                self.constrain_type(origin, cause, relation, keys, target)?
            }

            // memory forms decide through their constructors and readonly views
            _ if let Some(decision) =
                self.relate_form(origin, cause, relation, source, target)? =>
            {
                decision
            }

            // store a union as the exact arm set it already is
            (dir::Type::Union(source_union), dir::Type::Union(target_union)) if !is_inclusion => {
                let source_elements: SmallVec<[_; 8]> = self
                    .type_ids(source.module_id, source_union.elements)?
                    .into();
                let target_elements: SmallVec<[_; 8]> = self
                    .type_ids(target.module_id, target_union.elements)?
                    .into();
                let is_open = self.type_flags(source)?.has_variable()
                    || self.type_flags(target)?.has_variable();
                match is_open {
                    // closed sets store arm by arm
                    false => self.relate_type_sets_stored(
                        origin,
                        cause,
                        relation,
                        &source_elements,
                        &target_elements,
                    )?,
                    // bind each open source arm into one target arm of the same count
                    true if source_elements.len() != target_elements.len() => Verdict::Fails,
                    true => {
                        let shared: SmallVec<[_; 8]> = source_elements
                            .iter()
                            .copied()
                            .filter(|arm| target_elements.contains(arm))
                            .collect();
                        let source_elements: SmallVec<[_; 8]> = source_elements
                            .into_iter()
                            .filter(|arm| !shared.contains(arm))
                            .collect();
                        let target_elements: SmallVec<[_; 8]> = target_elements
                            .into_iter()
                            .filter(|arm| !shared.contains(arm))
                            .collect();
                        let mut verdict = Verdict::Holds;
                        for arm in source_elements {
                            let candidates = target_elements
                                .iter()
                                .map(|target| (arm, *target))
                                .collect::<SmallVec<[_; 4]>>();
                            let armed =
                                self.constrain_any_relation(origin, cause, relation, &candidates)?;
                            verdict = verdict.and(armed);
                            if verdict == Verdict::Fails {
                                break;
                            }
                        }

                        verdict
                    }
                }
            }
            // relate a union source whole to an interface
            (dir::Type::Union(union), _) => {
                if let dir::Type::Application(instance) = self.ty(target)?
                    && self.symbol_kind(instance.symbol)?.is_interface()
                    && self.relate_interface(origin, cause, relation, source, target)?
                        == Verdict::Holds
                {
                    return Ok(Verdict::Holds);
                }

                // relate every arm of a union source
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(source.module_id, union.elements)?.into();

                self.relate_all_sources(origin, cause, relation, &elements, target)?
            }
            // include a value in any arm of a union target
            (_, dir::Type::Union(union)) if is_inclusion => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(target.module_id, union.elements)?.into();

                self.relate_any_target(origin, cause, relation, source, &elements)?
            }
            // store a literal untagged in a union of its scalar domain
            (dir::Type::Literal(literal), dir::Type::Union(union))
                if self.stores_untagged(literal, target, union)? =>
            {
                let arms: SmallVec<[_; 8]> =
                    self.type_ids(target.module_id, union.elements)?.into();
                self.relate_any_target(origin, cause, Relation::Subtype, source, &arms)?
            }
            // store a value in the one inhabited arm of a union field
            (_, dir::Type::Union(union)) => {
                let arms: SmallVec<[_; 8]> =
                    self.type_ids(target.module_id, union.elements)?.into();

                // choose one open arm by constraint over the whole candidate set
                let mut is_open = self.type_flags(source)?.has_variable();
                for arm in &arms {
                    is_open = is_open || self.type_flags(*arm)?.has_variable();
                }
                if is_open {
                    let candidates = arms
                        .iter()
                        .map(|arm| (source, *arm))
                        .collect::<SmallVec<[_; 4]>>();

                    return self.constrain_any_relation(origin, cause, relation, &candidates);
                }

                let mut inhabited = SmallVec::<[dir::GlobalTypeId; 4]>::new();
                for arm in arms {
                    let arm = self.normalize(origin, arm)?;
                    if !matches!(self.ty(arm)?, dir::Type::Never) {
                        inhabited.push(arm);
                    }
                }
                match inhabited.as_slice() {
                    [arm] => self.constrain_type(origin, cause, relation, source, *arm)?,
                    _ => Verdict::Fails,
                }
            }

            // erased targets refuse concrete writes
            (_, dir::Type::Erased(_)) => Verdict::Fails,

            // relate two dynamic values through their constraints
            (dir::Type::Dynamic(source_dynamic), dir::Type::Dynamic(target_dynamic)) => {
                let constraint_relation = match is_inclusion {
                    true => relation,
                    false => Relation::Equal,
                };

                self.constrain_type(
                    origin,
                    cause,
                    constraint_relation,
                    source_dynamic.constraint,
                    target_dynamic.constraint,
                )?
            }
            // include a value behind a dynamic constraint it satisfies
            (_, dir::Type::Dynamic(dynamic)) if is_inclusion => {
                self.relate_dynamic_assignable(origin, cause, source, dynamic.constraint)?
            }
            // include a dynamic value by the constraint it carries
            (dir::Type::Dynamic(dynamic), _)
                if is_inclusion && !self.is_conformance_target(target)? =>
            {
                self.constrain_type(origin, cause, relation, dynamic.constraint, target)?
            }
            // storage changes representation across a dynamic boundary
            (_, dir::Type::Dynamic(_)) => Verdict::Fails,
            (dir::Type::Dynamic(_), _) if !self.is_conformance_target(target)? => Verdict::Fails,

            // scalar sources decide interface targets before literal widening
            (dir::Type::Literal(_) | dir::Type::Range(_), _)
                if self.is_conformance_target(target)? =>
            {
                self.relate_interface(origin, cause, relation, source, target)?
            }
            // include a literal in the base that holds it
            (dir::Type::Literal(literal), target) => {
                Verdict::decided(is_inclusion && literal.widens_to(&target))
            }
            // include an interval in the base holding it
            (dir::Type::Range(range), target) => {
                Verdict::decided(is_inclusion && range.widens_to(&target))
            }

            // precise variants share their declared owner's representation
            (dir::Type::Variant(variant), _)
                if !matches!(self.ty(target)?, dir::Type::Variant(_)) =>
            {
                self.constrain_type(origin, cause, relation, variant.owner, target)?
            }

            // view an array through a slice of the same element
            (dir::Type::Application(_), dir::Type::Slice(target))
                if let Some(element) = self.array_element(source)? =>
            {
                self.constrain_type(origin, cause, Relation::Equal, element, target.element)?
            }
            // refuse an array as fixed storage
            (dir::Type::Application(_), dir::Type::FixedArray(_))
                if self.array_element(source)?.is_some() =>
            {
                Verdict::Fails
            }
            // alias slices over the same element
            (dir::Type::Slice(source), dir::Type::Slice(target)) => self.constrain_type(
                origin,
                cause,
                Relation::Equal,
                source.element,
                target.element,
            )?,
            // copy fixed storage element by element at the same count
            (dir::Type::FixedArray(source), dir::Type::FixedArray(target)) => {
                let element =
                    self.constrain_type(origin, cause, relation, source.element, target.element)?;
                let count = self.constrain_type(
                    origin,
                    cause,
                    Relation::Equal,
                    source.count,
                    target.count,
                )?;

                element.and(count)
            }
            // view fixed storage through a slice of the same element
            (dir::Type::FixedArray(source), dir::Type::Slice(target)) => self.constrain_type(
                origin,
                cause,
                Relation::Equal,
                source.element,
                target.element,
            )?,
            // refuse fixed storage as an array
            (dir::Type::FixedArray(_), dir::Type::Application(_))
                if self.array_element(target)?.is_some() =>
            {
                Verdict::Fails
            }
            // relate tuples position by position
            (dir::Type::Tuple(_), dir::Type::Tuple(_)) => {
                self.relate_tuple_assignable(origin, cause, relation, source, target)?
            }
            // an array flows into the tuple writing its rest, `T[]` into `[...T[]]`
            (dir::Type::Application(_), dir::Type::Tuple(tuple))
                if let Some(rest) = self.sole_rest_container(target, tuple)? =>
            {
                self.constrain_type(origin, cause, relation, source, rest)?
            }

            // relate object types exactly by their member sets
            (dir::Type::Object(_), dir::Type::Object(_)) => self.relate_shape(
                origin,
                cause,
                relation,
                PropertySource::Stored,
                source,
                target,
            )?,
            // declaration references satisfy signatures and keyed shapes
            (dir::Type::Reference(_), dir::Type::Object(target_shape))
                if target_shape.declares_signatures() =>
            {
                self.relate_reference_shape_assignable(origin, cause, source, target)?
            }
            // construct a declaration through a construct signature
            (dir::Type::Reference(_), dir::Type::FunctionSignature(_))
                if self
                    .signature_head(target)?
                    .is_some_and(|head| head.is_construct) =>
            {
                self.relate_reference_construct_assignable(origin, cause, relation, source, target)?
            }
            // relate a static struct term by the type it holds
            (dir::Type::Static(value), _)
                if !matches!(self.ty(target)?, dir::Type::Static(_))
                    && let dir::StaticTerm::Struct { ty, .. } = self.r#static(value)?.clone() =>
            {
                self.constrain_type(origin, cause, relation, ty, target)?
            }
            // construct the declaration a static type term names
            (dir::Type::Static(value), dir::Type::FunctionSignature(_))
                if self
                    .signature_head(target)?
                    .is_some_and(|head| head.is_construct)
                    && let dir::StaticTerm::Type { ty } = self.r#static(value)?.clone()
                    && matches!(self.ty(ty)?, dir::Type::Reference(_)) =>
            {
                self.relate_reference_construct_assignable(origin, cause, relation, ty, target)?
            }
            // relate an instance against the signatures a shape declares
            (dir::Type::Application(reference), dir::Type::Object(target_shape))
                if target_shape.declares_signatures() =>
            {
                self.relate_reference_against_target(
                    origin,
                    cause,
                    relation,
                    source.module_id,
                    &reference,
                    target,
                )?
            }
            // store a struct or class value as its representation
            (dir::Type::Application(instance), dir::Type::Object(_))
                if relation == Relation::Storable
                    && matches!(
                        self.definition(instance.symbol)?.as_deref(),
                        Some(dir::Definition::Struct(_) | dir::Definition::Class(_))
                    ) =>
            {
                Verdict::Fails
            }
            // relate every other instance against the members the shape declares
            (dir::Type::Application(instance), dir::Type::Object(_)) => self
                .relate_reference_against_target(
                    origin,
                    cause,
                    relation,
                    source.module_id,
                    &instance,
                    target,
                )?,

            // reach an interface from a callable instance
            (dir::Type::Application(callable), dir::Type::Application(instance))
                if self.symbol_kind(instance.symbol)?.is_interface()
                    && self.is_function_language_item(callable.symbol)? =>
            {
                self.relate_interface(origin, cause, relation, source, target)?
            }
            // relate one declaration's arguments by variance
            (dir::Type::Application(source_instance), dir::Type::Application(target_instance))
                if source_instance.symbol == target_instance.symbol =>
            {
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
                    relation,
                    &source_arguments,
                    &target_arguments,
                )?
            }
            // relate two declarations through their heritage
            (dir::Type::Application(source_instance), dir::Type::Application(target_instance)) => {
                self.relate_application(
                    origin,
                    cause,
                    relation,
                    source,
                    &source_instance,
                    target,
                    &target_instance,
                )?
            }

            // relate two fat callables by receiver mode before their signatures
            (dir::Type::Function(source_function), dir::Type::Function(target_function))
                if self.constrain_receiver_mode(
                    origin,
                    source_function.receiver,
                    target_function.receiver,
                )? == Verdict::Fails =>
            {
                Verdict::Fails
            }
            // callables relate by signature variance
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

            // type a static arithmetic operation as each of its operands
            (dir::Type::Operation(operation), _)
                if let dir::TypeOperation::StaticBinary(binary) =
                    self.type_operation(source.module_id, operation)?
                    && !binary.operator.yields_boolean() =>
            {
                let left = self.constrain_type(origin, cause, relation, binary.left, target)?;
                let right = self.constrain_type(origin, cause, relation, binary.right, target)?;

                left.and(right)
            }
            (dir::Type::Operation(operation), _)
                if let dir::TypeOperation::StaticUnary(unary) =
                    self.type_operation(source.module_id, operation)?
                    && !unary.operator.yields_boolean() =>
            {
                self.constrain_type(origin, cause, relation, unary.target, target)?
            }

            // every other value relates to an interface target by conformance
            (_, _) if self.is_conformance_target(target)? => {
                self.relate_interface(origin, cause, relation, source, target)?
            }
            // reject every remaining pair
            _ => Verdict::Fails,
        };

        Ok(decision)
    }

    /// Return whether one source value erases behind a dynamic payload.
    pub(in crate::sema) fn erasable_source(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        self.decide_auto_interface(origin, source, dir::AutoInterface::DynamicSafe)
    }

    /// Relate one source value erasing into `Dynamic<constraint>`.
    pub(in crate::sema) fn relate_dynamic_assignable(
        &mut self,
        origin: Origin,
        cause: CauseId,
        source: dir::GlobalTypeId,
        constraint: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // read an already erased source through its constraint
        let source = match self.ty(source)? {
            dir::Type::Dynamic(dynamic) => dynamic.constraint,
            _ => source,
        };

        // box erasable values only behind a dynamic constraint
        match self.decide_auto_interface(origin, source, dir::AutoInterface::DynamicSafe)? {
            Verdict::Holds => {}
            verdict @ (Verdict::Fails | Verdict::Ambiguous) => return Ok(verdict),
        }

        self.constrain_type(origin, cause, Relation::Subtype, source, constraint)
    }

    /// Return the rest container of a tuple written as `...T[]`.
    fn sole_rest_container(
        &mut self,
        tuple_id: dir::GlobalTypeId,
        tuple: dir::TupleType,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // read the sole rest element of the tuple
        let elements = self.tuple_elements(tuple_id.module_id, tuple.elements)?;
        let rest = match elements {
            [element] if element.is_rest => Some(element.ty),
            _ => None,
        };

        Ok(rest)
    }

    /// Relate one parameter's bounds against a target, accepting any bound.
    pub(in crate::sema) fn relate_parameter_bounds(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        parameter: dir::GlobalGenericParameterId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        let bounds = self.parameter_bounds(origin, parameter)?;

        self.relate_bounds(origin, cause, relation, &bounds, target)
    }

    /// Relate the assumed `this` bounds against a target, accepting any bound.
    pub(in crate::sema) fn relate_this_bounds(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        let bounds = self.this_bounds(origin)?;

        self.relate_bounds(origin, cause, relation, &bounds, target)
    }

    /// Relate a bound set against a target: one open pair decides as an alternative set.
    fn relate_bounds(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        bounds: &[dir::GlobalTypeId],
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // read whether the target or any bound stays open
        let mut is_open = self.type_flags(target)?.has_variable();
        for bound in bounds {
            is_open = is_open || self.type_flags(*bound)?.has_variable();
        }

        // choose one open bound by constraint over the whole candidate set
        if is_open {
            let candidates = bounds
                .iter()
                .map(|bound| (*bound, target))
                .collect::<SmallVec<[_; 4]>>();

            return self.constrain_any_relation(origin, cause, relation, &candidates);
        }

        // accept any bound
        let mut verdict = Verdict::Fails;
        for bound in bounds {
            verdict = verdict.or(self.constrain_type(origin, cause, relation, *bound, target)?);
            if verdict == Verdict::Holds {
                return Ok(Verdict::Holds);
            }
        }

        Ok(verdict)
    }
}
