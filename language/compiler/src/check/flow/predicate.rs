use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    Answer, CheckState, Decision, Dependency, FlowPath, FlowPointChange, FlowPredicate, FlowSite,
    Origin, WalkState, answer,
};
use crate::{CompilerError, CompilerResult};

/// Direct predicate applied to one stable path during walking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum PathPredicate {
    /// Keep values assignable to the target type.
    Is(dir::GlobalTypeId),
    /// Keep values not assignable to the target type.
    IsNot(dir::GlobalTypeId),
}

impl CheckState<'_> {
    /// Return the stable flow path for one expression.
    pub(in crate::check) fn flow_path(
        &self,
        node: dir::GlobalNodeId<dir::Expression>,
    ) -> Option<FlowPath> {
        let module = self.module(node.module_id);
        let view = module.view();
        let id = node.local_id;

        match view.get(id) {
            // value
            dir::Expression::Identifier { .. } => {
                let reference = module
                    .resolved
                    .references
                    .get(node.into_any())?;
                let symbol = match reference {
                    dir::Reference::Bound(symbols) => {
                        let symbols = self.present_symbols(symbols);
                        match symbols.as_slice() {
                            [symbol] => *symbol,
                            _ => return None,
                        }
                    }
                    dir::Reference::Missing
                    | dir::Reference::Namespace(_)
                    | dir::Reference::Projected { .. }
                    | dir::Reference::Ambiguous(_) => return None,
                };

                Some(FlowPath::symbol(symbol))
            }
            // this
            dir::Expression::This => Some(FlowPath::receiver(dir::ReceiverKind::This)),
            // value.member
            dir::Expression::Member {
                left,
                name: Some(name),
            }
            // value.#member
            | dir::Expression::PrivateMember {
                left,
                name: Some(name),
            } => {
                // extend root path with the member key
                let left = left.into_global(node.module_id);
                let mut path = self.flow_path(left)?;
                path.push_segment(dir::StaticKey::Name(*name));

                Some(path)
            }
            // value[index]
            dir::Expression::Index {
                left,
                index: Some(index),
                ..
            } => {
                // extend root path with static index key
                let key = view.get(*index).static_key()?;
                let left = left.into_global(node.module_id);
                let mut path = self.flow_path(left)?;

                path.push_segment(key);

                Some(path)
            }
            // (value)
            dir::Expression::Parenthesized { expression } => {
                self.flow_path(expression.into_global(node.module_id))
            }
            // not a stable flow path
            _ => None,
        }
    }

    /// Return one source node type as viewed through one flow site.
    pub(in crate::check) fn node_type_at(
        &mut self,
        site: FlowSite,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let ty = answer!(self.node_type(site.node)?);

        self.flow_type_at(site, ty)
    }

    /// Return one type as viewed at one flow point.
    pub(in crate::check) fn flow_type_at(
        &mut self,
        site: FlowSite,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        // only expression occurrences participate in flow predicates
        let dir::NodeType::Expression = site.node.local_id.ty else {
            return Ok(Answer::Ready(ty));
        };

        // only stable paths can carry durable flow information
        let node = site.node.into_typed::<dir::Expression>();
        let Some(path) = self.flow_path(node) else {
            return Ok(Answer::Ready(ty));
        };

        match self.flow_narrowed_type(site, &path, ty)? {
            Answer::Ready(Some(narrowed)) => Ok(Answer::Ready(narrowed)),
            Answer::Ready(None) => Ok(Answer::Ready(ty)),
            Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
        }
    }

    /// Return the type after predicates visible at one flow site.
    pub(in crate::check) fn flow_narrowed_type(
        &mut self,
        site: FlowSite,
        path: &FlowPath,
        source: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let module = self.module(site.node.module_id);
        let flows = &module.flows;

        // collect every predicate of the path back to its last clear
        let mut predicates = Vec::new();
        let mut current = Some(site.flow);
        while let Some(point) = current {
            let Some(flow) = flows.get(point.index()) else {
                return Err(CompilerError::Internal {
                    message: format!("flow point {point:?} is not in module flow table"),
                });
            };

            // keep matching path predicates
            match &flow.change {
                FlowPointChange::Start => {}
                FlowPointChange::Predicate {
                    path: narrowed,
                    predicate,
                } if narrowed.as_ref() == path => {
                    predicates.push(*predicate);
                }
                FlowPointChange::Clear { path: cleared } if path.starts_with(cleared) => {
                    break;
                }
                FlowPointChange::Predicate { .. } | FlowPointChange::Clear { .. } => {}
            }

            // move toward the entry flow
            current = flow.parent;
        }

        // no predicate affects this path
        if predicates.is_empty() {
            return Ok(Answer::Ready(None));
        }

        // apply oldest first, so each later test refines the earlier result
        let mut narrowed = source;
        for predicate in predicates.into_iter().rev() {
            match self.resolve_flow_predicate(site.node, narrowed, predicate)? {
                Answer::Ready(Some(next)) => narrowed = next,
                Answer::Ready(None) => {}
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            }
        }

        Ok(Answer::Ready(Some(narrowed)))
    }

    /// Apply one flow predicate to a source type.
    fn resolve_flow_predicate(
        &mut self,
        node: dir::GlobalNodeIdAny,
        source: dir::GlobalTypeId,
        predicate: FlowPredicate,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        match predicate {
            FlowPredicate::Pattern {
                pattern,
                is_positive,
            } => {
                // resolve pattern decisions into the type accepted by the pattern
                let origin = self.node_site(node)?.origin();
                match self.pattern_predicate_target(origin, pattern)? {
                    Answer::Ready(Some(target)) => {
                        self.resolve_type_predicate(node, source, target, is_positive)
                    }
                    Answer::Ready(None) => Ok(Answer::Ready(None)),
                    Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
                }
            }
            FlowPredicate::Type {
                target,
                is_positive,
            } => self.resolve_type_predicate(node, source, target, is_positive),
            FlowPredicate::Guard { guard, is_positive } => {
                // resolve guard predicates through their selected guard decision
                let origin = self.node_site(node)?.origin();
                match self.guard_predicate_target(origin, guard)? {
                    Answer::Ready(Some(target)) => {
                        self.resolve_type_predicate(node, source, target, is_positive)
                    }
                    Answer::Ready(None) => Ok(Answer::Ready(None)),
                    Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
                }
            }
        }
    }

    /// Resolve one runtime type predicate.
    fn resolve_type_predicate(
        &mut self,
        node: dir::GlobalNodeIdAny,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        is_positive: bool,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let operation = dir::TypeOperation::Narrow(dir::NarrowType {
            source,
            target,
            is_positive,
        });

        // reduce the synthetic predicate operation through the normal reducer
        let narrowed = self.intern_type(node.module_id, dir::Type::Operation(operation))?;
        let narrowed = match self.reduce_type_head(self.node_site(node)?.origin(), narrowed)? {
            Answer::Ready(ty) => ty,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        Ok(Answer::Ready(Some(narrowed)))
    }

    /// Return the target type tested by one selected guard expression.
    fn guard_predicate_target(
        &mut self,
        origin: Origin,
        guard: dir::GlobalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let Some(decision) = self.decision(guard.into_any()).cloned() else {
            return Ok(Answer::pending([Dependency::Decision(guard.into_any())]));
        };

        let Decision::Guard(resolution) = decision else {
            return Ok(Answer::Ready(None));
        };

        match resolution {
            // type guards narrow both branches against the selected target
            dir::GuardResolution::Is(predicate) => Ok(Answer::Ready(Some(predicate.target_type))),

            // class guards narrow both branches against the selected instance type
            dir::GuardResolution::InstanceOf(predicate) => {
                Ok(Answer::Ready(Some(predicate.target_type)))
            }

            // static membership guards narrow through the tested shape
            dir::GuardResolution::In(predicate) => {
                self.membership_predicate_target(origin, &predicate)
            }
        }
    }

    /// Return the static receiver shape tested by one membership guard.
    fn membership_predicate_target(
        &mut self,
        origin: Origin,
        predicate: &dir::InGuardResolution,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let dir::PredicateTest::Membership(test) = &predicate.predicate.test else {
            return Ok(Answer::Ready(None));
        };
        let dir::PredicateKey::Static(key) = test.key else {
            return Ok(Answer::Ready(None));
        };

        let unknown = self.intern_type(origin.module(), dir::Type::Unknown)?;
        let target = self.member_shape_type(origin.module(), key, unknown)?;

        Ok(Answer::Ready(Some(target)))
    }

    /// Return the type subset accepted by one pattern.
    fn pattern_predicate_target(
        &mut self,
        origin: Origin,
        pattern: dir::GlobalNodeId<dir::Pattern>,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let Some(decision) = self.decision(pattern.into_any()).cloned() else {
            return Ok(Answer::pending([Dependency::Decision(pattern.into_any())]));
        };

        let Decision::Pattern(resolution) = decision else {
            return Ok(Answer::Ready(None));
        };

        self.pattern_resolution_predicate_target(origin, pattern, &resolution)
    }

    /// Return the type subset accepted by one pattern resolution.
    fn pattern_resolution_predicate_target(
        &mut self,
        origin: Origin,
        pattern: dir::GlobalNodeId<dir::Pattern>,
        resolution: &dir::PatternResolution,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        match resolution {
            dir::PatternResolution::Test(test) => Ok(Answer::Ready(test.predicate.narrowed)),
            dir::PatternResolution::Destructure(dir::PatternDestructureResolution::Nominal(
                nominal,
            )) => {
                let arguments: Vec<dir::GlobalTypeId> =
                    dir::GenericArgumentBinding::values(&nominal.generic_arguments).collect();
                let arguments = self.intern_type_ids(origin.module(), &arguments)?;
                let ty = self.intern_type(
                    origin.module(),
                    dir::Type::Instance(dir::GenericInstance {
                        symbol: nominal.symbol,
                        arguments,
                    }),
                )?;

                Ok(Answer::Ready(Some(ty)))
            }
            dir::PatternResolution::Destructure(dir::PatternDestructureResolution::Variant(
                variant,
            )) => Ok(Answer::Ready(variant.predicate.narrowed)),
            dir::PatternResolution::Bind(dir::PatternBindingResolution {
                pattern: Some(inner),
                ..
            })
            | dir::PatternResolution::Must(dir::PatternMustResolution { pattern: inner })
            | dir::PatternResolution::Default(dir::PatternDefaultResolution {
                pattern: inner,
                ..
            }) => self.pattern_node_predicate_target(origin, *inner),
            dir::PatternResolution::Or(or) => {
                self.or_pattern_predicate_target(origin, pattern, &or.patterns)
            }
            _ => Ok(Answer::Ready(None)),
        }
    }

    /// Return the type subset accepted by one child pattern node.
    fn pattern_node_predicate_target(
        &mut self,
        origin: Origin,
        pattern: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let dir::NodeType::Pattern = pattern.local_id.ty else {
            return Ok(Answer::Ready(None));
        };

        self.pattern_predicate_target(origin, pattern.into_typed())
    }

    /// Return the union target accepted by one or-pattern.
    fn or_pattern_predicate_target(
        &mut self,
        origin: Origin,
        pattern: dir::GlobalNodeId<dir::Pattern>,
        branches: &[dir::GlobalNodeIdAny],
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let mut targets = Vec::new();
        for branch in branches {
            let Some(target) = answer!(self.pattern_node_predicate_target(origin, *branch)?) else {
                return Ok(Answer::Ready(None));
            };
            if !targets.contains(&target) {
                targets.push(target);
            }
        }

        let target = match targets.as_slice() {
            [] => None,
            [single] => Some(*single),
            _ => Some(self.normalized_union_type(pattern.module_id, targets)?),
        };

        Ok(Answer::Ready(target))
    }

    /// Return one single-field structural shape type.
    pub(in crate::check) fn member_shape_type(
        &mut self,
        module: ModuleId,
        key: dir::StaticKey,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let field = dir::TypeField {
            key,
            ty,
            is_optional: false,
            is_readonly: false,
        };
        let fields = self.intern_fields(module, &[field])?;
        let shape = dir::ShapeType {
            fields,
            call_signatures: dir::TypeListId::EMPTY,
            construct_signatures: dir::TypeListId::EMPTY,
            index_signatures: dir::TypeListId::EMPTY,
        };

        self.intern_type(module, dir::Type::Shape(shape))
    }
}

