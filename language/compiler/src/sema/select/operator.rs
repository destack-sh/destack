use std::iter;

use destack_dir as dir;

use crate::sema::{
    Cause, CauseId, CauseKind, CheckOutcome, CheckState, Expectation, FailedCheck, FlowSite,
    InferMode, Obligation, OperatorExpressionResult, Origin, PlaceUse, ProtocolCall, Relation,
    RelationCheck, SignatureRejection, StoreTarget, Value, ValueUse, VariableKind,
    WritableTargetObligation, binary_operator_protocols, unary_operator_protocols,
};
use crate::{CompilerError, CompilerResult};

/// The integer domain an untyped literal shift produces.
const SHIFT_LITERAL_INTEGER: dir::IntegerType = dir::IntegerType::Fixed {
    width: 64,
    is_signed: true,
};

/// Operands displayed for a rejected operator.
pub(in crate::sema) enum OperatorOperands<'a> {
    /// Type operands.
    Types(&'a [dir::GlobalTypeId]),
    /// Required place operand.
    Place,
}

impl CheckState<'_> {
    /// Select one binary operation from known operand types.
    pub(in crate::sema) fn select_binary_operation(
        &mut self,
        site: FlowSite,
        operator: dir::BinaryOperator,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
        left_source: dir::GlobalNodeIdAny,
        right_source: dir::GlobalNodeIdAny,
        writeback: Option<dir::GlobalTypeId>,
        expectation: Option<Expectation>,
    ) -> CompilerResult<()> {
        let node = site.node;
        let origin = site.origin();

        // resolve solved variables out of the operands before judging them
        let left = self.deeply_resolve(origin, left)?;
        let right = self.deeply_resolve(origin, right)?;

        // poison the node when either operand carries a reported error
        let left = self.resolve_structurally(site, left)?;
        let right = self.resolve_structurally(site, right)?;
        if matches!(self.ty(left)?, dir::Type::Error) || matches!(self.ty(right)?, dir::Type::Error)
        {
            self.poison_node(node)?;

            return Ok(());
        }

        // classify nullish and never operands, which compare structurally
        let is_nullish_or_never = matches!(
            self.ty(left)?,
            dir::Type::Null | dir::Type::Undefined | dir::Type::Never
        ) || matches!(
            self.ty(right)?,
            dir::Type::Null | dir::Type::Undefined | dir::Type::Never
        );

        // scalar comparisons read values, so views compare their pointees
        let left_value = self.strip_form(origin, left)?;
        let right_value = self.strip_form(origin, right)?;
        let left_families = self.scalar_families(origin, left_value)?;
        let right_families = self.scalar_families(origin, right_value)?;

        // compare operands sharing exactly one scalar family
        let is_comparable = match (&left_families, &right_families) {
            (Some(left), Some(right)) => left.len() == 1 && left == right,
            (None, _) | (_, None) => false,
        };
        let is_numeric = is_comparable
            && left_families
                .as_ref()
                .is_some_and(dir::ScalarFamilySet::is_numeric);

        // equality and logic produce builtin results directly
        let builtin = match operator {
            // require overlapping values with one common representation
            dir::BinaryOperator::EqualStrict | dir::BinaryOperator::NotEqualStrict => {
                let supported = self.has_strict_equality(origin, left_value, right_value)?;
                if !supported {
                    // report an owned operand, which carries no identity to compare
                    for operand in [left_value, right_value] {
                        let is_scalar = self.scalar_families(origin, operand)?.is_some();
                        if !is_scalar
                            && self.default_ownership(origin, operand)?
                                == Some(dir::Ownership::Owned)
                        {
                            self.report_no_strict_identity(origin, operand)?;
                            self.commit_error_node(node)?;

                            return Ok(());
                        }
                    }

                    return self.report_rejected_operator(
                        node,
                        origin,
                        operator.text().to_string(),
                        &[left, right],
                        None,
                    );
                }

                let overlaps = self.has_strict_equality_witness(origin, left_value, right_value)?
                    || self.types_may_overlap(origin, left_value, right_value)?;
                if !overlaps {
                    self.report_invalid_strict_equality(origin, left, right)?;
                }

                // select the exact accepted type of each compared value
                let sources = [
                    (left_source, left_value, left),
                    (right_source, right_value, right),
                ];
                let operands = self.select_strict_equality_operands(origin, &sources)?;
                let [left_operand, right]: [dir::BuiltinOperand; 2] =
                    operands.try_into().map_err(|_| CompilerError::Internal {
                        message: "binary strict equality did not select two operands".to_string(),
                    })?;
                let result = self.intern_type(dir::Type::Primitive(dir::PrimitiveType::Boolean))?;

                return self.commit_builtin_binary_operator(
                    origin,
                    node,
                    operator,
                    left_operand,
                    right,
                    result,
                    writeback,
                );
            }
            // nullish and same-kind scalar equality produce booleans
            dir::BinaryOperator::Equal | dir::BinaryOperator::NotEqual
                if is_nullish_or_never || (is_comparable && !is_numeric) =>
            {
                let result = self.intern_type(dir::Type::Primitive(dir::PrimitiveType::Boolean))?;

                Some((result, left_value, right_value))
            }
            // same-kind ordered scalar comparisons produce booleans
            dir::BinaryOperator::LessThan
            | dir::BinaryOperator::LessThanOrEqual
            | dir::BinaryOperator::GreaterThan
            | dir::BinaryOperator::GreaterThanOrEqual
                if is_comparable
                    && !is_numeric
                    && left_families
                        .as_ref()
                        .is_some_and(dir::ScalarFamilySet::is_ordered) =>
            {
                let result = self.intern_type(dir::Type::Primitive(dir::PrimitiveType::Boolean))?;

                Some((result, left_value, right_value))
            }
            // logical joins produce the union of their operands
            dir::BinaryOperator::And | dir::BinaryOperator::Or => {
                Some((self.normalized_union_type([left, right])?, left, right))
            }
            // type `??` at the expected type, else as the output joined with the fallback
            dir::BinaryOperator::Coalesce => {
                let output = self
                    .reduce_operation_type(origin, dir::TypeOperation::TryOutput { value: left })?;
                let result = match expectation.and_then(Expectation::contextual_target) {
                    Some(target) => {
                        let left_site = self.visit_site(left_source)?;
                        let cause = self
                            .intern_cause(Cause::root(left_site.origin(), CauseKind::Expression));
                        let present = Value {
                            ty: output,
                            node: None,
                            place: None,
                            is_fresh: false,
                        };
                        self.check_coalesce_operand(left_site, cause, present, target)?;

                        target
                    }
                    None => self.normalized_union_type([output, right])?,
                };

                // record the fallback's conversion into the joined result
                if result != right {
                    let right_site = self.visit_site(right_source)?;
                    let cause =
                        self.intern_cause(Cause::root(right_site.origin(), CauseKind::Expression));
                    let right_value = self.expression_value(right_site, right)?;
                    self.check_coalesce_operand(right_site, cause, right_value, result)?;
                }

                Some((result, left, right))
            }
            _ => None,
        };
        if let Some((result, left_target, right_target)) = builtin {
            return self.commit_builtin_operands(
                origin,
                node,
                operator,
                (left_source, left),
                (right_source, right),
                [left_target, right_target],
                result,
                writeback,
            );
        }

        // check builtin numeric operands against their joined type
        if let Some((result, operands)) =
            self.builtin_numeric_result(origin, operator, left, right)?
        {
            // operands check as arguments of the builtin operation
            return self.commit_builtin_operands(
                origin,
                node,
                operator,
                (left_source, left),
                (right_source, right),
                [operands[0], operands[1]],
                result,
                writeback,
            );
        }

        // dispatch through the operator protocol interfaces
        let left_site = self.visit_site(left_source)?;
        let left_value = self.expression_value(left_site, left)?;

        // select the protocol at the operand's value, an owned operand at its default form
        let protocol_operand = self.owned_value(right_value)?.unwrap_or(right_value);
        let protocols = binary_operator_protocols(operator);
        let mut rejection = None;
        for protocol in protocols.iter() {
            let key = protocol.method.key(self.strings());
            let protocol_type = self.operator_protocol(origin, protocol, &[protocol_operand])?;
            let argument_sources = [dir::ArgumentSource::Provided(right_source)];

            // keep the first specific rejection of a selected method
            let call = match self.select_protocol_call(
                origin,
                left_value,
                left,
                dir::MemberSpace::Instance,
                key,
                &protocol_type,
                &argument_sources,
            )? {
                Ok(call) => call,
                Err(rejected) => {
                    if rejection.is_none() && rejected != SignatureRejection::Inapplicable {
                        rejection = Some(rejected);
                    }

                    continue;
                }
            };

            return self.commit_operator_call(
                origin,
                node,
                operator,
                protocol.expression_result,
                call,
                writeback,
            );
        }

        self.report_rejected_operator(
            node,
            origin,
            operator.text().to_string(),
            &[left, right],
            rejection,
        )
    }

    /// Check one coalesce operand's type against the joined result, leaving it unconverted.
    fn check_coalesce_operand(
        &mut self,
        site: FlowSite,
        cause: CauseId,
        value: Value,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let conversion = self.convert_value(
            site,
            cause,
            Relation::Storable,
            value,
            target,
            ValueUse::Output,
            InferMode::Regular,
        )?;
        if let CheckOutcome::Fails(failure) = conversion.outcome {
            self.push_failure(FailedCheck {
                cause,
                relation: Relation::Storable,
                use_: Some(ValueUse::Output),
                source: conversion.source,
                target: conversion.target,
                failure,
            })?;
        }

        Ok(())
    }

    /// Check and lower both operands as builtin values, committing the result.
    fn commit_builtin_operands(
        &mut self,
        origin: Origin,
        node: dir::GlobalNodeIdAny,
        operator: dir::BinaryOperator,
        left: (dir::GlobalNodeIdAny, dir::GlobalTypeId),
        right: (dir::GlobalNodeIdAny, dir::GlobalTypeId),
        operands: [dir::GlobalTypeId; 2],
        result: dir::GlobalTypeId,
        writeback: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<()> {
        self.check_builtin_operand(left.0, left.1, operands[0])?;
        self.check_builtin_operand(right.0, right.1, operands[1])?;
        let left = self.builtin_operand(origin, left.0, operands[0])?;
        let right = self.builtin_operand(origin, right.0, operands[1])?;

        self.commit_builtin_binary_operator(origin, node, operator, left, right, result, writeback)
    }

    /// Commit one selected operator protocol call at a node.
    fn commit_operator_call(
        &mut self,
        origin: Origin,
        node: dir::GlobalNodeIdAny,
        operator: dir::BinaryOperator,
        expression_result: OperatorExpressionResult,
        call: ProtocolCall,
        writeback: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<()> {
        let result = self.operator_expression_type(origin, expression_result, call.return_type)?;

        // build the operator decision from the selected call
        let resolution = match call.resolution {
            dir::OperationResolution::One(call) => {
                let application = dir::OperatorApplication::Binary {
                    operator,
                    target: dir::OperatorTarget::Call(Box::new(call)),
                    ty: result,
                    is_folded: false,
                };

                dir::OperationResolution::One(application)
            }
            dir::OperationResolution::Union { arms, .. } => {
                let mut applications = Vec::with_capacity(arms.len());
                for call in arms {
                    let ty =
                        self.operator_expression_type(origin, expression_result, call.return_type)?;
                    applications.push(dir::OperatorApplication::Binary {
                        operator,
                        target: dir::OperatorTarget::Call(Box::new(call)),
                        ty,
                        is_folded: false,
                    });
                }

                dir::OperationResolution::Union {
                    arms: applications,
                    ty: result,
                }
            }
        };

        self.commit_operator(origin, node, resolution, result, writeback)
    }

    /// Select builtin strict equality for one switch and all its cases.
    pub(in crate::sema) fn select_switch_equality(
        &mut self,
        value_source: dir::GlobalNodeIdAny,
        scrutinee: dir::GlobalTypeId,
        cases: &[(
            dir::GlobalNodeIdAny,
            dir::GlobalNodeIdAny,
            dir::GlobalTypeId,
        )],
    ) -> CompilerResult<()> {
        let value_site = self.visit_site(value_source)?;
        let origin = value_site.origin();
        let scrutinee_value = self.strip_form(origin, scrutinee)?;
        let mut selected = Vec::with_capacity(cases.len());

        // resolve open operands structurally before selecting a common representation
        for operand in iter::once(scrutinee).chain(cases.iter().map(|(_, _, ty)| *ty)) {
            let value = self.strip_form(origin, operand)?;
            self.resolve_structurally(value_site, value)?;
        }

        // reject cases outside builtin strict equality
        for (case, selector, ty) in cases {
            let case_site = self.visit_site(*case)?;
            let selector_site = self.visit_site(*selector)?;
            let selector_value = self.strip_form(selector_site.origin(), *ty)?;
            let supported =
                self.has_strict_equality(selector_site.origin(), scrutinee_value, selector_value)?;
            if !supported {
                self.report_rejected_operator(
                    *case,
                    case_site.origin(),
                    dir::BinaryOperator::EqualStrict.text().to_string(),
                    &[scrutinee, *ty],
                    None,
                )?;
                continue;
            }

            selected.push((*case, *selector, *ty, selector_value));
        }
        if selected.is_empty() {
            return Ok(());
        }

        // select the exact accepted type of the scrutinee and every admitted case
        let mut sources = Vec::with_capacity(selected.len() + 1);
        sources.push((value_source, scrutinee_value, scrutinee));
        sources.extend(
            selected
                .iter()
                .map(|(_, selector, ty, value)| (*selector, *value, *ty)),
        );
        let operands = self.select_strict_equality_operands(origin, &sources)?;
        let Some((scrutinee_operand, selector_operands)) = operands.split_first() else {
            return Err(CompilerError::Internal {
                message: "switch equality did not select its scrutinee".to_string(),
            });
        };

        // diagnose pairwise disjoint cases and commit their builtin operation
        let boolean = self.intern_type(dir::Type::Primitive(dir::PrimitiveType::Boolean))?;
        for ((case, _, ty, selector_value), selector_operand) in
            selected.into_iter().zip(selector_operands)
        {
            let overlaps = self.types_may_overlap(origin, scrutinee_value, selector_value)?;
            if !overlaps {
                self.report_invalid_strict_equality(origin, scrutinee, ty)?;
            }
            self.commit_builtin_binary_operator(
                origin,
                case,
                dir::BinaryOperator::EqualStrict,
                scrutinee_operand.clone(),
                selector_operand.clone(),
                boolean,
                None,
            )?;
        }

        Ok(())
    }

    /// Select one unary operator application.
    pub(in crate::sema) fn select_unary_operator(
        &mut self,
        site: FlowSite,
        operator: dir::UnaryOperator,
        operand_node: dir::LocalNodeId<dir::Expression>,
        use_: PlaceUse,
    ) -> CompilerResult<()> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = site.origin();
        let operand_site = self.visit_site(operand_node.into_global_any(module))?;

        // increments rewrite builtin numeric places by one
        if matches!(
            operator,
            dir::UnaryOperator::PostIncrement
                | dir::UnaryOperator::PostDecrement
                | dir::UnaryOperator::PreIncrement
                | dir::UnaryOperator::PreDecrement
        ) {
            let Some(place) =
                self.select_assignment(operand_site, operand_node, PlaceUse::Update)?
            else {
                self.report_no_matching_operator(
                    origin,
                    operator.text().to_string(),
                    OperatorOperands::Place,
                )?;
                self.commit_decision(node, dir::Decision::Rejected)?;
                self.commit_error_node(node)?;

                return Ok(());
            };
            let operand = place
                .read
                .as_ref()
                .map(dir::ReadResolution::ty)
                .ok_or_else(|| CompilerError::Internal {
                    message: "updated place has no readable type".to_string(),
                })?;
            let write_type = place.write.ty();

            if self.is_numeric_operand(origin, operand)? {
                // commit the updated place as the operand's own decision
                let source = place.source;
                let resolution = place.clone().resolution();
                self.commit_node_type(source, operand)?;
                self.commit_decision(source, dir::Decision::Assignment(Box::new(resolution)))?;
                self.commit_access_use(source, dir::BindingUse::WRITE);

                // require the place to accept a write at its scope
                let scope = self.origin_scope(origin)?;
                self.push_obligation(
                    Obligation::WritableTarget(Box::new(WritableTargetObligation {
                        target: place,
                        ty: operand,
                    })),
                    scope,
                )?;

                // require the rewritten value to store back into the place
                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
                self.push_relation(RelationCheck::new(
                    origin,
                    Relation::Storable,
                    operand,
                    write_type,
                    cause,
                ))?;

                // apply the builtin rewrite over the read value
                let result = operand;
                let operand = self.builtin_operand(origin, source, operand)?;

                return self.commit_builtin_unary_operator(node, operator, operand, result);
            }

            return self.report_rejected_operator(
                node,
                origin,
                operator.text().to_string(),
                &[operand],
                None,
            );
        }

        // read the operand's own type
        let operand = self.operand_type(origin, operand_site)?;

        // poison the node when the operand carries a reported error
        if matches!(self.ty(operand)?, dir::Type::Error) {
            self.poison_node(node)?;

            return Ok(());
        }

        // builtin logical not produces a boolean
        if matches!(operator, dir::UnaryOperator::Not) {
            let result = self.intern_type(dir::Type::Primitive(dir::PrimitiveType::Boolean))?;
            let operand = self.builtin_operand(origin, operand_site.node, operand)?;

            return self.commit_builtin_unary_operator(node, operator, operand, result);
        }

        // builtin numeric negation reduces singleton operands
        if matches!(
            operator,
            dir::UnaryOperator::Negate | dir::UnaryOperator::Plus
        ) && self.is_numeric_operand(origin, operand)?
        {
            let result = self.builtin_unary_result(origin, operator, operand)?;
            let operand = self.builtin_operand(origin, operand_site.node, operand)?;

            return self.commit_builtin_unary_operator(node, operator, operand, result);
        }

        // builtin bitwise not moves bits through integers
        if matches!(operator, dir::UnaryOperator::ElementwiseNot)
            && self.is_integral_operand(origin, operand)?
        {
            let result = self.builtin_unary_result(origin, operator, operand)?;
            let operand = self.builtin_operand(origin, operand_site.node, operand)?;

            return self.commit_builtin_unary_operator(node, operator, operand, result);
        }

        // dereferences select either a direct projection or protocol call
        let access = match use_ {
            PlaceUse::Read => dir::Access::Readonly,
            PlaceUse::Write | PlaceUse::Update => dir::Access::Mutable,
        };
        if operator == dir::UnaryOperator::Dereference {
            let operand_value = self.expression_value(operand_site, operand)?;
            let Some(selection) = self.select_dereference(origin, operand_value, access)? else {
                return self.report_rejected_operator(
                    node,
                    origin,
                    operator.text().to_string(),
                    &[operand],
                    None,
                );
            };
            let result = selection.ty();
            let resolution =
                self.dereference_operator_decision(origin, operand_site.node, operator, selection)?;

            return self.commit_operator(origin, node, resolution, result, None);
        }

        // dispatch through the operator protocol interfaces
        let operand_value = self.expression_value(operand_site, operand)?;
        let protocols = unary_operator_protocols(operator, access);
        let mut rejection = None;
        for protocol in protocols {
            let key = protocol.method.key(self.strings());
            let protocol_type = self.operator_protocol(origin, &protocol, &[])?;

            // keep the first specific rejection of a selected method
            let call = match self.select_protocol_call(
                origin,
                operand_value,
                operand,
                dir::MemberSpace::Instance,
                key,
                &protocol_type,
                &[],
            )? {
                Ok(call) => call,
                Err(rejected) => {
                    if rejection.is_none() && rejected != SignatureRejection::Inapplicable {
                        rejection = Some(rejected);
                    }

                    continue;
                }
            };
            let result = self.operator_expression_type(
                origin,
                protocol.expression_result,
                call.return_type,
            )?;

            let resolution = match call.resolution {
                dir::OperationResolution::One(call) => {
                    let application = dir::OperatorApplication::Unary {
                        operator,
                        target: dir::OperatorTarget::Call(Box::new(call)),
                        ty: result,
                        is_folded: false,
                    };

                    dir::OperationResolution::One(application)
                }
                dir::OperationResolution::Union { arms, .. } => {
                    let mut applications = Vec::with_capacity(arms.len());
                    for call in arms {
                        let ty = self.operator_expression_type(
                            origin,
                            protocol.expression_result,
                            call.return_type,
                        )?;
                        applications.push(dir::OperatorApplication::Unary {
                            operator,
                            target: dir::OperatorTarget::Call(Box::new(call)),
                            ty,
                            is_folded: false,
                        });
                    }

                    dir::OperationResolution::Union {
                        arms: applications,
                        ty: result,
                    }
                }
            };

            return self.commit_operator(origin, node, resolution, result, None);
        }

        self.report_rejected_operator(
            node,
            origin,
            operator.text().to_string(),
            &[operand],
            rejection,
        )
    }

    /// Return whether one operand is only builtin numerics.
    fn is_numeric_operand(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let families = self.builtin_scalar_families(origin, ty)?;

        Ok(families.is_some_and(|families| families.is_numeric()))
    }

    /// Return whether one operand is only builtin integers.
    fn is_integral_operand(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let families = self.builtin_scalar_families(origin, ty)?;

        Ok(families.is_some_and(|families| families.is_integral()))
    }

    /// Select the accepted operands for builtin strict equality.
    fn select_strict_equality_operands(
        &mut self,
        origin: Origin,
        sources: &[(dir::GlobalNodeIdAny, dir::GlobalTypeId, dir::GlobalTypeId)],
    ) -> CompilerResult<Vec<dir::BuiltinOperand>> {
        let operands = sources.iter().map(|(_, ty, _)| *ty).collect::<Vec<_>>();
        let Some((first, rest)) = operands.split_first() else {
            return Err(CompilerError::Internal {
                message: "strict equality requires at least one operand".to_string(),
            });
        };

        // classify the operands equality must share a representation across
        let mut is_numeric = true;
        for operand in &operands {
            is_numeric &= self.is_numeric_operand(origin, *operand)?;
        }
        let mut has_union = false;
        if !is_numeric {
            for operand in &operands {
                has_union |= matches!(self.ty(*operand)?, dir::Type::Union(_));
            }
        }

        // join every numeric operand into one representation
        let common = if is_numeric {
            let mut representation = *first;
            for operand in rest {
                let Some(joined) = self.builtin_numeric_join(origin, representation, *operand)?
                else {
                    return Err(CompilerError::Internal {
                        message: "comparable numeric operands have no common type".to_string(),
                    });
                };
                representation = joined;
            }

            Some(representation)
        }
        // compare union operands through their joined type
        else if has_union {
            Some(self.normalized_union_type(operands.iter().copied())?)
        }
        // require every other operand to agree exactly
        else {
            let mut is_equal = true;
            let first_value = self.strip_form(origin, *first)?;
            for operand in rest {
                let is_equal_operand = self
                    .decide_relation(origin, Relation::Equal, *first, *operand)?
                    .holds();
                if !is_equal_operand
                    && self.type_flags(first_value)?.has_parameter()
                    && self.strip_form(origin, *operand)? == first_value
                {
                    self.report_invalid_strict_equality(origin, *first, *operand)?;
                }
                is_equal &= is_equal_operand;
            }

            is_equal.then_some(*first)
        };

        // check each operand at the type the operation takes
        let mut selected = Vec::with_capacity(sources.len());
        for (source, source_type, handle) in sources {
            let target = common.unwrap_or(*source_type);
            let is_handle = matches!(
                self.ty(*source_type)?.head(),
                dir::TypeHead::Value | dir::TypeHead::Empty
            ) && self.ownership(*source_type)? == Some(dir::Ownership::Managed);
            let checked = match is_handle {
                true => *handle,
                false => *source_type,
            };
            self.check_builtin_operand(*source, checked, target)?;
            selected.push(self.builtin_operand(origin, *source, target)?);
        }

        Ok(selected)
    }

    /// Return the builtin numeric result and selected operand types.
    fn builtin_numeric_result(
        &mut self,
        origin: Origin,
        operator: dir::BinaryOperator,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Option<(dir::GlobalTypeId, [dir::GlobalTypeId; 2])>> {
        // read through views to the pointee builtin scalars operate on
        let left = self.strip_form(origin, left)?;
        let right = self.strip_form(origin, right)?;

        // classify both operands once
        let is_numeric =
            self.is_numeric_operand(origin, left)? && self.is_numeric_operand(origin, right)?;
        let is_integral = is_numeric
            && self.is_integral_operand(origin, left)?
            && self.is_integral_operand(origin, right)?;

        // fold literal operands through the const static operation
        if is_numeric
            && matches!(
                (self.ty(left)?, self.ty(right)?),
                (dir::Type::Literal(_), dir::Type::Literal(_))
            )
            && let Ok(static_operator) = dir::StaticBinaryOperator::try_from(operator)
            && !static_operator.yields_boolean()
        {
            // evaluate fractional joins in the float domain
            let (left, right) = if is_integral {
                (left, right)
            } else {
                (
                    self.operand_as_float_literal(left)?,
                    self.operand_as_float_literal(right)?,
                )
            };

            // fold the static operation over the two literals
            let operation =
                self.intern_operation(dir::TypeOperation::StaticBinary(dir::StaticBinaryType {
                    operator: static_operator,
                    left,
                    right,
                }))?;
            let folded = self.normalize(origin, operation)?;
            if matches!(self.ty(folded)?, dir::Type::Literal(_)) {
                return Ok(Some((folded, [left, right])));
            }
        }

        // type the result by the operator's own family
        match operator {
            // shifts move bits through integers, keeping the left type
            dir::BinaryOperator::ShiftLeft
            | dir::BinaryOperator::ShiftRight
            | dir::BinaryOperator::UnsignedShiftRight
                if is_integral =>
            {
                let result = match self.ty(left)? {
                    // bigint literals shift in the bigint domain
                    dir::Type::Literal(dir::Literal::Bigint(_)) => {
                        self.intern_type(dir::Type::Primitive(dir::PrimitiveType::Bigint))?
                    }
                    // every other literal shifts in the default integer domain
                    dir::Type::Literal(_) => self.intern_type(dir::Type::Primitive(
                        dir::PrimitiveType::Integer(SHIFT_LITERAL_INTEGER),
                    ))?,
                    // typed shifts keep the left operand's integer type
                    _ => left,
                };

                // constant shift amounts must stay below the shifted width
                if let dir::Type::Primitive(dir::PrimitiveType::Integer(dir::IntegerType::Fixed {
                    width,
                    ..
                })) = self.ty(result)?
                    && let dir::Type::Literal(dir::Literal::Integer(amount)) = self.ty(right)?
                    && (amount < 0 || amount >= i64::from(width))
                {
                    self.report_shift_out_of_range(origin, amount, result)?;
                }

                Ok(Some((result, [result, result])))
            }
            // elementwise bit operations join equal integer operands
            dir::BinaryOperator::ElementwiseAnd
            | dir::BinaryOperator::ElementwiseOr
            | dir::BinaryOperator::ElementwiseXor
                if is_integral =>
            {
                let joined = self.builtin_numeric_join(origin, left, right)?;

                Ok(joined.map(|joined| (joined, [joined, joined])))
            }
            // arithmetic joins equal numeric operands
            dir::BinaryOperator::Add
            | dir::BinaryOperator::Subtract
            | dir::BinaryOperator::Multiply
            | dir::BinaryOperator::Divide
            | dir::BinaryOperator::Remainder
            | dir::BinaryOperator::Exponent
                if is_numeric =>
            {
                let joined = self.builtin_numeric_join(origin, left, right)?;

                Ok(joined.map(|joined| (joined, [joined, joined])))
            }
            // comparisons produce booleans over the joined operand type
            dir::BinaryOperator::Equal
            | dir::BinaryOperator::NotEqual
            | dir::BinaryOperator::LessThan
            | dir::BinaryOperator::LessThanOrEqual
            | dir::BinaryOperator::GreaterThan
            | dir::BinaryOperator::GreaterThanOrEqual
                if is_numeric =>
            {
                let Some(joined) = self.builtin_numeric_join(origin, left, right)? else {
                    return Ok(None);
                };
                let boolean =
                    self.intern_type(dir::Type::Primitive(dir::PrimitiveType::Boolean))?;

                Ok(Some((boolean, [joined, joined])))
            }
            _ => Ok(None),
        }
    }

    /// Check one operand as an argument of the selected builtin operation.
    fn check_builtin_operand(
        &mut self,
        source: dir::GlobalNodeIdAny,
        source_type: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        // read both operands through their solutions
        let source_type = self.shallow_resolve(source_type)?;
        let target = self.shallow_resolve(target)?;

        // const-folded operations have no runtime operands
        if matches!(self.ty(target)?, dir::Type::Literal(_)) {
            return Ok(());
        }

        // visit the operand at its own site
        let operand_site = self.visit_site(source)?;

        // concrete operands keep the ordinary checked value relation
        let cause = self.intern_cause(Cause::root(operand_site.origin(), CauseKind::Expression));
        let expectation = Expectation {
            target,
            relation: Relation::Storable,
            cause,
            use_: ValueUse::Operand,
            mode: InferMode::Regular,
            store: StoreTarget::Exact,
        };
        self.check_value(operand_site, source_type, expectation)?;

        Ok(())
    }

    /// Build one checked builtin operand.
    fn builtin_operand(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::BuiltinOperand> {
        let scalar_families = self.scalar_families(origin, ty)?;
        let source = source
            .try_into_typed::<dir::Expression>()
            .map_err(|message| CompilerError::Internal { message })?;
        let operand = dir::BuiltinOperand {
            source,
            ty,
            scalar_families,
        };

        Ok(operand)
    }

    /// Return one builtin unary result.
    fn builtin_unary_result(
        &mut self,
        origin: Origin,
        operator: dir::UnaryOperator,
        operand: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // fold only literal operands
        if !matches!(self.ty(operand)?, dir::Type::Literal(_)) {
            return Ok(operand);
        }
        let Ok(operator) = dir::StaticUnaryOperator::try_from(operator) else {
            return Ok(operand);
        };
        let operation = dir::TypeOperation::StaticUnary(dir::StaticUnaryType {
            operator,
            target: operand,
        });
        let result = self.reduce_operation_type(origin, operation)?;

        Ok(result)
    }

    /// Return one const integer literal lifted into the float domain.
    fn operand_as_float_literal(
        &mut self,
        operand: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let dir::Type::Literal(dir::Literal::Integer(value)) = self.ty(operand)? else {
            return Ok(operand);
        };
        let literal = dir::Type::Literal(dir::Literal::Float(value as f64));

        self.intern_type(literal)
    }

    /// Return one interval operand widened to its base scalar.
    fn operand_as_base_scalar(
        &mut self,
        operand: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let dir::Type::Range(range) = self.ty(operand)? else {
            return Ok(operand);
        };
        let Some(base) = range.widen() else {
            return Ok(operand);
        };

        self.intern_type(base)
    }

    /// Return the open integer binding variable one operand names, if any.
    pub(in crate::sema) fn integer_variable(
        &mut self,
        operand: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::TypeVariableId>> {
        let operand = self.shallow_resolve(operand)?;
        Ok(match self.ty(operand)? {
            dir::Type::Variable(variable) => {
                let root = self.infer.alias_root(variable)?;
                (self.infer.variable(root)?.kind == VariableKind::Integer).then_some(root)
            }
            _ => None,
        })
    }

    /// Join two builtin numeric operands into one common operand type.
    fn builtin_numeric_join(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // expose the value domain behind computation heads before joining
        let left = self.normalize_computation(origin, left)?;
        let right = self.normalize_computation(origin, right)?;

        // interval operands widen to their base scalar under arithmetic
        let left = self.operand_as_base_scalar(left)?;
        let right = self.operand_as_base_scalar(right)?;

        // an open integer variable takes the other operand's width, literals leave it open
        let left_variable = self.integer_variable(left)?;
        let right_variable = self.integer_variable(right)?;
        if left_variable.is_some() || right_variable.is_some() {
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
            let joined = match (left_variable, right_variable) {
                (Some(_), Some(_)) => {
                    self.constrain_type(origin, cause, Relation::Equal, left, right)?;
                    left
                }
                (Some(_), None) if matches!(self.ty(right)?, dir::Type::Literal(_)) => left,
                (Some(_), None) => {
                    self.constrain_type(origin, cause, Relation::Equal, left, right)?;
                    right
                }
                (None, Some(_)) if matches!(self.ty(left)?, dir::Type::Literal(_)) => right,
                (None, Some(_)) => {
                    self.constrain_type(origin, cause, Relation::Equal, right, left)?;
                    left
                }
                (None, None) => unreachable!("an integer variable operand was classified"),
            };

            return Ok(Some(joined));
        }

        // literals adapt into the other operand's type
        let left_literal = match self.ty(left)? {
            dir::Type::Literal(literal) => Some(literal),
            _ => None,
        };
        let right_literal = match self.ty(right)? {
            dir::Type::Literal(literal) => Some(literal),
            _ => None,
        };

        // widen the operands by the literals they carry
        match (left_literal, right_literal) {
            // literal pairs widen to their base numeric type
            (Some(left), Some(_)) => {
                let widened = self.intern_type(left.widen())?;

                Ok(Some(widened))
            }
            (Some(_), None) => {
                let adapts = self
                    .decide_relation(origin, Relation::Subtype, left, right)?
                    .holds();

                Ok(adapts.then_some(right))
            }
            (None, Some(_)) => {
                let adapts = self
                    .decide_relation(origin, Relation::Subtype, right, left)?
                    .holds();

                Ok(adapts.then_some(left))
            }
            // typed operands must agree exactly
            (None, None) => {
                let equal = self
                    .decide_relation(origin, Relation::Equal, left, right)?
                    .holds();

                Ok(equal.then_some(left))
            }
        }
    }

    /// Return the expression result for one selected operator method.
    pub(in crate::sema) fn operator_expression_type(
        &mut self,
        origin: Origin,
        expression_result: OperatorExpressionResult,
        return_type: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // produce the protocol's declared result
        let result = match expression_result {
            OperatorExpressionResult::MethodReturn => return_type,
            // project the place behind the returned borrow, reducing a projected result first
            OperatorExpressionResult::Pointee => {
                let reduced = self.normalize(origin, return_type)?;
                match self.ty(reduced)? {
                    dir::Type::Form(form)
                        if matches!(form.form, dir::Form::Borrowed(_) | dir::Form::Raw) =>
                    {
                        form.value
                    }
                    _ => {
                        let actual = self.format_type(return_type);

                        return Err(CompilerError::Internal {
                            message: format!(
                                "dereference operator returned non-pointer type {actual}"
                            ),
                        });
                    }
                }
            }
            OperatorExpressionResult::Boolean => {
                self.intern_type(dir::Type::Primitive(dir::PrimitiveType::Boolean))?
            }
        };

        Ok(result)
    }

    /// Read one operand node's normalized input type, resolved through solutions.
    pub(in crate::sema) fn operand_type(
        &mut self,
        origin: Origin,
        site: FlowSite,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let ty = self.infer_node_type(site, PlaceUse::Read)?;
        let resolved = self.deeply_resolve(origin, ty)?;
        let normalized = self.normalize(origin, resolved)?;

        Ok(normalized)
    }

    /// Return whether one operand expression folded into its committed type.
    fn expression_folds(&self, node: dir::GlobalNodeId<dir::Expression>) -> CompilerResult<bool> {
        // fold literal spellings directly
        let expression = self.module(node.module_id).view().get(node.local_id);
        if matches!(expression, dir::Expression::Literal(_)) {
            return Ok(true);
        }

        // read the fold recorded at operator selection
        Ok(match self.decision(node.into_any()) {
            Some(dir::Decision::Operator(resolution)) => resolution.is_folded(),
            _ => false,
        })
    }

    /// Commit one builtin binary operator selection.
    fn commit_builtin_binary_operator(
        &mut self,
        origin: Origin,
        node: dir::GlobalNodeIdAny,
        operator: dir::BinaryOperator,
        left: dir::BuiltinOperand,
        right: dir::BuiltinOperand,
        result: dir::GlobalTypeId,
        writeback: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<()> {
        let is_folded = matches!(self.ty(result)?, dir::Type::Literal(_))
            && self.expression_folds(left.source)?
            && self.expression_folds(right.source)?;
        let application = dir::OperatorApplication::Binary {
            operator,
            target: dir::OperatorTarget::Builtin([left, right]),
            ty: result,
            is_folded,
        };
        let resolution = dir::OperationResolution::One(application);
        self.check_operator_writeback(node, origin, result, writeback)?;
        self.commit_decision(node, dir::Decision::Operator(resolution))?;
        self.commit_node_type(node, result)?;

        Ok(())
    }

    /// Commit one builtin unary operator selection.
    fn commit_builtin_unary_operator(
        &mut self,
        node: dir::GlobalNodeIdAny,
        operator: dir::UnaryOperator,
        operand: dir::BuiltinOperand,
        result: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let is_folded = matches!(self.ty(result)?, dir::Type::Literal(_))
            && self.expression_folds(operand.source)?;
        let application = dir::OperatorApplication::Unary {
            operator,
            target: dir::OperatorTarget::Builtin(operand),
            ty: result,
            is_folded,
        };
        let resolution = dir::OperationResolution::One(application);
        self.commit_decision(node, dir::Decision::Operator(resolution))?;
        self.commit_node_type(node, result)?;

        Ok(())
    }

    /// Build one operator resolution from an exact dereference selection.
    fn dereference_operator_decision(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        operator: dir::UnaryOperator,
        resolution: dir::DereferenceResolution,
    ) -> CompilerResult<dir::OperatorDecision> {
        // build the operator decision from the selected dereference
        match resolution {
            dir::OperationResolution::One(dereference) => {
                let application =
                    self.dereference_operator_application(origin, source, operator, dereference)?;

                Ok(dir::OperationResolution::One(application))
            }
            dir::OperationResolution::Union { arms, ty } => {
                let mut applications = Vec::with_capacity(arms.len());
                for dereference in arms {
                    let application = self.dereference_operator_application(
                        origin,
                        source,
                        operator,
                        dereference,
                    )?;
                    applications.push(application);
                }

                Ok(dir::OperationResolution::Union {
                    arms: applications,
                    ty,
                })
            }
        }
    }

    /// Build one operator application from one dereference operation.
    fn dereference_operator_application(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        operator: dir::UnaryOperator,
        dereference: dir::Dereference,
    ) -> CompilerResult<dir::OperatorApplication> {
        // build the application the dereference names
        let application = match dereference.protocol {
            // built-in dereferences use the compiler-defined operator
            None => {
                let operand = self.builtin_operand(origin, source, dereference.receiver)?;

                dir::OperatorApplication::Unary {
                    operator,
                    target: dir::OperatorTarget::Builtin(operand),
                    ty: dereference.ty,
                    is_folded: false,
                }
            }

            // protocol-backed values keep the selected method call
            Some(call) => dir::OperatorApplication::Unary {
                operator,
                target: dir::OperatorTarget::Call(call),
                ty: dereference.ty,
                is_folded: false,
            },
        };

        Ok(application)
    }

    /// Commit one operator selection.
    fn commit_operator(
        &mut self,
        origin: Origin,
        node: dir::GlobalNodeIdAny,
        resolution: dir::OperatorDecision,
        result: dir::GlobalTypeId,
        writeback: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<()> {
        self.check_operator_writeback(node, origin, result, writeback)?;
        self.commit_decision(node, dir::Decision::Operator(resolution))?;
        self.commit_node_type(node, result)?;

        Ok(())
    }

    /// Check one compound assignment result against the assigned place.
    fn check_operator_writeback(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        source: dir::GlobalTypeId,
        writeback: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<()> {
        let Some(writeback) = writeback else {
            return Ok(());
        };

        // store the produced value back into the written place
        let site = self.visit_site(node)?;
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
        let expectation = Expectation {
            target: writeback,
            relation: Relation::Storable,
            cause,
            use_: ValueUse::Store,
            mode: InferMode::Regular,
            store: StoreTarget::Exact,
        };
        self.check_value(site, source, expectation)?;

        Ok(())
    }

    /// Report one rejected operator application with a diagnostic.
    pub(in crate::sema) fn report_rejected_operator(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        operator: String,
        operands: &[dir::GlobalTypeId],
        rejection: Option<SignatureRejection>,
    ) -> CompilerResult<()> {
        // poison an operand that already reported an error
        if self.has_error_operand(operands)? {
            self.poison_node(node)?;

            return Ok(());
        }

        // report the selected method's rejection, else the missing operator
        match rejection {
            Some(rejection) => {
                self.report_signature_rejection(origin, rejection)?;
            }
            None => {
                self.report_no_matching_operator(
                    origin,
                    operator,
                    OperatorOperands::Types(operands),
                )?;
            }
        }
        self.commit_decision(node, dir::Decision::Rejected)?;
        self.commit_error_node(node)?;

        Ok(())
    }
}
