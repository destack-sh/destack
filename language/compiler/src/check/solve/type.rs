use destack_dir as dir;

use crate::check::{CheckModuleState, TypeInferId, TypeTerm};

impl CheckModuleState {
    /// Solve one type term.
    pub(in crate::check) fn solve_type_term(
        &mut self,
        result: TypeInferId,
        term: &TypeTerm,
    ) -> Option<dir::LocalTypeId> {
        match term {
            TypeTerm::Type(ty) => {
                let source = self.infer_source_node(result);
                Some(self.types_mut().insert_type_from_any(ty.clone(), source))
            }
            TypeTerm::Infer(infer) => self.infer_type_id(*infer),
            TypeTerm::Symbol(symbol) => self.symbol_type_id(*symbol),
            TypeTerm::Static(_) => None,
            TypeTerm::TypeExpression(source) => self.solve_source_type(source),
            TypeTerm::Function {
                source,
                asynchrony,
                parameters,
                return_type,
                is_generator,
            } => self.solve_function_type(
                *source,
                *asynchrony,
                parameters,
                *return_type,
                *is_generator,
            ),
        }
    }

    /// Solve one function type.
    fn solve_function_type(
        &mut self,
        source: dir::GlobalNodeIdAny,
        asynchrony: dir::Asynchrony,
        parameters: &[TypeInferId],
        return_type: Option<TypeInferId>,
        is_generator: bool,
    ) -> Option<dir::LocalTypeId> {
        let mut parameter_types = Vec::new();
        for parameter in parameters {
            parameter_types.push(self.infer_type_id(*parameter)?);
        }
        let return_type = match return_type {
            Some(ty) => Some(self.infer_type_id(ty)?),
            None => None,
        };
        let ty = dir::Type::Function(dir::FunctionType {
            asynchrony,
            generic_parameters: Vec::new(),
            this_parameter: None,
            parameters: parameter_types,
            return_type,
            is_generator,
        });

        Some(self.types_mut().insert_type_from_any(ty, source.local_id))
    }

    /// Solve one source type expression.
    fn solve_source_type(
        &mut self,
        node: &dir::GlobalNodeId<dir::TypeExpression>,
    ) -> Option<dir::LocalTypeId> {
        if node.module_id != self.module() {
            return None;
        }

        let local_id = dir::LocalNodeId::<dir::TypeExpression>::new(node.local_id.id);
        let node_any = local_id.into_global_any(node.module_id);
        if let Some(type_id) = self.node_type_id(node_any) {
            return Some(type_id);
        }

        let expression = self.parsed().tree.get(local_id).clone();

        self.solve_type_expression(local_id, &expression)
    }

