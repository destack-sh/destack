use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckState, Decision, MemberLookup, OperatorExpressionResult, OperatorProtocol, Origin,
    PlaceAccess, Relation, binary_operator_protocols, unary_operator_protocols,
};
use crate::{CheckError, CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Select one binary operator application.
    pub(in crate::check) fn select_binary_operator(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        left_node: dir::LocalNodeId<dir::Expression>,
        right_node: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let node = node.into_any();
        let origin = Origin::Node(node);
        let source = self.origin_source_node(origin)?;
        let left = self.operand_type(origin, left_node.into_global_any(module))?;
        let right = self.operand_type(origin, right_node.into_global_any(module))?;
        let (left, right) = match (left, right) {
            (Answer::Ready(left), Answer::Ready(right)) => (left, right),
            (Answer::Pending(blockers), _) | (_, Answer::Pending(blockers)) => {
                return Ok(Answer::Pending(blockers));
            }
        };

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
            dir::BinaryOperator::And | dir::BinaryOperator::Or => Some(self.push_type(
                module,
                dir::Type::Union(dir::UnionType {
                    elements: vec![left, right],
                }),
                source,
            )?),
            // try-coalesce opens the carrier and joins the fallback
            dir::BinaryOperator::Coalesce => {
                let output = self.push_type(
                    module,
                    dir::Type::Operation(dir::TypeOperation::TryOutput { value: left }),
                    source,
                )?;

                Some(self.push_type(
                    module,
                    dir::Type::Union(dir::UnionType {
                        elements: vec![output, right],
                    }),
                    source,
                )?)
            }
            _ => None,
        };
        if let Some(result) = builtin {
            return self.record_builtin_operator(node, operator, result);
        }

        // same-type builtin numerics produce their operand type
        if let Some(result) = self.builtin_numeric_result(origin, operator, left, right)? {
            return self.record_builtin_operator(node, operator, result);
        }

        // dispatch through the operator protocol interfaces
        let protocols = binary_operator_protocols(operator);
        for protocol in protocols {
            let key = protocol.method.key(&self.module(module).strings);
            let lookup =
                self.lookup_member(origin, module, left, dir::MemberSpace::Instance, key)?;

            match lookup {
                MemberLookup::Found(candidates) => {
                    // try every implementation in resolution order
                    for candidate in candidates {
                        let candidate_symbol = candidate.symbol;
                        let candidate_type = candidate.ty;

                        // require the right operand to fit the method parameter
                        let result =
                            self.protocol_result(origin, &protocol, candidate_type, Some(right))?;
                        match result {
                            Answer::Ready(Some(result)) => {
                                return self.record_protocol_operator(
                                    node,
                                    left,
                                    candidate_symbol,
                                    Some(result),
                                );
                            }
                            Answer::Ready(None) => continue,
                            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                        }
                    }
                }
                MemberLookup::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                MemberLookup::Missing | MemberLookup::Field(_) => continue,
            }
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
        node: dir::GlobalNodeId<dir::Expression>,
        operator: dir::UnaryOperator,
        operand_node: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let node = node.into_any();
        let origin = Origin::Node(node);
        let source = self.origin_source_node(origin)?;
        let operand = match self.operand_type(origin, operand_node.into_global_any(module))? {
            Answer::Ready(operand) => operand,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        // increments rewrite builtin numeric places by one
        if matches!(
            operator,
            dir::UnaryOperator::PostIncrement
                | dir::UnaryOperator::PostDecrement
                | dir::UnaryOperator::PreIncrement
                | dir::UnaryOperator::PreDecrement
        ) {
            if self.is_builtin_numeric(origin, operand)? {
                return self.record_builtin_unary_operator(node, operator, operand);
            }

            return self.reject_operator(
                node,
                origin,
                format!("{operator:?}"),
                format!("'{}'", self.format_type(operand)),
            );
        }

        // builtin logical not produces a boolean
        if matches!(operator, dir::UnaryOperator::Not) {
            let result = self.push_type(
                module,
                dir::Type::Primitive(dir::PrimitiveType::Boolean),
                source,
            )?;

            return self.record_builtin_unary_operator(node, operator, result);
        }

        // builtin numeric negation and plus keep their operand type
        if matches!(
            operator,
            dir::UnaryOperator::Negate | dir::UnaryOperator::Plus
        ) && self.is_builtin_numeric(origin, operand)?
        {
            return self.record_builtin_unary_operator(node, operator, operand);
        }

        // dereferences demand access by their place position: whole
        // value replacement may invalidate interior borrows, so writes
        // require exclusive access
        let access = match self.inputs.place_access(node) {
            PlaceAccess::Read => dir::Access::Readonly,
            PlaceAccess::Write | PlaceAccess::ReadWrite => dir::Access::Exclusive,
        };

        // dispatch through the operator protocol interfaces
        let protocols = unary_operator_protocols(operator, access);
        for protocol in protocols {
            let key = protocol.method.key(&self.module(module).strings);
            let lookup =
                self.lookup_member(origin, module, operand, dir::MemberSpace::Instance, key)?;

            match lookup {
                MemberLookup::Found(candidates) => {
                    // try every implementation in resolution order
                    for candidate in candidates {
                        let candidate_symbol = candidate.symbol;
                        let candidate_type = candidate.ty;

                        let result =
                            self.protocol_result(origin, &protocol, candidate_type, None)?;
                        match result {
                            Answer::Ready(Some(result)) => {
                                return self.record_protocol_operator(
                                    node,
                                    operand,
                                    candidate_symbol,
                                    Some(result),
                                );
                            }
                            Answer::Ready(None) => continue,
                            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                        }
                    }
                }
                MemberLookup::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                MemberLookup::Missing | MemberLookup::Field(_) => continue,
            }
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
        let root = self.resolve_root(ty)?;
        let kind = match self.ty(root)? {
            dir::Type::Literal(literal) => comparable_literal_kind(literal),
            dir::Type::Primitive(primitive) => comparable_primitive_kind(primitive),
            dir::Type::Range(_) => Some(ComparableKind::Integer),
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

    /// Return the builtin numeric result for one same-type application.
    fn builtin_numeric_result(
        &mut self,
        origin: Origin,
        operator: dir::BinaryOperator,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
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
            return Ok(None);
        }
        if !self.is_builtin_numeric(origin, left)? || !self.is_builtin_numeric(origin, right)? {
            return Ok(None);
        }

        // mixed-type arithmetic requires explicit conversion first
        let joined = self.builtin_numeric_join(origin, left, right)?;
        let Some(joined) = joined else {
            return Ok(None);
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

            return Ok(Some(boolean));
        }

        Ok(Some(joined))
    }

    /// Join two builtin numeric operands into one common operand type.
    fn builtin_numeric_join(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // literals adapt into the other operand's type
        let left_literal = matches!(self.ty(left)?, dir::Type::Literal(_));
        let right_literal = matches!(self.ty(right)?, dir::Type::Literal(_));

        match (left_literal, right_literal) {
            // literal pairs widen to their base numeric type
            (true, true) => {
                let widened = match self.ty(left)? {
                    dir::Type::Literal(literal) => literal.widen(),
                    _ => return Ok(Some(left)),
                };
                let module = origin.module();
                let source = self.origin_source_node(origin)?;

                Ok(Some(self.push_type(module, widened, source)?))
            }
            (true, false) => {
                let fits = self.decide_relation(origin, Relation::Assignable, left, right)?;

                Ok((fits == Answer::Ready(true)).then_some(right))
            }
            (false, true) => {
                let fits = self.decide_relation(origin, Relation::Assignable, right, left)?;

                Ok((fits == Answer::Ready(true)).then_some(left))
            }
            // typed operands must agree exactly
            (false, false) => {
                let equal = self.decide_relation(origin, Relation::Equal, left, right)?;

                Ok((equal == Answer::Ready(true)).then_some(left))
            }
        }
    }

    /// Return whether one type is a builtin numeric operand.
    fn is_builtin_numeric(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let reduced = match self.evaluate_root(origin, ty)? {
            Answer::Ready(reduced) => reduced,
            Answer::Pending(_) => return Ok(false),
        };

        Ok(match self.ty(reduced)? {
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
        })
    }

    /// Return the result type of one matched protocol method.
    fn protocol_result(
        &mut self,
        origin: Origin,
        protocol: &OperatorProtocol,
        method_type: dir::GlobalTypeId,
        argument: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        // close the method's function shape
        let method_type = match self.evaluate_root(origin, method_type)? {
            Answer::Ready(reduced) => reduced,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let (first_parameter, return_type) = match self.ty(method_type)? {
            dir::Type::Function(function) => (
                function.parameters.first().map(|parameter| parameter.ty),
                function.return_type,
            ),
            _ => return Ok(Answer::Ready(None)),
        };

        // require the right operand to fit the method parameter
        if let (Some(argument), Some(parameter)) = (argument, first_parameter) {
            let fits = self.decide_relation(origin, Relation::Assignable, argument, parameter)?;
            match fits {
                Answer::Ready(true) => {}
                Answer::Ready(false) => return Ok(Answer::Ready(None)),
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            }
        }

        // produce the protocol's declared result
        let result = match protocol.expression_result {
            OperatorExpressionResult::MethodReturn => match return_type {
                Some(return_type) => return_type,
                None => return Ok(Answer::Ready(None)),
            },
            // project the place behind the returned borrow
            OperatorExpressionResult::Pointee => {
                let Some(return_type) = return_type else {
                    return Ok(Answer::Ready(None));
                };
                let reduced = match self.evaluate_root(origin, return_type)? {
                    Answer::Ready(reduced) => reduced,
                    Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                };

                match self.ty(reduced)? {
                    dir::Type::Form(form)
                        if matches!(form.form, dir::Form::Borrowed { .. } | dir::Form::Raw) =>
                    {
                        form.value
                    }
                    // dereference methods must return indirection
                    _ => return Ok(Answer::Ready(None)),
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

        Ok(Answer::Ready(Some(result)))
    }

    /// Read one operand node's reduced input type.
    fn operand_type(
        &mut self,
        origin: Origin,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let Some(ty) = self.inputs.node_type(node) else {
            return Err(CompilerError::Internal {
                message: format!("operator operand {node:?} has no input type"),
            });
        };

        self.evaluate_root(origin, ty)
    }

    /// Record one builtin binary operator decision.
    fn record_builtin_operator(
        &mut self,
        node: dir::GlobalNodeIdAny,
        operator: dir::BinaryOperator,
        result: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        let target = dir::CallTarget::Builtin(dir::BuiltinCall::BinaryOperator { operator });
        let resolution = dir::CallResolution::new(target, Vec::new(), result);
        self.record_decision(node, Decision::Call(resolution))?;

        // flow the result into the node variable
        if let Some(variable) = self.node_variable(node)? {
            self.push_lower_bound(variable, result)?;
        }

        Ok(Answer::Ready(()))
    }

    /// Record one builtin unary operator decision.
    fn record_builtin_unary_operator(
        &mut self,
        node: dir::GlobalNodeIdAny,
        operator: dir::UnaryOperator,
        result: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        let target = dir::CallTarget::Builtin(dir::BuiltinCall::UnaryOperator { operator });
        let resolution = dir::CallResolution::new(target, Vec::new(), result);
        self.record_decision(node, Decision::Call(resolution))?;

        if let Some(variable) = self.node_variable(node)? {
            self.push_lower_bound(variable, result)?;
        }

        Ok(Answer::Ready(()))
    }

    /// Record one protocol-dispatched operator decision.
    fn record_protocol_operator(
        &mut self,
        node: dir::GlobalNodeIdAny,
        receiver: dir::GlobalTypeId,
        symbol: Option<dir::GlobalSymbolId>,
        result: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let source = node.local_id;
        let result = match result {
            Some(result) => result,
            None => self.push_type(module, dir::Type::Void, source)?,
        };

        let target = match symbol {
            Some(symbol) => dir::CallTarget::Symbol(dir::CallCandidate {
                receiver: Some(receiver),
                symbol,
                arguments: Vec::new(),
            }),
            None => dir::CallTarget::Expression {
                arguments: Vec::new(),
            },
        };
        let resolution = dir::CallResolution::new(target, Vec::new(), result);
        self.record_decision(node, Decision::Call(resolution))?;

        if let Some(variable) = self.node_variable(node)? {
            self.push_lower_bound(variable, result)?;
        }

        Ok(Answer::Ready(()))
    }

    /// Reject one operator application with a diagnostic.
    fn reject_operator(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        operator: String,
        operands: String,
    ) -> CompilerResult<Answer<()>> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::NoMatchingOperator {
            anchor,
            module,
            operator,
            operands,
        };
        self.module_mut(module).diagnostics.push(error.into());
        self.record_decision(node, Decision::Rejected)?;

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
