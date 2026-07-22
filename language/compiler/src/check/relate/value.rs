use destack_dir as dir;

use crate::check::{
    Answer, CauseId, CheckOutcome, CheckState, Dependency, Origin, Relation, ValueRelation, answer,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Check one runtime value relation.
    pub(in crate::check) fn check_value_relation(
        &mut self,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<ValueRelation>> {
        let holds = answer!(self.constrain_type(cause, relation, source, target)?);
        let outcome = self.complete_constraint_check(cause, relation, source, target, holds)?;
        let coercion = match (relation, outcome) {
            (Relation::Assignable | Relation::Writable, CheckOutcome::Holds) => {
                answer!(self.build_coercion(self.cause_origin(cause), relation, source, target,)?)
            }
            _ => None,
        };

        Ok(Answer::Ready(ValueRelation { outcome, coercion }))
    }

    /// Build the runtime coercion for one proven value relation.
    fn build_coercion(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<Box<dir::Coercion>>>> {
        let source = self.settled_root(source)?;
        let target = self.settled_root(target)?;

        // wait for the relation to settle every coercion input
        let mut variables = self.type_variables(source)?;
        variables.extend(self.type_variables(target)?);
        if !variables.is_empty() {
            let blockers = variables.into_iter().map(Dependency::Variable);

            return Ok(Answer::pending(blockers));
        }

        // coercions land on the value the target names: redundant forms drop
        let target = answer!(self.reduce_named_head(origin, target)?);

        // equal types and unreachable values retain their existing carrier
        let is_equal = answer!(self.decide_relation(origin, Relation::Equal, source, target)?);
        let source_value = answer!(self.strip_form(origin, source)?);
        if is_equal || matches!(self.ty(source_value)?, dir::Type::Never) {
            return Ok(Answer::Ready(None));
        }
        let target_value = answer!(self.strip_form(origin, target)?);
        let source_is_union = matches!(self.ty(source_value)?, dir::Type::Union(_));
        let target_is_union = matches!(self.ty(target_value)?, dir::Type::Union(_));

        // map every stored source case into the proven target
        if source_is_union {
            let relation = match relation {
                Relation::Writable => Relation::Assignable,
                relation => relation,
            };

            return self.build_source_union_coercion(
                origin,
                relation,
                source,
                target,
                target_is_union,
            );
        }

        // inject one non-union value into its selected target case
        if target_is_union {
            return self.build_target_union_coercion(origin, relation, source, target);
        }

        self.build_carrier_coercion(origin, source, target)
    }

    /// Build one coercion from every source union case.
    fn build_source_union_coercion(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        target_is_union: bool,
    ) -> CompilerResult<Answer<Option<Box<dir::Coercion>>>> {
        let Some(sources) = answer!(self.union_arms(origin, source)?) else {
            return Err(CompilerError::Internal {
                message: "source union has no stored cases".to_string(),
            });
        };
        let targets = match target_is_union {
            true => match answer!(self.union_arms(origin, target)?) {
                Some(targets) => Some(targets),
                None => {
                    return Err(CompilerError::Internal {
                        message: "target union has no stored cases".to_string(),
                    });
                }
            },
            false => None,
        };
        let mut cases = Vec::with_capacity(sources.len());

        // retain the selected target and payload adjustments for each source case
        for source_case in sources {
            let (target_case, target_type) = match &targets {
                Some(targets) => {
                    let Some((index, target)) = answer!(self.select_union_target(
                        origin,
                        relation,
                        source_case,
                        targets
                    )?) else {
                        return Err(CompilerError::Internal {
                            message: "proven source union case has no target union case"
                                .to_string(),
                        });
                    };

                    (Some(index), target)
                }
                None => (None, target),
            };
            let coercion =
                answer!(self.build_coercion(origin, relation, source_case, target_type,)?);
            let adjustments = coercion.map_or_else(Vec::new, |coercion| coercion.adjustments);
            cases.push(dir::CoercionCase {
                target: target_case,
                adjustments,
            });
        }
        let coercion = dir::Coercion::union(source, target, cases, dir::CastOrigin::Implicit);

        Ok(Answer::Ready(Some(Box::new(coercion))))
    }

    /// Build one coercion into a selected target union case.
    fn build_target_union_coercion(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<Box<dir::Coercion>>>> {
        let Some(targets) = answer!(self.union_arms(origin, target)?) else {
            return Err(CompilerError::Internal {
                message: "target union has no stored cases".to_string(),
            });
        };
        let Some((index, target_case)) =
            answer!(self.select_union_target(origin, relation, source, &targets)?)
        else {
            return Err(CompilerError::Internal {
                message: "proven value has no target union case".to_string(),
            });
        };
        let coercion = answer!(self.build_coercion(origin, relation, source, target_case,)?);
        let adjustments = coercion.map_or_else(Vec::new, |coercion| coercion.adjustments);
        let case = dir::CoercionCase {
            target: Some(index),
            adjustments,
        };
        let coercion = dir::Coercion::union(source, target, vec![case], dir::CastOrigin::Implicit);

        Ok(Answer::Ready(Some(Box::new(coercion))))
    }

    /// Select the first declared target union case accepting a closed source.
    fn select_union_target(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        targets: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<Option<(u32, dir::GlobalTypeId)>>> {
        // preserve an exact stored case before considering wider cases
        for (index, target) in targets.iter().copied().enumerate() {
            if answer!(self.decide_relation(origin, Relation::Equal, source, target)?) {
                return Ok(Answer::Ready(Some((index as u32, target))));
            }
        }

        // otherwise use the first accepted case in declaration order
        for (index, target) in targets.iter().copied().enumerate() {
            if answer!(self.decide_relation(origin, relation, source, target)?) {
                return Ok(Answer::Ready(Some((index as u32, target))));
            }
        }

        Ok(Answer::Ready(None))
    }

    /// Build one coercion between non-union value carriers.
    fn build_carrier_coercion(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<Box<dir::Coercion>>>> {
        let source_carrier = answer!(self.carrier_type(origin, source)?);
        let target_carrier = answer!(self.carrier_type(origin, target)?);

        // borrowing retains placement even when carrier checking strips it
        if self.borrow_conversion(origin, source, target)?.is_some() {
            let adjustment = dir::CoercionAdjustment {
                kind: dir::CoercionKind::Borrow,
                target,
                cases: Vec::new(),
            };
            let coercion = dir::Coercion::new(source, vec![adjustment], dir::CastOrigin::Implicit);

            return Ok(Answer::Ready(Some(Box::new(coercion))));
        }

        // distinct authored types may share one carrier
        if source_carrier == target_carrier {
            return Ok(Answer::Ready(None));
        }
        let source_head = self.ty(source_carrier)?;
        let target_head = self.ty(target_carrier)?;

        // tuples reshape only when their stored slots differ
        if let (dir::Type::Tuple(source_tuple), dir::Type::Tuple(target_tuple)) =
            (&source_head, &target_head)
        {
            let differs = self.tuple_carriers_differ(
                source_carrier,
                source_tuple,
                target_carrier,
                target_tuple,
            )?;
            if !differs {
                return Ok(Answer::Ready(None));
            }

            let adjustment = dir::CoercionAdjustment {
                kind: dir::CoercionKind::Carrier,
                target,
                cases: Vec::new(),
            };
            let coercion = dir::Coercion::new(source, vec![adjustment], dir::CastOrigin::Implicit);

            return Ok(Answer::Ready(Some(Box::new(coercion))));
        }

        // classify the remaining concrete carrier conversion
        let Some(kind) = dir::CoercionKind::classify(&source_head, &target_head) else {
            return Ok(Answer::Ready(None));
        };
        if kind == dir::CoercionKind::Union {
            return Err(CompilerError::Internal {
                message: "union coercion reached ordinary carrier classification".to_string(),
            });
        }
        let adjustment = dir::CoercionAdjustment {
            kind,
            target,
            cases: Vec::new(),
        };
        let coercion = dir::Coercion::new(source, vec![adjustment], dir::CastOrigin::Implicit);

        Ok(Answer::Ready(Some(Box::new(coercion))))
    }

    /// Return whether two tuple types require different runtime carriers.
    fn tuple_carriers_differ(
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

        // optional slots share their unioned value carrier
        let differs = source_elements
            .iter()
            .zip(target_elements)
            .any(|(source, target)| source.is_rest != target.is_rest);

        Ok(differs)
    }

    /// Return the reduced type determining one runtime carrier.
    fn carrier_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let mut ty = answer!(self.reduce_type_head(origin, ty)?);
        while let dir::Type::Form(form) = self.ty(ty)?
            && matches!(
                form.form,
                dir::Form::Readonly | dir::Form::Placed { .. } | dir::Form::Managed
            )
        {
            ty = answer!(self.reduce_type_head(origin, form.value)?);
        }

        Ok(Answer::Ready(ty))
    }
}
