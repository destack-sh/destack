use destack_dir as dir;

use crate::check::{
    Answer, BodyState, Cause, CauseKind, Constraint, Decision, FlowSite, Obligation,
    OperatorExpressionResult, Origin, PlaceUse, Relation, ValueUse, WritablePlaceObligation,
    answer, binary_operator_protocols, unary_operator_protocols,
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
        let right_source = right_node.into_global_any(module);

        self.select_binary_operation(site, operator, left, right, right_source, writeback)
    }

    /// Select one binary operation from known operand types.
    pub(in crate::check) fn select_binary_operation(
        &mut self,
        site: FlowSite,
        operator: dir::BinaryOperator,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
        right_source: dir::GlobalNodeIdAny,
        writeback: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = site.origin();

        // identity and logic produce builtin results directly
        let nullish_or_never = matches!(
            self.ty(left)?,
            dir::Type::Null | dir::Type::Undefined | dir::Type::Never
        ) || matches!(
            self.ty(right)?,
            dir::Type::Null | dir::Type::Undefined | dir::Type::Never
        );
        let comparable = match (
            answer!(self.scalar_families(origin, left)?),
            answer!(self.scalar_families(origin, right)?),
        ) {
            (Some(left), Some(right)) => left.len() == 1 && left == right,
            _ => false,
        };
        let builtin = match operator {
            // strict identity always produces a boolean; equality
            //  reads values, so views compare their pointees
            dir::BinaryOperator::EqualStrict | dir::BinaryOperator::NotEqualStrict => {
                let left_value = self.value_beneath_forms(origin, left)?;
                let right_value = self.value_beneath_forms(origin, right)?;
                if !answer!(self.types_may_overlap(origin, left_value, right_value)?) {
                    self.report_invalid_strict_equality(origin, left, right)?;
                }

                Some(self.intern_type(module, dir::Type::Primitive(dir::PrimitiveType::Boolean))?)
            }
            // nullish and same-kind scalar equality produce booleans
            dir::BinaryOperator::Equal | dir::BinaryOperator::NotEqual
                if nullish_or_never || comparable =>
            {
                Some(self.intern_type(module, dir::Type::Primitive(dir::PrimitiveType::Boolean))?)
            }
            // logical joins produce the union of their operands
            dir::BinaryOperator::And | dir::BinaryOperator::Or => {
                Some(self.normalized_union_type(module, [left, right])?)
            }
            // try-coalesce opens the carrier and joins the alternate
            dir::BinaryOperator::Coalesce => {
                let output = answer!(self.reduce_operation_type(
                    origin,
                    dir::TypeOperation::TryOutput { value: left },
                )?);

                Some(self.normalized_union_type(module, [output, right])?)
            }
            _ => None,
        };
        if let Some(result) = builtin {
            return self.commit_builtin_operator(origin, node, operator, result, writeback);
        }

        // check builtin numeric operands against their joined type
        if let Some((result, _)) =
            answer!(self.builtin_numeric_result(origin, operator, left, right)?)
        {
            return self.commit_builtin_operator(origin, node, operator, result, writeback);
        }

        // dispatch through the operator protocol interfaces
        let protocols = binary_operator_protocols(operator);
        for protocol in protocols {
            let key = protocol.method.key(self.strings());
            let protocol_type = self.operator_protocol(origin, &protocol, &[right])?;
            let argument_sources = [dir::ArgumentSource::Provided(right_source)];

            let Some(call) = answer!(self.select_protocol_call(
                origin,
                left,
                left,
                key,
                &protocol_type,
                &[right],
                &argument_sources,
            )?) else {
                continue;
            };
            let result = answer!(self.operator_expression_type(
                origin,
                protocol.expression_result,
                call.return_type,
            )?);

            return self.commit_protocol_operator(origin, node, call.resolution, result, writeback);
        }

        self.reject_operator(node, origin, operator.text().to_string(), &[left, right])
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
                answer!(self.select_assign_place(operand_site, operand_node, PlaceUse::Update)?)
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
            let operand = place.ty;

            if answer!(self.operand_is_numeric(origin, operand)?) {
                let resolution = place.clone().resolution();
                self.commit_node_type(place.source, operand)?;
                self.commit_decision(place.source, Decision::Place(resolution))?;
                let scope = self.origin_scope(origin)?;
                self.push_obligation(
                    Obligation::WritablePlace(Box::new(WritablePlaceObligation {
                        place,
                        ty: operand,
                    })),
                    scope,
                );
                self.commit_node_type(node, operand)?;

                return Ok(Answer::Ready(()));
            }

            return self.reject_operator(node, origin, operator.text().to_string(), &[operand]);
        }

        let operand = answer!(self.operand_type(origin, operand_site)?);

        // builtin logical not produces a boolean
        if matches!(operator, dir::UnaryOperator::Not) {
            let result =
                self.intern_type(module, dir::Type::Primitive(dir::PrimitiveType::Boolean))?;

            return self.commit_builtin_unary_operator(node, operator, result);
        }

        // builtin numeric negation reduces singleton operands
        if matches!(
            operator,
            dir::UnaryOperator::Negate | dir::UnaryOperator::Plus
        ) && answer!(self.operand_is_numeric(origin, operand)?)
        {
            let result = answer!(self.builtin_unary_result(origin, operator, operand)?);

            return self.commit_builtin_unary_operator(node, operator, result);
        }

        // builtin bitwise not moves bits through integers
        if matches!(operator, dir::UnaryOperator::ElementwiseNot)
            && answer!(self.operand_is_integral(origin, operand)?)
        {
            let result = answer!(self.builtin_unary_result(origin, operator, operand)?);

            return self.commit_builtin_unary_operator(node, operator, result);
        }

        // dereferences select either a direct projection or protocol call
        let access = match use_ {
            PlaceUse::Read => dir::Access::Readonly,
            PlaceUse::Write | PlaceUse::Update => dir::Access::Mutable,
        };
        if operator == dir::UnaryOperator::Dereference {
            let Some(selection) = answer!(self.select_dereference(origin, operand, access)?) else {
                return self.reject_operator(node, origin, operator.text().to_string(), &[operand]);
            };

            return match selection.operation {
                dir::DereferenceOperation::Direct => {
                    self.commit_builtin_unary_operator(node, operator, selection.ty)
                }
                dir::DereferenceOperation::Call(call) => {
                    self.commit_protocol_operator(origin, node, call, selection.ty, None)
                }
            };
        }

        // dispatch through the operator protocol interfaces
        let protocols = unary_operator_protocols(operator, access);
        for protocol in protocols {
            let key = protocol.method.key(self.strings());
            let protocol_type = self.operator_protocol(origin, &protocol, &[])?;
            let Some(call) = answer!(self.select_protocol_call(
                origin,
                operand,
                operand,
                key,
                &protocol_type,
                &[],
                &[],
            )?) else {
                continue;
            };
            let result = answer!(self.operator_expression_type(
                origin,
                protocol.expression_result,
                call.return_type,
            )?);

            return self.commit_protocol_operator(origin, node, call.resolution, result, None);
        }

        self.reject_operator(node, origin, operator.text().to_string(), &[operand])
    }

    /// Return whether one operand holds only builtin numerics.
    fn operand_is_numeric(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let families = answer!(self.scalar_families(origin, ty)?);

        Ok(Answer::Ready(families.is_some_and(|families| {
            !families.is_empty() && families.iter().all(|family| family.is_numeric())
        })))
    }

    /// Return whether one operand holds only builtin integers.
    fn operand_is_integral(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let families = answer!(self.scalar_families(origin, ty)?);

        Ok(Answer::Ready(families.is_some_and(|families| {
            !families.is_empty() && families.iter().all(|family| family.is_integral())
        })))
    }

    /// Return the builtin numeric result and joined operand type.
    fn builtin_numeric_result(
        &mut self,
        origin: Origin,
        operator: dir::BinaryOperator,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<(dir::GlobalTypeId, dir::GlobalTypeId)>>> {
        // classify both operands once
        let numeric = answer!(self.operand_is_numeric(origin, left)?)
            && answer!(self.operand_is_numeric(origin, right)?);
        let integral = numeric
            && answer!(self.operand_is_integral(origin, left)?)
            && answer!(self.operand_is_integral(origin, right)?);
        let module = origin.module();

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
            let operation = self.intern_operation(
                module,
                dir::TypeOperation::StaticBinary(dir::StaticBinaryType {
                    operator: static_operator,
                    left,
                    right,
                }),
            )?;
            let folded = answer!(self.reduce_type_head(origin, operation)?);
            if matches!(self.ty(folded)?, dir::Type::Literal(_)) {
                return Ok(Answer::Ready(Some((folded, folded))));
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
                        self.intern_type(module, dir::Type::Primitive(dir::PrimitiveType::Bigint))?
                    }
                    dir::Type::Literal(_) => self.intern_type(
                        module,
                        dir::Type::Primitive(dir::PrimitiveType::Integer(
                            dir::IntegerType::Fixed {
                                width: 64,
                                is_signed: true,
                            },
                        )),
                    )?,
                    _ => left,
                };

                Ok(Answer::Ready(Some((result, result))))
            }
            // elementwise bit operations join equal integer operands
            dir::BinaryOperator::ElementwiseAnd
            | dir::BinaryOperator::ElementwiseOr
            | dir::BinaryOperator::ElementwiseXor
                if integral =>
            {
                let joined = answer!(self.builtin_numeric_join(origin, left, right)?);

                Ok(Answer::Ready(joined.map(|joined| (joined, joined))))
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

                Ok(Answer::Ready(joined.map(|joined| (joined, joined))))
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
                    self.intern_type(module, dir::Type::Primitive(dir::PrimitiveType::Boolean))?;

                Ok(Answer::Ready(Some((boolean, joined))))
            }
            _ => Ok(Answer::Ready(None)),
        }
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
        origin: Origin,
        operand: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let dir::Type::Range(range) = self.ty(operand)? else {
            return Ok(operand);
        };
        let base = match range.scalar_domain() {
            Some(dir::ScalarDomain::Integer) => {
                dir::Type::Primitive(dir::PrimitiveType::Integer(dir::IntegerType::Fixed {
                    width: 32,
                    is_signed: true,
                }))
            }
            Some(dir::ScalarDomain::Float) => {
                dir::Type::Primitive(dir::PrimitiveType::Float(dir::FloatType::Float64))
            }
            Some(dir::ScalarDomain::Bigint) => dir::Type::Primitive(dir::PrimitiveType::Bigint),
            Some(dir::ScalarDomain::Character) => {
                dir::Type::Primitive(dir::PrimitiveType::Character)
            }
            _ => return Ok(operand),
        };

        self.intern_type(origin.module(), base)
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
        let right_literal = matches!(self.ty(right)?, dir::Type::Literal(_));

        match (left_literal, right_literal) {
            // literal pairs widen to their base numeric type
            (Some(literal), true) => {
                let widened = self.intern_type(origin.module(), literal.widen())?;

                Ok(Answer::Ready(Some(widened)))
            }
            (Some(_), false) => {
                let adapts = self.literal_adapts_to_operand(origin, left, right)?;

                Ok(adapts.then_some(right))
            }
            (None, true) => {
                let adapts = self.literal_adapts_to_operand(origin, right, left)?;

                Ok(adapts.then_some(left))
            }
            // typed operands must agree exactly
            (None, false) => {
                let equal = self.decide_relation(origin, Relation::Equal, left, right)?;

                Ok(equal.then_some(left))
            }
        }
    }

    /// Decide whether one literal adapts into one numeric operand.
    fn literal_adapts_to_operand(
        &mut self,
        origin: Origin,
        literal: dir::GlobalTypeId,
        operand: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let reduced = answer!(self.reduce_type_head(origin, operand)?);
        let dir::Type::Parameter(parameter) = self.ty(reduced)? else {
            return self.decide_relation(origin, Relation::Assignable, literal, operand);
        };

        // check the literal against every element of the scalar bound
        let Some(bound) = answer!(self.scalar_parameter_bound(origin, parameter)?) else {
            return Ok(Answer::Ready(false));
        };
        let bound = answer!(self.reduce_type_head(origin, bound)?);
        let elements = match self.ty(bound)? {
            dir::Type::Union(union) => self.type_ids(bound.module_id, union.elements)?.to_vec(),
            _ => vec![bound],
        };
        for element in elements {
            // bound elements are constraints, so markers hold under satisfies
            if !answer!(self.decide_relation(origin, Relation::Satisfies, literal, element)?) {
                return Ok(Answer::Ready(false));
            }
        }

        Ok(Answer::Ready(true))
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
                let module = origin.module();

                self.intern_type(module, dir::Type::Primitive(dir::PrimitiveType::Boolean))?
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
    fn commit_builtin_operator(
        &mut self,
        origin: Origin,
        node: dir::GlobalNodeIdAny,
        operator: dir::BinaryOperator,
        result: dir::GlobalTypeId,
        writeback: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<()>> {
        let target = dir::CallTarget::Builtin(dir::BuiltinCall::BinaryOperator { operator });
        let resolution = dir::CallResolution::new(target, None, Vec::new(), Vec::new(), result);
        self.commit_node_type(node, result)?;
        self.commit_decision(node, Decision::Call(resolution))?;
        self.push_operator_writeback(origin, result, writeback);

        Ok(Answer::Ready(()))
    }

    /// Commit one builtin unary operator selection.
    fn commit_builtin_unary_operator(
        &mut self,
        node: dir::GlobalNodeIdAny,
        operator: dir::UnaryOperator,
        result: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        let target = dir::CallTarget::Builtin(dir::BuiltinCall::UnaryOperator { operator });
        let resolution = dir::CallResolution::new(target, None, Vec::new(), Vec::new(), result);
        self.commit_node_type(node, result)?;
        self.commit_decision(node, Decision::Call(resolution))?;

        Ok(Answer::Ready(()))
    }

    /// Commit one protocol-dispatched operator selection.
    fn commit_protocol_operator(
        &mut self,
        origin: Origin,
        node: dir::GlobalNodeIdAny,
        mut resolution: dir::CallResolution,
        result: dir::GlobalTypeId,
        writeback: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<()>> {
        resolution.return_type = result;
        self.commit_node_type(node, result)?;
        self.commit_decision(node, Decision::Call(resolution))?;
        self.push_operator_writeback(origin, result, writeback);

        Ok(Answer::Ready(()))
    }

    /// Write one compound assignment result back into the assigned place.
    fn push_operator_writeback(
        &mut self,
        origin: Origin,
        result: dir::GlobalTypeId,
        writeback: Option<dir::GlobalTypeId>,
    ) {
        let Some(writeback) = writeback else {
            return;
        };

        let value_origin = self.intern_origin(origin);
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
        self.push_constraint(Constraint::value(
            Relation::Assignable,
            result,
            writeback,
            value_origin,
            cause,
            Some(ValueUse::Store),
        ));
    }

    /// Reject one operator application with a diagnostic.
    pub(in crate::check) fn reject_operator(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        operator: String,
        operands: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<()>> {
        // errored operands already reported, so the rejection stays silent
        let tainted = operands.iter().any(|operand| {
            self.type_flags(*operand)
                .is_ok_and(|flags| flags.has_error())
        });
        if !tainted {
            self.report_no_matching_operator(origin, operator, OperatorOperands::Types(operands))?;
        }
        self.commit_decision(node, Decision::Rejected)?;
        self.commit_error_node(node)?;

        Ok(Answer::Ready(()))
    }
}
