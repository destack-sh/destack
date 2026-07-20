use destack_dir as dir;
use indexmap::IndexMap;
use smallvec::SmallVec;

use super::InferMode;
use crate::check::{
    Answer, BodyState, CandidateOutcome, CandidateVerdict, Cause, CauseId, CauseKind, CheckAttempt,
    CheckOutcome, Decision, FlowSite, Origin, PlaceUse, ProbeReason, Relation, ValueUse, answer,
};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Infer one object literal from its properties.
    pub(in crate::check) fn infer_object_expression(
        &mut self,
        site: FlowSite,
        properties: &[dir::LocalNodeId<dir::Property>],
        mode: InferMode,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let mut fields = IndexMap::<dir::StaticKey, dir::TypeField>::new();

        // collect literal fields, methods, and spreads into one shape
        for property in properties {
            match self.module(module).view().get(*property).clone() {
                dir::Property::Field { key, value, .. } => {
                    let Some(key) = answer!(self.select_property_key(site, key)?) else {
                        continue;
                    };
                    let value_site = self.node_site(value.into_global_any(module))?;
                    answer!(self.infer_expression(value_site, PlaceUse::Read, mode)?);
                    let ty = answer!(self.node_type_at(value_site)?);
                    fields.insert(
                        key,
                        dir::TypeField {
                            key,
                            ty,
                            is_optional: false,
                            is_readonly: mode.is_readonly(),
                        },
                    );
                }
                dir::Property::Method { key, .. } => {
                    let Some(key) = answer!(match key {
                        Some(key) => self.select_property_key(site, key)?,
                        None => Answer::Ready(None),
                    }) else {
                        continue;
                    };
                    let Some(symbol) = self.module(module).declaration_symbol(property.into_any())
                    else {
                        continue;
                    };
                    let Some(ty) = self.symbol_type_maybe(symbol) else {
                        return Err(CompilerError::Internal {
                            message: format!("object method property {symbol:?} has no type"),
                        });
                    };
                    fields.insert(
                        key,
                        dir::TypeField {
                            key,
                            ty,
                            is_optional: false,
                            is_readonly: mode.is_readonly(),
                        },
                    );
                }
                dir::Property::Spread { value } => {
                    let source = value.into_global_any(module);
                    let source_site = self.node_site(source)?;
                    answer!(self.infer_expression(source_site, PlaceUse::Read, mode)?);
                    let spread = answer!(self.node_type_at(source_site)?);
                    let Some(spread_fields) = answer!(self.spread_fields(
                        Origin::Node(source, site.scope),
                        module,
                        spread
                    )?) else {
                        self.report_spread_not_object(Origin::Node(source, site.scope), spread)?;
                        self.commit_decision(node.into_any(), Decision::Rejected)?;
                        let error = self.commit_error_node(node.into_any())?;

                        return Ok(Answer::Ready(error));
                    };
                    for field in spread_fields {
                        let field = dir::TypeField {
                            is_readonly: field.is_readonly || mode.is_readonly(),
                            ..field
                        };
                        fields.insert(field.key, field);
                    }
                }
                dir::Property::Error => {}
            }
        }

        let fields: Vec<dir::TypeField> = fields.into_values().collect();
        let fields = self.intern_fields(module, &fields)?;
        let shape = self.intern_type(
            module,
            dir::Type::Shape(dir::ShapeType {
                fields,
                call_signatures: dir::TypeListId::EMPTY,
                construct_signatures: dir::TypeListId::EMPTY,
                index_signatures: dir::TypeListId::EMPTY,
            }),
        )?;

        let ty = if mode == InferMode::Widen {
            self.widen_type(shape)?
        } else {
            shape
        };

        Ok(Answer::Ready(ty))
    }

    /// Check one object literal under an expected object type.
    pub(in crate::check) fn check_object_expression(
        &mut self,
        site: FlowSite,
        properties: &[dir::LocalNodeId<dir::Property>],
        target: dir::GlobalTypeId,
        target_head: dir::GlobalTypeId,
        relation: Relation,
        cause: CauseId,
        use_: ValueUse,
    ) -> CompilerResult<Answer<CheckAttempt>> {
        let origin = self.cause_origin(cause);
        let node = site.node.into_typed::<dir::Expression>();
        let mut target_payload = target_head;
        while let dir::Type::Form(form) = self.ty(target_payload)?
            && matches!(form.form, dir::Form::Managed | dir::Form::Readonly)
        {
            target_payload = form.value;
        }

        // union targets select their first contextually viable arm
        if let dir::Type::Union(union) = self.ty(target_payload)? {
            let elements = SmallVec::<[_; 4]>::from_slice(
                self.type_ids(target_payload.module_id, union.elements)?,
            );
            for element in elements {
                let element_head = answer!(self.reduce_type_head(origin, element)?);
                let verdict = self.probe_candidate(ProbeReason::UnionArm, |state| {
                    let checked = state.check_object_expression(
                        site,
                        properties,
                        element,
                        element_head,
                        relation,
                        cause,
                        use_,
                    )?;

                    match checked {
                        Answer::Ready(CheckAttempt::Checked(CheckOutcome::Holds)) => {
                            Ok(Answer::Ready(CandidateOutcome::<(), ()>::Accepted(())))
                        }
                        Answer::Ready(_) => Ok(Answer::Ready(CandidateOutcome::Rejected(()))),
                        Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
                    }
                })?;
                if verdict == CandidateVerdict::Viable {
                    return self.check_object_expression(
                        site,
                        properties,
                        element,
                        element_head,
                        relation,
                        cause,
                        use_,
                    );
                }
            }

            return Ok(Answer::Ready(CheckAttempt::NotApplicable));
        }

        let Some(target_fields) = answer!(self.expected_object_fields(origin, target_payload)?)
        else {
            return Ok(Answer::Ready(CheckAttempt::NotApplicable));
        };
        let mut keys = SmallVec::<[dir::StaticKey; 4]>::new();
        let mut should_relate_result = false;
        let mut check = CheckOutcome::Holds;

        // check present fields against their matching expected fields
        for property in properties {
            let property = self.module(node.module_id).view().get(*property).clone();
            match property {
                dir::Property::Field { key, value, .. } => {
                    let Some(key) = answer!(self.select_property_key(site, key)?) else {
                        return Ok(Answer::Ready(CheckAttempt::NotApplicable));
                    };
                    keys.push(key);
                    let Some(field) = target_fields.iter().find(|field| field.key == key).copied()
                    else {
                        should_relate_result = true;

                        continue;
                    };
                    let child = value.into_global_any(node.module_id);
                    let child_site = self.node_site(child)?;
                    let field_cause = self.check.intern_cause(Cause::slot(
                        Origin::Node(child, site.scope),
                        CauseKind::Field { key },
                        cause,
                    ));
                    let child_check = answer!(self.check_node_expected(
                        child_site,
                        field.ty,
                        relation,
                        field_cause,
                        use_
                    )?);
                    check = check.and(child_check);
                }
                dir::Property::Spread { .. } => {
                    return Ok(Answer::Ready(CheckAttempt::NotApplicable));
                }
                dir::Property::Method { .. } | dir::Property::Error => {}
            }
        }

        // require the outer value relation when fields are missing or unmatched
        for field in &target_fields {
            if !keys.contains(&field.key) {
                should_relate_result = true;
            }
        }

        let source = answer!(self.infer_object_expression(site, properties, InferMode::Exact)?);
        let source = answer!(self.materialize_fresh_value(origin, source, Some(target))?);
        self.commit_node_type(node.into_any(), source)?;
        if should_relate_result {
            let (_, result_check) = answer!(self.check_node_value(site, relation, target, cause)?);
            check = check.and(result_check);
        }

        Ok(Answer::Ready(CheckAttempt::Checked(check)))
    }

    /// Return fields expected by an object literal target.
    fn expected_object_fields(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<SmallVec<[dir::TypeField; 8]>>>> {
        match self.ty(target)? {
            dir::Type::Shape(shape) => Ok(Answer::Ready(Some(SmallVec::from_slice(
                self.shape_fields(target.module_id, shape.fields)?,
            )))),
            // structural interfaces type literals contextually; nominal
            //  interfaces require their declared wrapper
            dir::Type::Instance(instance)
                if matches!(
                    self.definition(instance.symbol)?,
                    Some(dir::Definition::Interface(interface)) if !interface.is_nominal
                ) =>
            {
                let fields = answer!(self.check.interface_instance_fields(
                    origin,
                    target.module_id,
                    &instance,
                    target,
                )?);

                Ok(Answer::Ready(fields.map(SmallVec::from_vec)))
            }
            _ => Ok(Answer::Ready(None)),
        }
    }
}