    /// Solve one local source type expression.
    fn solve_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        expression: &dir::TypeExpression,
    ) -> Option<dir::LocalTypeId> {
        match expression {
            dir::TypeExpression::Parenthesized { expression } => {
                let node = expression.into_global(self.module());

                self.solve_source_type(&node)
            }
            dir::TypeExpression::ScalarLiteral { value } => {
                let ty = dir::Type::Literal(value.clone());

                Some(self.types_mut().insert_type_from_any(ty, id.into_any()))
            }
            dir::TypeExpression::Literal { value } => {
                let ty = dir::Type::from(value.clone());

                Some(self.types_mut().insert_type_from_any(ty, id.into_any()))
            }
            dir::TypeExpression::Reference {
                path,
                generic_arguments,
            } => self.solve_reference_type(id, path, generic_arguments),
            dir::TypeExpression::Union { elements } => self.solve_union_type(id, elements),
            dir::TypeExpression::Intersection { elements } => {
                self.solve_intersection_type(id, elements)
            }
            dir::TypeExpression::Object { members } => self.solve_shape_type(id, members),
            dir::TypeExpression::Tuple { elements }
            | dir::TypeExpression::ArrayTuple { elements } => self.solve_tuple_type(id, elements),
            dir::TypeExpression::Error => Some(
                self.types_mut()
                    .insert_type_from_any(dir::Type::Error, id.into_any()),
            ),
            dir::TypeExpression::Missing => None,
            _ => None,
        }
    }

    /// Solve one named type reference.
    fn solve_reference_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> Option<dir::LocalTypeId> {
        let name = path.last_segment()?;
        if name == dir::StringId::for_text("Function") {
            return self.solve_function_intrinsic_type(id, generic_arguments);
        }

        let key = dir::StaticKey::Name(name);
        let symbols = self.resolve_name_symbols(id.into_any(), key, dir::SymbolSpace::Type);
        let symbol = symbols.first().copied()?;
        let ty = if self.is_generic_parameter(symbol) {
            dir::Type::Parameter(dir::ParameterType { symbol })
        } else {
            dir::Type::Named(dir::NamedType {
                symbol,
                arguments: Vec::new(),
            })
        };

        Some(self.types_mut().insert_type_from_any(ty, id.into_any()))
    }

    /// Solve one intrinsic `Function<Parameters, Return>` reference.
    fn solve_function_intrinsic_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> Option<dir::LocalTypeId> {
        let [parameters, return_type] = generic_arguments else {
            return None;
        };
        let parameters = self.generic_argument_type_expression(*parameters)?;
        let return_type = self.generic_argument_type_expression(*return_type)?;
        let parameters = self.solve_function_parameter_types(parameters)?;
        let return_type = self.solve_source_type(&return_type.into_global(self.module()))?;
        let ty = dir::Type::Function(dir::FunctionType {
            asynchrony: dir::Asynchrony::Sync,
            generic_parameters: Vec::new(),
            this_parameter: None,
            parameters,
            return_type: Some(return_type),
            is_generator: false,
        });

        Some(self.types_mut().insert_type_from_any(ty, id.into_any()))
    }

    /// Return the type expression carried by one generic type argument.
    fn generic_argument_type_expression(
        &self,
        argument: dir::LocalNodeId<dir::GenericArgument>,
    ) -> Option<dir::LocalNodeId<dir::TypeExpression>> {
        match self.parsed().tree.get(argument) {
            dir::GenericArgument::Type { value } => Some(*value),
            dir::GenericArgument::Value { .. } | dir::GenericArgument::Error => None,
        }
    }

    /// Solve the parameter tuple of one intrinsic function type.
    fn solve_function_parameter_types(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> Option<Vec<dir::LocalTypeId>> {
        let expression = self.parsed().tree.get(id).clone();
        let elements = match expression {
            dir::TypeExpression::Tuple { elements }
            | dir::TypeExpression::ArrayTuple { elements } => elements,
            _ => return None,
        };
        let mut parameters = Vec::new();

        for element in elements {
            let element = self.parsed().tree.get(element).clone();
            match element {
                dir::TupleElement::Element { value, .. } => {
                    let ty = self.solve_source_type(&value.into_global(self.module()))?;

                    parameters.push(ty);
                }
                dir::TupleElement::Spread { .. } | dir::TupleElement::Error => return None,
            }
        }

        Some(parameters)
    }

    /// Solve one union type expression.
    fn solve_union_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        elements: &[dir::LocalNodeId<dir::TypeExpression>],
    ) -> Option<dir::LocalTypeId> {
        let mut types = Vec::new();
        for element in elements {
            let node = element.into_global(self.module());
            let type_id = self.solve_source_type(&node)?;
            if !types.contains(&type_id) {
                types.push(type_id);
            }
        }

        Some(self.types_mut().insert_type_from_any(
            dir::Type::Union(dir::UnionType { elements: types }),
            id.into_any(),
        ))
    }

    /// Solve one intersection type expression.
    fn solve_intersection_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        elements: &[dir::LocalNodeId<dir::TypeExpression>],
    ) -> Option<dir::LocalTypeId> {
        let mut types = Vec::new();
        for element in elements {
            let node = element.into_global(self.module());
            let type_id = self.solve_source_type(&node)?;
            if !types.contains(&type_id) {
                types.push(type_id);
            }
        }

        Some(self.types_mut().insert_type_from_any(
            dir::Type::Intersection(dir::IntersectionType { elements: types }),
            id.into_any(),
        ))
    }

    /// Solve one structural object type expression.
    fn solve_shape_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        members: &[dir::LocalNodeId<dir::TypeMember>],
    ) -> Option<dir::LocalTypeId> {
        let mut fields = Vec::new();

        for member_id in members {
            let member = self.parsed().tree.get(*member_id).clone();
            let dir::TypeMember::Field {
                key,
                declared_type: Some(declared_type),
                is_static: false,
                is_optional,
                is_readonly,
            } = member
            else {
                continue;
            };
            let key = key.direct_static_key()?;
            let node = declared_type.into_global(self.module());
            let ty = self.solve_source_type(&node)?;

            fields.push(dir::TypeField {
                key,
                ty,
                is_optional,
                is_readonly,
            });
        }

        let shape = dir::ShapeType {
            fields,
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        };

        Some(
            self.types_mut()
                .insert_type_from_any(dir::Type::Shape(shape), id.into_any()),
        )
    }

    /// Solve one tuple type expression.
    fn solve_tuple_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        elements: &[dir::LocalNodeId<dir::TupleElement>],
    ) -> Option<dir::LocalTypeId> {
        let mut types = Vec::new();

        for element_id in elements {
            let element = self.parsed().tree.get(*element_id).clone();
            let type_element = match element {
                dir::TupleElement::Element {
                    label,
                    value,
                    is_optional,
                    is_readonly,
                } => {
                    let node = value.into_global(self.module());
                    let ty = self.solve_source_type(&node)?;

                    dir::TypeElement {
                        label,
                        ty,
                        is_optional,
                        is_readonly,
                        is_rest: false,
                    }
                }
                dir::TupleElement::Spread { label, value } => {
                    let node = value.into_global(self.module());
                    let ty = self.solve_source_type(&node)?;

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

            types.push(type_element);
        }

        let ty = dir::Type::Tuple(dir::TupleType {
            elements: types,
            is_readonly: false,
        });

        Some(self.types_mut().insert_type_from_any(ty, id.into_any()))
    }

    /// Return whether one symbol is a generic parameter.
    fn is_generic_parameter(&self, symbol: dir::GlobalSymbolId) -> bool {
        if symbol.module_id != self.module() {
            return false;
        }

        self.binding_table()
            .get_symbol(symbol.local_id)
            .is_generic_parameter()
    }
}
