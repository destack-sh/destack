use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    Answer, BodyState, Decision, DecisionKind, FlowSite, Obligation, Origin, PlaceUse, Relation,
    RuntimePredicateObligation, answer,
};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
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
        let origin = site.origin();
        let value_node = value.into_global_any(module);
        let target_node = target.into_global_any(module);

        // reduce the tested value and target types
        let value_site = self.node_site(value_node)?;
        let value = answer!(self.predicate_operand_type(origin, value_site)?);
        let target = self.require_node_type(target_node)?;
        let target = answer!(self.reduce_type_head(origin, target)?);
        let predicate = answer!(self.select_guard_predicate(origin, value, target, target_node)?);

        let resolution = dir::GuardResolution::Is(dir::IsGuardResolution {
            value_type: value,
            target_type: target,
            predicate,
        });

        self.commit_predicate(origin, node, value_node, target_node, resolution)
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
        let origin = site.origin();
        let value_node = value.into_global_any(module);
        let target_node = target.into_global_any(module);

        // reduce the tested value type
        let value_site = self.node_site(value_node)?;
        let value = answer!(self.predicate_operand_type(origin, value_site)?);

        // reject failed target expressions without a second diagnostic
        if matches!(
            self.decision_kind(target_node),
            Some(DecisionKind::Rejected)
        ) {
            self.commit_decision(node, Decision::Rejected)?;
            self.commit_error_node(node)?;

            return Ok(Answer::Ready(()));
        }

        // resolve the class target after name and instantiation selection
        let Some((target, target_type)) =
            answer!(self.instanceof_target_type(origin, target_node)?)
        else {
            self.report_instanceof_target_not_class(target_node)?;
            self.commit_decision(node, Decision::Rejected)?;
            self.commit_error_node(node)?;

            return Ok(Answer::Ready(()));
        };
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

        self.commit_predicate(origin, node, value_node, target_node, resolution)
    }

    /// Return the class instance type named by one `instanceof` target.
    pub(in crate::check) fn instanceof_target_type(
        &mut self,
        origin: Origin,
        target: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Answer<Option<(dir::GlobalSymbolId, dir::GlobalTypeId)>>> {
        let module = origin.module();
        let target_node = target;
        let kind = answer!(self.decide_node(target)?);

        // read the selected target symbol and written arguments
        let resolutions = self.resolutions(target.module_id);
        let target = match kind {
            DecisionKind::Name => {
                let Some(resolution) = resolutions.name_resolution(target) else {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "instanceof target {target:?} has a name decision without a resolution"
                        ),
                    });
                };

                match resolution.symbols() {
                    [symbol] => Some((*symbol, None)),
                    _ => None,
                }
            }
            DecisionKind::Instantiation => {
                let Some(resolution) = resolutions.instantiation_resolution(target) else {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "instanceof target {target:?} has an instantiation decision without a resolution"
                        ),
                    });
                };
                let arguments =
                    dir::GenericArgumentBinding::values(&resolution.generic_arguments).collect();

                Some((resolution.symbol, Some(arguments)))
            }
            DecisionKind::Rejected => return Ok(Answer::Ready(None)),
            _ => None,
        };
        let Some((symbol, arguments)) = target else {
            return Ok(Answer::Ready(None));
        };

        // guard targets read as their declaration reference
        let reference = self.intern_type(dir::Type::Reference(dir::TypeReference { symbol }))?;
        self.commit_node_type(target_node, reference)?;
        if self.symbol_kind_maybe(symbol)? != Some(dir::SymbolKind::Class) {
            return Ok(Answer::Ready(None));
        }

        // bare generic class targets are existential over their type arguments
        let arguments = match arguments {
            Some(arguments) => arguments,
            None => self.instanceof_erased_arguments(module, symbol)?,
        };
        let arguments = self.intern_type_ids(&arguments)?;
        let target = self.intern_type(dir::Type::Application(dir::GenericApplication {
            symbol,
            arguments,
        }))?;

        Ok(Answer::Ready(Some((symbol, target))))
    }

    /// Return erased arguments for one bare `instanceof` class target.
    fn instanceof_erased_arguments(
        &mut self,
        _module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        let Some(template) = self.symbol_template(symbol)? else {
            return Ok(Vec::new());
        };
        let parameters = self.generic_template_parameters(template)?;
        let mut arguments = Vec::with_capacity(parameters.len());
        for parameter in parameters {
            let argument = self.intern_type(dir::Type::Erased(parameter))?;
            arguments.push(argument);
        }

        Ok(arguments)
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
        let origin = site.origin();
        let key_node = key.into_global_any(module);
        let receiver_node = receiver.into_global_any(module);

        // reduce both operand types
        let key_site = self.node_site(key_node)?;
        let receiver_site = self.node_site(receiver_node)?;
        let key_type = answer!(self.predicate_operand_type(origin, key_site)?);
        let receiver_type = answer!(self.predicate_operand_type(origin, receiver_site)?);
        let key = self.module(module).view().get(key).static_key();

        // select visible structural membership
        let predicate = answer!(self.membership_predicate(origin, receiver_type, key_type, key)?);

        let resolution = dir::GuardResolution::In(dir::InGuardResolution {
            key_type,
            receiver_type,
            predicate,
        });

        self.commit_predicate(origin, node, key_node, receiver_node, resolution)
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
            dir::Type::Any | dir::Type::Unknown => dir::PredicateCondition::Always,
            // refinements test through their base application
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(target.module_id, refined)?;

                return self.runtime_predicate(origin, value, refined.base);
            }
            dir::Type::Never => dir::PredicateCondition::Never,
            dir::Type::Null => dir::PredicateCondition::Literal(dir::ScalarLiteral::Null),
            dir::Type::Undefined => dir::PredicateCondition::Literal(dir::ScalarLiteral::Undefined),
            dir::Type::Primitive(primitive) => dir::PredicateCondition::Primitive(primitive),
            dir::Type::Literal(literal) => dir::PredicateCondition::Literal(literal),
            dir::Type::Variant(_) => dir::PredicateCondition::Type(target),
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
            dir::Type::Application(dir::GenericApplication { symbol, .. })
            | dir::Type::Reference(dir::TypeReference { symbol }) => {
                match self.symbol_kind_maybe(symbol)? {
                    Some(dir::SymbolKind::Class | dir::SymbolKind::NewtypeInterface) => {
                        dir::PredicateCondition::Subtype(target)
                    }
                    Some(
                        dir::SymbolKind::Struct | dir::SymbolKind::Enum | dir::SymbolKind::Newtype,
                    ) => dir::PredicateCondition::Type(target),
                    _ => return Ok(Answer::Ready(None)),
                }
            }
            dir::Type::Tuple(_)
            | dir::Type::Array(_)
            | dir::Type::FixedArray(_)
            | dir::Type::Slice(_) => dir::PredicateCondition::Type(target),
            dir::Type::Form(_) => dir::PredicateCondition::Type(target),
            dir::Type::Shape(_) | dir::Type::Object(_) => return Ok(Answer::Ready(None)),
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

                let predicate = answer!(self.predicate_with_narrowing(
                    origin,
                    dir::Predicate::new(dir::PredicateTest::Any(alternatives)),
                    value,
                    target,
                )?);

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
            | dir::Type::Erased(_)
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
            // reject erased values, they have no testable type
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
                    _ => answer!(self.predicate_with_narrowing(
                        origin,
                        dir::Predicate::new(dir::PredicateTest::Any(alternatives)),
                        value,
                        target,
                    )?),
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
            dir::Type::Shape(_) | dir::Type::Object(_) => Some(answer!(self.unary_predicate(
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
    fn membership_predicate(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key_type: dir::GlobalTypeId,
        key: Option<dir::StaticKey>,
    ) -> CompilerResult<Answer<dir::Predicate>> {
        let receiver_type = receiver;
        let receiver = dir::PredicateOperand::direct(receiver_type);
        let static_key = key;
        let key = match static_key {
            Some(key) => dir::PredicateKey::Static(key),
            None => dir::PredicateKey::Dynamic(dir::PredicateOperand::direct(key_type)),
        };
        let test = dir::PredicateMembershipTest { receiver, key };
        let predicate = dir::Predicate::new(dir::PredicateTest::Membership(Box::new(test)));

        let Some(key) = static_key else {
            return Ok(Answer::Ready(predicate));
        };
        let narrowed = answer!(self.narrow_membership_receiver(origin, receiver_type, key)?);

        Ok(Answer::Ready(predicate.with_narrowed(narrowed)))
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
        let operand = answer!(self.predicate_operand(origin, value, &condition)?);
        let predicate = dir::Predicate::unary(operand, condition);
        let predicate = match predicate.is_never() {
            true => predicate,
            false => answer!(self.predicate_with_narrowing(origin, predicate, value, target)?),
        };

        Ok(Answer::Ready(predicate))
    }

    /// Return one predicate with its successful branch narrowing.
    fn predicate_with_narrowing(
        &mut self,
        origin: Origin,
        predicate: dir::Predicate,
        value: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::Predicate>> {
        let operation = self.intern_operation(dir::TypeOperation::Narrow(dir::NarrowType {
            source: value,
            target,
            is_positive: true,
        }))?;
        let narrowed = answer!(self.reduce_type_head(origin, operation)?);
        let predicate = predicate.with_narrowed(narrowed);
        let predicate = match self.predicate_projection(value, target)? {
            Some(projection) => predicate.with_projection(projection),
            None => predicate,
        };

        Ok(Answer::Ready(predicate))
    }

    /// Select the operand read by one unary predicate.
    fn predicate_operand(
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
            _ => dir::PredicateOperand::direct(value),
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
    fn type_descriptor_type(&mut self, _origin: Origin) -> CompilerResult<dir::GlobalTypeId> {
        let unknown = self.intern_type(dir::Type::Unknown)?;
        let symbol = self.language_symbol(dir::LanguageItem::Type)?;
        let arguments = self.intern_type_ids(&[unknown])?;

        self.intern_type(dir::Type::Application(dir::GenericApplication {
            symbol,
            arguments,
        }))
    }

    /// Commit one predicate resolution and its boolean result.
    fn commit_predicate(
        &mut self,
        origin: Origin,
        node: dir::GlobalNodeIdAny,
        left: dir::GlobalNodeIdAny,
        right: dir::GlobalNodeIdAny,
        resolution: dir::GuardResolution,
    ) -> CompilerResult<Answer<()>> {
        self.push_runtime_predicate_obligation(origin, node, left, right, resolution.clone())?;
        self.commit_decision(node, Decision::Guard(resolution))?;
        let boolean = self.intern_type(dir::Type::Primitive(dir::PrimitiveType::Boolean))?;
        self.commit_node_type(node, boolean)?;

        Ok(Answer::Ready(()))
    }

    /// Push one runtime predicate obligation.
    fn push_runtime_predicate_obligation(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        left: dir::GlobalNodeIdAny,
        right: dir::GlobalNodeIdAny,
        predicate: dir::GuardResolution,
    ) -> CompilerResult<()> {
        let obligation = RuntimePredicateObligation {
            source,
            left,
            right,
            predicate,
        };
        let scope = self.origin_scope(origin)?;
        self.push_obligation(Obligation::RuntimePredicate(Box::new(obligation)), scope);

        Ok(())
    }
}
