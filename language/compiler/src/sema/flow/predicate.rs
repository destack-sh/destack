use smallvec::SmallVec;
use tspp_dir as dir;

use crate::sema::{
    CheckState, FlowPointChange, FlowPredicate, FlowSite, Origin, Relation, Verdict,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Return one type as viewed at one flow point.
    pub(in crate::sema) fn flow_type_at(
        &mut self,
        site: FlowSite,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read the type through its solution before narrowing
        let ty = self.shallow_resolve(ty)?;

        // narrow expression occurrences only
        let dir::NodeType::Expression = site.node.local_id.ty else {
            return Ok(ty);
        };

        // require a selected stored access path
        let Some(path) = self
            .module(site.node.module_id)
            .decisions_tail
            .access_resolution(site.node)
            .cloned()
        else {
            return Ok(ty);
        };

        // narrow through the predicates visible at this site
        let Some(narrowed) = self.flow_narrowed_type(site, path.path(), ty)? else {
            return Ok(ty);
        };
        self.commit_narrowing(site, ty, narrowed)?;

        Ok(narrowed)
    }

    /// Record the narrowing one read sees, settled to live members at writeback.
    fn commit_narrowing(
        &mut self,
        site: FlowSite,
        declared: dir::GlobalTypeId,
        narrowed: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        // keep the union the first narrowing of this read declared
        let node = site.node;
        let decisions = &mut self.module_mut(node.module_id).decisions_tail;
        let union = decisions
            .narrowing(node)
            .map_or(declared, |narrowing| narrowing.union);
        decisions.set_narrowing(
            node,
            dir::Narrowing {
                union,
                arms: vec![narrowed],
            },
        );

        Ok(())
    }

    /// Return the type after narrowings visible at one flow site, none without a narrowing.
    pub(in crate::sema) fn flow_narrowed_type(
        &mut self,
        site: FlowSite,
        path: &dir::AccessPath,
        source: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // read the checked module's flow graph from the live cursor before flushes
        let module_id = self.module_id;
        let module = self.module(site.node.module_id);
        let flows = &module.flow_points;
        let cursor = (site.node.module_id == module_id).then(|| self.flow.points());

        // collect every direct narrowing and descendant equality back to their clears
        let mut predicates = Vec::new();
        let mut cleared = Vec::new();
        let mut current = Some(site.flow);
        while let Some(point) = current {
            // read each point from the flushed table, then from the cursor
            let flow = flows.get(point.index());
            let flow = flow.or_else(|| cursor.and_then(|points| points.get(point.index())));
            let Some(flow) = flow else {
                return Err(CompilerError::Internal {
                    message: format!("flow point {point:?} is not in module flow table"),
                });
            };

            // keep relevant narrowings that remain valid at this point
            match &flow.change {
                FlowPointChange::Start => {}
                FlowPointChange::Narrowing {
                    path: narrowed,
                    predicate,
                } => {
                    let is_cleared = cleared
                        .iter()
                        .any(|cleared: &&dir::AccessPath| narrowed.starts_with(cleared));
                    let is_direct = narrowed.as_ref() == path;
                    let is_descendant_equality =
                        matches!(predicate, FlowPredicate::Equality { .. })
                            && narrowed.starts_with(path);

                    if !is_cleared && (is_direct || is_descendant_equality) {
                        let narrowing = (narrowed.as_ref().clone(), *predicate);
                        if !predicates.contains(&narrowing) {
                            predicates.push(narrowing);
                        }
                    }
                }
                FlowPointChange::Clear { path: cleared } if path.starts_with(cleared) => {
                    break;
                }
                FlowPointChange::Clear { path } => cleared.push(path.as_ref()),
            }

            // move toward the entry flow
            current = flow.parent;
        }

        // stop when no narrowing affects this path
        if predicates.is_empty() {
            return Ok(None);
        }

        // apply oldest first, so each later test refines the earlier result
        let mut narrowed = source;
        for (tested, predicate) in predicates.into_iter().rev() {
            // resolve a variable blocking the narrowing structurally, then narrow once more
            let mut narrowing =
                self.resolve_flow_predicate(site, path, &tested, narrowed, predicate)?;
            if let Err(blocker) = narrowing {
                let blocker = self.variable_type(blocker)?;
                self.resolve_structurally(site, blocker)?;
                narrowing =
                    self.resolve_flow_predicate(site, path, &tested, narrowed, predicate)?;
            }
            if let Ok(Some(next)) = narrowing {
                narrowed = next;
            }
        }

        Ok(Some(narrowed))
    }

    /// Apply one flow predicate to a source type.
    fn resolve_flow_predicate(
        &mut self,
        site: FlowSite,
        path: &dir::AccessPath,
        tested: &dir::AccessPath,
        source: dir::GlobalTypeId,
        predicate: FlowPredicate,
    ) -> CompilerResult<Result<Option<dir::GlobalTypeId>, dir::TypeVariableId>> {
        // narrow by the form of the predicate
        match predicate {
            // narrow through the subset a pattern accepts
            FlowPredicate::Pattern {
                pattern,
                is_positive,
            } => {
                // resolve pattern decisions into the type accepted by the pattern
                let origin = site.origin();
                match self.pattern_predicate_target(origin, pattern)? {
                    Some(target) => {
                        let narrowed =
                            self.resolve_type_predicate(site, source, target, is_positive)?;

                        Ok(Ok(narrowed))
                    }
                    None => Ok(Ok(None)),
                }
            }
            // narrow through a checked equality operand
            FlowPredicate::Equality {
                operation,
                is_equal,
            } => self.resolve_equality_predicate(site, path, tested, source, operation, is_equal),
            // narrow through the subset a guard selects
            FlowPredicate::Guard { guard, is_positive } => {
                let Some(narrowed) = self.guard_narrowing(guard)? else {
                    return Ok(Ok(None));
                };

                // return the exact narrowing check selected on positive branches
                if is_positive {
                    return Ok(Ok(Some(narrowed)));
                }

                // remove the selected positive subset on negative branches
                let narrowed = self.resolve_type_predicate(site, source, narrowed, false)?;

                Ok(Ok(narrowed))
            }
        }
    }

    /// Apply one selected equality operation to a flow path.
    fn resolve_equality_predicate(
        &mut self,
        site: FlowSite,
        path: &dir::AccessPath,
        tested: &dir::AccessPath,
        source: dir::GlobalTypeId,
        operation: dir::GlobalNodeIdAny,
        is_equal: bool,
    ) -> CompilerResult<Result<Option<dir::GlobalTypeId>, dir::TypeVariableId>> {
        // read the checked operands of the equality
        let Some((operator, operands)) = self.equality_operands(operation)? else {
            return Ok(Ok(None));
        };
        let pairs = [(&operands[0], &operands[1]), (&operands[1], &operands[0])];

        // apply the fact directed from the tested stable operand
        for (operand, compared) in pairs {
            let access = self
                .decisions(operand.source.module_id)
                .access_resolution(operand.source.into_any())
                .cloned();
            let Some(access) = access.filter(|access| access.path() == tested) else {
                continue;
            };
            let Some(mut target) =
                self.equality_predicate_target(site.origin(), compared.source)?
            else {
                continue;
            };

            // test null and undefined together under loose equality
            if matches!(
                operator,
                dir::BinaryOperator::Equal | dir::BinaryOperator::NotEqual
            ) && matches!(
                self.resolved_ty(target)?,
                dir::Type::Null | dir::Type::Undefined
            ) {
                let null = self.intern_type(dir::Type::Null)?;
                let undefined = self.intern_type(dir::Type::Undefined)?;
                target = self.normalized_union_type([null, undefined])?;
            }

            return self.narrow_equality_access(
                site,
                path,
                source,
                operand.source,
                &access,
                target,
                is_equal,
            );
        }

        Ok(Ok(None))
    }

    /// Return the checked builtin operands of one equality operation.
    fn equality_operands(
        &self,
        operation: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Option<(dir::BinaryOperator, [dir::BuiltinOperand; 2])>> {
        // skip rejected operations, which establish no runtime equality
        let resolution = match self.decision(operation) {
            Some(dir::Decision::Operator(resolution)) => resolution,
            Some(dir::Decision::Rejected | dir::Decision::Poisoned) => return Ok(None),
            Some(decision) => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "equality operation {} decided as {decision:?}",
                        self.node_label(operation),
                    ),
                });
            }
            None => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "flow equality operation is undecided: {}",
                        self.node_label(operation),
                    ),
                });
            }
        };

        // read the operands each resolution exposes
        match resolution {
            // return the exact checked operand nodes of compiler equality
            dir::OperationResolution::One(dir::OperatorApplication::Binary {
                operator,
                target: dir::OperatorTarget::Builtin(operands),
                ..
            }) if operator.is_equality() => Ok(Some((*operator, operands.clone()))),

            // skip protocol equality, which has no compiler-defined narrowing
            dir::OperationResolution::One(dir::OperatorApplication::Binary {
                operator,
                target: dir::OperatorTarget::Call(_),
                ..
            }) if operator.is_equality() => Ok(None),

            // skip union dispatch, which selects no single equality operation
            dir::OperationResolution::Union { .. } => Ok(None),

            // fail on any other resolution, flow records equality only
            resolution => Err(CompilerError::Internal {
                message: format!(
                    "flow equality {} selected non-equality resolution {resolution:?}",
                    self.node_label(operation),
                ),
            }),
        }
    }

    /// Narrow one flow path through one checked equality access.
    fn narrow_equality_access(
        &mut self,
        site: FlowSite,
        path: &dir::AccessPath,
        source: dir::GlobalTypeId,
        operand: dir::GlobalNodeId<dir::Expression>,
        access: &dir::AccessResolution,
        target: dir::GlobalTypeId,
        is_equal: bool,
    ) -> CompilerResult<Result<Option<dir::GlobalTypeId>, dir::TypeVariableId>> {
        // read the path the operand accesses
        let operand_path = access.path();

        // narrow the value stored at this exact path
        if operand_path == path {
            let narrowed = self.resolve_type_predicate(site, source, target, is_equal)?;

            return Ok(Ok(narrowed));
        }

        // keep the flow of paths the operand does not touch
        if !operand_path.starts_with(path) {
            return Ok(Ok(None));
        }

        // derive the tested member chain from the selected access paths
        let relative = &operand_path.keys()[path.keys().len()..];
        if relative.is_empty() {
            return Err(CompilerError::Internal {
                message: format!(
                    "equality operand {} has no projection below {path:?}",
                    self.node_label(operand.into_any()),
                ),
            });
        }

        self.narrow_arms(site.origin(), source, relative, target, is_equal)
    }

    /// Return the singleton type selected by one equality operand.
    fn equality_predicate_target(
        &mut self,
        origin: Origin,
        value: dir::GlobalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // peel the written assertions down to the compared value
        let mut value = value;
        loop {
            let node = self.module(value.module_id).view().get(value.local_id);
            match *node {
                // step through a cast that widens the value it wraps
                dir::Expression::As { expression, .. } => {
                    let widened = self.require_node_type(value.into_any())?;
                    let operand = expression.into_global(value.module_id);
                    let narrow = self.require_node_type(operand.into_any())?;

                    // stop where the cast converts the value
                    if self.decide_relation(origin, Relation::Subtype, narrow, widened)?
                        != Verdict::Holds
                    {
                        break;
                    }

                    value = operand;
                }
                // step through an assertion, which keeps its written value
                dir::Expression::Satisfies { expression, .. } => {
                    value = expression.into_global(value.module_id);
                }
                // stop at the compared value
                _ => break,
            }
        }

        // answer a written singleton directly
        let ty = self.require_node_type(value.into_any())?;
        if self.is_singleton_type(ty)? {
            return Ok(Some(ty));
        }

        // answer a newtype whose backing type is a singleton
        if let Some(instance) = self.decompose_newtype(origin, ty)? {
            let backing = instance.backing;
            if self.is_singleton_type(backing)? {
                return Ok(Some(ty));
            }
        }

        Ok(None)
    }

    /// Settle one runtime type predicate.
    fn resolve_type_predicate(
        &mut self,
        site: FlowSite,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        is_positive: bool,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // build the narrowing operation over both sides
        let operation = dir::TypeOperation::Narrow(dir::NarrowType {
            source,
            target,
            is_positive,
        });

        // reduce the narrowing through the normal type operation path
        let narrowed = self.intern_operation(operation)?;
        let narrowed = self.normalize(site.origin(), narrowed)?;

        Ok(Some(narrowed))
    }

    /// Return the positive narrowing selected for one guard expression.
    fn guard_narrowing(
        &self,
        guard: dir::GlobalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // skip rejected guards, which establish no runtime predicate
        match self.decision(guard.into_any()) {
            Some(dir::Decision::Guard(resolution)) => Ok(resolution.predicate().narrowed),
            Some(dir::Decision::Rejected | dir::Decision::Poisoned) => Ok(None),
            Some(decision) => Err(CompilerError::Internal {
                message: format!("guard {guard:?} decided as {decision:?}"),
            }),
            None => Err(CompilerError::Internal {
                message: format!("guard {guard:?} is undecided"),
            }),
        }
    }

    /// Return the type subset accepted by one pattern.
    pub(in crate::sema) fn pattern_predicate_target(
        &mut self,
        origin: Origin,
        pattern: dir::GlobalNodeId<dir::Pattern>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // skip rejected patterns, which narrow nothing
        match self.decision(pattern.into_any()).cloned() {
            Some(dir::Decision::Pattern(resolution)) => {
                self.pattern_decision_predicate_target(origin, pattern, &resolution)
            }
            Some(dir::Decision::Rejected | dir::Decision::Poisoned) => Ok(None),
            Some(decision) => Err(CompilerError::Internal {
                message: format!("pattern {pattern:?} decided as {decision:?}"),
            }),
            None => Err(CompilerError::Internal {
                message: format!("pattern {pattern:?} is undecided"),
            }),
        }
    }

    /// Return the type subset accepted by one pattern resolution.
    fn pattern_decision_predicate_target(
        &mut self,
        origin: Origin,
        pattern: dir::GlobalNodeId<dir::Pattern>,
        resolution: &dir::PatternDecision,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // read the subset each resolution accepts
        match resolution {
            dir::PatternDecision::Test(test) => Ok(test.predicate.narrowed),
            dir::PatternDecision::Variant(variant) => Ok(variant.predicate.narrowed),
            dir::PatternDecision::Destructure(destructure) => match destructure.as_ref() {
                dir::PatternDestructureResolution::Nominal(nominal) => {
                    let arguments: Vec<dir::GlobalTypeId> =
                        dir::GenericArgumentBinding::values(&nominal.key.arguments).collect();
                    let arguments = self.intern_type_ids(&arguments)?;
                    let ty = self.intern_type(dir::Type::Application(dir::GenericApplication {
                        symbol: nominal.key.symbol,
                        arguments,
                    }))?;

                    Ok(Some(ty))
                }
                // object destructures accept the union arm their literal tests select
                dir::PatternDestructureResolution::Object(object) => {
                    match object.adjustments.last() {
                        Some(dir::ReceiverAdjustment::UnionPayload { ty, .. }) => {
                            Ok(Some(self.form_chain(origin, *ty)?.base()))
                        }
                        _ => Ok(None),
                    }
                }
                // tuple destructures accept the tuple of their fields' accepted subsets
                dir::PatternDestructureResolution::Tuple(tuple) => {
                    let mut elements = SmallVec::<[dir::TypeElement; 4]>::new();
                    for field in &tuple.fields {
                        let accepted = match field.pattern {
                            Some(pattern) => self.pattern_node_predicate_target(origin, pattern)?,
                            None => None,
                        };

                        // widen a field without a subset to its whole projected element
                        let ty = match accepted {
                            Some(ty) => ty,
                            None => field.projection.ty(),
                        };
                        elements.push(dir::TypeElement::new(ty));
                    }

                    Ok(Some(self.intern_tuple(&elements)?))
                }
                _ => Ok(None),
            },
            dir::PatternDecision::Must(resolution) => Ok(Some(resolution.ty)),
            dir::PatternDecision::Bind(dir::PatternBindingResolution {
                pattern: Some(nested),
                ..
            })
            | dir::PatternDecision::Default(dir::PatternDefaultResolution {
                pattern: nested,
                ..
            }) => self.pattern_node_predicate_target(origin, *nested),
            dir::PatternDecision::Or(or) => {
                self.or_pattern_predicate_target(origin, pattern, &or.patterns)
            }
            _ => Ok(None),
        }
    }

    /// Return the type subset accepted by one child pattern node.
    fn pattern_node_predicate_target(
        &mut self,
        origin: Origin,
        pattern: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // require a pattern node
        let dir::NodeType::Pattern = pattern.local_id.ty else {
            return Ok(None);
        };

        self.pattern_predicate_target(origin, pattern.into_typed())
    }

    /// Return the union target accepted by one or-pattern.
    fn or_pattern_predicate_target(
        &mut self,
        origin: Origin,
        _pattern: dir::GlobalNodeId<dir::Pattern>,
        branches: &[dir::GlobalNodeIdAny],
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // collect the distinct subset every branch accepts
        let mut targets = Vec::new();
        for branch in branches {
            let Some(target) = self.pattern_node_predicate_target(origin, *branch)? else {
                return Ok(None);
            };
            if !targets.contains(&target) {
                targets.push(target);
            }
        }

        // join the branch subsets
        let target = match targets.as_slice() {
            [] => None,
            [single] => Some(*single),
            _ => Some(self.normalized_union_type(targets)?),
        };

        Ok(target)
    }

    /// Return one single-field object type.
    pub(in crate::sema) fn field_shape_type(
        &mut self,
        key: dir::StaticKey,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read the tested field covariantly
        let property = dir::TypeProperty {
            key,
            access: dir::PropertyAccess::Read(ty),
            is_optional: false,
        };

        // constrain the receiver through the tested field
        self.intern_object(&[property])
    }
}
