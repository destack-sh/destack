use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckState, Constraint, Decision, FlowSite, Obligation, OperatorExpressionResult,
    Origin, PlaceUse, Relation, ValueUse, WritablePlaceObligation, answer,
    binary_operator_protocols, unary_operator_protocols,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Select one binary operator application.
    pub(in crate::check) fn select_binary_operator(
        &mut self,
        site: FlowSite,
        operator: dir::BinaryOperator,
        left_node: dir::LocalNodeId<dir::Expression>,
        right_node: dir::LocalNodeId<dir::Expression>,
        writeback: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = Origin::Node(node);
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
        let origin = Origin::Node(node);
        let source = self.origin_source_node(origin)?;

        // identity and logic produce builtin results directly
        let nullish_operand = matches!(self.ty(left)?, dir::Type::Null | dir::Type::Undefined)
            || matches!(self.ty(right)?, dir::Type::Null | dir::Type::Undefined);
        let comparable = match (
            self.comparable_operand_kind(left)?,
            self.comparable_operand_kind(right)?,
        ) {
            (Some(left), Some(right)) => left == right,
            _ => false,
        };
        let builtin = match operator {
            // strict identity always produces a boolean
            dir::BinaryOperator::EqualStrict | dir::BinaryOperator::NotEqualStrict => {
                if !answer!(self.types_may_overlap(origin, left, right)?) {
                    self.report_invalid_strict_equality(origin, left, right)?;
                }

                Some(self.push_type(
                    module,
                    dir::Type::Primitive(dir::PrimitiveType::Boolean),
                    source,
                )?)
            }
            // nullish and same-kind scalar equality produce booleans
            dir::BinaryOperator::Equal | dir::BinaryOperator::NotEqual
                if nullish_operand || comparable =>
            {
                Some(self.push_type(
                    module,
                    dir::Type::Primitive(dir::PrimitiveType::Boolean),
                    source,
                )?)
            }
            // logical joins produce the union of their operands
            dir::BinaryOperator::And | dir::BinaryOperator::Or => {
                Some(self.normalized_union_type(module, [left, right], source)?)
            }
            // try-coalesce opens the carrier and joins the alternate
            dir::BinaryOperator::Coalesce => {
                let output = answer!(self.reduce_operation_type(
                    origin,
                    dir::TypeOperation::TryOutput { value: left },
                )?);

                Some(self.normalized_union_type(module, [output, right], source)?)
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
            let key = protocol.method.key(&self.module(module).strings);
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

        self.reject_operator(
            node,
            origin,
            format!("{operator:?}"),
            format!(
                "'{}' and '{}'",
                self.format_type(left),
                self.format_type(right)
            ),
        )
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
        let origin = Origin::Node(node);
        let source = self.origin_source_node(origin)?;
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
                return self.reject_operator(node, origin, format!("{operator:?}"), "place".into());
            };
            let operand = place.ty;

            if answer!(self.is_builtin_numeric(origin, operand)?) {
                let resolution = place.clone().resolution();
                self.commit_node_type(place.source, operand)?;
                self.commit_decision(place.source, Decision::Place(resolution))?;
                self.push_obligation(Obligation::WritablePlace(WritablePlaceObligation {
                    place,
                    ty: operand,
                }));
                self.commit_node_type(node, operand)?;

                return Ok(Answer::Ready(()));
            }

            return self.reject_operator(
                node,
                origin,
                format!("{operator:?}"),
                format!("'{}'", self.format_type(operand)),
            );
        }

        let operand = answer!(self.operand_type(origin, operand_site)?);

        // builtin logical not produces a boolean
        if matches!(operator, dir::UnaryOperator::Not) {
            let result = self.push_type(
                module,
                dir::Type::Primitive(dir::PrimitiveType::Boolean),
                source,
            )?;

            return self.commit_builtin_unary_operator(node, operator, result);
        }

        // builtin numeric negation reduces singleton operands
        if matches!(
            operator,
            dir::UnaryOperator::Negate | dir::UnaryOperator::Plus
        ) && answer!(self.is_builtin_numeric(origin, operand)?)
        {
            let result = answer!(self.builtin_unary_result(origin, operator, operand)?);

            return self.commit_builtin_unary_operator(node, operator, result);
        }

        // dereferences need readonly for reads and mutable access for writes
        let access = match use_ {
            PlaceUse::Read => dir::Access::Readonly,
            PlaceUse::Write | PlaceUse::Update => dir::Access::Mutable,
        };

        // dispatch through the operator protocol interfaces
        let protocols = unary_operator_protocols(operator, access);
        for protocol in protocols {
            let key = protocol.method.key(&self.module(module).strings);
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

        self.reject_operator(
            node,
            origin,
            format!("{operator:?}"),
            format!("'{}'", self.format_type(operand)),
        )
    }

    /// Return the builtin-comparable scalar family of one operand.
    /// Unions compare when every element shares one family.
    fn comparable_operand_kind(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<ComparableKind>> {
        let root = self.settled_root(ty)?;
        let kind = match self.ty(root)? {
            dir::Type::Literal(literal) => comparable_literal_kind(literal),
            dir::Type::Primitive(primitive) => comparable_primitive_kind(primitive),
            dir::Type::Range(_) => Some(ComparableKind::Integer),
            dir::Type::EnumMember(member) => Some(ComparableKind::Enum(member.owner)),
            dir::Type::Instance(instance)
                if matches!(
                    self.definition(instance.symbol),
                    Some(dir::Definition::Enum(_))
                ) =>
            {
                Some(ComparableKind::Enum(root))
            }
            dir::Type::Union(union) => {
                let elements = union.elements.iter().copied().collect::<SmallVec<[_; 4]>>();
                let mut shared: Option<ComparableKind> = None;
                for element in elements {
                    let Some(kind) = self.comparable_operand_kind(element)? else {
                        return Ok(None);
                    };
                    if *shared.get_or_insert(kind) != kind {
                        return Ok(None);
                    }
                }

                shared
            }
            _ => None,
        };

        Ok(kind)
    }

    /// Return the builtin numeric result and joined operand type.
    fn builtin_numeric_result(
        &mut self,
        origin: Origin,
        operator: dir::BinaryOperator,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<(dir::GlobalTypeId, dir::GlobalTypeId)>>> {
        // builtin arithmetic and comparison need numeric operands
        let is_arithmetic = matches!(
            operator,
            dir::BinaryOperator::Add
                | dir::BinaryOperator::Subtract
                | dir::BinaryOperator::Multiply
                | dir::BinaryOperator::Divide
                | dir::BinaryOperator::Remainder
                | dir::BinaryOperator::Exponent
        );
        let is_comparison = matches!(
            operator,
            dir::BinaryOperator::Equal
                | dir::BinaryOperator::NotEqual
                | dir::BinaryOperator::LessThan
                | dir::BinaryOperator::LessThanOrEqual
                | dir::BinaryOperator::GreaterThan
                | dir::BinaryOperator::GreaterThanOrEqual
        );
        if !is_arithmetic && !is_comparison {
            return Ok(Answer::Ready(None));
        }
        if !answer!(self.is_builtin_numeric(origin, left)?)
            || !answer!(self.is_builtin_numeric(origin, right)?)
        {
            return Ok(Answer::Ready(None));
        }

        // mixed-type arithmetic requires explicit conversion first
        let joined = answer!(self.builtin_numeric_join(origin, left, right)?);
        let Some(joined) = joined else {
            return Ok(Answer::Ready(None));
        };

        // comparisons produce booleans over the joined operand type
        if is_comparison {
            let module = origin.module();
            let source = self.origin_source_node(origin)?;
            let boolean = self.push_type(
                module,
                dir::Type::Primitive(dir::PrimitiveType::Boolean),
                source,
            )?;

            return Ok(Answer::Ready(Some((boolean, joined))));
        }

        Ok(Answer::Ready(Some((joined, joined))))
    }

    /// Return one builtin unary result.
    fn builtin_unary_result(
        &mut self,
        origin: Origin,
        operator: dir::UnaryOperator,
        operand: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
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

    /// Join two builtin numeric operands into one common operand type.
    fn builtin_numeric_join(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        // literals adapt into the other operand's type
        let left_literal = matches!(self.ty(left)?, dir::Type::Literal(_));
        let right_literal = matches!(self.ty(right)?, dir::Type::Literal(_));

        match (left_literal, right_literal) {
            // literal pairs widen to their base numeric type
            (true, true) => {
                let widened = match self.ty(left)? {
                    dir::Type::Literal(literal) => literal.widen(),
                    _ => return Ok(Answer::Ready(Some(left))),
                };
                let module = origin.module();
                let source = self.origin_source_node(origin)?;

                Ok(Answer::Ready(Some(
                    self.push_type(module, widened, source)?,
                )))
            }
            (true, false) => {
                let fits = self.decide_relation(origin, Relation::Assignable, left, right)?;

                Ok(fits.then_some(right))
            }
            (false, true) => {
                let fits = self.decide_relation(origin, Relation::Assignable, right, left)?;

                Ok(fits.then_some(left))
            }
            // typed operands must agree exactly
            (false, false) => {
                let equal = self.decide_relation(origin, Relation::Equal, left, right)?;

                Ok(equal.then_some(left))
            }
        }
    }

    /// Return whether one type is a builtin numeric operand.
    fn is_builtin_numeric(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let reduced = answer!(self.reduce_type_head(origin, ty)?);

        Ok(Answer::Ready(match self.ty(reduced)? {
            dir::Type::Primitive(primitive) => matches!(
                primitive,
                dir::PrimitiveType::Integer(_)
                    | dir::PrimitiveType::Float(_)
                    | dir::PrimitiveType::Bigint
            ),
            dir::Type::Literal(literal) => matches!(
                literal,
                dir::ScalarLiteral::Integer(_)
                    | dir::ScalarLiteral::Float(_)
                    | dir::ScalarLiteral::Bigint(_)
            ),
            _ => false,
        }))
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
                        if matches!(form.form, dir::Form::Borrowed { .. } | dir::Form::Raw) =>
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
                let source = self.origin_source_node(origin)?;

                self.push_type(
                    module,
                    dir::Type::Primitive(dir::PrimitiveType::Boolean),
                    source,
                )?
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

        // write compound assignment results back into the assigned place
        if let Some(writeback) = writeback {
            self.push_constraint(Constraint::value(
                Relation::Assignable,
                result,
                writeback,
                origin,
                ValueUse::Store,
            ));
        }

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

        // write compound assignment results back into the assigned place
        if let Some(writeback) = writeback {
            self.push_constraint(Constraint::value(
                Relation::Assignable,
                result,
                writeback,
                origin,
                ValueUse::Store,
            ));
        }

        Ok(Answer::Ready(()))
    }

    /// Reject one operator application with a diagnostic.
    pub(in crate::check) fn reject_operator(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        operator: String,
        operands: String,
    ) -> CompilerResult<Answer<()>> {
        self.report_no_matching_operator(origin, operator, operands)?;
        self.commit_decision(node, Decision::Rejected)?;
        self.commit_error_node(node)?;

        Ok(Answer::Ready(()))
    }
}

/// One builtin-comparable scalar family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ComparableKind {
    /// String values of any literal width.
    String,
    /// Character scalars.
    Character,
    /// Boolean scalars.
    Boolean,
    /// Machine integers and intervals.
    Integer,
    /// Machine floats.
    Float,
    /// Arbitrary-precision integers.
    Bigint,
    /// Nominal enum values.
    Enum(dir::GlobalTypeId),
}

/// Return the comparable family of one scalar literal.
fn comparable_literal_kind(literal: &dir::ScalarLiteral) -> Option<ComparableKind> {
    match literal {
        dir::ScalarLiteral::String(_) => Some(ComparableKind::String),
        dir::ScalarLiteral::Character(_) => Some(ComparableKind::Character),
        dir::ScalarLiteral::Boolean(_) => Some(ComparableKind::Boolean),
        dir::ScalarLiteral::Integer(_) => Some(ComparableKind::Integer),
        dir::ScalarLiteral::Float(_) => Some(ComparableKind::Float),
        dir::ScalarLiteral::Bigint(_) => Some(ComparableKind::Bigint),
        _ => None,
    }
}

/// Return the comparable family of one primitive type.
fn comparable_primitive_kind(primitive: &dir::PrimitiveType) -> Option<ComparableKind> {
    match primitive {
        dir::PrimitiveType::String => Some(ComparableKind::String),
        dir::PrimitiveType::Character => Some(ComparableKind::Character),
        dir::PrimitiveType::Boolean => Some(ComparableKind::Boolean),
        dir::PrimitiveType::Integer(_) => Some(ComparableKind::Integer),
        dir::PrimitiveType::Float(_) => Some(ComparableKind::Float),
        dir::PrimitiveType::Bigint => Some(ComparableKind::Bigint),
        _ => None,
    }
}
