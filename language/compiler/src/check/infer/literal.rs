use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{Answer, BodyState, FlowSite, PlaceUse, Relation, answer};

/// Inference mode for literal materialization contexts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum InferMode {
    /// Preserve the expression's direct literal precision.
    Exact,
    /// Materialize literals through their default widened type.
    Widen,
    /// Infer under `as const` literal-preserving rules.
    Const,
}

impl InferMode {
    /// Return whether object fields inferred in this mode are readonly.
    pub(in crate::check) fn is_readonly(self) -> bool {
        matches!(self, Self::Const)
    }
}

impl BodyState<'_, '_> {
    /// Return the relation carried by one authored literal value.
    pub(in crate::check) fn literal_relation(&self, mut value: dir::GlobalNodeIdAny) -> Relation {
        let Ok(mut expression) = value.try_into_typed::<dir::Expression>() else {
            return Relation::Assignable;
        };

        // follow transparent expressions to the literal they preserve
        loop {
            match self.module(value.module_id).view().get(expression.local_id) {
                dir::Expression::ObjectExpression { .. }
                | dir::Expression::ArrayExpression { .. }
                | dir::Expression::TupleExpression { .. } => return Relation::Writable,
                dir::Expression::Satisfies {
                    expression: child, ..
                } => {
                    value = child.into_global_any(value.module_id);
                    expression = child.into_global(value.module_id);
                }
                _ => return Relation::Assignable,
            }
        }
    }
    /// Return the type of one scalar literal expression.
    pub(in crate::check) fn scalar_literal_type(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        value: dir::ScalarLiteral,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match value {
            dir::ScalarLiteral::RegexString { .. } => {
                self.language_type(node.module_id, dir::LanguageItem::RegExp, &[])
            }
            dir::ScalarLiteral::Null => self.intern_type(node.module_id, dir::Type::Null),
            dir::ScalarLiteral::Undefined => self.intern_type(node.module_id, dir::Type::Undefined),
            value => self.intern_type(node.module_id, dir::Type::Literal(value)),
        }
    }

    /// Return one expression type under const literal materialization.
    pub(in crate::check) fn const_literal_expression_type(
        &mut self,
        source: dir::GlobalNodeIdAny,
        ordinary_type: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let site = self.node_site(source)?;
        let Ok(expression) = source.try_into_typed::<dir::Expression>() else {
            return Ok(Answer::Ready(ordinary_type));
        };
        let expression = self
            .module(expression.module_id)
            .view()
            .get(expression.local_id)
            .clone();

        match expression {
            dir::Expression::ArrayExpression { elements } => {
                self.const_literal_array_type(site, source.module_id, &elements)
            }
            dir::Expression::TupleExpression { elements } => {
                self.const_literal_tuple_type(source.module_id, &elements)
            }
            dir::Expression::ObjectExpression { properties } => {
                self.const_literal_object_type(site, source.module_id, &properties)
            }
            _ => Ok(Answer::Ready(ordinary_type)),
        }
    }

    /// Return one array literal type under const literal materialization.
    fn const_literal_array_type(
        &mut self,
        site: FlowSite,
        module: ModuleId,
        elements: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let mut fields = Vec::with_capacity(elements.len());

        // collect positional element types, spreads keep ordinary inference
        for element in elements {
            let ty = match self.module(module).view().get(*element) {
                dir::Argument::Positional { value }
                | dir::Argument::Named { value, .. }
                | dir::Argument::Labeled { value, .. } => {
                    let value_site = self.node_site(value.into_global_any(module))?;

                    answer!(self.infer_node_type(value_site, PlaceUse::Read)?)
                }
                dir::Argument::Spread { .. } | dir::Argument::Error => {
                    return self.infer_node_type(site, PlaceUse::Read);
                }
                dir::Argument::Elision => self.intern_type(module, dir::Type::Undefined)?,
            };
            fields.push(dir::TypeElement::new(ty));
        }

        // freeze the literal as a readonly array tuple
        let fields = self.intern_elements(module, &fields)?;
        let tuple = self.intern_type(
            module,
            dir::Type::Tuple(dir::TupleType {
                form: dir::TupleForm::Array,
                elements: fields,
            }),
        )?;
        let readonly = self.intern_type(
            module,
            dir::Type::Form(dir::FormType {
                form: dir::Form::Readonly,
                value: tuple,
            }),
        )?;

        Ok(Answer::Ready(readonly))
    }

    /// Return one tuple literal type under const literal materialization.
    fn const_literal_tuple_type(
        &mut self,
        module: ModuleId,
        elements: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let mut fields = Vec::with_capacity(elements.len());

        // collect tuple element types exactly
        for element in elements {
            let (label, value, is_rest) = match self.module(module).view().get(*element) {
                dir::Argument::Positional { value } => (None, Some(*value), false),
                dir::Argument::Named { value, .. } => (None, Some(*value), false),
                dir::Argument::Labeled { label, value } => (Some(*label), Some(*value), false),
                dir::Argument::Spread { value, .. } => (None, Some(*value), true),
                dir::Argument::Error => continue,
                dir::Argument::Elision => (None, None, false),
            };
            let ty = match value {
                Some(value) => {
                    let value_site = self.node_site(value.into_global_any(module))?;

                    answer!(self.infer_node_type(value_site, PlaceUse::Read)?)
                }
                None => self.intern_type(module, dir::Type::Undefined)?,
            };
            fields.push(dir::TypeElement {
                label,
                ty,
                is_optional: false,
                is_readonly: false,
                is_rest,
            });
        }

        // freeze the literal as a readonly tuple
        let fields = self.intern_elements(module, &fields)?;
        let tuple = self.intern_type(
            module,
            dir::Type::Tuple(dir::TupleType {
                form: dir::TupleForm::Tuple,
                elements: fields,
            }),
        )?;
        let readonly = self.intern_type(
            module,
            dir::Type::Form(dir::FormType {
                form: dir::Form::Readonly,
                value: tuple,
            }),
        )?;

        Ok(Answer::Ready(readonly))
    }

    /// Return one object literal type under const literal materialization.
    fn const_literal_object_type(
        &mut self,
        site: FlowSite,
        module: ModuleId,
        properties: &[dir::LocalNodeId<dir::Property>],
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let mut fields = Vec::new();

        // collect direct fields exactly, spreads keep ordinary inference
        for property in properties {
            let dir::Property::Field { key, value, .. } =
                self.module(module).view().get(*property).clone()
            else {
                return self.infer_node_type(site, PlaceUse::Read);
            };
            let Some(key) = answer!(self.select_property_key(site, key)?) else {
                continue;
            };
            let value_site = self.node_site(value.into_global_any(module))?;
            let ty = answer!(self.infer_node_type(value_site, PlaceUse::Read)?);
            fields.push(dir::TypeField {
                key,
                ty,
                is_optional: false,
                is_readonly: true,
            });
        }

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
}
