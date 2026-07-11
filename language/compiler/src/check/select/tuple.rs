use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, BodyState, Cause, CauseKind, Constraint, Decision, FlowPointId, Origin, Relation,
    answer,
};

impl BodyState<'_, '_> {
    /// Select one tuple pattern, projecting elements by position.
    pub(in crate::check) fn select_tuple_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        scrutinee: dir::GlobalTypeId,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        self.check_pattern_bindings(module, fields)?;

        if !self.check_pattern_rest_fields(module, fields) {
            return self.commit_rejected_pattern(node);
        }

        // reject non-tuple sources before building field projections
        let dir::Type::Tuple(tuple) = self.ty(scrutinee)? else {
            self.report_pattern_source_not_tuple_shaped(origin, scrutinee)?;

            return self.commit_rejected_pattern(node);
        };
        let elements = self
            .tuple_elements(scrutinee.module_id, tuple.elements)?
            .iter()
            .map(|element| element.ty)
            .collect::<SmallVec<[_; 4]>>();

        // project tuple elements into positional holes
        let mut projected = Vec::with_capacity(fields.len());
        let mut position = 0usize;
        for field in fields {
            let target = match self.module(module).view().get(*field) {
                dir::PatternField::Positional { pattern }
                | dir::PatternField::Named {
                    pattern: Some(pattern),
                    ..
                } => Some(pattern.into_global_any(module)),
                dir::PatternField::Named { pattern: None, .. } => {
                    Some(field.into_global_any(module))
                }
                dir::PatternField::Elision => {
                    position += 1;

                    continue;
                }
                _ => continue,
            };
            let Some(projected_value) = elements.get(position).copied() else {
                let key = position.to_string();
                self.report_pattern_field_missing(origin, scrutinee, key)?;
                position += 1;

                continue;
            };

            // flow the positional value into the nested hole
            if let Some(target) = target {
                if target.local_id.ty == dir::NodeType::Pattern {
                    let pattern = dir::LocalNodeId::<dir::Pattern>::new(target.local_id.id);
                    self.project_pattern_input(
                        flow,
                        scope,
                        projected_value,
                        pattern.into_global_any(module),
                    )?;
                } else {
                    let hole = self.require_node_type(target)?;
                    let cause = self
                        .intern_cause(Cause::root(origin, CauseKind::Pattern { pattern: target }));
                    self.push_constraint(Constraint::r#type(
                        Relation::Equal,
                        projected_value,
                        hole,
                        cause,
                    ));
                }
            }
            projected.push(dir::PatternFieldResolution {
                source: field.into_global_any(module),
                projection: dir::Projection::FieldGet {
                    field: dir::ProjectionField::Key(dir::StaticKey::Index(position)),
                    ty: projected_value,
                },
                pattern: target,
            });
            position += 1;
        }

        self.commit_pattern(
            node,
            dir::PatternResolution::Destructure(dir::PatternDestructureResolution::Tuple(
                dir::PatternTupleDestructureResolution { fields: projected },
            )),
        )
    }

    /// Select one tuple assignment pattern.
    pub(in crate::check) fn select_assign_tuple_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::AssignPattern>,
        origin: Origin,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        scrutinee: dir::GlobalTypeId,
        fields: &[dir::LocalNodeId<dir::AssignPatternField>],
    ) -> CompilerResult<Answer<bool>> {
        let module = node.module_id;
        if !self.check_assign_pattern_rest_fields(module, fields) {
            self.commit_decision(node.into_any(), Decision::Rejected)?;

            return Ok(Answer::Ready(false));
        }

        // reject non-tuple sources before projecting fields
        let dir::Type::Tuple(tuple) = self.ty(scrutinee)? else {
            self.report_pattern_source_not_tuple_shaped(origin, scrutinee)?;
            self.commit_decision(node.into_any(), Decision::Rejected)?;

            return Ok(Answer::Ready(false));
        };
        let elements = self
            .tuple_elements(scrutinee.module_id, tuple.elements)?
            .iter()
            .map(|element| element.ty)
            .collect::<SmallVec<[_; 4]>>();

        // project each positional value into its nested assignment target
        let mut projected = Vec::with_capacity(fields.len());
        let mut position = 0usize;
        for field in fields {
            let pattern = match self.module(module).view().get(*field) {
                dir::AssignPatternField::Positional { pattern } => *pattern,
                dir::AssignPatternField::Elision => {
                    position += 1;

                    continue;
                }
                _ => continue,
            };
            let Some(projected_value) = elements.get(position).copied() else {
                let key = position.to_string();
                self.report_pattern_field_missing(origin, scrutinee, key)?;
                position += 1;

                continue;
            };

            self.project_pattern_input(
                flow,
                scope,
                projected_value,
                pattern.into_global_any(module),
            )?;
            projected.push(dir::AssignPatternFieldResolution {
                source: field.into_global_any(module),
                projection: dir::Projection::FieldGet {
                    field: dir::ProjectionField::Key(dir::StaticKey::Index(position)),
                    ty: projected_value,
                },
                pattern: Some(pattern.into_global_any(module)),
            });
            position += 1;
        }

        let () = answer!(self.commit_assign_pattern(
            node,
            dir::AssignPatternResolution::Tuple(dir::AssignPatternTupleResolution {
                fields: projected,
            }),
        )?);

        Ok(Answer::Ready(true))
    }
}