impl WalkState<'_, '_> {
    /// Return the stable flow path for one expression.
    pub(in crate::check) fn flow_path(
        &self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<FlowPath> {
        self.check.flow_path(id.into_global(self.module))
    }

    /// Narrow one flow path.
    pub(in crate::check) fn apply_flow_predicate(
        &mut self,
        path: FlowPath,
        predicate: FlowPredicate,
    ) {
        self.flow_mut().apply_predicate(path, predicate);
    }

    /// Narrow one flow path with a runtime predicate.
    pub(in crate::check) fn apply_path_predicate(
        &mut self,
        path: FlowPath,
        predicate: PathPredicate,
    ) -> CompilerResult<()> {
        let predicate = match predicate {
            // keep matching values
            PathPredicate::Is(target) => FlowPredicate::Type {
                target,
                is_positive: true,
            },

            // keep non-matching values
            PathPredicate::IsNot(target) => FlowPredicate::Type {
                target,
                is_positive: false,
            },
        };
        self.apply_flow_predicate(path, predicate);

        Ok(())
    }

    /// Narrow one base flow path from a member predicate.
    pub(in crate::check) fn apply_member_path_predicate(
        &mut self,
        path: FlowPath,
        key: dir::StaticKey,
        predicate: PathPredicate,
    ) -> CompilerResult<()> {
        let predicate = match predicate {
            // keep parent values with a matching member type
            PathPredicate::Is(ty) => {
                let target = self.member_shape_type(key, ty)?;

                PathPredicate::Is(target)
            }

            // keep parent values without a matching member type
            PathPredicate::IsNot(ty) => {
                let target = self.member_shape_type(key, ty)?;

                PathPredicate::IsNot(target)
            }
        };

        self.apply_path_predicate(path, predicate)
    }

    /// Return one single-field structural shape type.
    fn member_shape_type(
        &mut self,
        key: dir::StaticKey,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.check.member_shape_type(self.module, key, ty)
    }

    /// Clear flow predicates invalidated by mutating an expression.
    pub(in crate::check) fn clear_mutated_expression_predicates(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
    ) {
        // ignore expressions without stable flow paths
        let Some(path) = self.flow_path(id) else {
            return;
        };

        // clear all dependent predicates
        self.flow_mut().clear_predicates_under(&path);
    }
}
