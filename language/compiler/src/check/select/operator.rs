use destack_dir as dir;

use crate::check::{
    Answer, BodyState, Cause, CauseKind, CheckOutcome, Constraint, Decision, Expectation, FlowSite,
    InferMode, Obligation, OperatorExpressionResult, Origin, PlaceUse, Relation, ValueUse,
    WritableTargetObligation, answer, binary_operator_protocols, unary_operator_protocols,
};
use crate::{CompilerError, CompilerResult};

/// Operands displayed for a rejected operator.
pub(in crate::check) enum OperatorOperands<'a> {
    /// Type operands.
    Types(&'a [dir::GlobalTypeId]),
    /// Required place operand.
    Place,
}

impl BodyState<'_, '_> {
    /// Select one binary operator application.
    pub(in crate::check) fn select_binary_operator(
        &mut self,
        site: FlowSite,
        operator: dir::BinaryOperator,
        left_node: dir::LocalNodeId<dir::Expression>,
        right_node: dir::LocalNodeId<dir::Expression>,
        writeback: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<()>> {
        let module = site.node.module_id;
        let origin = site.origin();
        let left_site = self.node_site(left_node.into_global_any(module))?;
        let right_site = self.node_site(right_node.into_global_any(module))?;
        let left = answer!(self.operand_type(origin, left_site)?);
        let right = answer!(self.operand_type(origin, right_site)?);
        let left_source = left_node.into_global_any(module);
        let right_source = right_node.into_global_any(module);

        self.select_binary_operation(
            site,
            operator,
            left,
            right,
            left_source,
            right_source,
            writeback,
        )
    }

    /// Select one binary operation from known operand types.
    pub(in crate::check) fn select_binary_operation(
        &mut self,
        site: FlowSite,
        operator: dir::BinaryOperator,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
        left_source: dir::GlobalNodeIdAny,
        right_source: dir::GlobalNodeIdAny,
        writeback: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node;
        let origin = site.origin();

        // poison the node when either operand already reported an error
        if matches!(self.ty(left)?, dir::Type::Error) || matches!(self.ty(right)?, dir::Type::Error)
        {
            self.poison_node(node)?;

            return Ok(Answer::Ready(()));
        }

        // equality and logic produce builtin results directly
        let nullish_or_never = matches!(
            self.ty(left)?,
            dir::Type::Null | dir::Type::Undefined | dir::Type::Never
        ) || matches!(
            self.ty(right)?,
            dir::Type::Null | dir::Type::Undefined | dir::Type::Never
        );
        // scalar comparisons read values, so views compare their pointees
        let left_value = answer!(self.strip_form(origin, left)?);
        let right_value = answer!(self.strip_form(origin, right)?);
        let left_families = answer!(self.scalar_families(origin, left_value)?);
        let right_families = answer!(self.scalar_families(origin, right_value)?);
        let comparable = match (&left_families, &right_families) {
            (Some(left), Some(right)) => left.len() == 1 && left == right,
            _ => false,
        };
        let numeric = comparable
            && left_families
                .as_ref()
                .is_some_and(dir::ScalarFamilySet::is_numeric);

        let builtin = match operator {
            // require overlapping values with one common representation
            dir::BinaryOperator::EqualStrict | dir::BinaryOperator::NotEqualStrict => {
                let supported = answer!(self.supports_builtin_strict_equality(
                    origin,
                    left_value,
                    right_value,
                )?);
                if !supported {
                    return self.reject_operator(
                        node,
                        origin,
                        operator.text().to_string(),
                        &[left, right],
                    );
                }

                let overlaps = answer!(self.types_may_overlap(origin, left_value, right_value)?);
                if !overlaps {
                    self.report_invalid_strict_equality(origin, left, right)?;
                }

                // select the exact accepted type of each compared value
                let sources = [(left_source, left_value), (right_source, right_value)];
                let operands = answer!(self.select_strict_equality_operands(origin, &sources)?);
                let [left_operand, right_operand]: [dir::BuiltinOperand; 2] =
                    operands.try_into().map_err(|_| CompilerError::Internal {
                        message: "binary strict equality did not select two operands".to_string(),
                    })?;
                let result = self.intern_type(dir::Type::Primitive(dir::PrimitiveType::Boolean))?;

                return self.commit_builtin_binary_operator(
                    origin,
                    node,
                    operator,
                    left_operand,
                    right_operand,
                    result,
                    writeback,
                );
            }
            // nullish and same-kind scalar equality produce booleans
            dir::BinaryOperator::Equal | dir::BinaryOperator::NotEqual
                if nullish_or_never || (comparable && !numeric) =>
            {
                let result = self.intern_type(dir::Type::Primitive(dir::PrimitiveType::Boolean))?;

                Some((result, left_value, right_value))
            }
            // logical joins produce the union of their operands
            dir::BinaryOperator::And | dir::BinaryOperator::Or => {
                Some((self.normalized_union_type([left, right])?, left, right))
            }
            // try-coalesce opens the carrier and joins the alternate
            dir::BinaryOperator::Coalesce => {
                let output = answer!(self.reduce_operation_type(
                    origin,
                    dir::TypeOperation::TryOutput { value: left },
                )?);

                Some((self.normalized_union_type([output, right])?, left, right))
            }
            _ => None,
        };
        if let Some((result, left_target, right_target)) = builtin {
            answer!(self.check_builtin_operand(left_source, left, left_target)?);
            answer!(self.check_builtin_operand(right_source, right, right_target)?);
            let left = answer!(self.builtin_operand(origin, left_source, left_target)?);
            let right = answer!(self.builtin_operand(origin, right_source, right_target)?);

            return self.commit_builtin_binary_operator(
                origin, node, operator, left, right, result, writeback,
            );
        }

        // check builtin numeric operands against their joined type
        if let Some((result, operands)) =
            answer!(self.builtin_numeric_result(origin, operator, left, right)?)
        {
            // operands check as arguments of the builtin operation
            answer!(self.check_builtin_operand(left_source, left, operands[0])?);
            answer!(self.check_builtin_operand(right_source, right, operands[1])?);
            let left = answer!(self.builtin_operand(origin, left_source, operands[0])?);
            let right = answer!(self.builtin_operand(origin, right_source, operands[1])?);

            return self.commit_builtin_binary_operator(
                origin, node, operator, left, right, result, writeback,
            );
        }

        // dispatch through the operator protocol interfaces
        let left_site = self.node_site(left_source)?;
        let left_value = answer!(self.expression_value(left_site, left)?);
        let protocols = binary_operator_protocols(operator);
        for protocol in protocols {
            let key = protocol.method.key(self.strings());
            let protocol_type = self.operator_protocol(origin, &protocol, &[right])?;
            let argument_sources = [dir::ArgumentSource::Provided(right_source)];

            let Some(call) = answer!(self.select_protocol_call(
                origin,
                left_value,
                left,
                dir::MemberSpace::Instance,
                key,
                &protocol_type,
                &argument_sources,
            )?) else {
                continue;
            };
            let result = answer!(self.operator_expression_type(
                origin,
                protocol.expression_result,
                call.return_type,
            )?);

            let resolution = match call.resolution {
                dir::OperationResolution::One(call) => {
                    let application = dir::OperatorApplication::Binary {
                        operator,
                        target: dir::OperatorTarget::Call(Box::new(call)),
                        ty: result,
                    };

                    dir::OperationResolution::One(application)
                }
                dir::OperationResolution::Union { arms, .. } => {
                    let mut applications = Vec::with_capacity(arms.len());
                    for call in arms {
                        let ty = answer!(self.operator_expression_type(
                            origin,
                            protocol.expression_result,
                            call.return_type,
                        )?);
                        applications.push(dir::OperatorApplication::Binary {
                            operator,
                            target: dir::OperatorTarget::Call(Box::new(call)),
                            ty,
                        });
                    }

                    dir::OperationResolution::Union {
                        arms: applications,
                        ty: result,
                    }
                }
            };

            return self.commit_operator(origin, node, resolution, result, writeback);
        }

        self.reject_operator(node, origin, operator.text().to_string(), &[left, right])
    }

    /// Select builtin strict equality for one switch and all its cases.
    pub(in crate::check) fn select_switch_equality(
        &mut self,
        value_source: dir::GlobalNodeIdAny,
        scrutinee: dir::GlobalTypeId,
        cases: &[(
            dir::GlobalNodeIdAny,
            dir::GlobalNodeIdAny,
            dir::GlobalTypeId,
        )],
    ) -> CompilerResult<Answer<()>> {
        let value_site = self.node_site(value_source)?;
        let origin = value_site.origin();
        let scrutinee_value = answer!(self.strip_form(origin, scrutinee)?);
        let mut selected = Vec::with_capacity(cases.len());

        // reject cases outside builtin strict equality
        for (case, selector, ty) in cases {
            let case_site = self.node_site(*case)?;
            let selector_site = self.node_site(*selector)?;
            let selector_value = answer!(self.strip_form(selector_site.origin(), *ty)?);
            let supported = answer!(self.supports_builtin_strict_equality(
                selector_site.origin(),
                scrutinee_value,
                selector_value,
            )?);
            if !supported {
                answer!(self.reject_operator(
                    *case,
                    case_site.origin(),
                    dir::BinaryOperator::EqualStrict.text().to_string(),
                    &[scrutinee, *ty],
                )?);
                continue;
            }

            selected.push((*case, *selector, *ty, selector_value));
        }
        if selected.is_empty() {
            return Ok(Answer::Ready(()));
        }

        // select the exact accepted type of the scrutinee and every admitted case
        let mut sources = Vec::with_capacity(selected.len() + 1);
        sources.push((value_source, scrutinee_value));
        sources.extend(
            selected
                .iter()
                .map(|(_, selector, _, value)| (*selector, *value)),
        );
        let operands = answer!(self.select_strict_equality_operands(origin, &sources)?);
        let Some((scrutinee_operand, selector_operands)) = operands.split_first() else {
            return Err(CompilerError::Internal {
                message: "switch equality did not select its scrutinee".to_string(),
            });
        };

        // diagnose pairwise disjoint cases and record their builtin operation
        let boolean = self.intern_type(dir::Type::Primitive(dir::PrimitiveType::Boolean))?;
        for ((case, _, ty, selector_value), selector_operand) in
            selected.into_iter().zip(selector_operands)
        {
            let overlaps =
                answer!(self.types_may_overlap(origin, scrutinee_value, selector_value,)?);
            if !overlaps {
                self.report_invalid_strict_equality(origin, scrutinee, ty)?;
            }
            answer!(self.commit_builtin_binary_operator(
                origin,
                case,
                dir::BinaryOperator::EqualStrict,
                scrutinee_operand.clone(),
                selector_operand.clone(),
                boolean,
                None,
            )?);
        }

        Ok(Answer::Ready(()))
    }

    /// Select one unary operator application.
    pub(in crate::check) fn select_unary_operator(
        &mut self,
        site: FlowSite,
        operator: dir::UnaryOperator,
        operand_node: dir::LocalNodeId<dir::Expression>,
        use_: PlaceUse,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = site.origin();
        let operand_site = self.node_site(operand_node.into_global_any(module))?;

        // increments rewrite builtin numeric places by one
        if matches!(
            operator,
            dir::UnaryOperator::PostIncrement
                | dir::UnaryOperator::PostDecrement
                | dir::UnaryOperator::PreIncrement
                | dir::UnaryOperator::PreDecrement
        ) {
            let Some(place) =
                answer!(self.select_assignment(operand_site, operand_node, PlaceUse::Update)?)
            else {
                self.report_no_matching_operator(
                    origin,
                    operator.text().to_string(),
                    OperatorOperands::Place,
                )?;
                self.commit_decision(node, Decision::Rejected)?;
                self.commit_error_node(node)?;

                return Ok(Answer::Ready(()));
            };
            let operand = place
                .read
                .as_ref()
                .map(dir::ReadResolution::ty)
                .ok_or_else(|| CompilerError::Internal {
                    message: "updated place has no readable type".to_string(),
                })?;
            let write_type = place.write.ty();

            if answer!(self.operand_is_numeric(origin, operand)?) {
                let source = place.source;
                let resolution = place.clone().resolution();
                self.commit_node_type(source, operand)?;
                self.commit_decision(source, Decision::Assignment(resolution))?;
                let scope = self.origin_scope(origin)?;
                self.push_obligation(
                    Obligation::WritableTarget(Box::new(WritableTargetObligation {
                        target: place,
                        ty: operand,
                    })),
                    scope,
                );
                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
                self.push_constraint(Constraint::r#type(
                    origin,
                    Relation::Assignable,
                    operand,
                    write_type,
                    cause,
                ));
                let result = operand;
                let operand = answer!(self.builtin_operand(origin, source, operand)?);

                return self.commit_builtin_unary_operator(node, operator, operand, result);
            }

            return self.reject_operator(node, origin, operator.text().to_string(), &[operand]);
        }

        let operand = answer!(self.operand_type(origin, operand_site)?);

        // poison the node when the operand already reported an error
        if matches!(self.ty(operand)?, dir::Type::Error) {
            self.poison_node(node)?;

            return Ok(Answer::Ready(()));
        }

        // builtin logical not produces a boolean
        if matches!(operator, dir::UnaryOperator::Not) {
            let result = self.intern_type(dir::Type::Primitive(dir::PrimitiveType::Boolean))?;
            let operand = answer!(self.builtin_operand(origin, operand_site.node, operand)?);

            return self.commit_builtin_unary_operator(node, operator, operand, result);
        }

        // builtin numeric negation reduces singleton operands
        if matches!(
            operator,
            dir::UnaryOperator::Negate | dir::UnaryOperator::Plus
        ) && answer!(self.operand_is_numeric(origin, operand)?)
        {
            let result = answer!(self.builtin_unary_result(origin, operator, operand)?);
            let operand = answer!(self.builtin_operand(origin, operand_site.node, operand)?);

            return self.commit_builtin_unary_operator(node, operator, operand, result);
        }

        // builtin bitwise not moves bits through integers
        if matches!(operator, dir::UnaryOperator::ElementwiseNot)
            && answer!(self.operand_is_integral(origin, operand)?)
        {
            let result = answer!(self.builtin_unary_result(origin, operator, operand)?);
            let operand = answer!(self.builtin_operand(origin, operand_site.node, operand)?);

            return self.commit_builtin_unary_operator(node, operator, operand, result);
        }

        // dereferences select either a direct projection or protocol call
        let access = match use_ {
            PlaceUse::Read => dir::Access::Readonly,
            PlaceUse::Write | PlaceUse::Update => dir::Access::Mutable,
        };
        if operator == dir::UnaryOperator::Dereference {
            let operand_value = answer!(self.expression_value(operand_site, operand)?);
            let Some(selection) =
                answer!(self.select_dereference(origin, operand_value, access)?)
            else {
                return self.reject_operator(node, origin, operator.text().to_string(), &[operand]);
            };
            let result = selection.ty();
            let resolution = answer!(self.dereference_operator_resolution(
                origin,
                operand_site.node,
                operator,
                selection,
            )?);

            return self.commit_operator(origin, node, resolution, result, None);
        }

        // dispatch through the operator protocol interfaces
        let operand_value = answer!(self.expression_value(operand_site, operand)?);
        let protocols = unary_operator_protocols(operator, access);
        for protocol in protocols {
            let key = protocol.method.key(self.strings());
            let protocol_type = self.operator_protocol(origin, &protocol, &[])?;
            let Some(call) = answer!(self.select_protocol_call(
                origin,
                operand_value,
                operand,
                dir::MemberSpace::Instance,
                key,
                &protocol_type,
                &[],
            )?) else {
                continue;
            };
            let result = answer!(self.operator_expression_type(
                origin,
                protocol.expression_result,
                call.return_type,
            )?);

            let resolution = match call.resolution {
                dir::OperationResolution::One(call) => {
                    let application = dir::OperatorApplication::Unary {
                        operator,
                        target: dir::OperatorTarget::Call(Box::new(call)),
                        ty: result,
                    };

                    dir::OperationResolution::One(application)
                }
                dir::OperationResolution::Union { arms, .. } => {
                    let mut applications = Vec::with_capacity(arms.len());
                    for call in arms {
                        let ty = answer!(self.operator_expression_type(
                            origin,
                            protocol.expression_result,
                            call.return_type,
                        )?);
                        applications.push(dir::OperatorApplication::Unary {
                            operator,
                            target: dir::OperatorTarget::Call(Box::new(call)),
                            ty,
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

        self.reject_operator(node, origin, operator.text().to_string(), &[operand])
    }

    /// Return whether one operand holds only builtin numerics.
    fn operand_is_numeric(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let families = answer!(self.builtin_scalar_families(origin, ty)?);

        Ok(Answer::Ready(
            families.is_some_and(|families| families.is_numeric()),
        ))
    }

    /// Return whether one operand holds only builtin integers.
    fn operand_is_integral(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let families = answer!(self.builtin_scalar_families(origin, ty)?);

        Ok(Answer::Ready(
            families.is_some_and(|families| families.is_integral()),
        ))
    }

    /// Select the accepted operands for builtin strict equality.
    fn select_strict_equality_operands(
        &mut self,
        origin: Origin,
        sources: &[(dir::GlobalNodeIdAny, dir::GlobalTypeId)],
    ) -> CompilerResult<Answer<Vec<dir::BuiltinOperand>>> {
        let operands = sources.iter().map(|(_, ty)| *ty).collect::<Vec<_>>();
        let Some((first, rest)) = operands.split_first() else {
            return Err(CompilerError::Internal {
                message: "strict equality requires at least one operand".to_string(),
            });
        };

        // select one common type when equality requires a shared representation
        let mut numeric = true;
        for operand in &operands {
            numeric &= answer!(self.operand_is_numeric(origin, *operand)?);
        }
        let mut has_union = false;
        if !numeric {
            for operand in &operands {
                has_union |= matches!(self.ty(*operand)?, dir::Type::Union(_));
            }
        }
        let common = if numeric {
            let mut carrier = *first;
            for operand in rest {
                let Some(joined) = answer!(self.builtin_numeric_join(origin, carrier, *operand)?)
                else {
                    return Err(CompilerError::Internal {
                        message: "comparable numeric operands have no common type".to_string(),
                    });
                };
                carrier = joined;
            }

            Some(carrier)
        } else if has_union {
            Some(self.normalized_union_type(operands.iter().copied())?)
        } else {
            let mut is_equal = true;
            for operand in rest {
                is_equal &=
                    answer!(self.decide_relation(origin, Relation::Equal, *first, *operand,)?);
            }

            is_equal.then_some(*first)
        };

        // record each operand with the exact type accepted by the operation
        let mut selected = Vec::with_capacity(sources.len());
        for (source, source_type) in sources {
            let target = common.unwrap_or(*source_type);
            answer!(self.check_builtin_operand(*source, *source_type, target)?);
            selected.push(answer!(self.builtin_operand(origin, *source, target)?));
        }

        Ok(Answer::Ready(selected))
    }

    /// Return the builtin numeric result and selected operand types.
    fn builtin_numeric_result(
        &mut self,
        origin: Origin,
        operator: dir::BinaryOperator,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<(dir::GlobalTypeId, [dir::GlobalTypeId; 2])>>> {
        // classify both operands once
        let numeric = answer!(self.operand_is_numeric(origin, left)?)
            && answer!(self.operand_is_numeric(origin, right)?);
        let integral = numeric
            && answer!(self.operand_is_integral(origin, left)?)
            && answer!(self.operand_is_integral(origin, right)?);

        // literal operands are static operations: the comptime
        //  reduction folds them exactly and reports overflow, so the
        //  result flows by representability like any written literal
        if numeric
            && matches!(
                (self.ty(left)?, self.ty(right)?),
                (dir::Type::Literal(_), dir::Type::Literal(_))
            )
            && let Ok(static_operator) = dir::StaticBinaryOperator::try_from(operator)
            && !static_operator.yields_boolean()
        {
            let operation =
                self.intern_operation(dir::TypeOperation::StaticBinary(dir::StaticBinaryType {
                    operator: static_operator,
                    left,
                    right,
                }))?;
            let folded = answer!(self.reduce_type_head(origin, operation)?);
            if matches!(self.ty(folded)?, dir::Type::Literal(_)) {
                return Ok(Answer::Ready(Some((folded, [left, right]))));
            }
        }

        match operator {
            // shifts move bits through integers, keeping the left type
            dir::BinaryOperator::ShiftLeft
            | dir::BinaryOperator::ShiftRight
            | dir::BinaryOperator::UnsignedShiftRight
                if integral =>
            {
                let result = match self.ty(left)? {
                    // typed shifts keep the left operand's integer type
                    dir::Type::Literal(dir::ScalarLiteral::Bigint(_)) => {
                        self.intern_type(dir::Type::Primitive(dir::PrimitiveType::Bigint))?
                    }
                    dir::Type::Literal(_) => self.intern_type(dir::Type::Primitive(
                        dir::PrimitiveType::Integer(dir::IntegerType::Fixed {
                            width: 64,
                            is_signed: true,
                        }),
                    ))?,
                    _ => left,
                };

                // constant shift amounts must stay below the shifted width
                if let dir::Type::Primitive(dir::PrimitiveType::Integer(dir::IntegerType::Fixed {
                    width,
                    ..
                })) = self.ty(result)?
                    && let dir::Type::Literal(dir::ScalarLiteral::Integer(amount)) =
                        self.ty(right)?
                    && (amount < 0 || amount >= i64::from(width))
                {
                    self.report_shift_out_of_range(origin, amount, result)?;
                }

                Ok(Answer::Ready(Some((result, [result, result]))))
            }
            // elementwise bit operations join equal integer operands
            dir::BinaryOperator::ElementwiseAnd
            | dir::BinaryOperator::ElementwiseOr
            | dir::BinaryOperator::ElementwiseXor
                if integral =>
            {
                let joined = answer!(self.builtin_numeric_join(origin, left, right)?);

                Ok(Answer::Ready(
                    joined.map(|joined| (joined, [joined, joined])),
                ))
            }
            // arithmetic joins equal numeric operands
            dir::BinaryOperator::Add
            | dir::BinaryOperator::Subtract
            | dir::BinaryOperator::Multiply
            | dir::BinaryOperator::Divide
            | dir::BinaryOperator::Remainder
            | dir::BinaryOperator::Exponent
                if numeric =>
            {
                let joined = answer!(self.builtin_numeric_join(origin, left, right)?);

                Ok(Answer::Ready(
                    joined.map(|joined| (joined, [joined, joined])),
                ))
            }
            // comparisons produce booleans over the joined operand type
            dir::BinaryOperator::Equal
            | dir::BinaryOperator::NotEqual
            | dir::BinaryOperator::LessThan
            | dir::BinaryOperator::LessThanOrEqual
            | dir::BinaryOperator::GreaterThan
            | dir::BinaryOperator::GreaterThanOrEqual
                if numeric =>
            {
                let Some(joined) = answer!(self.builtin_numeric_join(origin, left, right)?) else {
                    return Ok(Answer::Ready(None));
                };
                let boolean =
                    self.intern_type(dir::Type::Primitive(dir::PrimitiveType::Boolean))?;

                Ok(Answer::Ready(Some((boolean, [joined, joined]))))
            }
            _ => Ok(Answer::Ready(None)),
        }
    }

    /// Check one operand as an argument of the selected builtin operation.
    fn check_builtin_operand(
        &mut self,
        source: dir::GlobalNodeIdAny,
        source_type: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        // comptime-folded operations have no runtime operands
        if matches!(self.ty(target)?, dir::Type::Literal(_)) {
            return Ok(Answer::Ready(()));
        }

        let operand_site = self.node_site(source)?;

        // parametric literal adaptation belongs only to the selected builtin
        let target_root = answer!(self.reduce_type_head(operand_site.origin(), target)?);
        if matches!(self.ty(source_type)?, dir::Type::Literal(_))
            && matches!(self.ty(target_root)?, dir::Type::Parameter(_))
        {
            let accepts = answer!(self.builtin_scalar_accepts_literal(
                operand_site.origin(),
                source_type,
                target_root,
            )?);
            if !accepts {
                return Err(CompilerError::Internal {
                    message: "selected builtin operation rejects its literal operand".to_string(),
                });
            }

            return Ok(Answer::Ready(()));
        }

        // concrete operands retain the ordinary checked value relation
        let cause = self.intern_cause(Cause::root(operand_site.origin(), CauseKind::Expression));

        let expectation = Expectation {
            target,
            relation: Relation::Assignable,
            cause,
            use_: ValueUse::Operand,
            mode: InferMode::Exact,
        };
        answer!(self.check_value(operand_site, source_type, expectation)?);

        Ok(Answer::Ready(()))
    }

    /// Build one checked builtin operand.
    fn builtin_operand(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::BuiltinOperand>> {
        let scalar_families = answer!(self.scalar_families(origin, ty)?);
        let source = source
            .try_into_typed::<dir::Expression>()
            .map_err(|message| CompilerError::Internal { message })?;
        let operand = dir::BuiltinOperand {
            source,
            ty,
            scalar_families,
        };

        Ok(Answer::Ready(operand))
    }

    /// Return one builtin unary result.
    fn builtin_unary_result(
        &mut self,
        origin: Origin,
        operator: dir::UnaryOperator,
        operand: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        // fold only literal operands
        let reduced = answer!(self.reduce_type_head(origin, operand)?);
        if !matches!(self.ty(reduced)?, dir::Type::Literal(_)) {
            return Ok(Answer::Ready(operand));
        }
        let Ok(operator) = dir::StaticUnaryOperator::try_from(operator) else {
            return Ok(Answer::Ready(operand));
        };
        let operation = dir::TypeOperation::StaticUnary(dir::StaticUnaryType {
            operator,
            target: operand,
        });
        let result = answer!(self.reduce_operation_type(origin, operation)?);

        Ok(Answer::Ready(result))
    }

    /// Return one interval operand widened to its base scalar.
    fn interval_operand_base(
        &mut self,
        _origin: Origin,
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

    /// Join two builtin numeric operands into one common operand type.
    fn builtin_numeric_join(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        // interval operands widen to their base scalar under arithmetic
        let left = self.interval_operand_base(origin, left)?;
        let right = self.interval_operand_base(origin, right)?;

        // literals adapt into the other operand's type
        let left_literal = match self.ty(left)? {
            dir::Type::Literal(literal) => Some(literal),
            _ => None,
        };
        let right_literal = match self.ty(right)? {
            dir::Type::Literal(literal) => Some(literal),
            _ => None,
        };

        match (left_literal, right_literal) {
            // literal pairs widen to their base numeric type
            (Some(left), Some(_)) => {
                let widened = self.intern_type(left.widen())?;

                Ok(Answer::Ready(Some(widened)))
            }
            (Some(_), None) => {
                let adapts = match self.ty(right)? {
                    dir::Type::Parameter(_) => {
                        self.builtin_scalar_accepts_literal(origin, left, right)?
                    }
                    _ => self.decide_relation(origin, Relation::Assignable, left, right)?,
                };

                Ok(adapts.then_some(right))
            }
            (None, Some(_)) => {
                let adapts = match self.ty(left)? {
                    dir::Type::Parameter(_) => {
                        self.builtin_scalar_accepts_literal(origin, right, left)?
                    }
                    _ => self.decide_relation(origin, Relation::Assignable, right, left)?,
                };

                Ok(adapts.then_some(left))
            }
            // typed operands must agree exactly
            (None, None) => {
                let equal = self.decide_relation(origin, Relation::Equal, left, right)?;

                Ok(equal.then_some(left))
            }
        }
    }

    /// Return the expression result for one selected operator method.
    pub(in crate::check) fn operator_expression_type(
        &mut self,
        origin: Origin,
        expression_result: OperatorExpressionResult,
        return_type: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        // produce the protocol's declared result
        let result = match expression_result {
            OperatorExpressionResult::MethodReturn => return_type,
            // project the place behind the returned borrow
            OperatorExpressionResult::Pointee => {
                let reduced = answer!(self.reduce_type_head(origin, return_type)?);

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
                let _module = origin.module();

                self.intern_type(dir::Type::Primitive(dir::PrimitiveType::Boolean))?
            }
        };

        Ok(Answer::Ready(result))
    }

    /// Read one operand node's reduced input type.
    fn operand_type(
        &mut self,
        origin: Origin,
        site: FlowSite,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let ty = answer!(self.infer_node_type(site, PlaceUse::Read)?);

        self.reduce_type_head(origin, ty)
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
    ) -> CompilerResult<Answer<()>> {
        let application = dir::OperatorApplication::Binary {
            operator,
            target: dir::OperatorTarget::Builtin([left, right]),
            ty: result,
        };
        let resolution = dir::OperationResolution::One(application);
        answer!(self.check_operator_writeback(origin, result, writeback)?);
        self.commit_decision(node, Decision::Operator(resolution))?;
        self.commit_node_type(node, result)?;

        Ok(Answer::Ready(()))
    }

    /// Commit one builtin unary operator selection.
    fn commit_builtin_unary_operator(
        &mut self,
        node: dir::GlobalNodeIdAny,
        operator: dir::UnaryOperator,
        operand: dir::BuiltinOperand,
        result: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        let application = dir::OperatorApplication::Unary {
            operator,
            target: dir::OperatorTarget::Builtin(operand),
            ty: result,
        };
        let resolution = dir::OperationResolution::One(application);
        self.commit_decision(node, Decision::Operator(resolution))?;
        self.commit_node_type(node, result)?;

        Ok(Answer::Ready(()))
    }

    /// Build one operator resolution from an exact dereference selection.
    fn dereference_operator_resolution(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        operator: dir::UnaryOperator,
        resolution: dir::DereferenceResolution,
    ) -> CompilerResult<Answer<dir::OperatorResolution>> {
        match resolution {
            dir::OperationResolution::One(dereference) => {
                let application = answer!(self.dereference_operator_application(
                    origin,
                    source,
                    operator,
                    dereference,
                )?);

                Ok(Answer::Ready(dir::OperationResolution::One(application)))
            }
            dir::OperationResolution::Union { arms, ty } => {
                let mut applications = Vec::with_capacity(arms.len());
                for dereference in arms {
                    let application = answer!(self.dereference_operator_application(
                        origin,
                        source,
                        operator,
                        dereference,
                    )?);
                    applications.push(application);
                }

                Ok(Answer::Ready(dir::OperationResolution::Union {
                    arms: applications,
                    ty,
                }))
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
    ) -> CompilerResult<Answer<dir::OperatorApplication>> {
        let application = match dereference.target {
            // direct pointer forms use the compiler-defined operator
            dir::DereferenceTarget::Direct => {
                let operand =
                    answer!(self.builtin_operand(origin, source, dereference.receiver)?);

                dir::OperatorApplication::Unary {
                    operator,
                    target: dir::OperatorTarget::Builtin(operand),
                    ty: dereference.ty,
                }
            }

            // protocol-backed values retain the selected method call
            dir::DereferenceTarget::Call(call) => dir::OperatorApplication::Unary {
                operator,
                target: dir::OperatorTarget::Call(call),
                ty: dereference.ty,
            },
        };

        Ok(Answer::Ready(application))
    }

    /// Commit one operator selection.
    fn commit_operator(
        &mut self,
        origin: Origin,
        node: dir::GlobalNodeIdAny,
        resolution: dir::OperatorResolution,
        result: dir::GlobalTypeId,
        writeback: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<()>> {
        answer!(self.check_operator_writeback(origin, result, writeback)?);
        self.commit_decision(node, Decision::Operator(resolution))?;
        self.commit_node_type(node, result)?;

        Ok(Answer::Ready(()))
    }

    /// Check one compound assignment result against the assigned place.
    fn check_operator_writeback(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        writeback: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<()>> {
        let Some(writeback) = writeback else {
            return Ok(Answer::Ready(()));
        };

        // require the produced value to store into the written place
        let relation = Relation::Assignable;
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
        let holds = answer!(self.constrain_type(origin, cause, relation, source, writeback)?);
        let outcome =
            answer!(self.complete_constraint_check(origin, relation, source, writeback, holds)?);

        // record the failure against the store site
        if let CheckOutcome::Fails(failure) = outcome {
            self.check.record_failure(
                cause,
                relation,
                Some(ValueUse::Store),
                source,
                writeback,
                failure,
            );
        }

        Ok(Answer::Ready(()))
    }

    /// Reject one operator application with a diagnostic.
    pub(in crate::check) fn reject_operator(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        operator: String,
        operands: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<()>> {
        // poison instead of reporting again when an operand already reported an error
        if self.any_error_operand(operands)? {
            self.poison_node(node)?;

            return Ok(Answer::Ready(()));
        }
        self.report_no_matching_operator(origin, operator, OperatorOperands::Types(operands))?;
        self.commit_decision(node, Decision::Rejected)?;
        self.commit_error_node(node)?;

        Ok(Answer::Ready(()))
    }
}
