use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{CheckComponentState, VariableId};

impl CheckComponentState<'_> {
    /// Evaluate one source type expression.
    pub(in crate::check) fn evaluate_type_expression(
        &mut self,
        variable: VariableId,
        node: dir::GlobalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Option<dir::LocalTypeId>> {
        let ty = self.type_expression_type(variable.module, node.clone())?;
        let Some(ty) = ty else {
            return Ok(None);
        };

        let type_id = self
            .module_mut(variable.module)?
            .intern_type(ty, node.local_id.into_any());

        Ok(Some(type_id))
    }

    /// Evaluate one type expression into a semantic type.
    fn type_expression_type(
        &mut self,
        module: ModuleId,
        node: dir::GlobalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Option<dir::Type>> {
        if let Some(type_id) = self.module(module)?.node_type_id(node.clone().into_any()) {
            let ty = self.module(module)?.get_type(type_id);

            return Ok(Some(ty));
        }

        let expression = self
            .module(module)?
            .parsed()
            .tree
            .get(node.local_id)
            .clone();

        let ty = match expression {
            dir::TypeExpression::Parenthesized { expression } => {
                let expression = expression.into_global(module);

                return self.type_expression_type(module, expression);
            }
            dir::TypeExpression::ScalarLiteral { value } => dir::Type::from(value),
            dir::TypeExpression::Literal { value } => dir::Type::from(value),
            dir::TypeExpression::This => dir::Type::This,
            dir::TypeExpression::Tuple { elements }
            | dir::TypeExpression::ArrayTuple { elements } => {
                let Some(tuple) = self.tuple_type(module, &elements)? else {
                    return Ok(None);
                };

                dir::Type::Tuple(tuple)
            }
            dir::TypeExpression::Array { element } | dir::TypeExpression::Slice { element } => {
                let Some(element) = self.child_type_id(module, element)? else {
                    return Ok(None);
                };

                dir::Type::Slice(dir::SliceType {
                    element,
                    is_readonly: false,
                })
            }
            dir::TypeExpression::Object { members } => {
                let Some(shape) = self.shape_type(module, &members)? else {
                    return Ok(None);
                };

                dir::Type::Shape(shape)
            }
            dir::TypeExpression::Reference {
                path,
                generic_arguments,
            } => {
                let Some(ty) = self.reference_type(
                    module,
                    node.local_id.into_any(),
                    &path,
                    &generic_arguments,
                )?
                else {
                    return Ok(None);
                };

                ty
            }
            dir::TypeExpression::Readonly { target_type } => {
                let Some(value) = self.child_type_id(module, target_type)? else {
                    return Ok(None);
                };

                dir::Type::Form(dir::FormType {
                    form: dir::Form::Readonly,
                    value,
                })
            }
            dir::TypeExpression::OwnedOf { target_type, .. } => {
                let Some(value) = self.child_type_id(module, target_type)? else {
                    return Ok(None);
                };

                dir::Type::Form(dir::FormType {
                    form: dir::Form::Owned,
                    value,
                })
            }
            dir::TypeExpression::PointerOf { target_type, .. } => {
                let Some(value) = self.child_type_id(module, target_type)? else {
                    return Ok(None);
                };

                dir::Type::Form(dir::FormType {
                    form: dir::Form::Raw,
                    value,
                })
            }
            dir::TypeExpression::KeyOf { target_type } => {
                let Some(target) = self.child_type_id(module, target_type)? else {
                    return Ok(None);
                };

                dir::Type::Operation(dir::TypeOperation::KeyOf(dir::UnaryType { target }))
            }
            dir::TypeExpression::Union { elements } => {
                let Some(elements) = self.child_type_ids(module, &elements)? else {
                    return Ok(None);
                };

                dir::Type::Union(dir::UnionType { elements })
            }
            dir::TypeExpression::Intersection { elements } => {
                let Some(elements) = self.child_type_ids(module, &elements)? else {
                    return Ok(None);
                };

                dir::Type::Intersection(dir::IntersectionType { elements })
            }
            dir::TypeExpression::Conditional {
                left,
                extends_type,
                then_type,
                else_type,
            } => {
                let Some(left) = self.child_type_id(module, left)? else {
                    return Ok(None);
                };
                let Some(right) = self.child_type_id(module, extends_type)? else {
                    return Ok(None);
                };
                let Some(then_type) = self.child_type_id(module, then_type)? else {
                    return Ok(None);
                };
                let Some(else_type) = self.child_type_id(module, else_type)? else {
                    return Ok(None);
                };

                dir::Type::Operation(dir::TypeOperation::Conditional(dir::ConditionalType {
                    distributive_symbol: None,
                    left,
                    right,
                    then_type,
                    else_type,
                }))
            }
            dir::TypeExpression::Index { left, index } => {
                let Some(left) = self.child_type_id(module, left)? else {
                    return Ok(None);
                };
                let Some(index) = self.child_type_id(module, index)? else {
                    return Ok(None);
                };

                dir::Type::Operation(dir::TypeOperation::Index(dir::IndexType { left, index }))
            }
            dir::TypeExpression::TemplateLiteral { strings, spans } => {
                let Some(spans) = self.child_type_ids(module, &spans)? else {
                    return Ok(None);
                };

                dir::Type::Operation(dir::TypeOperation::TemplateLiteral(
                    dir::TemplateLiteralType { strings, spans },
                ))
            }
            dir::TypeExpression::Infer {
                name, constraint, ..
            } => {
                let constraint = if let Some(constraint) = constraint {
                    let Some(constraint) = self.child_type_id(module, constraint)? else {
                        return Ok(None);
                    };

                    Some(constraint)
                } else {
                    None
                };

                dir::Type::Operation(dir::TypeOperation::Infer(dir::InferType {
                    name,
                    constraint,
                }))
            }
            dir::TypeExpression::Missing | dir::TypeExpression::Error => dir::Type::Error,
            dir::TypeExpression::Intrinsic
            | dir::TypeExpression::FixedArray { .. }
            | dir::TypeExpression::Declaration { .. }
            | dir::TypeExpression::FunctionTypeDeclaration(_)
            | dir::TypeExpression::ConstructorTypeDeclaration(_)
            | dir::TypeExpression::Member { .. }
            | dir::TypeExpression::Range { .. }
            | dir::TypeExpression::Const
            | dir::TypeExpression::Local { .. }
            | dir::TypeExpression::Shared { .. }
            | dir::TypeExpression::TypeOfValue { .. }
            | dir::TypeExpression::Must { .. }
            | dir::TypeExpression::Not { .. }
            | dir::TypeExpression::BorrowedOf { .. }
            | dir::TypeExpression::Extends { .. }
            | dir::TypeExpression::Implements { .. }
            | dir::TypeExpression::Mapped { .. }
            | dir::TypeExpression::Predicate { .. } => return Ok(None),
        };

        Ok(Some(ty))
    }

    /// Evaluate one child type expression and intern it on the same module.
    fn child_type_id(
        &mut self,
        module: ModuleId,
        node: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Option<dir::LocalTypeId>> {
        let global = node.into_global(module);
        let variable = self
            .module_mut(module)?
            .node_type_variable(global.clone().into_any());
        let Some(ty) = self.type_expression_type(module, global)? else {
            return Ok(None);
        };
        let type_id = self.module_mut(module)?.intern_type(ty, node.into_any());
        let value = crate::check::VariableValue::Type(type_id);
        let _ = self.module_mut(module)?.bind_variable(variable, value);

        Ok(Some(type_id))
    }

    /// Evaluate child type expressions.
    fn child_type_ids(
        &mut self,
        module: ModuleId,
        nodes: &[dir::LocalNodeId<dir::TypeExpression>],
    ) -> CompilerResult<Option<Vec<dir::LocalTypeId>>> {
        let mut types = Vec::with_capacity(nodes.len());

        // evaluate each child in source order
        for node in nodes {
            let Some(type_id) = self.child_type_id(module, *node)? else {
                return Ok(None);
            };

            types.push(type_id);
        }

        Ok(Some(types))
    }

    /// Evaluate one tuple type.
    fn tuple_type(
        &mut self,
        module: ModuleId,
        elements: &[dir::LocalNodeId<dir::TupleElement>],
    ) -> CompilerResult<Option<dir::TupleType>> {
        let mut type_elements = Vec::with_capacity(elements.len());

        // evaluate tuple elements in source order
        for element in elements {
            let element_node = self.module(module)?.parsed().tree.get(*element).clone();
            let type_element = match element_node {
                dir::TupleElement::Element {
                    label,
                    value,
                    is_optional,
                    is_readonly,
                } => {
                    let Some(ty) = self.child_type_id(module, value)? else {
                        return Ok(None);
                    };

                    dir::TypeElement {
                        label,
                        ty,
                        is_optional,
                        is_readonly,
                        is_rest: false,
                    }
                }
                dir::TupleElement::Spread { label, value } => {
                    let Some(ty) = self.child_type_id(module, value)? else {
                        return Ok(None);
                    };

                    dir::TypeElement {
                        label,
                        ty,
                        is_optional: false,
                        is_readonly: false,
                        is_rest: true,
                    }
                }
                dir::TupleElement::Error => continue,
            };

            type_elements.push(type_element);
        }

        Ok(Some(dir::TupleType {
            elements: type_elements,
            is_readonly: false,
        }))
    }

    /// Evaluate one structural shape type.
    fn shape_type(
        &mut self,
        module: ModuleId,
        members: &[dir::LocalNodeId<dir::TypeMember>],
    ) -> CompilerResult<Option<dir::ShapeType>> {
        let mut fields = Vec::new();
        let mut index_signatures = Vec::new();

        // evaluate fields and index signatures
        for member in members {
            let member_node = self.module(module)?.parsed().tree.get(*member).clone();
            match member_node {
                dir::TypeMember::Field {
                    key,
                    declared_type,
                    is_optional,
                    is_readonly,
                    ..
                } => {
                    let Some(key) = key.static_key(&self.module(module)?.parsed().tree) else {
                        continue;
                    };
                    let Some(declared_type) = declared_type else {
                        return Ok(None);
                    };
                    let Some(ty) = self.child_type_id(module, declared_type)? else {
                        return Ok(None);
                    };

                    fields.push(dir::TypeField {
                        key,
                        ty,
                        is_optional,
                        is_readonly,
                    });
                }
                dir::TypeMember::IndexSignature {
                    name,
                    key_type,
                    value_type,
                    is_optional,
                    is_readonly,
                } => {
                    let Some(key_type) = self.child_type_id(module, key_type)? else {
                        return Ok(None);
                    };
                    let Some(value_type) = self.child_type_id(module, value_type)? else {
                        return Ok(None);
                    };

                    index_signatures.push(dir::TypeIndexSignature {
                        name,
                        key_type,
                        value_type,
                        is_optional,
                        is_readonly,
                    });
                }
                dir::TypeMember::Error => {}
                dir::TypeMember::Method { .. }
                | dir::TypeMember::CallSignature { .. }
                | dir::TypeMember::ConstructSignature { .. }
                | dir::TypeMember::AssociatedType { .. }
                | dir::TypeMember::AssociatedConst { .. } => return Ok(None),
            }
        }

        Ok(Some(dir::ShapeType {
            fields,
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures,
        }))
    }

    /// Evaluate one simple type reference.
    fn reference_type(
        &self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<Option<dir::Type>> {
        if !generic_arguments.is_empty() {
            return Ok(None);
        }
        if path.segments.len() != 1 {
            return Ok(None);
        }

        let key = dir::StaticKey::Name(path.segments[0]);
        let bindings = self.module(module)?.binding_table();
        let Some(scope) = self.module(module)?.find_visible_scope(&bindings, source) else {
            return Ok(None);
        };
        let symbols =
            self.module(module)?
                .find_scope_symbols(&bindings, scope, key, dir::SymbolSpace::Type);
        let Some(symbol) = symbols.first().copied() else {
            return Ok(None);
        };
        if symbols.len() != 1 {
            return Ok(None);
        }

        Ok(Some(dir::Type::Named(dir::NamedType {
            symbol,
            arguments: Vec::new(),
        })))
    }
}
