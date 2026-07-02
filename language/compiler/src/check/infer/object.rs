use destack_dir as dir;
use indexmap::IndexMap;
use smallvec::SmallVec;

use super::InferMode;
use crate::check::{
    Answer, CheckState, Decision, FlowSite, Origin, PlaceUse, Relation, ValueUse, answer,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
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
                    let Some(spread_fields) =
                        answer!(self.spread_fields(Origin::Node(source), module, spread)?)
                    else {
                        self.report_spread_not_object(Origin::Node(source), spread)?;
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

        Ok(Answer::Ready(shape))
    }

    /// Check one object literal under an expected object type.
    pub(in crate::check) fn check_object_expression(
        &mut self,
        site: FlowSite,
        properties: &[dir::LocalNodeId<dir::Property>],
        target: dir::GlobalTypeId,
        target_head: dir::GlobalTypeId,
        relation: Relation,
        origin: Origin,
        use_: ValueUse,
    ) -> CompilerResult<Answer<bool>> {
        let node = site.node.into_typed::<dir::Expression>();
        let mut target_payload = target_head;
        while let dir::Type::Form(form) = self.ty(target_payload)?
            && matches!(form.form, dir::Form::Managed | dir::Form::Readonly)
        {
            target_payload = form.value;
        }

        let dir::Type::Shape(shape) = self.ty(target_payload)? else {
            return Ok(Answer::Ready(false));
        };
        let mut keys = SmallVec::<[dir::StaticKey; 4]>::new();
        let mut should_relate_result = false;

        // check present fields against their matching expected fields
        for property in properties {
            let property = self.module(node.module_id).view().get(*property).clone();
            match property {
                dir::Property::Field { key, value, .. } => {
                    let Some(key) = answer!(self.select_property_key(site, key)?) else {
                        return Ok(Answer::Ready(false));
                    };
                    keys.push(key);
                    let Some(field) = self
                        .shape_fields(target_payload.module_id, shape.fields)?
                        .iter()
                        .find(|field| field.key == key)
                        .copied()
                    else {
                        should_relate_result = true;

                        continue;
                    };
                    let child = value.into_global_any(node.module_id);
                    let child_site = self.node_site(child)?;
                    answer!(self.check_node(
                        child_site,
                        field.ty,
                        relation,
                        Origin::Node(child),
                        use_
                    )?);
                }
                dir::Property::Spread { .. } => {
                    return Ok(Answer::Ready(false));
                }
                dir::Property::Method { .. } | dir::Property::Error => {}
            }
        }

        // require the outer value relation when fields are missing or unmatched
        for field in self.shape_fields(target_payload.module_id, shape.fields)? {
            if !keys.contains(&field.key) {
                should_relate_result = true;
            }
        }

        let source = answer!(self.infer_object_expression(site, properties, InferMode::Normal)?);
        self.commit_node_type(node.into_any(), source)?;
        if should_relate_result {
            let () = answer!(self.constrain_node_value(site, relation, target, origin, use_)?);
        }

        Ok(Answer::Ready(true))
    }
}
