use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    Answer, CheckState, Constraint, ConstraintId, ConstraintState, Origin, Relation,
    ValueConstraint, ValueSource,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Return implicit coercions derived from held value constraints.
    pub(in crate::check) fn implicit_coercions(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Vec<(dir::GlobalNodeIdAny, dir::Coercion)>> {
        let mut constraints = Vec::new();
        for (id, constraint) in self.solver.constraints.iter() {
            if self.solver.constraints.state(id)? != ConstraintState::Holds {
                continue;
            }
            let Constraint::Value(constraint) = constraint else {
                continue;
            };
            if constraint.relation == Relation::Assignable && constraint.use_.is_stored() {
                constraints.push((id, *constraint));
            }
        }

        // derive at most one exact runtime adjustment path per value node
        let mut coercions = FxIndexMap::default();
        for (id, constraint) in constraints {
            let Some((node, coercion)) = self.value_constraint_coercion(module, id, constraint)?
            else {
                continue;
            };
            match coercions.get(&node) {
                None => {
                    coercions.insert(node, coercion);
                }
                Some(previous) if previous == &coercion => {}
                Some(previous) => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "node {node:?} has conflicting implicit coercions {previous:?} and {coercion:?}"
                        ),
                    });
                }
            }
        }

        Ok(coercions.into_iter().collect())
    }

    /// Return the runtime coercion derived from one held value constraint.
    fn value_constraint_coercion(
        &mut self,
        module: ModuleId,
        id: ConstraintId,
        constraint: ValueConstraint,
    ) -> CompilerResult<Option<(dir::GlobalNodeIdAny, dir::Coercion)>> {
        let (node, origin, source) = match constraint.source {
            ValueSource::Node(node) if node.module_id == module => {
                let site = self.node_site(node)?;
                let declared = self.require_node_type(node)?;
                let source = match self.flow_type_at(site, declared)? {
                    Answer::Ready(source) => source,
                    Answer::Pending(blockers) => {
                        return Err(CompilerError::Internal {
                            message: format!(
                                "implicit coercion source for {node:?} is pending: {blockers:?}"
                            ),
                        });
                    }
                };

                (node, site.origin(), source)
            }
            ValueSource::Node(_) => return Ok(None),
            ValueSource::Type(source) => {
                let origin = self.cause_origin(constraint.cause);
                let Some(node) = origin.expression() else {
                    return Ok(None);
                };
                if node.module_id != module {
                    return Ok(None);
                }

                (node.into_any(), origin, source)
            }
        };

        let Some(value_target) = self.solver.constraints.value_target(id)? else {
            return Err(CompilerError::Internal {
                message: format!("held value constraint {id:?} has no checked target"),
            });
        };

        // settle every judged type before choosing its representation path
        let source = self.settled_root(source)?;
        let source = self.settled_union_root(source)?;
        let value_target = self.settled_root(value_target)?;
        let value_target = self.settled_union_root(value_target)?;
        let target = self.settled_root(constraint.target)?;
        let target = self.settled_union_root(target)?;
        let reduced_source = self.reduce_type_ready(origin, source, "implicit coercion source")?;
        let reduced_value_target =
            self.reduce_type_ready(origin, value_target, "checked value target")?;
        let reduced_target = self.reduce_type_ready(origin, target, "implicit coercion target")?;
        let mut variables = self.type_variables(reduced_source)?;
        variables.extend(self.type_variables(reduced_value_target)?);
        variables.extend(self.type_variables(reduced_target)?);
        if !variables.is_empty() {
            return Err(CompilerError::Internal {
                message: format!(
                    "implicit coercion from '{}' to '{}' retained open variables {variables:?}",
                    self.format_type(source),
                    self.format_type(target),
                ),
            });
        }

        // coercions never create ownership: value forms that reduce away drop
        let value_target = self.coercion_target(value_target, reduced_value_target)?;
        let target = self.coercion_target(target, reduced_target)?;
        let mut adjustments = self
            .implicit_coercion(origin, source, value_target)?
            .map_or_else(Vec::new, |coercion| coercion.adjustments);

        // connect the checked member to its storage carrier
        if let Some(coercion) = self.implicit_coercion(origin, value_target, target)? {
            let current = adjustments
                .last()
                .map_or(source, |adjustment| adjustment.target);
            if current != value_target {
                adjustments.push(dir::CoercionAdjustment {
                    kind: dir::CoercionKind::Direct,
                    target: value_target,
                });
            }
            adjustments.extend(coercion.adjustments);
        }
        if adjustments.is_empty() {
            return Ok(None);
        }
        let coercion = dir::Coercion::new(source, adjustments, dir::CastOrigin::Implicit);

        Ok(Some((node, coercion)))
    }

    /// Return the coercion target, dropping one value form head that reduces away.
    fn coercion_target(
        &self,
        target: dir::GlobalTypeId,
        reduced_target: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let dir::Type::Form(form) = self.ty(target)? else {
            return Ok(target);
        };
        if matches!(self.ty(reduced_target)?, dir::Type::Form(_)) {
            return Ok(target);
        }

        Ok(form.value)
    }

    /// Collapse one union whose elements all settle to the same root.
    fn settled_union_root(&mut self, ty: dir::GlobalTypeId) -> CompilerResult<dir::GlobalTypeId> {
        let dir::Type::Union(union) = self.ty(ty)? else {
            return Ok(ty);
        };
        let elements = self.type_ids(ty.module_id, union.elements)?.to_vec();
        let mut settled = Vec::with_capacity(elements.len());
        for element in elements {
            settled.push(self.settled_root(element)?);
        }

        match settled.as_slice() {
            [first, rest @ ..] if rest.iter().all(|element| element == first) => Ok(*first),
            _ => Ok(ty),
        }
    }

    /// Return the implicit coercion required by one solved source-target pair.
    fn implicit_coercion(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Coercion>> {
        // identical or equal settled types store directly
        if source == target {
            return Ok(None);
        }
        let judged_source = self.coercion_type(origin, source)?;
        let judged_target = self.coercion_type(origin, target)?;
        if judged_source == judged_target
            || self.types_are_equal(origin, judged_source, judged_target)?
        {
            return Ok(None);
        }

        // borrowing retains placement even though representation judging strips it
        if self.borrow_conversion(origin, source, target)?.is_some() {
            return Ok(Some(dir::Coercion::new(
                source,
                vec![dir::CoercionAdjustment {
                    kind: dir::CoercionKind::Borrow,
                    target,
                }],
                dir::CastOrigin::Implicit,
            )));
        }

        // classify the settled heads at the DIR level
        let source_head = self.ty(judged_source)?;
        let target_head = self.ty(judged_target)?;

        // tuples reshape when their element layouts differ
        if let (dir::Type::Tuple(source_tuple), dir::Type::Tuple(target_tuple)) =
            (source_head, target_head)
        {
            let reshapes = self.tuple_layouts_differ(
                judged_source,
                &source_tuple,
                judged_target,
                &target_tuple,
            )?;

            return Ok(reshapes.then(|| {
                dir::Coercion::new(
                    source,
                    vec![dir::CoercionAdjustment {
                        kind: dir::CoercionKind::Carrier,
                        target,
                    }],
                    dir::CastOrigin::Implicit,
                )
            }));
        }

        let Some(kind) = dir::CoercionKind::classify(&source_head, &target_head) else {
            return Ok(None);
        };
        let adjustment = dir::CoercionAdjustment { kind, target };

        Ok(Some(dir::Coercion::new(
            source,
            vec![adjustment],
            dir::CastOrigin::Implicit,
        )))
    }

    /// Return whether two tuple types store their elements differently.
    fn tuple_layouts_differ(
        &self,
        source: dir::GlobalTypeId,
        source_tuple: &dir::TupleType,
        target: dir::GlobalTypeId,
        target_tuple: &dir::TupleType,
    ) -> CompilerResult<bool> {
        let source_elements = self.tuple_elements(source.module_id, source_tuple.elements)?;
        let target_elements = self.tuple_elements(target.module_id, target_tuple.elements)?;
        if source_elements.len() != target_elements.len() {
            return Ok(true);
        }

        // optional slots store as their unioned value, so only rest
        //  slots change the stored layout at equal arity
        let differs = source_elements
            .iter()
            .zip(target_elements)
            .any(|(source, target)| source.is_rest != target.is_rest);

        Ok(differs)
    }

    /// Return the type used to judge coercion representation.
    fn coercion_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut ty = self.reduce_type_ready(origin, ty, "coercion representation")?;
        while let dir::Type::Form(form) = self.ty(ty)?
            && matches!(
                form.form,
                dir::Form::Readonly | dir::Form::Placed { .. } | dir::Form::Managed
            )
        {
            ty = self.reduce_type_ready(origin, form.value, "coercion payload")?;
        }

        Ok(ty)
    }

    /// Return whether two closed types are equal at write time.
    fn types_are_equal(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        match self.decide_relation(origin, Relation::Equal, source, target)? {
            Answer::Ready(holds) => Ok(holds),
            Answer::Pending(blockers) => {
                let source = self.format_type(source);
                let target = self.format_type(target);

                Err(CompilerError::Internal {
                    message: format!(
                        "coercion equality from '{source}' to '{target}' is still pending: {blockers:?}"
                    ),
                })
            }
        }
    }

    /// Return one reduced type or fail when write sees unsolved state.
    fn reduce_type_ready(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        operation: &str,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match self.reduce_type_head(origin, ty)? {
            Answer::Ready(ty) => Ok(ty),
            Answer::Pending(blockers) => {
                let ty = self.format_type(ty);

                Err(CompilerError::Internal {
                    message: format!("{operation} for '{ty}' is still pending: {blockers:?}"),
                })
            }
        }
    }
}
