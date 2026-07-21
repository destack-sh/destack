use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CandidateOutcome, Cause, CauseId, CauseKind, CheckFailure, CheckOutcome, CheckState,
    Dependency, ObligationCheck, Origin, ProbeReason, Relation, ValueCheck, ValueUse, VariableRole,
    answer,
};

impl CheckState<'_> {
    /// Match one generic satisfaction relation.
    fn satisfy_type_constraint(
        &mut self,
        cause: CauseId,
        argument: dir::GlobalTypeId,
        constraint: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let origin = self.cause_origin(cause);
        let constraint = answer!(self.reduce_type_head(origin, constraint)?);
        let argument = answer!(self.reduce_type_head(origin, argument)?);

        // union sources must satisfy the constraint through every element
        if let dir::Type::Union(union) = self.ty(argument)? {
            let elements = self.type_ids(argument.module_id, union.elements)?.to_vec();
            let mut decision = Answer::Ready(true);
            for element in elements {
                decision = decision.and(self.satisfy_type_constraint(cause, element, constraint)?);
                if decision.is_ready_false() {
                    break;
                }
            }

            return Ok(decision);
        }

        // parameters prove satisfaction through their declared and assumed bounds
        if matches!(
            self.ty(argument)?,
            dir::Type::Parameter(_) | dir::Type::Erased(_)
        ) {
            return self.solve_type_relation(cause, Relation::Satisfies, argument, constraint);
        }

        // check conjunction bounds element-wise
        if let dir::Type::Intersection(intersection) = self.ty(constraint)? {
            let elements = self
                .type_ids(constraint.module_id, intersection.elements)?
                .to_vec();
            let mut decision = Answer::Ready(true);
            for element in elements {
                decision = decision.and(self.satisfy_type_constraint(cause, argument, element)?);
                if decision.is_ready_false() {
                    break;
                }
            }

            return Ok(decision);
        }

        // union bounds accept any member bound that accepts the argument
        if let dir::Type::Union(union) = self.ty(constraint)? {
            let elements = self
                .type_ids(constraint.module_id, union.elements)?
                .to_vec();

            return self.constrain_union_target(cause, Relation::Satisfies, argument, &elements);
        }

        // normalize compiler-known static domains before relation checking
        let item = self
            .type_symbol(constraint)?
            .map(|symbol| self.language_item(symbol))
            .transpose()?
            .flatten();

        match item {
            Some(
                item @ (dir::LanguageItem::Access
                | dir::LanguageItem::Lifetime
                | dir::LanguageItem::Ownership
                | dir::LanguageItem::Place
                | dir::LanguageItem::Space),
            ) => {
                let argument = self.normalize_memory_parameter_value(origin, argument, item)?;

                self.solve_type_relation(cause, Relation::Satisfies, argument, constraint)
            }
            Some(dir::LanguageItem::Concrete) => {
                match answer!(self.check_representation(origin, argument)?) {
                    ObligationCheck::Holds => Ok(Answer::Ready(true)),
                    ObligationCheck::Fails(_) => Ok(Answer::Ready(false)),
                }
            }
            Some(item)
                if let Some(interface) = dir::AutoInterface::from_language_item(item)
                    && interface.has_auto_conformance() =>
            {
                self.satisfies_auto_interface(origin, argument, interface)
            }
            _ => {
                // open nominal pairs prove satisfaction through heritage or a
                //  sole conforming extension, binding their open holes
                if let (
                    dir::Type::Instance(argument_instance),
                    dir::Type::Instance(constraint_instance),
                ) = (self.ty(argument)?, self.ty(constraint)?)
                    && matches!(
                        self.definition(constraint_instance.symbol)?,
                        Some(dir::Definition::Interface(_))
                    )
                    && !self.type_variables(argument)?.is_empty()
                {
                    return self.constrain_nominal_satisfies(
                        cause,
                        argument,
                        &argument_instance,
                        constraint,
                        &constraint_instance,
                    );
                }

                self.solve_type_relation(cause, Relation::Satisfies, argument, constraint)
            }
        }
    }

    /// Return the call signature carried by one callable value representation.
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
        cause: CauseId,
        relation: Relation,
        value_use: Option<ValueUse>,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        let origin = self.cause_origin(cause);
        let check = match value_use {
            Some(_) => {
                let check =
                    answer!(self.check_value_constraint(cause, origin, relation, source, target)?);

                Answer::Ready(check.outcome)
            }
            None => {
                let holds = answer!(self.constrain_type(cause, relation, source, target)?);
                let check =
                    self.complete_constraint_check(cause, relation, source, target, holds)?;

                Answer::Ready(check)
            }
        };

        match check {
            Answer::Ready(CheckOutcome::Holds) => Ok(Answer::Ready(())),
            Answer::Ready(CheckOutcome::Fails(failure)) => {
                self.report_constraint_failure(
                    cause, relation, value_use, source, target, failure,
                )?;

                Ok(Answer::Ready(()))
            }
            Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
        }
    }

    /// Check one value constraint and return the completed result.
    pub(in crate::check) fn check_value_constraint(
        &mut self,
        cause: CauseId,
        value_origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<ValueCheck>> {
        // bodies commit their nodes before fulfillment relates them
        if let Some(node) = value_origin.expression() {
            let site = self.node_site(node.into())?;
            let (_, check) = answer!(self.check_node_value(site, relation, target, cause)?);

            return Ok(Answer::Ready(check));
        }

        self.check_value_relation(cause, relation, source, target)
    }

    /// Check one already typed value relation.
    pub(in crate::check) fn check_value_relation(
        &mut self,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<ValueCheck>> {
        let origin = self.cause_origin(cause);
        let source_value = answer!(self.reduce_type_head(origin, source)?);
        let source_value = answer!(self.strip_form(origin, source_value)?);
        let source_is_union = matches!(self.ty(source_value)?, dir::Type::Union(_));

        // select the exact carrier member at the runtime value boundary
        if relation.distributes_over_union_target()
            && !source_is_union
            && let Some(arms) = answer!(self.union_arms(origin, target)?)
        {
            for target in arms {
                let selected = answer!(self.confirm_candidate(ProbeReason::UnionArm, |state| {
                    let checked =
                        answer!(state.check_value_target(cause, relation, source, target,)?);
                    let outcome = match checked.outcome {
                        CheckOutcome::Holds => CandidateOutcome::Accepted(checked),
                        CheckOutcome::Fails(_) => CandidateOutcome::Rejected(()),
                    };

                    Ok(Answer::Ready(outcome))
                })?);
                if let Some(checked) = selected {
                    return Ok(Answer::Ready(checked));
                }
            }
        }

        self.check_value_target(cause, relation, source, target)
    }

    /// Check one already typed value against one concrete target.
    fn check_value_target(
        &mut self,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<ValueCheck>> {
        let holds = answer!(self.constrain_type(cause, relation, source, target)?);
        let outcome = self.complete_constraint_check(cause, relation, source, target, holds)?;

        Ok(Answer::Ready(ValueCheck { outcome, target }))
    }

    /// Check one type constraint and return the completed result.
    pub(in crate::check) fn check_type_constraint(
        &mut self,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<CheckOutcome>> {
        let holds = answer!(self.constrain_type(cause, relation, source, target)?);
        let check = self.complete_constraint_check(cause, relation, source, target, holds)?;

        Ok(Answer::Ready(check))
    }

    /// Return the completed check for one closed constraint.
    pub(in crate::check) fn complete_constraint_check(
        &mut self,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        holds: bool,
    ) -> CompilerResult<CheckOutcome> {
        let origin = self.cause_origin(cause);

        let is_property_relation = matches!(
            relation,
            Relation::Assignable | Relation::Writable | Relation::Satisfies
        );

        let check = match (holds, is_property_relation) {
            // ordinary successful property relations still run object-literal exactness
            (true, true) => match self.property_literal_excess_key(origin, source, target)? {
                Some(key) => CheckOutcome::Fails(CheckFailure::ExcessProperty { key }),
                None => CheckOutcome::Holds,
            },
            // ordinary successful non-property relations are complete
            (true, false) => CheckOutcome::Holds,
            // failed non-property relations only carry the relation failure
            (false, false) => CheckOutcome::Fails(CheckFailure::Relation),
            // failed property relations explain the same order as relation checking
            (false, true) => {
                if let Some(key) = self.property_literal_missing_key(origin, source, target)? {
                    CheckOutcome::Fails(CheckFailure::MissingRequiredProperty { key })
                } else if let Some(key) = self.first_missing_struct_field(origin, source, target)? {
                    CheckOutcome::Fails(CheckFailure::MissingRequiredProperty { key })
                } else if let Some(key) =
                    self.property_literal_excess_key(origin, source, target)?
                {
                    CheckOutcome::Fails(CheckFailure::ExcessProperty { key })
                } else if let Some(key) = self.first_excess_struct_field(source, target)? {
                    CheckOutcome::Fails(CheckFailure::ExcessProperty { key })
                } else if let Some(signature) =
                    self.first_writable_index_signature(origin, target)?
                {
                    CheckOutcome::Fails(CheckFailure::WritableIndexRequiresIndexSet { signature })
                } else {
                    CheckOutcome::Fails(CheckFailure::Relation)
                }
            }
        };

        Ok(check)
    }

    /// Match one relation between two types, bounding open variables.
    pub(in crate::check) fn constrain_type(
        &mut self,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // collect open-root constraints before relation-specific normalization
        if relation == Relation::Satisfies {
            let source = self.settled_root(source)?;
            let target = self.settled_root(target)?;
            let has_open_root =
                self.root_variable(source)?.is_some() || self.root_variable(target)?.is_some();
            if !has_open_root {
                return self.satisfy_type_constraint(cause, source, target);
            }
        }

        self.solve_type_relation(cause, relation, source, target)
    }

    /// Solve one type relation after relation-specific normalization.
    fn solve_type_relation(
        &mut self,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let origin = self.cause_origin(cause);

        // substitute solved variables before comparing
        let source = self.settled_root(source)?;
        let target = self.settled_root(target)?;
        if source == target {
            // retain directed self dependencies for declaration inference
            if relation != Relation::Equal
                && let Some(variable) = self.root_variable(source)?
                && self.solver.variable_role(variable)? == VariableRole::Return
            {
                self.push_lower_bound(variable, cause, source, relation)?;
            }

            return Ok(Answer::Ready(true));
        }

        let source_variable = self.root_variable(source)?;
        let target_variable = self.root_variable(target)?;

        match (source_variable, target_variable, relation) {
            // variable equality forms one dependency component
            (Some(source_variable), Some(target_variable), Relation::Equal) => {
                self.push_upper_bound(source_variable, cause, target, Relation::Equal)?;
                self.push_lower_bound(target_variable, cause, source, Relation::Equal)?;

                Ok(Answer::Ready(true))
            }
            // unify one open side with the other type
            (Some(variable), None, Relation::Equal) => {
                self.commit_solution(variable, target)?;

                Ok(Answer::Ready(true))
            }
            (None, Some(variable), Relation::Equal) => {
                self.commit_solution(variable, source)?;

                Ok(Answer::Ready(true))
            }
            // directed relations bound the open side directionally, and
            //  writes into open places choose from the written value
            (
                Some(_),
                Some(variable),
                Relation::Assignable | Relation::Widens | Relation::Castable | Relation::Writable,
            ) => {
                self.push_lower_bound(variable, cause, source, relation)?;

                Ok(Answer::Ready(true))
            }
            (Some(variable), _, Relation::Assignable | Relation::Widens | Relation::Castable) => {
                self.push_upper_bound(variable, cause, target, relation)?;

                Ok(Answer::Ready(true))
            }
            (
                None,
                Some(variable),
                Relation::Assignable | Relation::Widens | Relation::Castable | Relation::Writable,
            ) => {
                self.push_lower_bound(variable, cause, source, relation)?;

                Ok(Answer::Ready(true))
            }
            // constraint relations restrict the open source without choosing it
            (Some(variable), _, Relation::Satisfies) => {
                self.push_upper_bound(variable, cause, target, relation)?;

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
                    Relation::Writable | Relation::Castable => Relation::Assignable,
                    relation => relation,
                };

                // reduce aliases and intrinsics at the root; open template
                //  patterns pass through raw so bounds flow into their spans
                let source = match self.reduce_type_head(origin, source)? {
                    Answer::Ready(source) => source,
                    Answer::Pending(_) if self.is_open_template(source)? => source,
                    Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                };
                let target = match self.reduce_type_head(origin, target)? {
                    Answer::Ready(target) => target,
                    Answer::Pending(_) if self.is_open_template(target)? => target,
                    Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                };

                // enum members flow into their owner instantiation
                if let dir::Type::EnumMember(member) = self.ty(source)?
                    && !matches!(self.ty(target)?, dir::Type::EnumMember(_))
                    && relation != Relation::Equal
                {
                    return self.constrain_type(cause, relation, member.owner, target);
                }

                // push bounds into known composites that still contain holes
                if !self.type_variables(source)?.is_empty()
                    || !self.type_variables(target)?.is_empty()
                {
                    return match structural {
                        Relation::Equal | Relation::Assignable | Relation::Widens => {
                            match self.constrain_structural(cause, structural, source, target)? {
                                Some(answer) => Ok(answer),
                                None => Ok(self.pending_on_open_leaves(source, target)?),
                            }
                        }
                        _ => Ok(self.pending_on_open_leaves(source, target)?),
                    };
                }

                // reduce closed operands before comparing relation truth
                let source = answer!(self.reduce_type(origin, source)?);
                let target = answer!(self.reduce_type(origin, target)?);

                self.decide_relation(origin, relation, source, target)
            }
        }
    }

    /// Relate matching composites slot by slot, bounding open leaves.
    fn constrain_structural(
        &mut self,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Answer<bool>>> {
        let origin = self.cause_origin(cause);
        // only composites with open leaves need constraint recursion
        if self.type_variables(source)?.is_empty() && self.type_variables(target)?.is_empty() {
            return Ok(None);
        }

        // same-symbol applications constrain arguments under their default handle context
        let same_symbol = match (self.ty(source)?, self.ty(target)?) {
            (dir::Type::Instance(source_instance), dir::Type::Instance(target_instance))
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
            let context = self.default_symbol_context(symbol);

            return Ok(Some(self.relate_type_arguments(
                cause,
                symbol,
                context,
                relation.interior(),
                &source,
                &target,
            )?));
        }

        // memory forms own placement and readonly views; open template
        //  patterns skip the gate so bounds still flow into their spans
        if matches!(relation, Relation::Assignable | Relation::Widens)
            && !self.is_open_template(source)?
            && !self.is_open_template(target)?
            && let Some(decision) =
                self.constrain_form_assignable(cause, relation, source, target)?
        {
            return Ok(Some(decision));
        }

        // test known nominal roots through heritage before waiting on arguments
        if matches!(relation, Relation::Assignable | Relation::Widens)
            && let (dir::Type::Instance(_), dir::Type::Instance(_)) =
                (self.ty(source)?, self.ty(target)?)
        {
            return Ok(Some(
                self.decide_nominal_assignable(origin, source, target)?,
            ));
        }

        // open composites constrain against one matching union arm only,
        //  but membership tags the value and never widens
        if relation != Relation::Widens
            && !matches!(self.ty(source)?, dir::Type::Union(_))
            && let dir::Type::Union(union) = self.ty(target)?
        {
            let elements = self.type_ids(target.module_id, union.elements)?.to_vec();
            let mut matching = None;
            for element in elements {
                let Answer::Ready(element) = self.reduce_type_head(origin, element)? else {
                    continue;
                };
                let is_equal = matches!(
                    self.decide_relation(origin, Relation::Equal, source, element)?,
                    Answer::Ready(true),
                );
                let is_matching = is_equal || self.decompose_type_pair(source, element)?.is_some();
                if !is_matching {
                    continue;
                }
                if matching.is_some() {
                    matching = None;
                    break;
                }
                matching = Some(element);
            }
            if let Some(arm) = matching {
                return Ok(Some(self.constrain_type(cause, relation, source, arm)?));
            }
        }

        let source_signature = self.callable_signature(source)?;
        let target_signature = self.callable_signature(target)?;

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
            (dir::Type::Shape(source_shape), dir::Type::Shape(target_shape)) => {
                let source_fields = self.shape_fields(source.module_id, source_shape.fields)?;
                let target_fields = self.shape_fields(target.module_id, target_shape.fields)?;

                for target_field in target_fields {
                    let source_field = source_fields
                        .iter()
                        .find(|field| field.key == target_field.key);

                    match source_field {
                        Some(source_field) => {
                            let field_relation =
                                if matches!(relation, Relation::Assignable | Relation::Widens) {
                                    let Some(field_relation) =
                                        self.shape_field_relation(source_field, target_field)
                                    else {
                                        return Ok(Some(Answer::Ready(false)));
                                    };

                                    field_relation
                                } else {
                                    relation
                                };

                            pairs.push((
                                Some(CauseKind::Field {
                                    key: target_field.key,
                                }),
                                field_relation,
                                source_field.ty,
                                target_field.ty,
                            ));
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
            (dir::Type::Union(elements), _) if relation == Relation::Assignable => {
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

                return Ok(Some(
                    self.constrain_union_target(cause, relation, source, &elements)?,
                ));
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
            // ambiguous composites wait for their open leaves instead
            _ => return Ok(Some(self.pending_on_open_leaves(source, target)?)),
        }

        // constrain every slot pair through the bounding path
        let mut decision = Answer::Ready(true);
        for (kind, relation, source, target) in pairs {
            let child = match kind {
                Some(kind) => self.intern_cause(Cause::slot(origin, kind, cause)),
                None => cause,
            };
            decision = decision.and(self.constrain_type(child, relation, source, target)?);
            if decision.is_ready_false() {
                return Ok(Some(decision));
            }
        }

        Ok(Some(decision))
    }

    /// Constrain one value into a union target.
    fn constrain_union_target(
        &mut self,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        elements: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<bool>> {
        // try every arm speculatively, keeping the first viable arm
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for element in elements.iter().copied() {
            match self.confirm_candidate(ProbeReason::UnionArm, |state| {
                match state.constrain_type(cause, relation, source, element)? {
                    Answer::Ready(true) => Ok(Answer::Ready(CandidateOutcome::Accepted(()))),
                    Answer::Ready(false) => Ok(Answer::Ready(CandidateOutcome::Rejected(()))),
                    Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
                }
            })? {
                Answer::Ready(Some(())) => return Ok(Answer::Ready(true)),
                Answer::Ready(None) => {}
                Answer::Pending(dependencies) => {
                    blockers.extend(dependencies);
                }
            }
        }

        Ok(Answer::ready_unless_blocked(false, blockers))
    }

    /// Park one ambiguous structural pair on its open leaves.
    fn pending_on_open_leaves(
        &self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for variable in self
            .type_variables(source)?
            .into_iter()
            .chain(self.type_variables(target)?)
        {
            if !blockers.contains(&Dependency::Variable(variable)) {
                blockers.push(Dependency::Variable(variable));
            }
        }

        Ok(Answer::pending(blockers))
    }

    /// Return the root after substituting solved inference variables.
    pub(in crate::check) fn settled_root(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut current = id;

        // substitute a solved top variable for its solution
        while let dir::Type::Variable(variable) = self.ty(current)? {
            let Some(solution) = self.solver.solution(variable)? else {
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

    /// Return the local source node anchoring one work origin.
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
