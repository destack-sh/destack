use destack_dir as dir;
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckState, Constraint, Decision, FlowSite, Origin, Relation, ValueUse, Widening,
    answer,
};
use crate::{CompilerError, CompilerResult};

/// Literal inference mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum InferMode {
    /// Infer ordinary source types.
    Normal,
    /// Infer under `as const` literal rules.
    Const,
}

impl InferMode {
    /// Return whether object fields inferred in this mode are readonly.
    pub(in crate::check) fn is_readonly(self) -> bool {
        matches!(self, Self::Const)
    }
}

impl CheckState<'_> {
    /// Return the type of one scalar literal expression.
    pub(in crate::check) fn scalar_literal_type(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        value: dir::ScalarLiteral,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match value {
            dir::ScalarLiteral::RegexString { .. } => self.push_language_type(
                node.module_id,
                node.local_id.into_any(),
                dir::LanguageItem::RegExp,
                Vec::new(),
            ),
            dir::ScalarLiteral::Null => {
                self.push_type(node.module_id, dir::Type::Null, node.local_id.into_any())
            }
            dir::ScalarLiteral::Undefined => self.push_type(
                node.module_id,
                dir::Type::Undefined,
                node.local_id.into_any(),
            ),
            value => self.push_type(
                node.module_id,
                dir::Type::Literal(value),
                node.local_id.into_any(),
            ),
        }
    }

    /// Infer one array literal from its elements.
    pub(in crate::check) fn infer_array_expression(
        &mut self,
        site: FlowSite,
        elements: &[dir::LocalNodeId<dir::Argument>],
        mode: InferMode,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let mut values =
            SmallVec::<[(dir::LocalNodeId<dir::Expression>, dir::GlobalTypeId); 8]>::new();
        let mut spreads =
            SmallVec::<[(dir::LocalNodeId<dir::Expression>, dir::GlobalTypeId); 2]>::new();

        for argument in elements {
            match self.module(module).view().get(*argument) {
                dir::Argument::Spread { value, .. } => {
                    let value = *value;
                    let ty =
                        answer!(self.node_type_at(site.sibling(value.into_global_any(module)))?);
                    spreads.push((value, ty));
                }
                dir::Argument::Positional { value }
                | dir::Argument::Named { value, .. }
                | dir::Argument::Labeled { value, .. } => {
                    let value = *value;
                    let ty = answer!(self.infer_expression_type(
                        site.sibling(value.into_global_any(module)),
                        value.into_global(module),
                        mode,
                    )?);
                    values.push((value, ty));
                }
                dir::Argument::Error => {}
            }
        }

        if mode == InferMode::Const && spreads.is_empty() {
            let elements = values
                .iter()
                .map(|(_, ty)| dir::TypeElement {
                    label: None,
                    ty: *ty,
                    is_optional: false,
                    is_readonly: false,
                    is_rest: false,
                })
                .collect();
            let tuple = self.push_type(
                module,
                dir::Type::Tuple(dir::TupleType {
                    form: dir::TupleForm::Array,
                    elements,
                }),
                node.local_id.into_any(),
            )?;
            let readonly = self.push_type(
                module,
                dir::Type::Form(dir::FormType {
                    form: dir::Form::Readonly,
                    value: tuple,
                }),
                node.local_id.into_any(),
            )?;

            return Ok(Answer::Ready(readonly));
        }

        let element = if spreads.is_empty() {
            match values.as_slice() {
                [] => self.push_type(module, dir::Type::Never, node.local_id.into_any())?,
                [(_, single)] => *single,
                _ => self.normalized_union_type(
                    module,
                    values.iter().map(|(_, value)| *value),
                    node.local_id.into_any(),
                )?,
            }
        } else {
            let origin = Origin::Node(node.into_any());
            let variable = self.allocate_variable(module, origin, Widening::Preserve);
            let element = self.push_variable_type(variable, node.local_id.into_any())?;

            for (source, value) in &values {
                self.push_constraint(Constraint::check(
                    Relation::Assignable,
                    *value,
                    element,
                    Origin::Node(source.into_global_any(module)),
                ));
            }

            element
        };
        let array = self.push_type(
            module,
            dir::Type::Array(dir::ArrayType { element }),
            node.local_id.into_any(),
        )?;

        for (value, spread) in spreads {
            self.push_constraint(Constraint::check(
                Relation::Assignable,
                spread,
                array,
                Origin::Node(value.into_global_any(module)),
            ));
        }

        Ok(Answer::Ready(array))
    }

    /// Infer one fixed array literal from its repeated value.
    pub(in crate::check) fn infer_fixed_array_expression(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
        count: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let element = answer!(self.node_type_at(site.sibling(value.into_global_any(module)))?);
        let array = self.push_type(
            module,
            dir::Type::FixedArray(dir::FixedArrayType { element, count }),
            node.local_id.into_any(),
        )?;
        self.commit_node_type(node.into_any(), array)?;

        Ok(Answer::Ready(()))
    }

    /// Infer one tuple literal from its elements.
    pub(in crate::check) fn infer_tuple_expression(
        &mut self,
        site: FlowSite,
        elements: &[dir::LocalNodeId<dir::Argument>],
        mode: InferMode,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let mut fields = Vec::with_capacity(elements.len());

        for element in elements {
            let (label, value, is_rest) = match self.module(module).view().get(*element) {
                dir::Argument::Positional { value } => (None, Some(*value), false),
                dir::Argument::Named { value, .. } => (None, Some(*value), false),
                dir::Argument::Labeled { label, value } => (Some(*label), Some(*value), false),
                dir::Argument::Spread { value, .. } => (None, Some(*value), true),
                dir::Argument::Error => (None, None, false),
            };
            let Some(value) = value else {
                continue;
            };
            let ty = answer!(self.infer_expression_type(
                site.sibling(value.into_global_any(module)),
                value.into_global(module),
                mode,
            )?);
            fields.push(dir::TypeElement {
                label,
                ty,
                is_optional: false,
                is_readonly: false,
                is_rest,
            });
        }

        let tuple = self.push_type(
            module,
            dir::Type::Tuple(dir::TupleType {
                form: dir::TupleForm::Tuple,
                elements: fields,
            }),
            node.local_id.into_any(),
        )?;

        if mode == InferMode::Const {
            let readonly = self.push_type(
                module,
                dir::Type::Form(dir::FormType {
                    form: dir::Form::Readonly,
                    value: tuple,
                }),
                node.local_id.into_any(),
            )?;

            Ok(Answer::Ready(readonly))
        } else {
            Ok(Answer::Ready(tuple))
        }
    }

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

        for property in properties {
            match self.module(module).view().get(*property).clone() {
                dir::Property::Field { key, value, .. } => {
                    let Some(key) = answer!(self.select_property_key(site, key)?) else {
                        continue;
                    };
                    let ty = answer!(self.infer_expression_type(
                        site.sibling(value.into_global_any(module)),
                        value.into_global(module),
                        mode,
                    )?);
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
                    let spread = answer!(self.infer_expression_type(
                        site.sibling(source),
                        value.into_global(module),
                        mode,
                    )?);
                    let Some(spread_fields) =
                        answer!(self.spread_fields(Origin::Node(source), module, spread)?)
                    else {
                        self.report_spread_not_object(Origin::Node(source), spread)?;
                        self.record_decision(node.into_any(), Decision::Rejected)?;
                        let error =
                            self.push_type(module, dir::Type::Error, node.local_id.into_any())?;

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

        let shape = self.push_type(
            module,
            dir::Type::Shape(dir::ShapeType {
                fields: fields.into_values().collect(),
                call_signatures: Vec::new(),
                construct_signatures: Vec::new(),
                index_signatures: Vec::new(),
            }),
            node.local_id.into_any(),
        )?;

        Ok(Answer::Ready(shape))
    }

    /// Infer one expression type under one inference mode.
    pub(in crate::check) fn infer_expression_type(
        &mut self,
        site: FlowSite,
        node: dir::GlobalNodeId<dir::Expression>,
        mode: InferMode,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let module = node.module_id;
        let expression = self.module(module).view().get(node.local_id).clone();

        if mode == InferMode::Normal {
            return self.node_type_at(site);
        }

        match expression {
            dir::Expression::Parenthesized { expression } => self.infer_expression_type(
                site.sibling(expression.into_global_any(module)),
                expression.into_global(module),
                mode,
            ),
            dir::Expression::ObjectExpression { properties } => {
                self.infer_object_expression(site, &properties, mode)
            }
            dir::Expression::ArrayExpression { elements } => {
                self.infer_array_expression(site, &elements, mode)
            }
            dir::Expression::TupleExpression { elements } => {
                self.infer_tuple_expression(site, &elements, mode)
            }
            _ => self.node_type_at(site),
        }
    }

    /// Check one array literal under an expected array or slice type.
    pub(in crate::check) fn check_array_expression(
        &mut self,
        site: FlowSite,
        elements: &[dir::LocalNodeId<dir::Argument>],
        target: dir::GlobalTypeId,
        relation: Relation,
        origin: Origin,
        use_: ValueUse,
    ) -> CompilerResult<Answer<bool>> {
        let node = site.node.into_typed::<dir::Expression>();
        let element = match self.ty(target)? {
            dir::Type::Array(array) => Some((array.element, None, false)),
            dir::Type::Slice(slice) => Some((slice.element, None, true)),
            dir::Type::FixedArray(array) => Some((array.element, Some(array.count), true)),
            _ => None,
        };
        let Some((element, count, mut needs_value_constraint)) = element else {
            return Ok(Answer::Ready(false));
        };

        for argument in elements {
            let (dir::Argument::Positional { value }
            | dir::Argument::Named { value, .. }
            | dir::Argument::Labeled { value, .. }) =
                self.module(node.module_id).view().get(*argument)
            else {
                return Ok(Answer::Ready(false));
            };
            let child = value.into_global_any(node.module_id);
            let () = answer!(self.check_node(
                site.sibling(child),
                element,
                relation,
                Origin::Node(child),
                use_
            )?);
        }

        let array = if count.is_some() {
            let actual_count = self.push_type(
                node.module_id,
                dir::Type::Literal(dir::ScalarLiteral::Integer(elements.len() as i64)),
                node.local_id.into_any(),
            )?;
            self.push_type(
                node.module_id,
                dir::Type::FixedArray(dir::FixedArrayType {
                    element,
                    count: actual_count,
                }),
                node.local_id.into_any(),
            )?
        } else {
            self.push_type(
                node.module_id,
                dir::Type::Array(dir::ArrayType { element }),
                node.local_id.into_any(),
            )?
        };
        self.commit_node_type(node.into_any(), array)?;
        needs_value_constraint |= elements.is_empty();
        if needs_value_constraint {
            let () = answer!(self.constrain_node_value(site, relation, target, origin, use_)?);
        }

        Ok(Answer::Ready(true))
    }

    /// Check one tuple literal under an expected tuple type.
    pub(in crate::check) fn check_tuple_expression(
        &mut self,
        site: FlowSite,
        elements: &[dir::LocalNodeId<dir::Argument>],
        target: dir::GlobalTypeId,
        relation: Relation,
        use_: ValueUse,
    ) -> CompilerResult<Answer<bool>> {
        let node = site.node.into_typed::<dir::Expression>();
        let dir::Type::Tuple(tuple) = self.ty(target)? else {
            return Ok(Answer::Ready(false));
        };
        let tuple = tuple.clone();
        if tuple.elements.len() != elements.len() {
            return Ok(Answer::Ready(false));
        }

        for (argument, element) in elements.iter().zip(tuple.elements.iter()) {
            let (dir::Argument::Positional { value }
            | dir::Argument::Named { value, .. }
            | dir::Argument::Labeled { value, .. }) =
                self.module(node.module_id).view().get(*argument)
            else {
                return Ok(Answer::Ready(false));
            };
            let child = value.into_global_any(node.module_id);
            answer!(self.check_node(
                site.sibling(child),
                element.ty,
                relation,
                Origin::Node(child),
                use_
            )?);
        }

        let source = answer!(self.infer_tuple_expression(site, elements, InferMode::Normal)?);
        self.commit_node_type(node.into_any(), source)?;

        Ok(Answer::Ready(true))
    }

    /// Check one object literal under an expected object type.
    pub(in crate::check) fn check_object_expression(
        &mut self,
        site: FlowSite,
        properties: &[dir::LocalNodeId<dir::Property>],
        target: dir::GlobalTypeId,
        relation: Relation,
        origin: Origin,
        use_: ValueUse,
    ) -> CompilerResult<Answer<bool>> {
        let node = site.node.into_typed::<dir::Expression>();
        let mut target_payload = target;
        while let dir::Type::Form(form) = self.ty(target_payload)?
            && matches!(form.form, dir::Form::Managed | dir::Form::Readonly)
        {
            target_payload = form.value;
        }

        let dir::Type::Shape(shape) = self.ty(target_payload)? else {
            return Ok(Answer::Ready(false));
        };
        let shape = shape.clone();
        let mut keys = SmallVec::<[dir::StaticKey; 4]>::new();
        let mut needs_value_constraint = false;

        for property in properties {
            let property = self.module(node.module_id).view().get(*property).clone();
            match property {
                dir::Property::Field { key, value, .. } => {
                    let Some(key) = answer!(self.select_property_key(site, key)?) else {
                        return Ok(Answer::Ready(false));
                    };
                    keys.push(key);
                    let Some(field) = shape.fields.iter().find(|field| field.key == key) else {
                        needs_value_constraint = true;

                        continue;
                    };
                    let child = value.into_global_any(node.module_id);
                    answer!(self.check_node(
                        site.sibling(child),
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
        for field in &shape.fields {
            if !keys.contains(&field.key) {
                needs_value_constraint = true;
            }
        }

        let source = answer!(self.infer_object_expression(site, properties, InferMode::Normal)?);
        self.commit_node_type(node.into_any(), source)?;
        if needs_value_constraint {
            let () = answer!(self.constrain_node_value(site, relation, target, origin, use_)?);
        }

        Ok(Answer::Ready(true))
    }
}
