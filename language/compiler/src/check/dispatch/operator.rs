use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CallArgument, CallableApplicability, CallableSignature, CheckState, Dependency,
    FunctionTerm, GenericArgument, MemberCandidate, MemberLookup, MemberProtocol,
    OperatorExpressionResult, OperatorFailureReason, OperatorProtocol, OperatorProtocolArgument,
    OperatorTerm, OperatorTermKind, Origin, StaticTerm, TypeLiteralTerm, TypeOperand, TypeRelation,
    TypeTerm, binary_operator_protocols, unary_operator_protocols,
};

/// Transient operator selection while reducing an operator term.
pub(in crate::check) enum OperatorDispatch {
    /// OperatorTerm resolution is waiting for dependency.
    Pending(SmallVec<[Dependency; 2]>),
    /// No operator candidate accepts the operands.
    NoMatch {
        /// Why operator resolution failed.
        reason: OperatorFailureReason,
    },
    /// Builtin operator behavior resolved.
    Builtin {
        /// The resolved builtin return type.
        return_type: TypeOperand,
    },
    /// One symbol-backed operator method resolved.
    Method {
        /// The resolved operator method symbol.
        symbol: dir::GlobalSymbolId,
        /// The resolved function signature.
        function: FunctionTerm,
        /// The operator expression result.
        return_type: TypeOperand,
    },
}

