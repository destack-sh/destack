use destack_dir as dir;

use crate::sema::{
    Check, CheckState, FlowPointChange, FlowPredicate, FlowSite, NarrowingCheck, Origin,
    VariableRole, Widening,
};
use crate::{CompilerError, CompilerResult};

/// The outcome of resolving narrowings at one flow site.
pub(in crate::sema) enum FlowNarrowing {
    /// The narrowed type.
    Narrowed(dir::GlobalTypeId),
    /// No narrowing applies.
    Unchanged,
    /// A consulted equality operation stays undecided behind its selection hole.
    Pending {
        /// The consulted equality operation.
        operation: dir::GlobalNodeIdAny,
        /// The open variable the narrowing stalled on.
        blocker: dir::TypeVariableId,
    },
}

impl CheckState<'_> {
    /// Return one type as viewed at one flow point.
    pub(in crate::sema) fn flow_type_at(
        &mut self,
        site: FlowSite,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // narrow expression occurrences only
        let dir::NodeType::Expression = site.node.local_id.ty else {
            return Ok(ty);
        };

        // require a selected stored access path
        let Some(path) = self
            .module(site.node.module_id)
            .decisions
            .access_resolution(site.node)
            .cloned()
        else {
            return Ok(ty);
        };

        match self.flow_narrowed_type(site, path.path(), ty)? {
            FlowNarrowing::Narrowed(narrowed) => Ok(narrowed),
            FlowNarrowing::Unchanged => Ok(ty),
            // wait for the consulted operation to decide, then re-narrow and close the hole
            FlowNarrowing::Pending { operation, .. } => {
                let hole =
                    self.allocate_variable(site.origin(), Widening::Never, VariableRole::Regular);
                self.register_check(Check::Narrowing(NarrowingCheck {
                    site,
                    path: path.path().clone(),
                    operation,
                    source: ty,
                    hole,
                }));

                self.variable_type(hole)
            }
        }
    }

    /// Return the type after narrowings visible at one flow site.
    pub(in crate::sema) fn flow_narrowed_type(
        &mut self,
        site: FlowSite,
        path: &dir::AccessPath,
        source: dir::GlobalTypeId,
    ) -> CompilerResult<FlowNarrowing> {
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
            return Ok(FlowNarrowing::Unchanged);
        }

        // apply oldest first, so each later test refines the earlier result
        let mut narrowed = source;
        for (tested, predicate) in predicates.into_iter().rev() {
            // wait for a consulted operation to decide
            if let FlowPredicate::Equality { operation, .. } = predicate
                && let Some(blocker) = self.open_operation_hole(operation)?
            {
                return Ok(FlowNarrowing::Pending { operation, blocker });
            }
            // wait for an open variable blocking the narrowing itself
            match self.resolve_flow_predicate(site, path, &tested, narrowed, predicate)? {
                Ok(Some(next)) => narrowed = next,
                Ok(None) => {}
                // require equality behind a narrowing stalled on a structural variable
                Err(blocker) => {
                    let FlowPredicate::Equality { operation, .. } = predicate else {
                        return Err(CompilerError::Internal {
                            message: "non-equality flow predicate stalled on a variable".into(),
                        });
                    };

                    return Ok(FlowNarrowing::Pending { operation, blocker });
                }
            }
        }

        Ok(FlowNarrowing::Narrowed(narrowed))
    }

    /// Return the open hole standing for one undecided operation.
    pub(in crate::sema) fn open_operation_hole(
        &mut self,
        operation: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Option<dir::TypeVariableId>> {
        if self.decision(operation).is_some() {
            return Ok(None);
        }
        let Some(ty) = self.committed_node_type(operation) else {
            return Ok(None);
        };

        self.root_variable(ty)
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
        match predicate {
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
            FlowPredicate::Equality {
                operation,
                is_equal,
            } => self.resolve_equality_predicate(site, path, tested, source, operation, is_equal),
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
            ) && matches!(self.ty(target)?, dir::Type::Null | dir::Type::Undefined)
            {
                let null = self.intern_type(dir::Type::Null)?;
                let undefined = self.intern_type(dir::Type::Undefined)?;
                let elements = self.intern_type_ids(&[null, undefined])?;
                target = self.intern_type(dir::Type::Union(dir::UnionType { elements }))?;
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
        let operand_path = access.path();

        // narrow the value stored at this exact path
        if operand_path == path {
            let narrowed = self.resolve_type_predicate(site, source, target, is_equal)?;

            return Ok(Ok(narrowed));
        }

        // leave paths the operand does not reach untouched
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
        let ty = self.require_node_type(value.into_any())?;
        if self.is_singleton_type(ty)? {
            return Ok(Some(ty));
        }
        if let Some(instance) = self.decompose_newtype(origin, ty)? {
            let backing = instance.backing;
            if self.is_singleton_type(backing)? {
                return Ok(Some(ty));
            }
        }

        Ok(None)
    }

    /// Resolve one runtime type predicate.
    fn resolve_type_predicate(
        &mut self,
        site: FlowSite,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        is_positive: bool,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
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
    fn pattern_predicate_target(
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
        match resolution {
            dir::PatternDecision::Test(test) => Ok(test.predicate.narrowed),
            dir::PatternDecision::Variant(variant) => Ok(variant.predicate.narrowed),
            dir::PatternDecision::Destructure(destructure) => match destructure.as_ref() {
                dir::PatternDestructureResolution::Nominal(nominal) => {
                    let arguments: Vec<dir::GlobalTypeId> =
                        dir::GenericArgumentBinding::values(&nominal.selection.arguments).collect();
                    let arguments = self.intern_type_ids(&arguments)?;
                    let ty = self.intern_type(dir::Type::Application(dir::GenericApplication {
                        symbol: nominal.selection.symbol,
                        arguments,
                    }))?;

                    Ok(Some(ty))
                }
                _ => Ok(None),
            },
            dir::PatternDecision::Must(resolution) => Ok(Some(resolution.ty)),
            dir::PatternDecision::Bind(dir::PatternBindingResolution {
                pattern: Some(inner),
                ..
            })
            | dir::PatternDecision::Default(dir::PatternDefaultResolution {
                pattern: inner, ..
            }) => self.pattern_node_predicate_target(origin, *inner),
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
        let mut targets = Vec::new();
        for branch in branches {
            let Some(target) = self.pattern_node_predicate_target(origin, *branch)? else {
                return Ok(None);
            };
            if !targets.contains(&target) {
                targets.push(target);
            }
        }

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
