use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckState, Condition, Constraint, ConstraintCause, Decision, Dependency, MemberLookup,
    Origin, Relation,
};
use crate::{CheckError, CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Select the pattern meaning of one pattern node.
    ///
    /// Every pattern node carries its own scrutinee component: the match
    /// site flowed the value into the root, and each parent selection
    /// projects components into its nested pattern holes.
    pub(in crate::check) fn select_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let origin = Origin::Node(node.into_any());

        // read each shape's payload once and dispatch with it
        let view = self.module(module).view();
        match view.get(node.local_id) {
            // _
            dir::Pattern::Wildcard => self.record_pattern(node, dir::PatternResolution::Wildcard),

            // name, name: pattern
            dir::Pattern::Binding { pattern, .. } => {
                let pattern = *pattern;
                let symbol = self
                    .module(module)
                    .declaration_symbol(node.local_id.into_any());

                self.record_pattern(
                    node,
                    dir::PatternResolution::Binding(dir::PatternBindingResolution {
                        symbol,
                        pattern: pattern.map(|pattern| pattern.into_global_any(module)),
                    }),
                )
            }

            // pattern!, pattern = value
            dir::Pattern::Must(pattern) => {
                let pattern = *pattern;

                self.record_pattern(
                    node,
                    dir::PatternResolution::Binding(dir::PatternBindingResolution {
                        symbol: None,
                        pattern: Some(pattern.into_global_any(module)),
                    }),
                )
            }
            dir::Pattern::Assign { pattern, .. } => {
                let pattern = *pattern;

                self.record_pattern(
                    node,
                    dir::PatternResolution::Binding(dir::PatternBindingResolution {
                        symbol: None,
                        pattern: Some(pattern.into_global_any(module)),
                    }),
                )
            }

            // &pattern, ^pattern, *pattern
            dir::Pattern::BorrowOf { right, .. } => {
                let right = *right;

                self.record_pattern(
                    node,
                    dir::PatternResolution::Borrow(dir::PatternBorrowResolution {
                        access: None,
                        pattern: right.into_global_any(module),
                    }),
                )
            }
            dir::Pattern::MoveOf { right, .. } => {
                let right = *right;

                self.record_pattern(
                    node,
                    dir::PatternResolution::Move(dir::PatternMoveResolution {
                        access: None,
                        pattern: right.into_global_any(module),
                    }),
                )
            }
            dir::Pattern::DereferenceOf { right } => {
                let right = *right;

                self.record_pattern(
                    node,
                    dir::PatternResolution::Dereference(dir::PatternDereferenceResolution {
                        pattern: right.into_global_any(module),
                    }),
                )
            }

            // 1, "ready"
            dir::Pattern::Expression { value } => {
                let value = *value;

                self.select_literal_pattern(node, origin, value)
            }

            // value is T
            dir::Pattern::TypeExpression { .. } => {
                // TODO(check): record a dedicated type-test resolution once
                // DIR grows one; coverage still checks through node types.
                self.record_pattern(node, dir::PatternResolution::Wildcard)
            }

            // start..end
            dir::Pattern::Range {
                start,
                end,
                end_kind,
            } => {
                let (start, end, end_kind) = (*start, *end, *end_kind);

                self.select_range_pattern(node, origin, start, end, end_kind)
            }

            // (a, b)
            dir::Pattern::Tuple { fields } => {
                let fields = fields.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.select_tuple_pattern(node, origin, &fields)
            }

            // [a, b, ...rest]
            dir::Pattern::Sequence { fields } => {
                let fields = fields.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.select_sequence_pattern(node, origin, &fields)
            }

            // { name, nested: pattern }
            dir::Pattern::Object { fields } => {
                let fields = fields.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.select_object_pattern(node, origin, &fields)
            }

            // T(value), T { name }
            dir::Pattern::Newtype { ty, fields } => {
                let (ty, fields) = (*ty, fields.iter().copied().collect::<SmallVec<[_; 4]>>());

                self.select_newtype_pattern(node, origin, ty, &fields)
            }
            dir::Pattern::NominalObject { ty, fields } => {
                let (ty, fields) = (*ty, fields.iter().copied().collect::<SmallVec<[_; 4]>>());

                self.select_nominal_pattern(node, origin, ty, &fields)
            }

            // a | b
            dir::Pattern::Union { patterns } => {
                let patterns = patterns.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.select_union_pattern(node, origin, &patterns)
            }
        }
    }

    /// Select one literal pattern from its closed expression value.
    fn select_literal_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let value_node = value.into_global_any(module);
        let Some(ty) = self.inputs.node_type(value_node) else {
            return Err(CompilerError::Internal {
                message: format!("pattern expression {value_node:?} has no input type"),
            });
        };
        let ty = match self.evaluate_root(origin, ty)? {
            Answer::Ready(ty) => ty,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        // closed literal values select literal patterns
        let literal = match self.ty(ty)? {
            dir::Type::Literal(literal) => Some(*literal),
            dir::Type::Null => Some(dir::ScalarLiteral::Null),
            dir::Type::Undefined => Some(dir::ScalarLiteral::Undefined),
            _ => None,
        };
        match literal {
            Some(value) => self.record_pattern(
                node,
                dir::PatternResolution::Literal(dir::PatternLiteralResolution { value }),
            ),
            // non-literal comparisons match through their value types
            None => {
                self.report_invalid_control_flow(
                    module,
                    node.local_id.into_any(),
                    "expression pattern does not close to a literal",
                );

                self.record_pattern(node, dir::PatternResolution::Wildcard)
            }
        }
    }

    /// Select one range pattern from its closed bounds.
    fn select_range_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        start: Option<dir::LocalNodeId<dir::Expression>>,
        end: Option<dir::LocalNodeId<dir::Expression>>,
        end_kind: dir::RangeEnd,
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;

        // close both written bounds to literals
        let mut bounds = [None, None];
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for (slot, bound) in [start, end].into_iter().enumerate() {
            let Some(bound) = bound else {
                continue;
            };
            let bound_node = bound.into_global_any(module);
            let Some(ty) = self.inputs.node_type(bound_node) else {
                return Err(CompilerError::Internal {
                    message: format!("range bound {bound_node:?} has no input type"),
                });
            };
            let ty = match self.evaluate_root(origin, ty)? {
                Answer::Ready(ty) => ty,
                Answer::Pending(dependencies) => {
                    blockers.extend(dependencies);

                    continue;
                }
            };
            if let dir::Type::Literal(literal) = self.ty(ty)? {
                bounds[slot] = Some(*literal);
            }
        }
        if !blockers.is_empty() {
            return Ok(Answer::pending(blockers));
        }

        // the matched scalar domain is the pattern's own component
        let Some(domain) = self.inputs.node_type(node.into_any()) else {
            return Err(CompilerError::Internal {
                message: format!("range pattern {node:?} has no input type"),
            });
        };

        self.record_pattern(
            node,
            dir::PatternResolution::Range(dir::PatternRangeResolution {
                domain,
                start: bounds[0],
                end: bounds[1],
                end_bound: end_kind,
            }),
        )
    }

    /// Select one tuple pattern, projecting elements by position.
    fn select_tuple_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let scrutinee = match self.pattern_component(node, origin)? {
            Answer::Ready(scrutinee) => scrutinee,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        // project tuple elements into positional holes
        let elements = match self.ty(scrutinee)? {
            dir::Type::Tuple(tuple) => tuple
                .elements
                .iter()
                .map(|element| element.ty)
                .collect::<SmallVec<[_; 4]>>(),
            _ => SmallVec::new(),
        };

        let mut rows = Vec::with_capacity(fields.len());
        let mut position = 0usize;
        for field in fields {
            let (pattern, target) = match self.module(module).view().get(*field) {
                dir::PatternField::Positional { pattern } => {
                    (Some(*pattern), dir::PatternFieldTarget::Index(position))
                }
                dir::PatternField::Elision => {
                    position += 1;

                    continue;
                }
                _ => continue,
            };

            // flow the positional component into the nested hole
            if let (Some(pattern), Some(component)) = (pattern, elements.get(position).copied()) {
                self.project_pattern_component(origin, module, component, pattern)?;
            }
            rows.push(dir::PatternFieldResolution {
                source: field.into_global_any(module),
                target,
                pattern: pattern.map(|pattern| pattern.into_global_any(module)),
            });
            position += 1;
        }

        self.record_pattern(
            node,
            dir::PatternResolution::Tuple(dir::PatternTupleResolution { fields: rows }),
        )
    }

    /// Select one sequence pattern, projecting shared elements.
    fn select_sequence_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let scrutinee = match self.pattern_component(node, origin)? {
            Answer::Ready(scrutinee) => scrutinee,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        // every sequence form shares one element component
        let source_node = node.local_id.into_any();
        let (element, rest_type, fixed_length) = match self.ty(scrutinee)?.clone() {
            dir::Type::Array(array) => {
                let rest = self.push_type(
                    module,
                    dir::Type::Array(dir::ArrayType {
                        element: array.element,
                    }),
                    source_node,
                )?;

                (Some(array.element), Some(rest), None)
            }
            dir::Type::Slice(slice) => {
                let rest = self.push_type(
                    module,
                    dir::Type::Slice(dir::SliceType {
                        element: slice.element,
                    }),
                    source_node,
                )?;

                (Some(slice.element), Some(rest), None)
            }
            dir::Type::FixedArray(array) => {
                let rest = self.push_type(
                    module,
                    dir::Type::Slice(dir::SliceType {
                        element: array.element,
                    }),
                    source_node,
                )?;

                (Some(array.element), Some(rest), Some(array.count))
            }
            _ => (None, None, None),
        };

        // project elements and the rest binding
        let mut rows = Vec::with_capacity(fields.len());
        let mut rest = None;
        let mut position = 0usize;
        for field in fields {
            match self.module(module).view().get(*field) {
                dir::PatternField::Positional { pattern } => {
                    let pattern = *pattern;
                    if let Some(element) = element {
                        self.project_pattern_component(origin, module, element, pattern)?;
                    }
                    rows.push(dir::PatternFieldResolution {
                        source: field.into_global_any(module),
                        target: dir::PatternFieldTarget::Index(position),
                        pattern: Some(pattern.into_global_any(module)),
                    });
                    position += 1;
                }
                dir::PatternField::Spread { pattern } => {
                    let pattern = *pattern;
                    if let (Some(pattern), Some(rest_type)) = (pattern, rest_type) {
                        self.project_pattern_component(origin, module, rest_type, pattern)?;
                    }
                    rest = Some(dir::PatternRestResolution {
                        source: field.into_global_any(module),
                        pattern: pattern.map(|pattern| pattern.into_global_any(module)),
                    });
                }
                dir::PatternField::Elision => {
                    position += 1;
                }
                _ => {}
            }
        }

        // record the sequence form behind the scrutinee
        let resolution = match (self.ty(scrutinee)?, fixed_length) {
            (dir::Type::Slice(_), _) => {
                dir::PatternSequenceResolution::Slice { fields: rows, rest }
            }
            (dir::Type::FixedArray(_), Some(length)) => {
                dir::PatternSequenceResolution::FixedArray {
                    fields: rows,
                    length,
                }
            }
            _ => dir::PatternSequenceResolution::Array { fields: rows, rest },
        };

        self.record_pattern(node, dir::PatternResolution::Sequence(resolution))
    }

    /// Select one object pattern, projecting fields by key.
    fn select_object_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Answer<()>> {
        let scrutinee = match self.pattern_component(node, origin)? {
            Answer::Ready(scrutinee) => scrutinee,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        let rows = match self.project_named_fields(node, origin, scrutinee, fields)? {
            Answer::Ready(rows) => rows,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        self.record_pattern(
            node,
            dir::PatternResolution::Shape(dir::PatternShapeResolution { fields: rows }),
        )
    }

    /// Select one newtype pattern, unwrapping the substituted backing.
    fn select_newtype_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;

        // close the written nominal tag
        let tag_node = ty.into_global_any(module);
        let Some(tag) = self.inputs.node_type(tag_node) else {
            return Err(CompilerError::Internal {
                message: format!("newtype pattern tag {tag_node:?} has no input type"),
            });
        };
        let tag = match self.evaluate_root(origin, tag)? {
            Answer::Ready(tag) => tag,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let instance = match self.ty(tag)? {
            dir::Type::Reference(instance) => instance.clone(),
            _ => return self.reject_pattern(node, origin, tag),
        };

        // unwrap the substituted newtype backing
        let backing = match self.definition(instance.symbol) {
            Some(dir::Definition::Newtype(definition)) => definition.value,
            _ => return self.reject_pattern(node, origin, tag),
        };
        let substitution = self.parameter_substitution(&instance)?.with_receiver(tag);
        let source = self.origin_source_node(origin)?;
        let backing = if substitution.is_empty() {
            backing
        } else {
            self.fold_type(module, source, backing, substitution.rewrite())?
        };

        // flow the backing into the wrapped hole
        let value = fields
            .first()
            .and_then(|field| match self.module(module).view().get(*field) {
                dir::PatternField::Positional { pattern } => Some(*pattern),
                _ => None,
            });
        if let Some(value) = value {
            self.project_pattern_component(origin, module, backing, value)?;
        }

        self.record_pattern(
            node,
            dir::PatternResolution::Newtype(dir::PatternNewtypeResolution {
                symbol: instance.symbol,
                arguments: instance.arguments.clone(),
                value: value.map(|value| value.into_global_any(module)),
            }),
        )
    }

    /// Select one nominal object pattern, projecting declared fields.
    fn select_nominal_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;

        // close the written nominal tag
        let tag_node = ty.into_global_any(module);
        let Some(tag) = self.inputs.node_type(tag_node) else {
            return Err(CompilerError::Internal {
                message: format!("nominal pattern tag {tag_node:?} has no input type"),
            });
        };
        let tag = match self.evaluate_root(origin, tag)? {
            Answer::Ready(tag) => tag,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let instance = match self.ty(tag)? {
            dir::Type::Reference(instance) => instance.clone(),
            _ => return self.reject_pattern(node, origin, tag),
        };

        // project the declared fields off the matched declaration
        let rows = match self.project_named_fields(node, origin, tag, fields)? {
            Answer::Ready(rows) => rows,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        self.record_pattern(
            node,
            dir::PatternResolution::Nominal(dir::PatternNominalResolution {
                symbol: instance.symbol,
                arguments: instance.arguments.clone(),
                fields: rows,
            }),
        )
    }

    /// Select one union pattern, sharing the component across alternatives.
    fn select_union_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        patterns: &[dir::LocalNodeId<dir::Pattern>],
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let Some(component) = self.inputs.node_type(node.into_any()) else {
            return Err(CompilerError::Internal {
                message: format!("union pattern {node:?} has no input type"),
            });
        };

        // every alternative matches the same component
        for pattern in patterns {
            self.project_pattern_component(origin, module, component, *pattern)?;
        }

        self.record_pattern(
            node,
            dir::PatternResolution::Union(dir::PatternUnionResolution {
                alternatives: patterns
                    .iter()
                    .map(|pattern| pattern.into_global_any(module))
                    .collect(),
            }),
        )
    }

    /// Project named pattern fields off one closed owner component.
    fn project_named_fields(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        owner: dir::GlobalTypeId,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Answer<Vec<dir::PatternFieldResolution>>> {
        let module = node.module_id;

        let mut rows = Vec::with_capacity(fields.len());
        for field in fields {
            let (name, pattern) = match self.module(module).view().get(*field) {
                dir::PatternField::Named { name, pattern, .. } => (*name, *pattern),
                // spreads and computed keys keep the whole component
                _ => continue,
            };
            let key = name.static_key();

            // look the component up on the owner type
            let lookup =
                self.lookup_member(origin, module, owner, dir::MemberSpace::Instance, key)?;
            let component = match lookup {
                MemberLookup::Field(ty) => Some(ty),
                MemberLookup::Found(candidates) => candidates.first().map(|candidate| candidate.ty),
                MemberLookup::Missing => None,
                MemberLookup::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            };
            let Some(component) = component else {
                let key = self.format_static_key(&key);
                self.report_missing_member(origin, owner, key)?;

                continue;
            };

            // flow the component into the nested or shorthand hole
            match pattern {
                Some(pattern) => {
                    self.project_pattern_component(origin, module, component, pattern)?;
                }
                // shorthand fields bind through their own field node
                None => {
                    let field_node = field.into_global_any(module);
                    if let Some(hole) = self.inputs.node_type(field_node) {
                        self.push_constraint(Constraint {
                            relation: Relation::Assignable,
                            left: component,
                            right: hole,
                            origin,
                            condition: Condition::Always,
                            cause: ConstraintCause::General,
                        });
                    }
                }
            }
            rows.push(dir::PatternFieldResolution {
                source: field.into_global_any(module),
                target: dir::PatternFieldTarget::Key(key),
                pattern: pattern.map(|pattern| pattern.into_global_any(module)),
            });
        }

        Ok(Answer::Ready(rows))
    }

    /// Return one pattern node's closed scrutinee component.
    fn pattern_component(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let Some(ty) = self.inputs.node_type(node.into_any()) else {
            return Err(CompilerError::Internal {
                message: format!("pattern {node:?} has no input type"),
            });
        };

        self.evaluate_root(origin, ty)
    }

    /// Flow one projected component into a nested pattern hole.
    fn project_pattern_component(
        &mut self,
        origin: Origin,
        module: destack_source::ModuleId,
        component: dir::GlobalTypeId,
        pattern: dir::LocalNodeId<dir::Pattern>,
    ) -> CompilerResult<()> {
        let pattern_node = pattern.into_global_any(module);
        let Some(hole) = self.inputs.node_type(pattern_node) else {
            return Err(CompilerError::Internal {
                message: format!("pattern hole {pattern_node:?} has no input type"),
            });
        };

        self.push_constraint(Constraint {
            relation: Relation::Assignable,
            left: component,
            right: hole,
            origin,
            condition: Condition::Always,
            cause: ConstraintCause::General,
        });

        Ok(())
    }

    /// Record one pattern decision.
    fn record_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        resolution: dir::PatternResolution,
    ) -> CompilerResult<Answer<()>> {
        self.record_decision(node.into_any(), Decision::Pattern(resolution))?;

        Ok(Answer::Ready(()))
    }

    /// Reject one pattern whose tag is not nominal.
    fn reject_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        tag: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        let ty = self.format_type(tag);
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::InvalidPatternTag { anchor, module, ty };
        self.module_mut(module).diagnostics.push(error.into());
        self.record_decision(node.into_any(), Decision::Rejected)?;

        Ok(Answer::Ready(()))
    }
}
