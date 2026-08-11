use std::iter;

use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    CandidateOutcome, CandidateVerdict, Cause, CauseId, CauseKind, CheckFailure, CheckOutcome,
    CheckState, Origin, Relation, Verdict,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Return the call signature of one callable value representation.
    pub(in crate::check) fn callable_signature(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let ty = self.resolve_head(ty)?;
        let signature = match self.ty(ty)? {
            dir::Type::Function(function) => Some(function.signature),
            dir::Type::FunctionPointer(function) => Some(function.signature),
            _ => None,
        };

        Ok(signature)
    }

    /// Enforce one relation between two types, bounding open variables.
    pub(in crate::check) fn relate(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let holds = self.constrain_type(origin, cause, relation, source, target)?;
        let check = self.complete_constraint_check(relation, source, target, holds)?;

        match check {
            CheckOutcome::Holds => Ok(()),
            CheckOutcome::Fails(failure) => {
                self.record_failure(cause, relation, None, source, target, failure)?;

                Ok(())
            }
        }
    }

    /// Check one type constraint and return the completed result.
    pub(in crate::check) fn check_type_constraint(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<CheckOutcome> {
        let holds = self.constrain_type(origin, cause, relation, source, target)?;
        let check = self.complete_constraint_check(relation, source, target, holds)?;

        Ok(check)
    }

    /// Return the completed check for one closed constraint.
    pub(in crate::check) fn complete_constraint_check(
        &mut self,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        holds: bool,
    ) -> CompilerResult<CheckOutcome> {
        // property relations explain themselves through their first bad field
        let is_property_relation = matches!(relation, Relation::Assignable | Relation::Satisfies);
        let check = match (holds, is_property_relation) {
            // complete successful relations
            (true, _) => CheckOutcome::Holds,
            // report a failed non-property relation as the relation failure
            (false, false) => CheckOutcome::Fails(CheckFailure::Relation),
            // failed property relations explain the same order as relation checking
            (false, true) => {
                // blame missing IndexSet support on nominal sources only
                let is_nominal_source = matches!(
                    self.ty(source)?,
                    dir::Type::Application(_) | dir::Type::Reference(_)
                );
                if let Some(key) = self.first_missing_struct_field(source, target)? {
                    CheckOutcome::Fails(CheckFailure::MissingRequiredProperty { key })
                } else if let Some(key) = self.first_excess_struct_field(source, target)? {
                    CheckOutcome::Fails(CheckFailure::ExcessProperty { key })
                } else if is_nominal_source
                    && let Some(signature) = self.first_writable_index_signature(target)?
                {
                    CheckOutcome::Fails(CheckFailure::WritableIndexRequiresIndexSet { signature })
                } else {
                    CheckOutcome::Fails(CheckFailure::Relation)
                }
            }
        };

        Ok(check)
    }

    /// Constrain one relation between two types, bounding open variables.
    pub(in crate::check) fn constrain_type(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let holds = self.constrain_type_pair(origin, cause, relation, source, target)?;

        Ok(holds)
    }

    /// Constrain one relation, binding through open variables, where ambiguity is a verdict.
    pub(in crate::check) fn constrain_relation(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        let holds = self.constrain_type(origin, cause, relation, source, target)?;

        self.verdict(holds, origin, relation, source, target)
    }

    /// Constrain one resolved pair.
    fn constrain_type_pair(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // substitute solved variables before comparing
        let source = self.resolve_head(source)?;
        let target = self.resolve_head(target)?;
        if source == target {
            return Ok(true);
        }

        // identity mapped targets over an open variable bind it whole
        if let Some(variable) = self.reverse_mapped_variable(target)? {
            return self.constrain_type(origin, cause, relation, source, variable);
        }

        // read the open variable standing at each root
        let source_variable = self.root_variable(source)?;
        let target_variable = self.root_variable(target)?;

        // inference barriers contribute no target bounds
        if self.no_infer_target(target)?.is_some() {
            return Ok(true);
        }

        // accept lifetime extent pairs, since MIR verification enforces them
        if source_variable.is_none()
            && target_variable.is_none()
            && self.is_lifetime_extent(source)?
            && self.is_lifetime_extent(target)?
        {
            return Ok(true);
        }

        match (source_variable, target_variable, relation) {
            // alias one open side onto the other for variable equality
            (Some(source_variable), Some(target_variable), Relation::Equal) => {
                self.alias_variable(source_variable, target_variable)?;

                Ok(true)
            }
            // unify one open side with a closed type, or record the equation as a bound
            (Some(variable), None, Relation::Equal) => {
                if self.type_variables(target)?.is_empty() {
                    self.commit_solution(variable, target)?;
                } else {
                    self.push_upper_bound(variable, origin, cause, target, Relation::Equal)?;
                }

                Ok(true)
            }
            (None, Some(variable), Relation::Equal) => {
                if self.type_variables(source)?.is_empty() {
                    self.commit_solution(variable, source)?;
                } else {
                    self.push_lower_bound(variable, origin, cause, source, Relation::Equal)?;
                }

                Ok(true)
            }
            // directed relations bound open sides directionally
            (
                Some(_),
                Some(variable),
                Relation::Assignable | Relation::Widens | Relation::Castable,
            ) => {
                self.push_lower_bound(variable, origin, cause, source, relation)?;

                Ok(true)
            }
            (Some(variable), _, Relation::Assignable | Relation::Widens | Relation::Castable) => {
                self.push_upper_bound(variable, origin, cause, target, relation)?;

                Ok(true)
            }
            (
                None,
                Some(variable),
                Relation::Assignable | Relation::Widens | Relation::Castable | Relation::Satisfies,
            ) => {
                self.push_lower_bound(variable, origin, cause, source, relation)?;

                Ok(true)
            }
            // constraint relations restrict the open source without choosing it
            (Some(variable), _, Relation::Satisfies) => {
                self.push_upper_bound(variable, origin, cause, target, relation)?;

                Ok(true)
            }
            // check-only relations require both sides to be closed
            (Some(_), _, _) | (_, Some(_), _) => Err(CompilerError::Internal {
                message: format!(
                    "open variable reached the {relation:?} relation: {source:?} against {target:?}"
                ),
            }),
            // decompose open composites before reducing the whole graph
            (None, None, _) => {
                let structural = match relation {
                    Relation::Castable => Relation::Assignable,
                    relation => relation,
                };

                // rigid parameters contribute their declared relation clauses
                if relation != Relation::Equal
                    && let dir::Type::Parameter(parameter) | dir::Type::Erased(parameter) =
                        self.ty(source)?
                {
                    let candidates = self
                        .parameter_bounds(origin, parameter)?
                        .into_iter()
                        .map(|bound| (bound, target))
                        .collect::<SmallVec<[_; 4]>>();
                    if !candidates.is_empty() {
                        let held =
                            self.constrain_any_relation(origin, cause, relation, &candidates)?;
                        if held {
                            return Ok(true);
                        }
                    }
                }

                // relate this through the interface clauses active at its origin
                if relation != Relation::Equal && matches!(self.ty(source)?, dir::Type::This) {
                    let candidates = self
                        .this_bounds(origin)?
                        .into_iter()
                        .map(|bound| (bound, target))
                        .collect::<SmallVec<[_; 4]>>();
                    if !candidates.is_empty()
                        && self.constrain_any_relation(origin, cause, relation, &candidates)?
                    {
                        return Ok(true);
                    }
                }

                // variants flow into their owner instantiation
                if let dir::Type::Variant(member) = self.ty(source)?
                    && !matches!(self.ty(target)?, dir::Type::Variant(_))
                    && relation != Relation::Equal
                {
                    return self.constrain_type(origin, cause, relation, member.owner, target);
                }

                // decompose matching constructors before dispatching open composites
                let has_variables = !self.type_variables(source)?.is_empty()
                    || !self.type_variables(target)?.is_empty();
                if has_variables
                    && let Some(decided) =
                        self.constrain_open_relation(origin, cause, structural, source, target)?
                {
                    if decided {
                        return Ok(true);
                    }

                    return self.constrain_stuck(origin, cause, relation, source, target);
                }

                // dispatch known constructors while their children remain open
                if self.decide_relation(origin, relation, source, target)? {
                    return Ok(true);
                }

                self.constrain_stuck(origin, cause, relation, source, target)
            }
        }
    }

    /// Retry one stuck relation over reduced heads once the written ones fail to relate.
    fn constrain_stuck(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let reduced_source = self.normalize_stuck(origin, source)?;
        let reduced_target = self.normalize_stuck(origin, target)?;
        if reduced_source == source && reduced_target == target {
            return Ok(false);
        }

        self.constrain_type(origin, cause, relation, reduced_source, reduced_target)
    }

    /// Relate types containing open variables through their known constructors.
    fn constrain_open_relation(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<bool>> {
        // same-symbol applications constrain arguments under their default handle context
        let same_symbol = match (self.ty(source)?, self.ty(target)?) {
            (dir::Type::Application(source_instance), dir::Type::Application(target_instance))
                if source_instance.symbol == target_instance.symbol =>
            {
                let source = SmallVec::<[_; 4]>::from_slice(
                    self.type_ids(source.module_id, source_instance.arguments)?,
                );
                let target = SmallVec::<[_; 4]>::from_slice(
                    self.type_ids(target.module_id, target_instance.arguments)?,
                );

                Some((source_instance.symbol, source, target))
            }
            _ => None,
        };
        if let Some((symbol, source, target)) = same_symbol {
            // slot arguments by kind, collecting elided lifetimes proof-only
            let Some(slots) = self.slot_application_arguments(&source, &target)? else {
                return Ok(Some(false));
            };
            let (source, target): (SmallVec<[_; 4]>, SmallVec<[_; 4]>) =
                slots.iter().copied().unzip();
            let form = self.default_variance_form(symbol)?;

            return Ok(Some(self.relate_type_arguments(
                origin,
                cause,
                symbol,
                form,
                relation.interior(),
                &source,
                &target,
            )?));
        }

        // decide memory forms through their placement and readonly views
        if matches!(relation, Relation::Assignable | Relation::Widens)
            && let Some(decision) =
                self.constrain_form_assignable(origin, cause, relation, source, target)?
        {
            return Ok(Some(decision));
        }

        // decompose existential conversions through their represented constraints
        if relation == Relation::Assignable {
            let dynamic = match (self.ty(source)?, self.ty(target)?) {
                (dir::Type::Dynamic(source), dir::Type::Dynamic(target)) => {
                    Some((source.constraint, target.constraint, None))
                }
                (_, dir::Type::Dynamic(target)) => Some((source, target.constraint, Some(source))),
                (dir::Type::Dynamic(source), _) => Some((source.constraint, target, None)),
                _ => None,
            };
            if let Some((source, target, erased)) = dynamic {
                let safe = match erased {
                    Some(erased) => self.satisfies_auto_interface(
                        origin,
                        erased,
                        dir::AutoInterface::DynamicSafe,
                    )?,
                    None => true,
                };
                if !safe {
                    return Ok(Some(false));
                }
                let constraint =
                    self.constrain_type(origin, cause, Relation::Assignable, source, target)?;

                return Ok(Some(constraint));
            }
        }

        // test known nominal roots through heritage before waiting on arguments
        if matches!(relation, Relation::Assignable | Relation::Widens)
            && let (dir::Type::Application(_), dir::Type::Application(_)) =
                (self.ty(source)?, self.ty(target)?)
        {
            return Ok(Some(
                self.decide_application_assignable(origin, source, target)?,
            ));
        }

        // read each side's call signature once for the callable arms below
        let source_signature = self.callable_signature(source)?;
        let target_signature = self.callable_signature(target)?;

        // union representations widen only through exact set equality
        if matches!(relation, Relation::Equal | Relation::Widens)
            && let (dir::Type::Union(source_union), dir::Type::Union(target_union)) =
                (self.ty(source)?, self.ty(target)?)
        {
            let source_elements = SmallVec::<[_; 4]>::from_slice(
                self.type_ids(source.module_id, source_union.elements)?,
            );
            let target_elements = SmallVec::<[_; 4]>::from_slice(
                self.type_ids(target.module_id, target_union.elements)?,
            );
            let equal =
                self.constrain_type_sets_equal(origin, cause, &source_elements, &target_elements)?;

            return Ok(Some(equal));
        }

        // intersections compare as unordered sets under equality
        if relation == Relation::Equal
            && let (
                dir::Type::Intersection(source_intersection),
                dir::Type::Intersection(target_intersection),
            ) = (self.ty(source)?, self.ty(target)?)
        {
            let source_elements = SmallVec::<[_; 4]>::from_slice(
                self.type_ids(source.module_id, source_intersection.elements)?,
            );
            let target_elements = SmallVec::<[_; 4]>::from_slice(
                self.type_ids(target.module_id, target_intersection.elements)?,
            );
            let equal =
                self.constrain_type_sets_equal(origin, cause, &source_elements, &target_elements)?;

            return Ok(Some(equal));
        }

        // collect fixed slot pairs with their slot relations
        let mut pairs = SmallVec::<
            [(
                Option<CauseKind>,
                Relation,
                dir::GlobalTypeId,
                dir::GlobalTypeId,
            ); 4],
        >::new();
        match (self.ty(source)?, self.ty(target)?) {
            // keep mutable collection elements invariant, since they alias
            (dir::Type::Array(source_array), dir::Type::Array(target_array)) => {
                pairs.push((
                    None,
                    Relation::Equal,
                    source_array.element,
                    target_array.element,
                ));
            }
            (dir::Type::Slice(source_slice), dir::Type::Slice(target_slice)) => {
                pairs.push((
                    None,
                    Relation::Equal,
                    source_slice.element,
                    target_slice.element,
                ));
            }
            (dir::Type::Array(source_array), dir::Type::Slice(target_slice)) => {
                pairs.push((
                    None,
                    Relation::Equal,
                    source_array.element,
                    target_slice.element,
                ));
            }
            (dir::Type::FixedArray(source_array), dir::Type::FixedArray(target_array)) => {
                pairs.push((
                    None,
                    relation.interior(),
                    source_array.element,
                    target_array.element,
                ));
                pairs.push((
                    None,
                    Relation::Equal,
                    source_array.count,
                    target_array.count,
                ));
            }
            (dir::Type::Tuple(source_tuple), dir::Type::Tuple(target_tuple))
                if source_tuple.form == target_tuple.form
                    && source_tuple.elements.len() == target_tuple.elements.len() =>
            {
                let source_elements =
                    self.tuple_elements(source.module_id, source_tuple.elements)?;
                let target_elements =
                    self.tuple_elements(target.module_id, target_tuple.elements)?;
                for (index, (source_element, target_element)) in source_elements
                    .iter()
                    .zip(target_elements.iter())
                    .enumerate()
                {
                    let kind = CauseKind::Element {
                        index: index as u32,
                    };
                    pairs.push((
                        Some(kind),
                        relation.interior(),
                        source_element.ty,
                        target_element.ty,
                    ));
                }
            }
            // string sources bind open template spans by captured text
            (
                dir::Type::Literal(dir::ScalarLiteral::String(text)),
                dir::Type::Operation(operation),
            ) if let dir::TypeOperation::TemplateLiteral(template) =
                self.type_operation(target.module_id, operation)? =>
            {
                let text = self.strings().get(text).to_string();
                let parts = match self.split_template_captures(
                    origin,
                    &text,
                    target.module_id,
                    &template,
                )? {
                    Some(parts) => parts,
                    None => return Ok(Some(false)),
                };
                let mut seen: SmallVec<[(dir::GlobalTypeId, String); 2]> = SmallVec::new();
                for (span, captured) in parts {
                    if self.template_piece_text(span)?.is_some() {
                        continue;
                    }

                    // repeated spans must capture identical text
                    let root = self.resolve_head(span)?;
                    if let Some((_, previous)) = seen.iter().find(|(other, _)| *other == root) {
                        if *previous != captured {
                            return Ok(Some(false));
                        }

                        continue;
                    }
                    seen.push((root, captured.clone()));
                    let Some(captured) = self.template_capture_bound(origin, span, &captured)?
                    else {
                        return Ok(Some(false));
                    };
                    pairs.push((None, Relation::Assignable, captured, span));
                }
            }
            (dir::Type::Primitive(dir::PrimitiveType::String), dir::Type::Operation(operation))
                if let dir::TypeOperation::TemplateLiteral(template) =
                    self.type_operation(target.module_id, operation)? =>
            {
                // absorb the string domain into unconstraining open patterns only
                for segment in self.template_strings(target.module_id, template.strings)? {
                    if !self.strings().get(*segment).is_empty() {
                        return Ok(Some(false));
                    }
                }
                for span in self.type_ids(target.module_id, template.spans)?.to_vec() {
                    pairs.push((None, Relation::Assignable, source, span));
                }
            }

            // shapes relate matching fields by target writeability
            (
                dir::Type::Shape(source_shape) | dir::Type::Object(source_shape),
                dir::Type::Shape(target_shape) | dir::Type::Object(target_shape),
            ) => {
                let is_constructed = matches!(self.ty(source)?, dir::Type::Object(_));
                let source_fields =
                    self.shape_properties(source.module_id, source_shape.properties)?;
                let target_fields =
                    self.shape_properties(target.module_id, target_shape.properties)?;

                for target_field in target_fields {
                    let source_field = source_fields
                        .iter()
                        .find(|field| field.key == target_field.key);

                    match source_field {
                        Some(source_field) => {
                            let Some(relations) = self.shape_property_relations(
                                relation,
                                is_constructed,
                                source_field,
                                target_field,
                            ) else {
                                return Ok(Some(false));
                            };
                            for (field_relation, source_ty, target_ty) in relations {
                                pairs.push((
                                    Some(CauseKind::Field {
                                        key: target_field.key,
                                    }),
                                    field_relation,
                                    source_ty,
                                    target_ty,
                                ));
                            }
                        }
                        // missing members satisfy optional targets only
                        None => {
                            if !target_field.is_optional {
                                return Ok(Some(false));
                            }
                        }
                    }
                }
            }
            // functions relate parameters contravariantly and returns covariantly
            (_, dir::Type::FunctionSignature(_))
                if matches!(relation, Relation::Assignable | Relation::Widens)
                    && let Some(source) = source_signature =>
            {
                pairs.push((None, relation, source, target));
            }
            (dir::Type::FunctionSignature(_), _)
                if matches!(relation, Relation::Assignable | Relation::Widens)
                    && let Some(target) = target_signature =>
            {
                pairs.push((None, relation, source, target));
            }
            (_, _)
                if matches!(relation, Relation::Assignable | Relation::Widens)
                    && let (Some(source), Some(target)) = (source_signature, target_signature) =>
            {
                pairs.push((None, relation, source, target));
            }
            (dir::Type::FunctionSignature(_), dir::Type::FunctionSignature(_)) => {
                // pair splatted signature slots positionally
                let source = self.normalize(origin, source)?;
                let target = self.normalize(origin, target)?;
                let (
                    dir::Type::FunctionSignature(source_function),
                    dir::Type::FunctionSignature(target_function),
                ) = (self.ty(source)?, self.ty(target)?)
                else {
                    return Ok(None);
                };
                let source_function = self.type_signature(source.module_id, source_function)?;
                let target_function = self.type_signature(target.module_id, target_function)?;
                let source_parameters =
                    self.signature_parameters(source.module_id, source_function.parameters)?;
                let target_parameters =
                    self.signature_parameters(target.module_id, target_function.parameters)?;
                let shared = source_parameters.len().min(target_parameters.len());
                for (index, (source_parameter, target_parameter)) in source_parameters[..shared]
                    .iter()
                    .zip(target_parameters[..shared].iter())
                    .enumerate()
                {
                    let kind = CauseKind::Parameter {
                        index: index as u32,
                    };
                    pairs.push((
                        Some(kind),
                        relation.interior(),
                        target_parameter.ty,
                        source_parameter.ty,
                    ));
                }

                // equate the return slots so a callable value adopts its contextual signature
                if let (Some(source_return), Some(target_return)) =
                    (source_function.return_type, target_function.return_type)
                {
                    pairs.push((
                        Some(CauseKind::ReturnSlot),
                        relation.interior(),
                        source_return,
                        target_return,
                    ));
                }
            }
            // memory forms relate payloads directly, borrows bind their slots
            (dir::Type::Form(source_form), dir::Type::Form(target_form))
                if source_form.form.same_constructor(&target_form.form) =>
            {
                if let (dir::Form::Borrowed(source_borrow), dir::Form::Borrowed(target_borrow)) =
                    (source_form.form, target_form.form)
                {
                    let source_borrow = self.type_borrow(source.module_id, source_borrow)?;
                    let target_borrow = self.type_borrow(target.module_id, target_borrow)?;
                    pairs.push((
                        None,
                        relation,
                        source_borrow.lifetime,
                        target_borrow.lifetime,
                    ));
                    pairs.push((None, relation, source_borrow.access, target_borrow.access));
                }
                if let (
                    dir::Form::Placed {
                        place: source_place,
                    },
                    dir::Form::Placed {
                        place: target_place,
                    },
                ) = (source_form.form, target_form.form)
                {
                    pairs.push((None, Relation::Equal, source_place, target_place));
                }
                pairs.push((
                    Some(CauseKind::Payload),
                    relation,
                    source_form.value,
                    target_form.value,
                ));
            }
            // union sources flow every element into the target
            (dir::Type::Union(elements), _) if relation.distributes_over_union_source() => {
                let elements = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(
                    self.type_ids(source.module_id, elements.elements)?,
                );
                for element in elements {
                    pairs.push((None, relation, element, target));
                }
            }
            // union targets accept when any member accepts
            (_, dir::Type::Union(elements)) if relation.distributes_over_union_target() => {
                let elements = SmallVec::<[_; 4]>::from_slice(
                    self.type_ids(target.module_id, elements.elements)?,
                );

                let candidates = elements
                    .into_iter()
                    .map(|target| (source, target))
                    .collect::<SmallVec<[_; 4]>>();

                return Ok(Some(self.constrain_any_relation(
                    origin,
                    cause,
                    relation,
                    &candidates,
                )?));
            }
            (dir::Type::Dynamic(source), dir::Type::Dynamic(target)) => {
                // constraint changes rebuild the fat pointer and never widen
                let constraint_relation = match relation {
                    Relation::Widens => Relation::Equal,
                    relation => relation,
                };
                pairs.push((
                    None,
                    constraint_relation,
                    source.constraint,
                    target.constraint,
                ));
            }
            // fixed constructors recurse through their slots
            (_, _) if let Some(slots) = self.decompose_type_pair(source, target)? => {
                for (source, target) in slots {
                    pairs.push((None, relation, source, target));
                }
            }
            _ => return Ok(None),
        }

        // constrain every slot pair through the bounding path
        for (kind, relation, source, target) in pairs {
            let child = match kind {
                Some(kind) => self.intern_cause(Cause::child(origin, kind, cause)),
                None => cause,
            };
            if !self.constrain_type(origin, child, relation, source, target)? {
                return Ok(Some(false));
            }
        }

        Ok(Some(true))
    }

    /// Constrain one relation through exactly one applicable alternative.
    fn constrain_any_relation(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        candidates: &[(dir::GlobalTypeId, dir::GlobalTypeId)],
    ) -> CompilerResult<bool> {
        // take a single alternative without speculative selection
        if let [candidate] = candidates {
            return self.constrain_type(origin, cause, relation, candidate.0, candidate.1);
        }

        // classify every arm without retaining speculative bounds
        let mut viables = SmallVec::<[_; 4]>::new();
        let mut indeterminate = None;
        let mut is_indeterminate_ambiguous = false;
        for candidate in candidates.iter().copied() {
            let verdict = self.probe_candidate(|state| {
                match state.constrain_type(origin, cause, relation, candidate.0, candidate.1)? {
                    true => Ok(CandidateOutcome::Accepted(())),
                    false => Ok(CandidateOutcome::Rejected(())),
                }
            })?;
            match verdict {
                CandidateVerdict::Viable => viables.push(candidate),
                CandidateVerdict::Indeterminate if indeterminate.replace(candidate).is_some() => {
                    is_indeterminate_ambiguous = true;
                }
                CandidateVerdict::Indeterminate | CandidateVerdict::Rejected => {}
            }
        }

        // disambiguate tied arms by matching constructors first
        if viables.len() > 1 {
            let matching = viables
                .iter()
                .copied()
                .filter(|(source, target)| {
                    self.same_type_constructor(*source, *target)
                        .unwrap_or(false)
                })
                .collect::<SmallVec<[_; 4]>>();
            match matching.as_slice() {
                [matched] => viables = SmallVec::from_slice(&[*matched]),
                [] => {
                    let naked = viables
                        .iter()
                        .copied()
                        .filter(|(_, target)| matches!(self.root_variable(*target), Ok(Some(_))))
                        .collect::<SmallVec<[_; 4]>>();
                    if let [matched] = naked.as_slice() {
                        viables = SmallVec::from_slice(&[*matched]);
                    }
                }
                _ => {}
            }
        }

        // commit only one unambiguous arm, preferring established bounds
        let selected = match (
            viables.as_slice(),
            is_indeterminate_ambiguous,
            indeterminate,
        ) {
            ([selected], _, _) => Some(*selected),
            ([], false, Some(selected)) => Some(selected),
            _ => None,
        };
        if let Some(selected) = selected {
            return self.constrain_type(origin, cause, relation, selected.0, selected.1);
        }

        // wait for open variables to close before disambiguating applicable arms
        let open = self.open_type_variables(
            candidates
                .iter()
                .flat_map(|(source, target)| iter::once(*source).chain(iter::once(*target))),
        )?;
        if !open.is_empty() {
            return Ok(false);
        }

        // multiple closed arms all prove the same relation
        Ok(!viables.is_empty())
    }

    /// Return whether one closed type denotes a lifetime extent.
    pub(in crate::check) fn is_lifetime_extent(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        match self.ty(id)? {
            dir::Type::Memory(dir::MemoryLiteral::Lifetime(_)) => Ok(true),
            dir::Type::Parameter(parameter) | dir::Type::Erased(parameter) => {
                Ok(self.generic_parameter(parameter).is_some_and(|binding| {
                    binding.memory_parameter() == Some(dir::MemoryParameter::Lifetime)
                }))
            }
            dir::Type::Union(union) => {
                for member in self.type_ids(id.module_id, union.elements)? {
                    if !self.is_lifetime_extent(*member)? {
                        return Ok(false);
                    }
                }

                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Return whether one pair shares its top type constructor.
    fn same_type_constructor(
        &self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let source = self.resolve_head(source)?;
        let target = self.resolve_head(target)?;

        match (self.ty(source)?, self.ty(target)?) {
            (dir::Type::Form(source), dir::Type::Form(target)) => {
                Ok(source.form.same_constructor(&target.form))
            }
            (dir::Type::Application(source), dir::Type::Application(target)) => {
                Ok(source.symbol == target.symbol)
            }
            (dir::Type::Reference(source), dir::Type::Reference(target)) => {
                Ok(source.symbol == target.symbol)
            }
            (source, target) => {
                Ok(std::mem::discriminant(&source) == std::mem::discriminant(&target))
            }
        }
    }

    /// Collect the open inference variables referenced by some types.
    pub(in crate::check) fn open_type_variables(
        &self,
        types: impl IntoIterator<Item = dir::GlobalTypeId>,
    ) -> CompilerResult<SmallVec<[dir::TypeVariableId; 2]>> {
        let mut variables = SmallVec::<[dir::TypeVariableId; 2]>::new();
        for ty in types {
            for variable in self.type_variables(ty)? {
                if !variables.contains(&variable) {
                    variables.push(variable);
                }
            }
        }

        Ok(variables)
    }

    /// Return the root after substituting solved inference variables.
    pub(in crate::check) fn resolve_head(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut current = id;

        // substitute a solved top variable for its solution
        while let dir::Type::Variable(variable) = self.ty(current)? {
            let solution = self.infer.solution(variable)?;
            let Some(solution) = solution else {
                return Ok(current);
            };

            current = solution;
        }

        Ok(current)
    }

    /// Return the open variable at one settled type root.
    pub(in crate::check) fn root_variable(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::TypeVariableId>> {
        match self.ty(id)? {
            dir::Type::Variable(variable) => self.open_variable(variable),
            _ => Ok(None),
        }
    }

    /// Return the source node behind one origin for type allocation.
    pub(in crate::check) fn origin_source(
        &self,
        origin: Origin,
    ) -> CompilerResult<dir::GlobalNodeIdAny> {
        let module = origin.module();

        Ok(self.origin_source_node(origin)?.into_global(module))
    }

    /// Return the local source node anchoring one check origin.
    pub(in crate::check) fn origin_source_node(
        &self,
        origin: Origin,
    ) -> CompilerResult<dir::LocalNodeIdAny> {
        match origin {
            Origin::Node(node, _) => Ok(node.local_id),
            Origin::Symbol(symbol) => {
                if let Some(module) = self.module_maybe(symbol.module_id) {
                    return module.symbol_declaration_node(symbol.local_id);
                }

                // read foreign declarations from the imported external tables
                let external = self
                    .external_modules
                    .get(&symbol.module_id)
                    .ok_or_else(|| CompilerError::Internal {
                        message: format!("origin symbol {symbol:?} has no loaded module"),
                    })?;
                let binding = external.bindings.get_symbol(symbol.local_id);
                binding
                    .declaration
                    .map(|declaration| declaration.local_id)
                    .ok_or_else(|| CompilerError::Internal {
                        message: format!("origin symbol {symbol:?} has no declaration node"),
                    })
            }
        }
    }
}
