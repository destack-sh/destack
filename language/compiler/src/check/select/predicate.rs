use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Decision, Dependency, Obligation, Origin, Relation,
    RuntimePredicateObligation, answer, membership_operator_protocol,
};

impl CheckState<'_> {
    /// Select one `value is T` predicate.
    pub(in crate::check) fn select_type_predicate(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
        target: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let node = node.into_any();
        let origin = Origin::Node(node);
        let value_node = value.into_global_any(module);
        let target_node = target.into_global_any(module);

        // close the tested value and target types
        let value = answer!(self.predicate_operand_type(origin, value_node)?);
        let target = answer!(self.predicate_operand_type(origin, target_node)?);
        let predicate = answer!(self.select_guard_predicate(origin, value, target, target_node)?);

        let resolution = dir::GuardResolution::Is(dir::IsGuardResolution {
            value_type: value,
            target_type: target,
            predicate,
        });

        self.record_predicate(node, value_node, target_node, resolution)
    }

    /// Select one `value instanceof Class` predicate.
    pub(in crate::check) fn select_class_predicate(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
        target: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let node = node.into_any();
        let origin = Origin::Node(node);
        let value_node = value.into_global_any(module);
        let target_node = target.into_global_any(module);

        // close the tested value type
        let value = answer!(self.predicate_operand_type(origin, value_node)?);

        // wait until the target expression has selected its declaration
        let target = match self.solver.decision(target_node) {
            Some(Decision::Name(resolution)) => match resolution.symbols() {
                [symbol] => Some((*symbol, Vec::new())),
                _ => None,
            },
            Some(Decision::Instantiation(resolution)) => {
                let arguments = dir::GenericArgumentBinding::values(&resolution.generic_arguments)
                    .collect::<Vec<_>>();

                Some((resolution.symbol, arguments))
            }
            Some(Decision::Rejected) => {
                self.record_decision(node, Decision::Rejected)?;

                return Ok(Answer::Ready(()));
            }
            Some(_) | None => return Ok(Answer::pending([Dependency::Decision(target_node)])),
        };

        // reject targets that do not name one class declaration
        let Some((target, arguments)) = target else {
            self.report_instanceof_target_not_class(target_node)?;
            self.record_decision(node, Decision::Rejected)?;

            return Ok(Answer::Ready(()));
        };
        if self.symbol_kind(target) != dir::SymbolKind::Class {
            self.report_instanceof_target_not_class(target_node)?;
            self.record_decision(node, Decision::Rejected)?;

            return Ok(Answer::Ready(()));
        }

        let target_type = self.push_type(
            module,
            dir::Type::Instance(dir::GenericInstance {
                symbol: target,
                arguments: arguments.clone(),
            }),
            target_node.local_id,
        )?;
        let predicate = answer!(self.predicate_for_condition(
            origin,
            value,
            target_type,
            dir::PredicateCondition::Subtype(target_type),
        )?);

        let resolution = dir::GuardResolution::InstanceOf(dir::InstanceOfGuardResolution {
            value_type: value,
            target,
            target_type,
            predicate,
        });

        self.record_predicate(node, value_node, target_node, resolution)
    }

    /// Select one `key in value` predicate.
    pub(in crate::check) fn select_member_predicate(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        key: dir::LocalNodeId<dir::Expression>,
        receiver: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let node = node.into_any();
        let origin = Origin::Node(node);
        let key_node = key.into_global_any(module);
        let receiver_node = receiver.into_global_any(module);

        // close both operand types
        let key_type = answer!(self.predicate_operand_type(origin, key_node)?);
        let receiver_type = answer!(self.predicate_operand_type(origin, receiver_node)?);
        let key = self.module(module).view().get(key).static_key();
        let nominal_receiver = answer!(self.nominal_membership_receiver(origin, receiver_type)?);

        // select protocol membership for nominal receivers
        let predicate = if nominal_receiver.is_some() {
            let Some(resolution) =
                answer!(self.select_has_operator(origin, receiver_type, key_type, key_node)?)
            else {
                self.report_no_matching_operator(
                    origin,
                    "in".to_string(),
                    format!(
                        "'{}' and '{}'",
                        self.format_type(key_type),
                        self.format_type(receiver_type)
                    ),
                )?;
                self.record_decision(node, Decision::Rejected)?;

                return Ok(Answer::Ready(()));
            };

            dir::Predicate::new(dir::PredicateTest::Call(resolution))
        }
        // select structural membership for structural receivers
        else {
            self.has_predicate(receiver_type, key_type, key)
        };

        let resolution = dir::GuardResolution::In(dir::InGuardResolution {
            key_type,
            receiver_type,
            predicate,
        });

        self.record_predicate(node, key_node, receiver_node, resolution)
    }

    /// Select one custom `Has<K>` membership implementation.
    fn select_has_operator(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key: dir::GlobalTypeId,
        key_node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Answer<Option<dir::CallResolution>>> {
        let module = origin.module();
        let source = self.origin_source_node(origin)?;
        let key_type = self.widen_type(module, source, key)?;
        let borrowed_key = self.readonly_borrow_type(origin, key_type)?;
        let protocol = self.language_protocol(dir::LanguageItem::Has, vec![key_type]);
        let method = membership_operator_protocol().method;
        let key = method.key(&self.module(module).strings);
        let arguments = [borrowed_key];
        let sources = [dir::ArgumentSource::Provided(key_node)];
        let Some(call) = answer!(self.select_protocol_call(
            origin, receiver, receiver, key, &protocol, &arguments, &sources
        )?) else {
            return Ok(Answer::Ready(None));
        };
        let boolean = self.push_type(
            module,
            dir::Type::Primitive(dir::PrimitiveType::Boolean),
            source,
        )?;
        let accepts = answer!(self.decide_relation(
            origin,
            Relation::Assignable,
            call.return_type,
            boolean
        )?);

        if accepts {
            Ok(Answer::Ready(Some(call.resolution)))
        } else {
            Ok(Answer::Ready(None))
        }
    }

    /// Return the nominal declaration behind one membership receiver.
    fn nominal_membership_receiver(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let receiver = answer!(self.reduce_type_root(origin, receiver)?);

        let result = match self.ty(receiver)?.clone() {
            dir::Type::Instance(_) => Some(receiver),
            dir::Type::Form(form) => {
                answer!(self.nominal_membership_receiver(origin, form.value)?)
            }
            _ => None,
        };

        Ok(Answer::Ready(result))
    }

    /// Return the readonly borrow type used by membership protocols.
    fn readonly_borrow_type(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let module = origin.module();
        let source = self.origin_source_node(origin)?;

        // build canonical borrowed axes
        let lifetime = self.push_type(
            module,
            dir::Type::Memory(dir::MemoryLiteral::Lifetime(dir::Lifetime::Frame)),
            source,
        )?;
        let access = self.push_type(
            module,
            dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Readonly)),
            source,
        )?;

        self.push_type(
            module,
            dir::Type::Form(dir::FormType {
                form: dir::Form::Borrowed { lifetime, access },
                value,
            }),
            source,
        )
    }

    /// Return one predicate operand type.
    fn predicate_operand_type(
        &mut self,
        origin: Origin,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let ty = answer!(self.node_type_answer(node)?);

        self.reduce_type_root(origin, ty)
    }

    /// Select the executable predicate for one `is` guard.
    fn select_guard_predicate(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        target_node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Answer<dir::Predicate>> {
        let target = answer!(self.reduce_type_root(origin, target)?);
        let value = answer!(self.reduce_type_root(origin, value)?);

        // use executable RTTI predicates when the target names one
        if let Some(predicate) = answer!(self.runtime_predicate_for_type(origin, value, target)?) {
            return Ok(Answer::Ready(predicate));
        }

        // reduce structural targets when the source type already decides them
        if let Some(predicate) = answer!(self.static_predicate(origin, value, target)?) {
            return Ok(Answer::Ready(predicate));
        }

        self.report_runtime_predicate_not_testable(target_node, target)?;

        self.predicate_for_condition(origin, value, target, dir::PredicateCondition::Never)
    }

    /// Return the executable predicate for one runtime-testable target type.
    fn runtime_predicate_for_type(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::Predicate>>> {
        let condition = match self.ty(target)?.clone() {
            dir::Type::Any | dir::Type::Unknown | dir::Type::Object => {
                dir::PredicateCondition::Always
            }
            dir::Type::Never => dir::PredicateCondition::Never,
            dir::Type::Null => dir::PredicateCondition::Literal(dir::ScalarLiteral::Null),
            dir::Type::Undefined => dir::PredicateCondition::Literal(dir::ScalarLiteral::Undefined),
            dir::Type::Primitive(primitive) => dir::PredicateCondition::Primitive(primitive),
            dir::Type::Literal(literal) => dir::PredicateCondition::Literal(literal),
            dir::Type::EnumMember(_) => dir::PredicateCondition::Type(target),
            dir::Type::Range(range) => dir::PredicateCondition::Range(dir::PredicateRange {
                domain: target,
                start: range.start,
                end: range.end,
                end_bound: if range.is_inclusive {
                    dir::RangeEnd::Inclusive
                } else {
                    dir::RangeEnd::Open
                },
            }),
            dir::Type::Instance(instance) => match self.symbol_kind(instance.symbol) {
                dir::SymbolKind::Class | dir::SymbolKind::NewtypeInterface => {
                    dir::PredicateCondition::Subtype(target)
                }
                dir::SymbolKind::Struct | dir::SymbolKind::Enum | dir::SymbolKind::Newtype => {
                    dir::PredicateCondition::Type(target)
                }
                _ => return Ok(Answer::Ready(None)),
            },
            dir::Type::Reference(reference) => match self.symbol_kind(reference.symbol) {
                dir::SymbolKind::Class | dir::SymbolKind::NewtypeInterface => {
                    dir::PredicateCondition::Subtype(target)
                }
                dir::SymbolKind::Struct | dir::SymbolKind::Enum | dir::SymbolKind::Newtype => {
                    dir::PredicateCondition::Type(target)
                }
                _ => return Ok(Answer::Ready(None)),
            },
            dir::Type::Tuple(_)
            | dir::Type::Array(_)
            | dir::Type::FixedArray(_)
            | dir::Type::Slice(_) => dir::PredicateCondition::Type(target),
            dir::Type::Shape(_) => return Ok(Answer::Ready(None)),
            dir::Type::Union(union) => {
                let mut alternatives = Vec::with_capacity(union.elements.len());
                for element in union.elements {
                    let element = answer!(self.reduce_type_root(origin, element)?);
                    let Some(predicate) =
                        answer!(self.runtime_predicate_for_type(origin, value, element)?)
                    else {
                        return Ok(Answer::Ready(None));
                    };
                    alternatives.push(predicate);
                }

                let predicate = dir::Predicate::new(dir::PredicateTest::Any(alternatives))
                    .with_success(self.predicate_success_projection(value, target)?);

                return Ok(Answer::Ready(Some(predicate)));
            }
            dir::Type::Dynamic(_) => dir::PredicateCondition::Type(target),
            dir::Type::Error
            | dir::Type::Void
            | dir::Type::Variable(_)
            | dir::Type::Memory(_)
            | dir::Type::Static(_)
            | dir::Type::Intrinsic
            | dir::Type::Parameter(_)
            | dir::Type::This
            | dir::Type::Member(_)
            | dir::Type::Form(_)
            | dir::Type::Operation(_)
            | dir::Type::FunctionSignature(_)
            | dir::Type::Function(_)
            | dir::Type::FunctionPointer(_)
            | dir::Type::Intersection(_) => return Ok(Answer::Ready(None)),
        };

        let predicate = answer!(self.predicate_for_condition(origin, value, target, condition)?);

        Ok(Answer::Ready(Some(predicate)))
    }

    /// Reduce a non-executable predicate from static source and target types.
    fn static_predicate(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::Predicate>>> {
        match self.ty(value)?.clone() {
            // erased values need a runtime witness predicate
            dir::Type::Dynamic(_) => Ok(Answer::Ready(None)),

            // tagged unions can still test their known arms
            dir::Type::Union(union) => {
                let mut alternatives = Vec::with_capacity(union.elements.len());
                for element in union.elements {
                    let element = answer!(self.reduce_type_root(origin, element)?);
                    let satisfies = answer!(self.decide_relation(
                        origin,
                        Relation::Satisfies,
                        element,
                        target
                    )?);
                    let predicate = self.runtime_union_arm_predicate(origin, value, element)?;
                    if satisfies && let Some(predicate) = answer!(predicate) {
                        alternatives.push(predicate);
                    }
                }

                let predicate = match alternatives.len() {
                    0 => answer!(self.predicate_for_condition(
                        origin,
                        value,
                        target,
                        dir::PredicateCondition::Never,
                    )?),
                    1 => alternatives.remove(0),
                    _ => dir::Predicate::new(dir::PredicateTest::Any(alternatives))
                        .with_success(self.predicate_success_projection(value, target)?),
                };

                Ok(Answer::Ready(Some(predicate)))
            }

            // plain values either satisfy the target statically or never can
            _ => {
                let satisfies =
                    answer!(self.decide_relation(origin, Relation::Satisfies, value, target)?);
                let condition = if satisfies {
                    dir::PredicateCondition::Always
                } else {
                    dir::PredicateCondition::Never
                };
                let predicate =
                    answer!(self.predicate_for_condition(origin, value, target, condition,)?);

                Ok(Answer::Ready(Some(predicate)))
            }
        }
    }

    /// Return the runtime predicate that selects one tagged union arm.
    fn runtime_union_arm_predicate(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::Predicate>>> {
        if let Some(predicate) = answer!(self.runtime_predicate_for_type(origin, value, ty)?) {
            return Ok(Answer::Ready(Some(predicate)));
        }

        let predicate = match self.ty(ty)? {
            dir::Type::Shape(_) => Some(answer!(self.predicate_for_condition(
                origin,
                value,
                ty,
                dir::PredicateCondition::Type(ty),
            )?)),
            _ => None,
        };

        Ok(Answer::Ready(predicate))
    }

    /// Return one structural membership predicate.
    fn has_predicate(
        &self,
        receiver: dir::GlobalTypeId,
        key_type: dir::GlobalTypeId,
        key: Option<dir::StaticKey>,
    ) -> dir::Predicate {
        let receiver = dir::Projection::Identity { ty: receiver };
        let key = match key {
            Some(key) => dir::PredicateKey::Static(key),
            None => dir::PredicateKey::Dynamic(dir::Projection::Identity { ty: key_type }),
        };
        let test = dir::PredicateHasTest { receiver, key };

        dir::Predicate::new(dir::PredicateTest::Has(test))
    }

    /// Select one unary predicate with its success projection.
    fn predicate_for_condition(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        condition: dir::PredicateCondition,
    ) -> CompilerResult<Answer<dir::Predicate>> {
        let value = answer!(self.reduce_type_root(origin, value)?);
        let target = answer!(self.reduce_type_root(origin, target)?);
        let input = answer!(self.predicate_input_projection(origin, value, &condition)?);
        let predicate = dir::Predicate::unary(input, condition);
        let predicate = match predicate.is_never() {
            true => predicate,
            false => predicate.with_success(self.predicate_success_projection(value, target)?),
        };

        Ok(Answer::Ready(predicate))
    }

    /// Select the projection read by one unary predicate.
    fn predicate_input_projection(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        condition: &dir::PredicateCondition,
    ) -> CompilerResult<Answer<dir::Projection>> {
        let projection = match condition {
            dir::PredicateCondition::Type(_) | dir::PredicateCondition::Subtype(_)
                if matches!(self.ty(value)?, dir::Type::Dynamic(_)) =>
            {
                dir::Projection::DynamicType {
                    ty: self.type_descriptor_type(origin)?,
                }
            }
            _ => dir::Projection::Identity { ty: value },
        };

        Ok(Answer::Ready(projection))
    }

    /// Select the projection available after one predicate succeeds.
    fn predicate_success_projection(
        &self,
        value: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<dir::Projection> {
        let projection = if matches!(self.ty(value)?, dir::Type::Dynamic(_)) {
            dir::Projection::DynamicPayload { ty: target }
        } else {
            dir::Projection::Identity { ty: target }
        };

        Ok(projection)
    }

    /// Return the reflected type descriptor type.
    fn type_descriptor_type(&mut self, origin: Origin) -> CompilerResult<dir::GlobalTypeId> {
        let module = origin.module();
        let source = self.origin_source_node(origin)?;
        let unknown = self.push_type(module, dir::Type::Unknown, source)?;
        let symbol = self.language_symbol(dir::LanguageItem::Type);

        self.push_type(
            module,
            dir::Type::Instance(dir::GenericInstance {
                symbol,
                arguments: vec![unknown],
            }),
            source,
        )
    }

    /// Record one predicate resolution and its boolean result.
    fn record_predicate(
        &mut self,
        node: dir::GlobalNodeIdAny,
        left: dir::GlobalNodeIdAny,
        right: dir::GlobalNodeIdAny,
        resolution: dir::GuardResolution,
    ) -> CompilerResult<Answer<()>> {
        self.push_runtime_predicate_obligation(node, left, right, resolution.clone());
        self.record_decision(node, Decision::Guard(resolution))?;

        let result = answer!(self.node_type_answer(node)?);
        self.bind_node_type(node, result)?;

        Ok(Answer::Ready(()))
    }

    /// Collect one runtime predicate validity obligation.
    fn push_runtime_predicate_obligation(
        &mut self,
        source: dir::GlobalNodeIdAny,
        left: dir::GlobalNodeIdAny,
        right: dir::GlobalNodeIdAny,
        predicate: dir::GuardResolution,
    ) {
        let condition = self.node_static_condition(source);
        let obligation = RuntimePredicateObligation {
            source,
            condition,
            left,
            right,
            predicate,
        };

        self.push_obligation(Obligation::RuntimePredicate(obligation));
    }
}
