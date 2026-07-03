use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    Answer, CheckState, Decision, Dependency, FlowNarrowing, FlowPath, FlowPointChange, FlowSite,
    Origin, WalkState, answer,
};
use crate::{CompilerError, CompilerResult};

/// Runtime flow predicate used to narrow one stable path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum NarrowPredicate {
    /// Keep values assignable to the target type.
    Is(dir::GlobalTypeId),
    /// Keep values not assignable to the target type.
    IsNot(dir::GlobalTypeId),
    /// Keep objects with a known member key.
    Has(dir::StaticKey),
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
        let ty = match self.committed_node_type(site.node)? {
            Answer::Ready(ty) => ty,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        self.flow_type_at(site, ty)
    }

    /// Return one type as viewed at one flow point.
    pub(in crate::check) fn flow_type_at(
        &mut self,
        site: FlowSite,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let dir::NodeType::Expression = site.node.local_id.ty else {
            return Ok(Answer::Ready(ty));
        };

        let node = site.node.into_typed::<dir::Expression>();
        let Some(path) = self.flow_path(node) else {
            return Ok(Answer::Ready(ty));
        };

        match self.flow_narrowing(site, &path, ty)? {
            Answer::Ready(Some(narrowed)) => Ok(Answer::Ready(narrowed)),
            Answer::Ready(None) => Ok(Answer::Ready(ty)),
            Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
        }
    }

    /// Return the narrowing visible for one path at one flow site.
    pub(in crate::check) fn flow_narrowing(
        &mut self,
        site: FlowSite,
        path: &FlowPath,
        source: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let module = self.module(site.node.module_id);
        let flows = &module.flows;
        let mut current = Some(site.flow);

        while let Some(point) = current {
            let Some(flow) = flows.get(point.index()) else {
                return Err(CompilerError::Internal {
                    message: format!("flow point {point:?} is not in module flow table"),
                });
            };

            match &flow.change {
                FlowPointChange::Start => {}
                FlowPointChange::Narrow {
                    path: narrowed,
                    narrowing,
                } if narrowed.as_ref() == path => {
                    return self.resolve_flow_narrowing(site.node, source, *narrowing);
                }
                FlowPointChange::Clear { path: cleared } if path.starts_with(cleared) => {
                    return Ok(Answer::Ready(None));
                }
                FlowPointChange::Narrow { .. } | FlowPointChange::Clear { .. } => {}
            }

            current = flow.parent;
        }

        Ok(Answer::Ready(None))
    }

    /// Return the type named by one flow narrowing.
    fn resolve_flow_narrowing(
        &mut self,
        node: dir::GlobalNodeIdAny,
        source: dir::GlobalTypeId,
        narrowing: FlowNarrowing,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        match narrowing {
            FlowNarrowing::Pattern {
                pattern,
                is_positive,
            } => match self.pattern_narrowing_target(self.node_site(node)?.origin(), pattern)? {
                Answer::Ready(Some(target)) => {
                    self.resolve_type_narrowing(node, source, target, is_positive)
                }
                Answer::Ready(None) => Ok(Answer::Ready(None)),
                Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
            },
            FlowNarrowing::Narrow {
                target,
                is_positive,
            } => self.resolve_type_narrowing(node, source, target, is_positive),
        }
    }

    /// Resolve one runtime type narrowing.
    fn resolve_type_narrowing(
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
        let narrowed = self.intern_type(node.module_id, dir::Type::Operation(operation))?;
        let narrowed = match self.reduce_type_head(self.node_site(node)?.origin(), narrowed)? {
            Answer::Ready(ty) => ty,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        Ok(Answer::Ready(Some(narrowed)))
    }

    /// Return the type subset accepted by one pattern.
    fn pattern_narrowing_target(
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

        self.pattern_resolution_narrowing_target(origin, pattern, &resolution)
    }

    /// Return the type subset accepted by one pattern resolution.
    fn pattern_resolution_narrowing_target(
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
            }) => self.pattern_node_narrowing_target(origin, *inner),
            dir::PatternResolution::Or(or) => {
                self.or_pattern_narrowing_target(origin, pattern, &or.patterns)
            }
            _ => Ok(Answer::Ready(None)),
        }
    }

    /// Return the type subset accepted by one child pattern node.
    fn pattern_node_narrowing_target(
        &mut self,
        origin: Origin,
        pattern: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let dir::NodeType::Pattern = pattern.local_id.ty else {
            return Ok(Answer::Ready(None));
        };

        self.pattern_narrowing_target(origin, pattern.into_typed())
    }

    /// Return the union target accepted by one or-pattern.
    fn or_pattern_narrowing_target(
        &mut self,
        origin: Origin,
        pattern: dir::GlobalNodeId<dir::Pattern>,
        branches: &[dir::GlobalNodeIdAny],
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let mut targets = Vec::new();
        for branch in branches {
            let Some(target) = answer!(self.pattern_node_narrowing_target(origin, *branch)?) else {
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
        source_node: dir::LocalNodeIdAny,
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
    pub(in crate::check) fn narrow_flow_path(&mut self, path: FlowPath, narrowing: FlowNarrowing) {
        self.flow_mut().narrow(path, narrowing);
    }

    /// Narrow one flow path with a runtime predicate.
    pub(in crate::check) fn narrow_flow_path_by(
        &mut self,
        path: FlowPath,
        source_node: dir::LocalNodeIdAny,
        predicate: NarrowPredicate,
    ) -> CompilerResult<()> {
        let narrowing = match predicate {
            // keep matching values
            NarrowPredicate::Is(target) => FlowNarrowing::Narrow {
                target,
                is_positive: true,
            },

            // keep non-matching values
            NarrowPredicate::IsNot(target) => FlowNarrowing::Narrow {
                target,
                is_positive: false,
            },

            // keep objects with the requested key
            NarrowPredicate::Has(key) => {
                let unknown = self.intern_type(dir::Type::Unknown)?;
                let target = self.member_shape_type(key, unknown, source_node)?;
                let predicate = NarrowPredicate::Is(target);

                return self.narrow_flow_path_by(path, source_node, predicate);
            }
        };
        self.narrow_flow_path(path, narrowing);

        Ok(())
    }

    /// Narrow one base flow path from a member predicate.
    pub(in crate::check) fn narrow_base_flow_path_by_member(
        &mut self,
        path: FlowPath,
        source_node: dir::LocalNodeIdAny,
        key: dir::StaticKey,
        predicate: NarrowPredicate,
    ) -> CompilerResult<()> {
        let predicate = match predicate {
            // keep parent values with a matching member type
            NarrowPredicate::Is(ty) => {
                let target = self.member_shape_type(key, ty, source_node)?;

                NarrowPredicate::Is(target)
            }

            // keep parent values without a matching member type
            NarrowPredicate::IsNot(ty) => {
                let target = self.member_shape_type(key, ty, source_node)?;

                NarrowPredicate::IsNot(target)
            }

            // keep parent values with a member that has the nested key
            NarrowPredicate::Has(member_key) => {
                let unknown = self.intern_type(dir::Type::Unknown)?;
                let member = self.member_shape_type(member_key, unknown, source_node)?;
                let target = self.member_shape_type(key, member, source_node)?;

                NarrowPredicate::Is(target)
            }
        };

        self.narrow_flow_path_by(path, source_node, predicate)
    }

    /// Return one single-field structural shape type.
    fn member_shape_type(
        &mut self,
        key: dir::StaticKey,
        ty: dir::GlobalTypeId,
        source_node: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.check
            .member_shape_type(self.module, key, ty, source_node)
    }

    /// Clear flow narrowings invalidated by mutating an expression.
    pub(in crate::check) fn clear_mutated_expression_narrowings(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
    ) {
        // ignore expressions without stable flow paths
        let Some(path) = self.flow_path(id) else {
            return;
        };

        // clear all dependent narrowings
        self.flow_mut().clear_narrowings_under(&path);
    }
}
