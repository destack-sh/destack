use std::iter;

use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Answer, CandidateOutcome, CandidateVerdict, Cause, CauseId, CauseKind, CheckFailure,
    CheckOutcome, CheckState, Dependency, Origin, Relation, VariableRole, answer,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Return the call signature of one callable value representation.
    pub(in crate::check) fn callable_signature(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let ty = self.settled_root(ty)?;
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
    ) -> CompilerResult<Answer<()>> {
        let holds = answer!(self.constrain_type(origin, cause, relation, source, target)?);
        let check =
            answer!(self.complete_constraint_check(origin, relation, source, target, holds,)?);

        match check {
            CheckOutcome::Holds => Ok(Answer::Ready(())),
            CheckOutcome::Fails(failure) => {
                self.record_failure(cause, relation, None, source, target, failure);

                Ok(Answer::Ready(()))
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
    ) -> CompilerResult<Answer<CheckOutcome>> {
        let holds = answer!(self.constrain_type(origin, cause, relation, source, target)?);
        let check =
            answer!(self.complete_constraint_check(origin, relation, source, target, holds,)?);

        Ok(Answer::Ready(check))
    }

    /// Return the completed check for one closed constraint.
    pub(in crate::check) fn complete_constraint_check(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        holds: bool,
    ) -> CompilerResult<Answer<CheckOutcome>> {
        let is_property_relation = matches!(relation, Relation::Assignable | Relation::Satisfies);

        let check = match (holds, is_property_relation) {
            // successful relations are complete
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
                    && let Some(signature) =
                        answer!(self.first_writable_index_signature(origin, target)?)
                {
                    CheckOutcome::Fails(CheckFailure::WritableIndexRequiresIndexSet { signature })
                } else {
                    CheckOutcome::Fails(CheckFailure::Relation)
                }
            }
        };

        Ok(Answer::Ready(check))
    }

    /// Constrain one relation between two types, bounding open variables.
    pub(in crate::check) fn constrain_type(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // substitute solved variables before comparing
        let source = self.settled_root(source)?;
        let target = self.settled_root(target)?;
        if source == target {
            return Ok(Answer::Ready(true));
        }

        let source_variable = self.root_variable(source)?;
        let target_variable = self.root_variable(target)?;

        // inference barriers contribute no target bounds
        if self.no_infer_target(target)?.is_some() {
            return Ok(Answer::Ready(true));
        }

        match (source_variable, target_variable, relation) {
            // alias one open side onto the other for variable equality
            (Some(source_variable), Some(target_variable), Relation::Equal) => {
                self.alias_variable(source_variable, target_variable)?;

                Ok(Answer::Ready(true))
            }
            // unify one open side with a closed type, or record the
            //  equation as a bound until its composite closes
            (Some(variable), None, Relation::Equal) => {
                if self.type_variables(target)?.is_empty() {
                    self.commit_solution(variable, target)?;
                } else {
                    self.push_upper_bound(variable, origin, cause, target, Relation::Equal)?;
                }

                Ok(Answer::Ready(true))
            }
            (None, Some(variable), Relation::Equal) => {
                if self.type_variables(source)?.is_empty() {
                    self.commit_solution(variable, source)?;
                } else {
                    self.push_lower_bound(variable, origin, cause, source, Relation::Equal)?;
                }

                Ok(Answer::Ready(true))
            }
            // directed relations bound open sides directionally
            (
                Some(_),
                Some(variable),
                Relation::Assignable | Relation::Widens | Relation::Castable,
            ) => {
                self.push_lower_bound(variable, origin, cause, source, relation)?;

                Ok(Answer::Ready(true))
            }
            (Some(variable), _, Relation::Assignable | Relation::Widens | Relation::Castable) => {
                self.push_upper_bound(variable, origin, cause, target, relation)?;

                Ok(Answer::Ready(true))
            }
            (
                None,
                Some(variable),
                Relation::Assignable | Relation::Widens | Relation::Castable,
            ) => {
                self.push_lower_bound(variable, origin, cause, source, relation)?;

                Ok(Answer::Ready(true))
            }
            // constraint relations restrict the open source without choosing it
            (Some(variable), _, Relation::Satisfies) => {
                self.push_upper_bound(variable, origin, cause, target, relation)?;

                Ok(Answer::Ready(true))
            }
            // check-only relations wait for both sides to close
            (Some(_), _, _) | (_, Some(_), _) => {
                let mut blockers = SmallVec::<[Dependency; 2]>::new();
                blockers.extend(source_variable.map(Dependency::Variable));
                blockers.extend(target_variable.map(Dependency::Variable));

                Ok(Answer::Pending(blockers))
            }
            // decompose open composites before reducing the whole graph
            (None, None, _) => {
                let structural = match relation {
                    Relation::Castable => Relation::Assignable,
                    relation => relation,
                };

                // reduce aliases and intrinsic operations at each root
                let source = answer!(self.reduce_type_head(origin, source)?);
                let target = match self.reduce_type_head(origin, target)? {
                    Answer::Ready(target) => target,
                    // identity mapped targets over an open variable bind it whole
                    Answer::Pending(blockers) => {
                        if let Some(variable) = self.reverse_mapped_variable(origin, target)? {
                            return self.constrain_type(origin, cause, relation, source, variable);
                        }

                        return Ok(Answer::Pending(blockers));
                    }
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
                        match self.constrain_any_relation(origin, cause, relation, &candidates)? {
                            Answer::Ready(true) => return Ok(Answer::Ready(true)),
                            Answer::Ready(false) => {}
                            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                        }
                    }
                }

                // this contributes the interface clauses active at its origin
                if relation != Relation::Equal && matches!(self.ty(source)?, dir::Type::This) {
                    let candidates = self
                        .this_bounds(origin)?
                        .into_iter()
                        .map(|bound| (bound, target))
                        .collect::<SmallVec<[_; 4]>>();
                    if !candidates.is_empty() {
                        match self.constrain_any_relation(origin, cause, relation, &candidates)? {
                            Answer::Ready(true) => return Ok(Answer::Ready(true)),
                            Answer::Ready(false) => {}
                            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                        }
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
                    && let Some(answer) =
                        self.constrain_open_relation(origin, cause, structural, source, target)?
                {
                    return Ok(answer);
                }

                // dispatch known constructors while their children remain open
                self.decide_relation(origin, relation, source, target)
            }
        }
    }

    /// Return the relation used by one function return slot.
    fn return_slot_relation(
        &self,
        source: dir::GlobalTypeId,
        relation: Relation,
    ) -> CompilerResult<Relation> {
        let root = self.settled_root(source)?;
        let is_contextual = match self.root_variable(root)? {
            Some(variable) => self.solver.variable_role(variable)? == VariableRole::Parameter,
            None => false,
        };

        Ok(match is_contextual {
            true => Relation::Equal,
            false => relation.interior(),
        })
    }

    /// Relate types containing open variables through their known constructors.
    fn constrain_open_relation(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Answer<bool>>> {
        // same-symbol applications constrain arguments under their default handle context
        let same_symbol = match (self.ty(source)?, self.ty(target)?) {
            (dir::Type::Application(source_instance), dir::Type::Application(target_instance))
                if source_instance.symbol == target_instance.symbol
                    && source_instance.arguments.len() == target_instance.arguments.len() =>
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

        // memory forms own placement and readonly views
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
                    None => Answer::Ready(true),
                };
                if safe.is_ready_false() {
                    return Ok(Some(safe));
                }
                let constraint =
                    self.constrain_type(origin, cause, Relation::Assignable, source, target)?;

                return Ok(Some(safe.and(constraint)));
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
            // mutable collections alias their elements and stay invariant
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
                    Answer::Ready(Some(parts)) => parts,
                    Answer::Ready(None) => return Ok(Some(Answer::Ready(false))),
                    Answer::Pending(blockers) => return Ok(Some(Answer::Pending(blockers))),
                };
                let mut seen: SmallVec<[(dir::GlobalTypeId, String); 2]> = SmallVec::new();
                for (span, captured) in parts {
                    if self.template_piece_text(span)?.is_some() {
                        continue;
                    }
                    // repeated spans must capture identical text
                    let root = self.settled_root(span)?;
                    if let Some((_, previous)) = seen.iter().find(|(other, _)| *other == root) {
                        if *previous != captured {
                            return Ok(Some(Answer::Ready(false)));
                        }

                        continue;
                    }
                    seen.push((root, captured.clone()));
                    let Some(captured) = self.template_capture_bound(origin, span, &captured)?
                    else {
                        return Ok(Some(Answer::Ready(false)));
                    };
                    pairs.push((None, Relation::Assignable, captured, span));
                }
            }
            (dir::Type::Primitive(dir::PrimitiveType::String), dir::Type::Operation(operation))
                if let dir::TypeOperation::TemplateLiteral(template) =
                    self.type_operation(target.module_id, operation)? =>
            {
                // only unconstraining open patterns absorb the string domain
                for segment in self.template_strings(target.module_id, template.strings)? {
                    if !self.strings().get(*segment).is_empty() {
                        return Ok(Some(Answer::Ready(false)));
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
                            let Some(relations) =
                                self.shape_property_relations(relation, source_field, target_field)
                            else {
                                return Ok(Some(Answer::Ready(false)));
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
                                return Ok(Some(Answer::Ready(false)));
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
            (
                dir::Type::FunctionSignature(source_function),
                dir::Type::FunctionSignature(target_function),
            ) => {
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
                if let (Some(source_return), Some(target_return)) =
                    (source_function.return_type, target_function.return_type)
                {
                    let relation = self.return_slot_relation(source_return, relation)?;
                    pairs.push((
                        Some(CauseKind::ReturnSlot),
                        relation,
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
        let mut decision = Answer::Ready(true);
        for (kind, relation, source, target) in pairs {
            let child = match kind {
                Some(kind) => self.intern_cause(Cause::child(origin, kind, cause)),
                None => cause,
            };
            decision = decision.and(self.constrain_type(origin, child, relation, source, target)?);
            if decision.is_ready_false() {
                return Ok(Some(decision));
            }
        }

        Ok(Some(decision))
    }

    /// Constrain one relation through exactly one applicable alternative.
    fn constrain_any_relation(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        candidates: &[(dir::GlobalTypeId, dir::GlobalTypeId)],
    ) -> CompilerResult<Answer<bool>> {
        // a single alternative requires no speculative selection
        if let [candidate] = candidates {
            return self.constrain_type(origin, cause, relation, candidate.0, candidate.1);
        }

        let mut viable = None;
        let mut is_viable_ambiguous = false;
        let mut indeterminate = None;
        let mut is_indeterminate_ambiguous = false;
        let mut blockers = SmallVec::<[Dependency; 2]>::new();

        // classify every arm without retaining speculative bounds
        for candidate in candidates.iter().copied() {
            let verdict = self.probe_candidate(|state| {
                match state.constrain_type(origin, cause, relation, candidate.0, candidate.1)? {
                    Answer::Ready(true) => Ok(Answer::Ready(CandidateOutcome::Accepted(()))),
                    Answer::Ready(false) => Ok(Answer::Ready(CandidateOutcome::Rejected(()))),
                    Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
                }
            })?;
            match verdict {
                Answer::Ready(CandidateVerdict::Viable) if viable.replace(candidate).is_some() => {
                    is_viable_ambiguous = true;
                }
                Answer::Ready(CandidateVerdict::Viable) => {}
                Answer::Ready(CandidateVerdict::Indeterminate)
                    if indeterminate.replace(candidate).is_some() =>
                {
                    is_indeterminate_ambiguous = true;
                }
                Answer::Ready(CandidateVerdict::Indeterminate | CandidateVerdict::Rejected) => {}
                Answer::Pending(dependencies) => {
                    for dependency in dependencies {
                        if !blockers.contains(&dependency) {
                            blockers.push(dependency);
                        }
                    }
                }
            }
        }

        // commit only one unambiguous arm, preferring established bounds
        let selected = match (
            is_viable_ambiguous,
            viable,
            is_indeterminate_ambiguous,
            indeterminate,
            blockers.is_empty(),
        ) {
            (false, Some(selected), _, _, _) => Some(selected),
            (_, None, false, Some(selected), true) => Some(selected),
            _ => None,
        };
        if let Some(selected) = selected {
            return self.constrain_type(origin, cause, relation, selected.0, selected.1);
        }

        // wait until open variables disambiguate otherwise applicable arms
        let dependencies = self.variable_dependencies(
            candidates
                .iter()
                .flat_map(|(source, target)| iter::once(*source).chain(iter::once(*target))),
        )?;
        for dependency in dependencies {
            if !blockers.contains(&dependency) {
                blockers.push(dependency);
            }
        }
        if !blockers.is_empty() {
            return Ok(Answer::Pending(blockers));
        }

        // multiple closed arms all prove the same relation
        Ok(Answer::Ready(viable.is_some()))
    }

    /// Collect the inference dependencies referenced by some types.
    pub(in crate::check) fn variable_dependencies(
        &self,
        types: impl IntoIterator<Item = dir::GlobalTypeId>,
    ) -> CompilerResult<SmallVec<[Dependency; 2]>> {
        let mut dependencies = SmallVec::<[Dependency; 2]>::new();
        for ty in types {
            for variable in self.type_variables(ty)? {
                let dependency = Dependency::Variable(variable);
                if !dependencies.contains(&dependency) {
                    dependencies.push(dependency);
                }
            }
        }

        Ok(dependencies)
    }

    /// Return the root after substituting solved inference variables.
    pub(in crate::check) fn settled_root(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut current = id;

        // substitute a solved top variable for its solution
        while let dir::Type::Variable(variable) = self.ty(current)? {
            let solution = self
                .solver
                .solution(variable)
                .map_err(|_| CompilerError::Internal {
                    message: format!(
                        "type {current:?} contains unallocated inference variable {variable:?}"
                    ),
                })?;
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
            Origin::Symbol(symbol) => self
                .module(symbol.module_id)
                .symbol_declaration_node(symbol.local_id),
        }
    }
}