impl CheckState<'_> {
    /// Select one runtime operator from builtin and method candidates.
    pub(in crate::check) fn select_operator(
        &mut self,
        origin: Origin,
        operator: &OperatorTerm,
        expected: Option<TypeOperand>,
    ) -> CompilerResult<OperatorDispatch> {
        let Answer::Ready(receiver) = self.reduce_type_operand(origin, operator.receiver)? else {
            return Ok(OperatorDispatch::Pending(
                operator.receiver.dependencies(self),
            ));
        };

        // choose primitive operators
        if let Some(result) =
            self.select_builtin_operator_dispatch(origin, operator, receiver, expected)?
        {
            return Ok(result);
        }

        // choose operator method overload
        if let Some(result) =
            self.select_operator_protocols(origin, operator, receiver, expected)?
        {
            return Ok(result);
        }

        Ok(Self::operator_rejected())
    }

    /// Return the generic no-match operator result.
    pub(in crate::check) fn operator_rejected() -> OperatorDispatch {
        OperatorDispatch::NoMatch {
            reason: OperatorFailureReason::NoMatch,
        }
    }

    /// Select one operator method.
    fn select_operator_protocols(
        &mut self,
        origin: Origin,
        operator: &OperatorTerm,
        receiver: TypeOperand,
        expected: Option<TypeOperand>,
    ) -> CompilerResult<Option<OperatorDispatch>> {
        let protocols = operator.protocols();
        if protocols.is_empty() {
            return Ok(None);
        }

        // run a single protocol without speculative state
        if let [protocol] = protocols.as_slice() {
            let result = self.select_operator_protocol(
                origin,
                operator,
                receiver,
                expected,
                protocol.clone(),
            )?;

            return Ok(Some(result));
        }

        // choose the first protocol candidate that resolves
        for protocol in protocols.iter().cloned() {
            let probe = self.inference.begin_probe();
            let result =
                self.select_operator_protocol(origin, operator, receiver, expected, protocol)?;
            match result {
                OperatorDispatch::Method { .. } | OperatorDispatch::Builtin { .. } => {
                    self.inference.commit_probe(probe)?;

                    return Ok(Some(result));
                }
                OperatorDispatch::Pending(blockers) => {
                    let has_external_dependency =
                        probe.has_external_dependency(&blockers, &self.inference);
                    self.inference.drop_probe(probe)?;

                    // block declaration order only on state outside this candidate
                    if has_external_dependency {
                        return Ok(Some(OperatorDispatch::Pending(blockers)));
                    }
                }
                OperatorDispatch::NoMatch { reason: _ } => {
                    self.inference.drop_probe(probe)?;
                }
            }
        }

        Ok(Some(Self::operator_rejected()))
    }

    /// Select one operator method protocol candidate.
    fn select_operator_protocol(
        &mut self,
        origin: Origin,
        operator: &OperatorTerm,
        receiver: TypeOperand,
        expected: Option<TypeOperand>,
        protocol: OperatorProtocol,
    ) -> CompilerResult<OperatorDispatch> {
        // resolve protocol member on the receiver
        let module = operator.source.module_id;
        let key = protocol.method.key(&self.module(module).strings);
        let member_protocol = self.operator_member_protocol(&protocol)?;
        let members =
            match self.resolve_protocol_member(origin, module, receiver, &key, &member_protocol)? {
                MemberLookup::Pending(blockers) => return Ok(OperatorDispatch::Pending(blockers)),
                MemberLookup::Missing => return Ok(Self::operator_rejected()),
                MemberLookup::Field(_) => return Ok(Self::operator_rejected()),
                MemberLookup::Found(members) => members,
            };

        self.select_operator_members(origin, operator, expected, protocol, members)
    }

    /// Select one operator method member candidate.
    fn select_operator_members(
        &mut self,
        origin: Origin,
        operator: &OperatorTerm,
        expected: Option<TypeOperand>,
        protocol: OperatorProtocol,
        members: Vec<MemberCandidate>,
    ) -> CompilerResult<OperatorDispatch> {
        // run a single member without speculative state
        if let [member] = members.as_slice() {
            return self.select_operator_member(origin, operator, expected, &protocol, member);
        }

        // choose the first member candidate that accepts this operator
        for member in &members {
            let probe = self.inference.begin_probe();
            let result =
                self.select_operator_member(origin, operator, expected, &protocol, member)?;

            match result {
                OperatorDispatch::Method { .. } => {
                    self.inference.commit_probe(probe)?;

                    return Ok(result);
                }
                OperatorDispatch::Pending(blockers) => {
                    let has_external_dependency =
                        probe.has_external_dependency(&blockers, &self.inference);
                    self.inference.drop_probe(probe)?;

                    // block declaration order only on state outside this candidate
                    if has_external_dependency {
                        return Ok(OperatorDispatch::Pending(blockers));
                    }
                }
                OperatorDispatch::Builtin { .. } => {
                    self.inference.commit_probe(probe)?;

                    return Ok(result);
                }
                OperatorDispatch::NoMatch { reason: _ } => {
                    self.inference.drop_probe(probe)?;
                }
            }
        }

        Ok(Self::operator_rejected())
    }

    /// Select one operator method signature.
    fn select_operator_member(
        &mut self,
        origin: Origin,
        operator: &OperatorTerm,
        expected: Option<TypeOperand>,
        protocol: &OperatorProtocol,
        member: &MemberCandidate,
    ) -> CompilerResult<OperatorDispatch> {
        let module = operator.source.module_id;
        let Some(ty) = member.ty else {
            return Ok(Self::operator_rejected());
        };

        // reduce member type to one callable signature
        let reduction = self.reduce_type_operand(origin, ty)?;
        let Answer::Ready(term) = reduction else {
            return Ok(OperatorDispatch::Pending(ty.dependencies(self)));
        };
        let function = match self.callable_signature(origin, module, term)? {
            CallableSignature::Pending(blockers) => return Ok(OperatorDispatch::Pending(blockers)),
            CallableSignature::Absent => return Ok(Self::operator_rejected()),
            CallableSignature::Present(function) => function,
        };

        // select the method as a callable signature
        let arguments = operator
            .argument
            .into_iter()
            .map(|ty| CallArgument {
                source: operator.source,
                ty,
                is_spread: false,
            })
            .collect::<SmallVec<[CallArgument; 4]>>();
        let signature = self.select_callable_signature(
            origin,
            module,
            operator.source,
            Some(member.symbol),
            member.instance.clone(),
            function,
            &[],
            &arguments,
            None,
            Some(operator.receiver),
        )?;
        let function = match signature {
            CallableApplicability::Applicable {
                instance: _,
                function,
            } => function,
            CallableApplicability::Pending(blockers) => {
                return Ok(OperatorDispatch::Pending(blockers));
            }
            CallableApplicability::Rejected(_) => return Ok(Self::operator_rejected()),
        };

        // decide expression output
        let Some(expression_result) =
            self.operator_expression_result_operand(&function, protocol.expression_result)?
        else {
            return Ok(OperatorDispatch::Pending(
                operator.receiver.dependencies(self),
            ));
        };
        let expected = self.decide_expected_result(expression_result, expected)?;

        // commit selected method
        match expected {
            Answer::Ready(true) => Ok(OperatorDispatch::Method {
                symbol: member.symbol,
                function,
                return_type: expression_result,
            }),
            Answer::Pending(blockers) => Ok(OperatorDispatch::Pending(blockers)),
            Answer::Ready(false) => Ok(Self::operator_rejected()),
        }
    }

    /// Decide whether one type operand can flow into an expected result.
    pub(in crate::check) fn decide_expected_result(
        &mut self,
        source: TypeOperand,
        expected: Option<TypeOperand>,
    ) -> CompilerResult<Answer<bool>> {
        let Some(expected) = expected else {
            return Ok(Answer::Ready(true));
        };

        self.decide_type_relation(TypeRelation::Assignable, source, expected)
    }

    /// Return one operator expression result operand.
    fn operator_expression_result_operand(
        &mut self,
        function: &FunctionTerm,
        expression_result: OperatorExpressionResult,
    ) -> CompilerResult<Option<TypeOperand>> {
        let operand = match expression_result {
            OperatorExpressionResult::MethodReturn => match function.return_type {
                Some(return_type) => return Ok(Some(return_type)),
                None => self.type_term_operand(TypeTerm::Literal(TypeLiteralTerm::Void)),
            },
            OperatorExpressionResult::Boolean => {
                self.type_term_operand(TypeTerm::Literal(TypeLiteralTerm::boolean()))
            }
        };

        Ok(Some(operand))
    }

    /// Return whether one type can flow into boolean.
    pub(in crate::check) fn is_boolean_assignable(
        &mut self,
        operand: TypeOperand,
    ) -> CompilerResult<bool> {
        let target = Self::boolean_type_term();
        let target = self.inference.push_term(target);

        Ok(
            self.decide_type_relation(TypeRelation::Assignable, operand, target)?
                == Answer::Ready(true),
        )
    }

    /// Convert one operator protocol into member protocol arguments.
    fn operator_member_protocol(
        &mut self,
        protocol: &OperatorProtocol,
    ) -> CompilerResult<MemberProtocol> {
        let mut arguments = Vec::with_capacity(protocol.arguments.len());

        for argument in &protocol.arguments {
            let argument = match argument {
                OperatorProtocolArgument::Access(access) => {
                    let value = dir::StaticTerm::Access { access: *access };
                    let value = self.inference.push_term(StaticTerm::Literal(value));

                    GenericArgument::Static(value.into())
                }
            };

            arguments.push(argument);
        }

        Ok(MemberProtocol {
            item: protocol.item,
            arguments: arguments.into(),
        })
    }
}

impl OperatorTerm {
    /// Return the operator protocols used by this term.
    pub(in crate::check) fn protocols(&self) -> SmallVec<[OperatorProtocol; 2]> {
        match self.kind {
            OperatorTermKind::Unary(operator) => unary_operator_protocols(operator),
            OperatorTermKind::Binary(operator) => binary_operator_protocols(operator),
        }
    }
}
