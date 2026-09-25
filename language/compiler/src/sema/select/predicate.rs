use smallvec::SmallVec;
use tspp_dir as dir;

use crate::CompilerResult;
use crate::sema::{
    CheckState, FlowSite, Obligation, Origin, PlaceUse, Relation, RuntimePredicateObligation,
    Verdict,
};

impl CheckState<'_> {
    /// Select one `value is T` predicate.
    pub(in crate::sema) fn select_type_predicate(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
        target: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = site.origin();
        let value_node = value.into_global_any(module);
        let target_node = target.into_global_any(module);

        // read the tested value and target types
        let value_site = self.visit_site(value_node)?;
        let value = self.predicate_operand_type(origin, value_site)?;
        let target = self.require_node_type(target_node)?;
        let predicate = self.select_guard_predicate(origin, value, target, target_node)?;

        // commit the is guard decision
        let resolution = dir::GuardDecision::Is(dir::IsGuardDecision {
            value_type: value,
            target_type: target,
            predicate,
        });

        self.commit_predicate(origin, node, value_node, target_node, resolution)
    }

    /// Select one `value instanceof Class` predicate.
    pub(in crate::sema) fn select_class_predicate(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
        target: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = site.origin();
        let value_node = value.into_global_any(module);
        let target_node = target.into_global_any(module);

        // reduce the tested value type
        let value_site = self.visit_site(value_node)?;
        let value = self.predicate_operand_type(origin, value_site)?;

        // reject a failed target expression, its own diagnostic stands
        if matches!(
            self.decision(target_node),
            Some(dir::Decision::Rejected | dir::Decision::Poisoned)
        ) {
            self.commit_decision(node, dir::Decision::Rejected)?;
            self.commit_error_node(node)?;

            return Ok(());
        }

        // resolve the class target after name and instantiation selection
        let Some((target, target_type)) = self.instanceof_target_type(origin, target_node)? else {
            self.report_instanceof_target_not_class(target_node)?;
            self.commit_decision(node, dir::Decision::Rejected)?;
            self.commit_error_node(node)?;

            return Ok(());
        };
        let predicate = self.unary_predicate(
            origin,
            value,
            target_type,
            dir::PredicateCondition::Subtype(target_type),
        )?;

        // commit the instanceof guard decision
        let resolution = dir::GuardDecision::InstanceOf(dir::InstanceOfGuardDecision {
            value_type: value,
            target,
            target_type,
            predicate,
        });

        self.commit_predicate(origin, node, value_node, target_node, resolution)
    }

    /// Return the class instance type named by one `instanceof` target.
    pub(in crate::sema) fn instanceof_target_type(
        &mut self,
        origin: Origin,
        target: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Option<(dir::GlobalSymbolId, dir::GlobalTypeId)>> {
        // read the class declaration selected by the predicate
        let site = self.visit_site(target)?;
        let (_, ty) = self.infer_receiver(site)?;
        let ty = self.strip_form(origin, ty)?;
        let ty = self.normalize(origin, ty)?;
        let dir::Type::Reference(reference) = self.ty(ty)? else {
            return Ok(None);
        };
        let symbol = reference.symbol;
        if self.symbol_kind(symbol)? != dir::SymbolKind::Class {
            return Ok(None);
        }

        // preserve an applied constructor or erase the bare class parameters
        let arguments = if reference.arguments.is_empty() {
            self.instanceof_erased_arguments(symbol)?
        } else {
            self.type_ids(ty.module_id, reference.arguments)?.to_vec()
        };
        let arguments = self.intern_type_ids(&arguments)?;
        let target = self.intern_type(dir::Type::Application(dir::GenericApplication {
            symbol,
            arguments,
        }))?;

        Ok(Some((symbol, target)))
    }

    /// Return erased arguments for one bare `instanceof` class target.
    fn instanceof_erased_arguments(
        &mut self,
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
    pub(in crate::sema) fn select_member_predicate(
        &mut self,
        site: FlowSite,
        key: dir::LocalNodeId<dir::Expression>,
        receiver: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = site.origin();
        let key_node = key.into_global_any(module);
        let receiver_node = receiver.into_global_any(module);

        // reduce both operand types
        let key_site = self.visit_site(key_node)?;
        let receiver_site = self.visit_site(receiver_node)?;
        let key_type = self.predicate_operand_type(origin, key_site)?;
        let receiver_type = self.predicate_operand_type(origin, receiver_site)?;
        let key = self.module(module).view().get(key).static_key();

        // select visible structural membership
        let predicate = self.membership_predicate(origin, receiver_type, key_type, key)?;

        // commit the in guard decision
        let resolution = dir::GuardDecision::In(dir::InGuardDecision {
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
    ) -> CompilerResult<dir::GlobalTypeId> {
        let ty = self.infer_node_type(site, PlaceUse::Read)?;

        self.normalize(origin, ty)
    }

    /// Select the executable predicate for one `is` guard.
    pub(in crate::sema) fn select_guard_predicate(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        target_node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::Predicate> {
        // use executable RTTI predicates when the target names one
        if let Some(predicate) = self.runtime_predicate(origin, value, target)? {
            return Ok(predicate);
        }

        // reduce structural targets when the source type already decides them
        if let Some(predicate) = self.static_predicate(origin, value, target)? {
            return Ok(predicate);
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
    ) -> CompilerResult<Option<dir::Predicate>> {
        // build the condition each target head tests
        let condition = match self.ty(target)? {
            dir::Type::Unknown => dir::PredicateCondition::Always,
            // refinements test through their base application
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(target.module_id, refined)?;

                return self.runtime_predicate(origin, value, refined.base);
            }
            dir::Type::Never => dir::PredicateCondition::Never,
            dir::Type::Null => dir::PredicateCondition::Literal(dir::Literal::Null),
            dir::Type::Undefined => dir::PredicateCondition::Literal(dir::Literal::Undefined),
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
            | dir::Type::Reference(dir::TypeReference { symbol, .. }) => {
                match self.symbol_kind(symbol)? {
                    dir::SymbolKind::Class | dir::SymbolKind::NewtypeInterface => {
                        dir::PredicateCondition::Subtype(target)
                    }
                    dir::SymbolKind::Struct | dir::SymbolKind::Enum | dir::SymbolKind::Newtype => {
                        dir::PredicateCondition::Type(target)
                    }
                    _ => return Ok(None),
                }
            }
            dir::Type::Tuple(_) | dir::Type::FixedArray(_) | dir::Type::Slice(_) => {
                dir::PredicateCondition::Type(target)
            }
            dir::Type::Form(_) => dir::PredicateCondition::Type(target),
            dir::Type::Object(_) => return Ok(None),
            dir::Type::Union(union) => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(target.module_id, union.elements)?.into();
                let mut alternatives = Vec::with_capacity(elements.len());
                for element in elements {
                    let Some(predicate) = self.runtime_predicate(origin, value, element)? else {
                        return Ok(None);
                    };
                    alternatives.push(predicate);
                }

                let predicate = self.predicate_with_narrowing(
                    dir::Predicate::new(dir::PredicateTest::Any(alternatives)),
                    value,
                    target,
                )?;

                return Ok(Some(predicate));
            }
            dir::Type::Dynamic(_) => dir::PredicateCondition::Type(target),
            dir::Type::Error
            | dir::Type::Void
            | dir::Type::Variable(_)
            | dir::Type::Key(_)
            | dir::Type::Region(_)
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
            | dir::Type::Intersection(_) => return Ok(None),
        };

        let predicate = self.unary_predicate(origin, value, target, condition)?;

        Ok(Some(predicate))
    }

    /// Reduce one predicate decided by the static source and target types alone.
    fn static_predicate(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Predicate>> {
        // reject erased values, they have no testable static type
        if self.is_erased_value(value)? {
            return Ok(None);
        }

        // test each runtime arm of the value
        match self.union_leaves(origin, value)? {
            // test the known runtime arms of a union, expanded to their leaves
            Some(elements) => {
                let mut alternatives = Vec::with_capacity(elements.len());
                for element in elements {
                    // give up the whole predicate on an undecided arm
                    let is_matched =
                        match self.decide_relation(origin, Relation::Subtype, element, target)? {
                            Verdict::Holds => true,
                            Verdict::Fails => false,
                            Verdict::Ambiguous => return Ok(None),
                        };
                    let predicate = self.runtime_union_arm_predicate(origin, value, element)?;
                    if is_matched && let Some(predicate) = predicate {
                        alternatives.push(predicate);
                    }
                }

                let predicate = match alternatives.len() {
                    0 => {
                        self.unary_predicate(origin, value, target, dir::PredicateCondition::Never)?
                    }
                    1 => alternatives.remove(0),
                    _ => self.predicate_with_narrowing(
                        dir::Predicate::new(dir::PredicateTest::Any(alternatives)),
                        value,
                        target,
                    )?,
                };

                Ok(Some(predicate))
            }

            // plain values decide the target statically
            None => {
                let condition =
                    match self.decide_relation(origin, Relation::Subtype, value, target)? {
                        Verdict::Holds => dir::PredicateCondition::Always,
                        Verdict::Fails => dir::PredicateCondition::Never,
                        Verdict::Ambiguous => return Ok(None),
                    };
                let predicate = self.unary_predicate(origin, value, target, condition)?;

                Ok(Some(predicate))
            }
        }
    }

    /// Return the runtime predicate that selects one union arm.
    fn runtime_union_arm_predicate(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Predicate>> {
        if let Some(predicate) = self.runtime_predicate(origin, value, ty)? {
            return Ok(Some(predicate));
        }

        // test the remaining arms by their type descriptor
        let predicate = match self.ty(ty)? {
            dir::Type::Object(_)
            | dir::Type::FunctionSignature(_)
            | dir::Type::Function(_)
            | dir::Type::FunctionPointer(_) => {
                Some(self.unary_predicate(origin, value, ty, dir::PredicateCondition::Type(ty))?)
            }
            _ => None,
        };

        Ok(predicate)
    }

    /// Return one structural membership predicate.
    fn membership_predicate(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key_type: dir::GlobalTypeId,
        key: Option<dir::StaticKey>,
    ) -> CompilerResult<dir::Predicate> {
        // test a written key statically and any other key through its type
        let receiver_type = receiver;
        let receiver = dir::PredicateOperand::direct(receiver_type);
        let static_key = key;
        let key = match static_key {
            Some(key) => dir::PredicateKey::Static(key),
            None => dir::PredicateKey::Dynamic(dir::PredicateOperand::direct(key_type)),
        };
        let test = dir::PredicateMembershipTest { receiver, key };
        let predicate = dir::Predicate::new(dir::PredicateTest::Membership(Box::new(test)));

        // narrow the receiver by a written key on the successful branch
        let Some(key) = static_key else {
            return Ok(predicate);
        };
        let narrowed = self.narrow_membership_receiver(origin, receiver_type, key)?;

        Ok(predicate.with_narrowed(narrowed))
    }

    /// Select one unary predicate and its successful branch value.
    fn unary_predicate(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        condition: dir::PredicateCondition,
    ) -> CompilerResult<dir::Predicate> {
        let operand = self.predicate_operand(origin, value, &condition)?;
        let predicate = dir::Predicate::unary(operand, condition);
        let predicate = match predicate.is_never() {
            true => predicate,
            false => self.predicate_with_narrowing(predicate, value, target)?,
        };

        Ok(predicate)
    }

    /// Return one predicate with its successful branch narrowing.
    fn predicate_with_narrowing(
        &mut self,
        predicate: dir::Predicate,
        value: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<dir::Predicate> {
        let operation = self.intern_operation(dir::TypeOperation::Narrow(dir::NarrowType {
            source: value,
            target,
            is_positive: true,
        }))?;
        let predicate = predicate.with_narrowed(operation);
        let predicate = match self.predicate_projection(value, target)? {
            Some(projection) => predicate.with_projection(projection),
            None => predicate,
        };

        Ok(predicate)
    }

    /// Select the operand read by one unary predicate.
    fn predicate_operand(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        condition: &dir::PredicateCondition,
    ) -> CompilerResult<dir::PredicateOperand> {
        // read the operand each condition tests
        let input = match condition {
            dir::PredicateCondition::Type(_) | dir::PredicateCondition::Subtype(_)
                if self.is_erased_value(value)? =>
            {
                dir::PredicateOperand::projected(dir::Projection::DynamicType {
                    ty: self.type_descriptor_type(origin)?,
                })
            }
            _ => dir::PredicateOperand::direct(value),
        };

        Ok(input)
    }

    /// Select the projected value exposed by one predicate.
    fn predicate_projection(
        &mut self,
        value: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Projection>> {
        let projection = if self.is_erased_value(value)? {
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

        // apply the Type declaration over the unknown constraint
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
        resolution: dir::GuardDecision,
    ) -> CompilerResult<()> {
        self.push_runtime_predicate_obligation(origin, node, left, right, resolution.clone())?;
        self.commit_decision(node, dir::Decision::Guard(resolution))?;
        let boolean = self.intern_type(dir::Type::Primitive(dir::PrimitiveType::Boolean))?;
        self.commit_node_type(node, boolean)?;

        Ok(())
    }

    /// Push one runtime predicate obligation.
    fn push_runtime_predicate_obligation(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        left: dir::GlobalNodeIdAny,
        right: dir::GlobalNodeIdAny,
        predicate: dir::GuardDecision,
    ) -> CompilerResult<()> {
        let obligation = RuntimePredicateObligation {
            source,
            left,
            right,
            predicate,
        };
        let scope = self.origin_scope(origin)?;
        self.push_obligation(Obligation::RuntimePredicate(Box::new(obligation)), scope)?;

        Ok(())
    }
}
