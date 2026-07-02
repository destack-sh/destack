use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Decision, Dependency, FlowSite, Obligation, Origin, PlaceUse, Relation,
    RuntimePredicateObligation, answer, membership_operator_protocol,
};

impl CheckState<'_> {
    /// Select one `value is T` predicate.
    pub(in crate::check) fn select_type_predicate(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
        target: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = Origin::Node(node);
        let value_node = value.into_global_any(module);
        let target_node = target.into_global_any(module);

        // reduce the tested value and target types
        let value_site = self.node_site(value_node)?;
        let value = answer!(self.predicate_operand_type(origin, value_site)?);
        let target = answer!(self.committed_node_type(target_node)?);
        let target = answer!(self.reduce_type_head(origin, target)?);
        let predicate = answer!(self.select_guard_predicate(origin, value, target, target_node)?);

        let resolution = dir::GuardResolution::Is(dir::IsGuardResolution {
            value_type: value,
            target_type: target,
            predicate,
        });

        self.commit_predicate(node, value_node, target_node, resolution)
    }

    /// Select one `value instanceof Class` predicate.
    pub(in crate::check) fn select_class_predicate(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
        target: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = Origin::Node(node);
        let value_node = value.into_global_any(module);
        let target_node = target.into_global_any(module);

        // reduce the tested value type
        let value_site = self.node_site(value_node)?;
        let value = answer!(self.predicate_operand_type(origin, value_site)?);

        // wait until the target expression resolves its declaration
        let target = match self.decision(target_node) {
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
                self.commit_decision(node, Decision::Rejected)?;
                self.commit_error_node(node)?;

                return Ok(Answer::Ready(()));
            }
            Some(_) | None => return Ok(Answer::pending([Dependency::Decision(target_node)])),
        };

        // reject targets that do not name one class declaration
        let Some((target, arguments)) = target else {
            self.report_instanceof_target_not_class(target_node)?;
            self.commit_decision(node, Decision::Rejected)?;
            self.commit_error_node(node)?;

            return Ok(Answer::Ready(()));
        };
        if self.symbol_kind(target) != dir::SymbolKind::Class {
            self.report_instanceof_target_not_class(target_node)?;
            self.commit_decision(node, Decision::Rejected)?;
            self.commit_error_node(node)?;

            return Ok(Answer::Ready(()));
        }

        let arguments = self.intern_type_ids(module, &arguments)?;
        let target_type = self.intern_type(
            module,
            dir::Type::Instance(dir::GenericInstance {
                symbol: target,
                arguments,
            }),
        )?;
        let predicate = answer!(self.unary_predicate(
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

        self.commit_predicate(node, value_node, target_node, resolution)
    }

    /// Select one `key in value` predicate.
    pub(in crate::check) fn select_member_predicate(
        &mut self,
        site: FlowSite,
        key: dir::LocalNodeId<dir::Expression>,
        receiver: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = Origin::Node(node);
        let key_node = key.into_global_any(module);
        let receiver_node = receiver.into_global_any(module);

        // reduce both operand types
        let key_site = self.node_site(key_node)?;
        let receiver_site = self.node_site(receiver_node)?;
        let key_type = answer!(self.predicate_operand_type(origin, key_site)?);
        let receiver_type = answer!(self.predicate_operand_type(origin, receiver_site)?);
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
                self.commit_decision(node, Decision::Rejected)?;
                self.commit_error_node(node)?;

                return Ok(Answer::Ready(()));
            };

            dir::Predicate::new(dir::PredicateTest::Call(resolution))
        }
        // select structural membership for structural receivers
        else {
            answer!(self.has_predicate(origin, receiver_type, key_type, key)?)
        };

        let resolution = dir::GuardResolution::In(dir::InGuardResolution {
            key_type,
            receiver_type,
            predicate,
        });

        self.commit_predicate(node, key_node, receiver_node, resolution)
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
        let key_type = self.widen_type(key)?;
        let protocol = self.language_protocol(dir::LanguageItem::Has, vec![key_type]);
        let method = membership_operator_protocol().method;
        let key = method.key(&self.module(module).strings);
        let arguments = [key_type];
        let sources = [dir::ArgumentSource::Provided(key_node)];
        let Some(call) = answer!(self.select_protocol_call(
            origin, receiver, receiver, key, &protocol, &arguments, &sources
        )?) else {
            return Ok(Answer::Ready(None));
        };
        let boolean =
            self.intern_type(module, dir::Type::Primitive(dir::PrimitiveType::Boolean))?;
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
        let receiver = answer!(self.reduce_type_head(origin, receiver)?);

        let result = match self.ty(receiver)? {
            dir::Type::Instance(_) => Some(receiver),
            dir::Type::Form(form) => {
                answer!(self.nominal_membership_receiver(origin, form.value)?)
            }
            _ => None,
        };

        Ok(Answer::Ready(result))
    }

    /// Return one predicate operand type.
    fn predicate_operand_type(
        &mut self,
        origin: Origin,
        site: FlowSite,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let ty = answer!(self.infer_node_type(site, PlaceUse::Read)?);

        self.reduce_type_head(origin, ty)
    }

    /// Select the executable predicate for one `is` guard.
    fn select_guard_predicate(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        target_node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Answer<dir::Predicate>> {
        let target = answer!(self.reduce_type_head(origin, target)?);
        let value = answer!(self.reduce_type_head(origin, value)?);

        // use executable RTTI predicates when the target names one
        if let Some(predicate) = answer!(self.runtime_predicate(origin, value, target)?) {
            return Ok(Answer::Ready(predicate));
        }

        // reduce structural targets when the source type already decides them
        if let Some(predicate) = answer!(self.static_predicate(origin, value, target)?) {
            return Ok(Answer::Ready(predicate));
        }

        self.report_runtime_predicate_not_testable(target_node, target)?;

        self.unary_predicate(origin, value, target, dir::PredicateCondition::Never)
    }

    /// Return the executable predicate for one runtime-testable target.
    fn runtime_predicate(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::Predicate>>> {
        let condition = match self.ty(target)? {
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
            dir::Type::Form(_) => dir::PredicateCondition::Type(target),
            dir::Type::Shape(_) => return Ok(Answer::Ready(None)),
            dir::Type::Union(union) => {
                let elements = self.type_ids(target.module_id, union.elements)?.to_vec();
                let mut alternatives = Vec::with_capacity(elements.len());
                for element in elements {
                    let element = answer!(self.reduce_type_head(origin, element)?);
                    let Some(predicate) = answer!(self.runtime_predicate(origin, value, element)?)
                    else {
                        return Ok(Answer::Ready(None));
                    };
                    alternatives.push(predicate);
                }

                let predicate = self.predicate_with_narrowing(
                    dir::Predicate::new(dir::PredicateTest::Any(alternatives)),
                    value,
                    target,
                )?;

                return Ok(Answer::Ready(Some(predicate)));
            }
            dir::Type::Dynamic(_) => dir::PredicateCondition::Type(target),
            dir::Type::Error
            | dir::Type::Void
            | dir::Type::Variable(_)
            | dir::Type::Key(_)
            | dir::Type::Memory(_)
            | dir::Type::Static(_)
            | dir::Type::Intrinsic
            | dir::Type::Parameter(_)
            | dir::Type::This
            | dir::Type::Member(_)
            | dir::Type::Operation(_)
            | dir::Type::FunctionSignature(_)
            | dir::Type::Function(_)
            | dir::Type::FunctionPointer(_)
            | dir::Type::Intersection(_) => return Ok(Answer::Ready(None)),
        };

        let predicate = answer!(self.unary_predicate(origin, value, target, condition)?);

        Ok(Answer::Ready(Some(predicate)))
    }

    /// Reduce a non-executable predicate from static source and target types.
    fn static_predicate(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::Predicate>>> {
        match self.ty(value)? {
            // erased values need a runtime witness predicate
            dir::Type::Dynamic(_) => Ok(Answer::Ready(None)),

            // tagged unions can still test their known arms
            dir::Type::Union(union) => {
                let elements = self.type_ids(value.module_id, union.elements)?.to_vec();
                let mut alternatives = Vec::with_capacity(elements.len());
                for element in elements {
                    let element = answer!(self.reduce_type_head(origin, element)?);
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
                    0 => answer!(self.unary_predicate(
                        origin,
                        value,
                        target,
                        dir::PredicateCondition::Never,
                    )?),
                    1 => alternatives.remove(0),
                    _ => self.predicate_with_narrowing(
                        dir::Predicate::new(dir::PredicateTest::Any(alternatives)),
                        value,
                        target,
                    )?,
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
                let predicate = answer!(self.unary_predicate(origin, value, target, condition,)?);

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
        if let Some(predicate) = answer!(self.runtime_predicate(origin, value, ty)?) {
            return Ok(Answer::Ready(Some(predicate)));
        }

        let predicate = match self.ty(ty)? {
            dir::Type::Shape(_) => Some(answer!(self.unary_predicate(
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
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key_type: dir::GlobalTypeId,
        key: Option<dir::StaticKey>,
    ) -> CompilerResult<Answer<dir::Predicate>> {
        let receiver_type = receiver;
        let receiver = dir::PredicateOperand::new(receiver_type);
        let static_key = key;
        let key = match static_key {
            Some(key) => dir::PredicateKey::Static(key),
            None => dir::PredicateKey::Dynamic(dir::PredicateOperand::new(key_type)),
        };
        let test = dir::PredicateHasTest { receiver, key };
        let predicate = dir::Predicate::new(dir::PredicateTest::Has(test));

        let Some(key) = static_key else {
            return Ok(Answer::Ready(predicate));
        };
        let narrowed = answer!(self.narrowed_membership_receiver(origin, receiver_type, key)?);

        Ok(Answer::Ready(predicate.with_narrowed(narrowed)))
    }

    /// Return the receiver type after a successful static membership test.
    fn narrowed_membership_receiver(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let module = origin.module();
        let source = self.origin_source_node(origin)?;
        let unknown = self.intern_type(module, dir::Type::Unknown)?;
        let target = self.member_shape_type(module, key, unknown, source)?;
        let operation = dir::TypeOperation::Narrow(dir::NarrowType {
            source: receiver,
            target,
            is_positive: true,
        });
        let narrowed = self.intern_type(module, dir::Type::Operation(operation))?;

        self.reduce_type_head(origin, narrowed)
    }

    /// Select one unary predicate and its successful branch value.
    fn unary_predicate(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        condition: dir::PredicateCondition,
    ) -> CompilerResult<Answer<dir::Predicate>> {
        let value = answer!(self.reduce_type_head(origin, value)?);
        let target = answer!(self.reduce_type_head(origin, target)?);
        let input = answer!(self.predicate_input(origin, value, &condition)?);
        let predicate = dir::Predicate::unary(input, condition);
        let predicate = match predicate.is_never() {
            true => predicate,
            false => self.predicate_with_narrowing(predicate, value, target)?,
        };

        Ok(Answer::Ready(predicate))
    }

    /// Return one predicate with its successful branch narrowing.
    fn predicate_with_narrowing(
        &self,
        predicate: dir::Predicate,
        value: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<dir::Predicate> {
        let predicate = predicate.with_narrowed(target);
        let predicate = match self.predicate_projection(value, target)? {
            Some(projection) => predicate.with_projection(projection),
            None => predicate,
        };

        Ok(predicate)
    }

    /// Select the input read by one unary predicate.
    fn predicate_input(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        condition: &dir::PredicateCondition,
    ) -> CompilerResult<Answer<dir::PredicateOperand>> {
        let input = match condition {
            dir::PredicateCondition::Type(_) | dir::PredicateCondition::Subtype(_)
                if matches!(self.ty(value)?, dir::Type::Dynamic(_)) =>
            {
                dir::PredicateOperand::projected(dir::Projection::DynamicType {
                    ty: self.type_descriptor_type(origin)?,
                })
            }
            _ => dir::PredicateOperand::new(value),
        };

        Ok(Answer::Ready(input))
    }

    /// Select the projected value exposed by one predicate.
    fn predicate_projection(
        &self,
        value: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Projection>> {
        let projection = if matches!(self.ty(value)?, dir::Type::Dynamic(_)) {
            Some(dir::Projection::DynamicPayload { ty: target })
        } else {
            None
        };

        Ok(projection)
    }

    /// Return the reflected type descriptor type.
    fn type_descriptor_type(&mut self, origin: Origin) -> CompilerResult<dir::GlobalTypeId> {
        let module = origin.module();
        let unknown = self.intern_type(module, dir::Type::Unknown)?;
        let symbol = self.language_symbol(dir::LanguageItem::Type);
        let arguments = self.intern_type_ids(module, &[unknown])?;

        self.intern_type(
            module,
            dir::Type::Instance(dir::GenericInstance { symbol, arguments }),
        )
    }

    /// Commit one predicate resolution and its boolean result.
    fn commit_predicate(
        &mut self,
        node: dir::GlobalNodeIdAny,
        left: dir::GlobalNodeIdAny,
        right: dir::GlobalNodeIdAny,
        resolution: dir::GuardResolution,
    ) -> CompilerResult<Answer<()>> {
        self.push_runtime_predicate_obligation(node, left, right, resolution.clone());
        self.commit_decision(node, Decision::Guard(resolution))?;
        let boolean = self.intern_type(
            node.module_id,
            dir::Type::Primitive(dir::PrimitiveType::Boolean),
        )?;
        self.commit_node_type(node, boolean)?;

        Ok(Answer::Ready(()))
    }

    /// Push one runtime predicate obligation.
    fn push_runtime_predicate_obligation(
        &mut self,
        source: dir::GlobalNodeIdAny,
        left: dir::GlobalNodeIdAny,
        right: dir::GlobalNodeIdAny,
        predicate: dir::GuardResolution,
    ) {
        let obligation = RuntimePredicateObligation {
            source,
            left,
            right,
            predicate,
        };

        self.push_obligation(Obligation::RuntimePredicate(obligation));
    }
}
