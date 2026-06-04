use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    FormTerm, MappedParameter, MemberTerm, Origin, ReceiverCapture, ShapeMember, StaticOperand,
    StaticTerm, TupleElement, TypeLiteralTerm, TypeOperand, TypeOperationTerm, TypeRelation,
    TypeTerm, WalkState,
};

impl WalkState<'_, '_> {
    /// Walk one type expression.
    ///
    /// Example:
    /// ```ds
    /// readonly Box<T>
    /// ```
    pub(in crate::check) fn walk_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        type_expression: &dir::TypeExpression,
    ) -> CompilerResult<()> {
        // enter static owner guard
        let Some(_guard) = self.enter_static_guard_for(id.into_any(), None)? else {
            return Ok(());
        };

        match type_expression {
            // (T)
            dir::TypeExpression::Parenthesized { expression } => {
                self.walk_parenthesized_type_expression(id, *expression)?;
            }
            // 1
            dir::TypeExpression::ScalarLiteral { value } => {
                self.walk_scalar_literal_type_expression(id, value)?;
            }
            // null
            dir::TypeExpression::Literal { value } => {
                self.walk_literal_type_expression(id, value)?;
            }
            // intrinsic marker
            dir::TypeExpression::Intrinsic => {
                self.walk_intrinsic_type_expression(id)?;
            }
            // const
            dir::TypeExpression::Const => {
                self.walk_const_type_expression(id)?;
            }
            // this
            dir::TypeExpression::This => {
                self.walk_this_type_expression(id)?;
            }
            // (T, U)
            dir::TypeExpression::Tuple { elements } => {
                self.walk_tuple_type_expression(id, elements)?;
            }
            // [T, U]
            dir::TypeExpression::ArrayTuple { elements } => {
                self.walk_array_tuple_type_expression(id, elements)?;
            }
            // T[]
            dir::TypeExpression::Array { element } => {
                self.walk_array_type_expression(id, *element)?;
            }
            // [T]
            dir::TypeExpression::Slice { element } => {
                self.walk_slice_type_expression(id, *element)?;
            }
            // [T; N]
            dir::TypeExpression::FixedArray { element, length } => {
                self.walk_fixed_array_type_expression(id, *element, *length)?;
            }
            // { name: T }
            dir::TypeExpression::Object { members } => {
                self.walk_object_type_expression(id, members)?;
            }
            // (value: T) => U
            dir::TypeExpression::Function(declaration) => {
                self.walk_function_type_expression(id, declaration)?;
            }
            // new (value: T) => U
            dir::TypeExpression::Constructor(declaration) => {
                self.walk_constructor_type_expression(id, declaration)?;
            }
            // T
            dir::TypeExpression::Reference {
                path,
                generic_arguments,
            } => {
                self.walk_reference_type_expression(id, path, generic_arguments)?;
            }
            // T.Item
            dir::TypeExpression::Member {
                left,
                name,
                generic_arguments,
            } => {
                self.walk_member_type_expression(id, *left, *name, generic_arguments)?;
            }
            // 0..10
            dir::TypeExpression::Range {
                start,
                end,
                end_kind,
            } => {
                self.walk_range_type_expression(id, *start, *end, *end_kind)?;
            }
            // readonly T
            dir::TypeExpression::Readonly { target_type } => {
                self.walk_readonly_type_expression(id, *target_type)?;
            }
            // ^T
            dir::TypeExpression::OwnedOf { target_type, .. } => {
                self.walk_owned_type_expression(id, *target_type)?;
            }
            // &T
            dir::TypeExpression::BorrowedOf {
                mutability,
                target_type,
                ..
            } => {
                self.walk_borrowed_type_expression(id, *mutability, *target_type)?;
            }
            // *T
            dir::TypeExpression::PointerOf { target_type, .. } => {
                self.walk_pointer_type_expression(id, *target_type)?;
            }
            // local T
            dir::TypeExpression::Local { target_type } => {
                self.walk_local_type_expression(id, *target_type)?;
            }
            // shared T
            dir::TypeExpression::Shared { target_type } => {
                self.walk_shared_type_expression(id, *target_type)?;
            }
            // T!
            dir::TypeExpression::Must { target_type } => {
                self.walk_must_type_expression(id, *target_type)?;
            }
            // !T
            dir::TypeExpression::Not { target_type } => {
                self.walk_not_type_expression(id, *target_type)?;
            }
            // keyof T
            dir::TypeExpression::KeyOf { target_type } => {
                self.walk_keyof_type_expression(id, *target_type)?;
            }
            // typeof value
            dir::TypeExpression::TypeOfValue { value } => {
                self.walk_typeof_value_type_expression(id, *value)?;
            }
            // T | U
            dir::TypeExpression::Union { elements } => {
                self.walk_union_type_expression(id, elements)?;
            }
            // T & U
            dir::TypeExpression::Intersection { elements } => {
                self.walk_intersection_type_expression(id, elements)?;
            }
            // T extends U ? X : Y
            dir::TypeExpression::Conditional {
                left,
                extends_type,
                then_type,
                else_type,
            } => self.walk_conditional_type_expression(
                id,
                *left,
                *extends_type,
                *then_type,
                *else_type,
            )?,
            // T extends U
            dir::TypeExpression::Extends { left, right } => {
                self.walk_extends_type_expression(id, *left, *right)?;
            }
            // T implements U
            dir::TypeExpression::Implements { left, right } => {
                self.walk_implements_type_expression(id, *left, *right)?;
            }
            // { [K in keyof T]: T[K] }
            dir::TypeExpression::Mapped {
                parameter,
                readonly,
                optional,
                value,
            } => {
                self.walk_mapped_type_expression(id, *parameter, *readonly, *optional, *value)?;
            }
            // T[K]
            dir::TypeExpression::Index { left, index } => {
                self.walk_index_type_expression(id, *left, *index)?;
            }
            // `get${Name}`
            dir::TypeExpression::TemplateLiteral { strings, spans } => {
                self.walk_template_literal_type_expression(id, strings, spans)?;
            }
            // _
            dir::TypeExpression::Infer {
                form: dir::InferForm::Hole,
                name: _,
                constraint,
            } => {
                self.walk_hole_type_expression(id, *constraint)?;
            }
            // infer T extends U
            dir::TypeExpression::Infer {
                form: dir::InferForm::Infer,
                name,
                constraint,
                ..
            } => {
                self.walk_infer_type_expression(id, *name, *constraint)?;
            }
            // missing type
            dir::TypeExpression::Missing => {}
            // ignore damaged syntax
            dir::TypeExpression::Error => {}
        }

        Ok(())
    }

    /// Lower one type expression operand without committing a checked node operand.
    ///
    /// Example:
    /// ```ds
    /// Box<T>
    /// ```
    pub(in crate::check) fn lower_type_expression_operand(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<TypeOperand> {
        // return directly lowered terms when possible
        if let Some(term) = self.lower_type_expression_term(id)? {
            let term = self.check.inference.push_term(term);

            return Ok(term.into());
        }

        // fall back to an argument local hole
        let source = id.into_global_any(self.module);
        let origin = Origin::Node(source);
        let variable = self.check.create_type_variable(self.module, origin);

        Ok(variable.into())
    }

    /// Lower one type expression term without committing a checked node operand.
    ///
    /// Example:
    /// ```ds
    /// readonly Box<T>
    /// ```
    fn lower_type_expression_term(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Option<TypeTerm>> {
        let term = match self.tree.get(id) {
            // (T)
            dir::TypeExpression::Parenthesized { expression } => {
                let operand = self.lower_type_expression_operand(*expression)?;
                let Some(term) = operand.known_type_term(self.check) else {
                    return Ok(None);
                };

                term
            }
            // 1
            dir::TypeExpression::ScalarLiteral { value } => {
                TypeTerm::Literal(TypeLiteralTerm::Scalar(value.clone()))
            }
            // null
            dir::TypeExpression::Literal { value } => {
                self.lower_type_literal_term(self.module, id, value)
            }
            // this
            dir::TypeExpression::This => TypeTerm::This,
            // T[]
            dir::TypeExpression::Array { element } => TypeTerm::Array {
                element: self.lower_type_expression_operand(*element)?.into(),
            },
            // [T]
            dir::TypeExpression::Slice { element } => TypeTerm::Slice {
                element: self.lower_type_expression_operand(*element)?.into(),
            },
            // [T; N]
            dir::TypeExpression::FixedArray { element, length } => TypeTerm::FixedArray {
                element: self.lower_type_expression_operand(*element)?.into(),
                length: self.walk_static_expression_operand(*length)?,
            },
            // T
            dir::TypeExpression::Reference {
                path,
                generic_arguments,
            } => {
                if let Some(term) = self.lower_string_mapping_term(path, generic_arguments)? {
                    term
                } else {
                    let Some(term) = self.bind_reference_type_term(id, path, generic_arguments)?
                    else {
                        return Ok(None);
                    };

                    term
                }
            }
            // T.Item
            dir::TypeExpression::Member {
                left,
                name,
                generic_arguments,
            } => {
                let arguments = self.walk_generic_arguments(generic_arguments)?;
                let owner = self.lower_type_expression_operand(*left)?;
                let member = self.check.inference.push_term(MemberTerm {
                    origin: Origin::Node(id.into_global_any(self.module)),
                    owner,
                    key: dir::StaticKey::Name(*name),
                    arguments,
                });

                TypeTerm::Member(member)
            }
            // readonly T
            dir::TypeExpression::Readonly { target_type } => {
                let form = self.check.inference.push_term(FormTerm::Readonly);

                TypeTerm::Form {
                    form,
                    payload: self.lower_type_expression_operand(*target_type)?.into(),
                }
            }
            // ^T
            dir::TypeExpression::OwnedOf { target_type, .. } => {
                let form = self.check.inference.push_term(FormTerm::Owned);

                TypeTerm::Form {
                    form,
                    payload: self.lower_type_expression_operand(*target_type)?.into(),
                }
            }
            // &T
            dir::TypeExpression::BorrowedOf {
                mutability,
                target_type,
                ..
            } => {
                let source = id.into_global_any(self.module);
                let payload = self.lower_type_expression_operand(*target_type)?.into();

                self.lower_borrowed_form_type(self.module, source, *mutability, payload)
            }
            // *T
            dir::TypeExpression::PointerOf { target_type, .. } => {
                let form = self.check.inference.push_term(FormTerm::Raw);

                TypeTerm::Form {
                    form,
                    payload: self.lower_type_expression_operand(*target_type)?.into(),
                }
            }
            // local T
            dir::TypeExpression::Local { target_type } => {
                let place = dir::StaticTerm::Place {
                    place: dir::Place::Space(dir::Space::Local),
                };
                let place = self.check.inference.push_term(StaticTerm::Literal(place));
                let form = self.check.inference.push_term(FormTerm::Placed {
                    place: place.into(),
                });

                TypeTerm::Form {
                    form,
                    payload: self.lower_type_expression_operand(*target_type)?.into(),
                }
            }
            // shared T
            dir::TypeExpression::Shared { target_type } => {
                let place = dir::StaticTerm::Place {
                    place: dir::Place::Space(dir::Space::Shared),
                };
                let place = self.check.inference.push_term(StaticTerm::Literal(place));
                let form = self.check.inference.push_term(FormTerm::Placed {
                    place: place.into(),
                });

                TypeTerm::Form {
                    form,
                    payload: self.lower_type_expression_operand(*target_type)?.into(),
                }
            }
            // T!
            dir::TypeExpression::Must { target_type } => {
                let source = self.lower_type_expression_operand(*target_type)?;
                let null = self
                    .check
                    .inference
                    .push_term(TypeTerm::Literal(TypeLiteralTerm::Null));
                let without_null = self.check.inference.push_term(TypeOperationTerm::Exclude {
                    source,
                    target: null.into(),
                });
                let without_null = self
                    .check
                    .inference
                    .push_term(TypeTerm::Operation(without_null));
                let undefined = self
                    .check
                    .inference
                    .push_term(TypeTerm::Literal(TypeLiteralTerm::Undefined));
                let without_undefined =
                    self.check.inference.push_term(TypeOperationTerm::Exclude {
                        source: without_null.into(),
                        target: undefined.into(),
                    });

                TypeTerm::Operation(without_undefined)
            }
            // keyof T
            dir::TypeExpression::KeyOf { target_type } => {
                let target = self.lower_type_expression_operand(*target_type)?;
                let operation = self
                    .check
                    .inference
                    .push_term(TypeOperationTerm::KeyOf { target });

                TypeTerm::Operation(operation)
            }
            // T | U
            dir::TypeExpression::Union { elements } => {
                let mut element_terms = Vec::with_capacity(elements.len());

                // collect union operands
                for element in elements {
                    element_terms.push(self.lower_type_expression_operand(*element)?);
                }

                TypeTerm::Union {
                    elements: element_terms,
                }
            }
            // T & U
            dir::TypeExpression::Intersection { elements } => {
                let mut element_terms = Vec::with_capacity(elements.len());

                // collect intersection operands
                for element in elements {
                    element_terms.push(self.lower_type_expression_operand(*element)?);
                }

                TypeTerm::Intersection {
                    elements: element_terms,
                }
            }
            // T extends U ? X : Y
            dir::TypeExpression::Conditional {
                left,
                extends_type,
                then_type,
                else_type,
            } => {
                // lower all conditional operands before interning the operation
                let left = self.lower_type_expression_operand(*left)?;
                let right = self.lower_type_expression_operand(*extends_type)?;
                let then_type = self.lower_type_expression_operand(*then_type)?;
                let else_type = self.lower_type_expression_operand(*else_type)?;
                let operation = self
                    .check
                    .inference
                    .push_term(TypeOperationTerm::Conditional {
                        left,
                        right,
                        then_type,
                        else_type,
                    });

                TypeTerm::Operation(operation)
            }
            // T[K]
            dir::TypeExpression::Index { left, index } => {
                // lower both sides before interning the operation
                let left = self.lower_type_expression_operand(*left)?;
                let index = self.lower_type_expression_operand(*index)?;
                let operation = self
                    .check
                    .inference
                    .push_term(TypeOperationTerm::Index { left, index });

                TypeTerm::Operation(operation)
            }
            // `get${Name}`
            dir::TypeExpression::TemplateLiteral { strings, spans } => {
                let mut span_terms = Vec::with_capacity(spans.len());

                // collect template span operands
                for span in spans {
                    span_terms.push(self.lower_type_expression_operand(*span)?);
                }
                let operation =
                    self.check
                        .inference
                        .push_term(TypeOperationTerm::TemplateLiteral {
                            strings: strings.to_vec(),
                            spans: span_terms,
                        });

                TypeTerm::Operation(operation)
            }
            // infer T extends U
            dir::TypeExpression::Infer {
                form: dir::InferForm::Infer,
                name,
                constraint,
                ..
            } => {
                let constraint = constraint
                    .map(|constraint| self.lower_type_expression_operand(constraint))
                    .transpose()?;
                let operation = self.check.inference.push_term(TypeOperationTerm::Infer {
                    name: *name,
                    constraint,
                });

                TypeTerm::Operation(operation)
            }
            // unsupported type expression lowering
            _ => return Ok(None),
        };

        Ok(Some(term))
    }

    /// Walk one parenthesized type expression.
    ///
    /// Example:
    /// ```ds
    /// (T)
    /// ```
    fn walk_parenthesized_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        expression: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        self.walk_type_expression(expression, self.tree.get(expression))?;

        let ty = self.allocate_node_type_operand(expression)?;

        self.bind_node_type_operand(id, ty)?;

        Ok(())
    }

    /// Walk one scalar literal type expression.
    ///
    /// Example:
    /// ```ds
    /// 1
    /// ```
    fn walk_scalar_literal_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        value: &dir::ScalarLiteral,
    ) -> CompilerResult<()> {
        let term = TypeTerm::Literal(TypeLiteralTerm::Scalar(value.clone()));

        self.bind_node_type(id, term)?;

        Ok(())
    }

    /// Walk one literal type expression.
    ///
    /// Example:
    /// ```ds
    /// null
    /// ```
    fn walk_literal_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        value: &dir::TypeLiteral,
    ) -> CompilerResult<()> {
        let term = self.lower_type_literal_term(self.module, id, value);

        self.bind_node_type(id, term)?;

        Ok(())
    }

    /// Lower one source type literal into a type term.
    ///
    /// Example:
    /// ```ds
    /// string
    /// ```
    fn lower_type_literal_term(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::TypeExpression>,
        value: &dir::TypeLiteral,
    ) -> TypeTerm {
        // report unsupported object constraint
        if *value == dir::TypeLiteral::Object {
            self.check
                .report_unsupported_type(module, id.into_any(), "object");

            return TypeTerm::Literal(TypeLiteralTerm::Error);
        }

        // normalize void to the unit type
        if *value == dir::TypeLiteral::Void {
            return TypeTerm::unit();
        }

        let literal = TypeLiteralTerm::from_literal(value);

        TypeTerm::Literal(literal)
    }

    /// Walk one intrinsic marker type expression.
    ///
    /// Example:
    /// ```ds
    /// intrinsic
    /// ```
    fn walk_intrinsic_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        self.check
            .report_invalid_intrinsic_type(self.module, id.into_any());

        self.bind_node_type(id, TypeTerm::Literal(TypeLiteralTerm::Error))?;

        Ok(())
    }

    /// Walk one const marker type expression.
    ///
    /// Example:
    /// ```ds
    /// const
    /// ```
    fn walk_const_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        self.check
            .report_invalid_const_type(self.module, id.into_any());

        self.bind_node_type(id, TypeTerm::Literal(TypeLiteralTerm::Error))?;

        Ok(())
    }

    /// Walk one this type expression.
    ///
    /// Example:
    /// ```ds
    /// this
    /// ```
    fn walk_this_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        self.bind_node_type(id, TypeTerm::This)?;

        Ok(())
    }

    /// Walk one tuple type expression.
    ///
    /// Example:
    /// ```ds
    /// (left: string, right?: number)
    /// ```
    fn walk_tuple_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        elements: &[dir::LocalNodeId<dir::TupleElement>],
    ) -> CompilerResult<()> {
        // walk tuple elements
        for element in elements {
            self.walk_tuple_element(*element, self.tree.get(*element))?;
        }

        let term = self.lower_tuple_type_term(elements, dir::TupleForm::Tuple)?;

        self.bind_node_type(id, term)?;

        Ok(())
    }

    /// Walk one array tuple type expression.
    ///
    /// Example:
    /// ```ds
    /// [string, number]
    /// ```
    fn walk_array_tuple_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        elements: &[dir::LocalNodeId<dir::TupleElement>],
    ) -> CompilerResult<()> {
        // walk tuple elements
        for element in elements {
            self.walk_tuple_element(*element, self.tree.get(*element))?;
        }

        let term = self.lower_tuple_type_term(elements, dir::TupleForm::Array)?;

        self.bind_node_type(id, term)?;

        Ok(())
    }

    /// Walk one array type expression.
    ///
    /// Example:
    /// ```ds
    /// T[]
    /// ```
    fn walk_array_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        element: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        self.walk_type_expression(element, self.tree.get(element))?;

        let term = TypeTerm::Array {
            element: self.allocate_node_type_operand(element)?.into(),
        };

        self.bind_node_type(id, term)?;

        Ok(())
    }

    /// Walk one slice type expression.
    ///
    /// Example:
    /// ```ds
    /// [T]
    /// ```
    fn walk_slice_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        element: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        self.walk_type_expression(element, self.tree.get(element))?;

        let term = TypeTerm::Slice {
            element: self.allocate_node_type_operand(element)?.into(),
        };

        self.bind_node_type(id, term)?;

        Ok(())
    }

    /// Walk one fixed array type expression.
    ///
    /// Example:
    /// ```ds
    /// [T; 4]
    /// ```
    fn walk_fixed_array_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        element: dir::LocalNodeId<dir::TypeExpression>,
        length: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        self.walk_type_expression(element, self.tree.get(element))?;

        // check fixed array length in type context
        let before_length = self.fork_flow();

        self.walk_expression(length, self.tree.get(length))?;
        self.restore_flow(before_length);

        let condition = self.active_static_guard();
        let length_variable = self.allocate_static_expression_variable(length, condition)?;
        let term = TypeTerm::FixedArray {
            element: self.allocate_node_type_operand(element)?.into(),
            length: length_variable.into(),
        };

        self.bind_node_type(id, term)?;

        Ok(())
    }

    /// Walk one object type expression.
    ///
    /// Example:
    /// ```ds
    /// { name: string, count?: number }
    /// ```
    fn walk_object_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        members: &[dir::LocalNodeId<dir::TypeMember>],
    ) -> CompilerResult<()> {
        // walk shape members
        for member in members {
            self.walk_type_member(*member, self.tree.get(*member))?;
        }

        let members = self.lower_shape_member_terms(members)?;
        let term = self.check.push_shape_type(members);

        self.bind_node_type(id, term)?;

        Ok(())
    }

    /// Walk one function type expression.
    ///
    /// Example:
    /// ```ds
    /// (value: T) => U
    /// ```
    fn walk_function_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        declaration: &dir::FunctionTypeExpression,
    ) -> CompilerResult<()> {
        self.walk_function_type(declaration)?;

        let return_type = declaration
            .return_type
            .map(|return_type| self.allocate_node_type_operand(return_type))
            .transpose()?;
        let term = self.lower_function_type_term(declaration, return_type)?;

        self.bind_node_type(id, TypeTerm::Function(term))?;

        Ok(())
    }

    /// Walk one constructor type expression.
    ///
    /// Example:
    /// ```ds
    /// new (value: T) => Box<T>
    /// ```
    fn walk_constructor_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        declaration: &dir::ConstructorType,
    ) -> CompilerResult<()> {
        self.walk_constructor_type(declaration)?;

        let return_type = declaration
            .return_type
            .map(|return_type| self.allocate_node_type_operand(return_type))
            .transpose()?;
        let term = self.lower_constructor_type_term(declaration, return_type)?;

        self.bind_node_type(id, TypeTerm::Function(term))?;

        Ok(())
    }

    /// Walk one reference type expression.
    ///
    /// Example:
    /// ```ds
    /// Box<T>
    /// ```
    fn walk_reference_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<()> {
        if let Some(term) = self.lower_string_mapping_term(path, generic_arguments)? {
            self.bind_node_type(id, term)?;
            return Ok(());
        }

        if let Some(term) = self.bind_reference_type_term(id, path, generic_arguments)? {
            self.bind_node_type(id, term)?;
        }

        Ok(())
    }

    /// Walk one member type expression.
    ///
    /// Example:
    /// ```ds
    /// Box<T>.Item
    /// ```
    fn walk_member_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        left: dir::LocalNodeId<dir::TypeExpression>,
        name: dir::StringId,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<()> {
        self.walk_type_expression(left, self.tree.get(left))?;

        let arguments = self.walk_generic_arguments(generic_arguments)?;
        let owner = self.allocate_node_type_operand(left)?;
        let member = self.check.inference.push_term(MemberTerm {
            origin: Origin::Node(id.into_global_any(self.module)),
            owner,
            key: dir::StaticKey::Name(name),
            arguments,
        });
        let term = TypeTerm::Member(member);

        self.bind_node_type(id, term)?;

        Ok(())
    }

    /// Walk one range type expression.
    ///
    /// Example:
    /// ```ds
    /// 0..10
    /// ```
    fn walk_range_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        start: Option<dir::LocalNodeId<dir::TypeExpression>>,
        end: Option<dir::LocalNodeId<dir::TypeExpression>>,
        end_kind: dir::RangeEnd,
    ) -> CompilerResult<()> {
        if let Some(term) = self.lower_range_type_term(start, end, end_kind) {
            self.bind_node_type(id, term)?;
        }

        // walk range boundaries
        if let Some(start) = start {
            self.walk_type_expression(start, self.tree.get(start))?;
        }
        if let Some(end) = end {
            self.walk_type_expression(end, self.tree.get(end))?;
        }

        Ok(())
    }

    /// Walk one readonly type expression.
    ///
    /// Example:
    /// ```ds
    /// readonly T
    /// ```
    fn walk_readonly_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        self.walk_type_expression(target_type, self.tree.get(target_type))?;

        let form = self.check.inference.push_term(FormTerm::Readonly);
        let term = TypeTerm::Form {
            form,
            payload: self.allocate_node_type_operand(target_type)?.into(),
        };

        self.bind_node_type(id, term)?;

        Ok(())
    }

    /// Walk one owned type expression.
    ///
    /// Example:
    /// ```ds
    /// ^T
    /// ```
    fn walk_owned_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        self.walk_type_expression(target_type, self.tree.get(target_type))?;

        let form = self.check.inference.push_term(FormTerm::Owned);
        let term = TypeTerm::Form {
            form,
            payload: self.allocate_node_type_operand(target_type)?.into(),
        };

        self.bind_node_type(id, term)?;

        Ok(())
    }

    /// Walk one borrowed type expression.
    ///
    /// Example:
    /// ```ds
    /// &mut T
    /// ```
    fn walk_borrowed_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        mutability: Option<dir::Mutability>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        self.walk_type_expression(target_type, self.tree.get(target_type))?;

        let source = id.into_global_any(self.module);
        let payload = self.allocate_node_type_operand(target_type)?.into();
        let term = self.lower_borrowed_form_type(self.module, source, mutability, payload);

        self.bind_node_type(id, term)?;

        Ok(())
    }

    /// Return one borrowed form over a payload type.
    ///
    /// Example:
    /// ```ds
    /// &mut T
    /// ```
    pub(in crate::check) fn lower_borrowed_form_type(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        mutability: Option<dir::Mutability>,
        payload: TypeOperand,
    ) -> TypeTerm {
        let origin = Origin::Node(source);
        let access = mutability
            .map(dir::Mutability::access)
            .unwrap_or(dir::Access::Mutable);
        let access = self
            .check
            .inference
            .push_term(StaticTerm::Literal(dir::StaticTerm::Access { access }));
        let lifetime = self.check.create_static_variable(module, origin);
        self.check
            .induce_language_static_generic(lifetime, "L", dir::LanguageItem::Lifetime);
        let form = self.check.inference.push_term(FormTerm::Borrowed {
            lifetime: lifetime.into(),
            access: access.into(),
        });

        TypeTerm::Form { form, payload }
    }

    /// Walk one pointer type expression.
    ///
    /// Example:
    /// ```ds
    /// *T
    /// ```
    fn walk_pointer_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        self.walk_type_expression(target_type, self.tree.get(target_type))?;

        let form = self.check.inference.push_term(FormTerm::Raw);
        let term = TypeTerm::Form {
            form,
            payload: self.allocate_node_type_operand(target_type)?.into(),
        };

        self.bind_node_type(id, term)?;

        Ok(())
    }

    /// Walk one local type expression.
    ///
    /// Example:
    /// ```ds
    /// local T
    /// ```
    fn walk_local_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        self.walk_type_expression(target_type, self.tree.get(target_type))?;

        let place = dir::StaticTerm::Place {
            place: dir::Place::Space(dir::Space::Local),
        };
        let place = self.check.inference.push_term(StaticTerm::Literal(place));
        let form = self.check.inference.push_term(FormTerm::Placed {
            place: place.into(),
        });
        let term = TypeTerm::Form {
            form,
            payload: self.allocate_node_type_operand(target_type)?.into(),
        };

        self.bind_node_type(id, term)?;

        Ok(())
    }

    /// Walk one shared type expression.
    ///
    /// Example:
    /// ```ds
    /// shared T
    /// ```
    fn walk_shared_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        self.walk_type_expression(target_type, self.tree.get(target_type))?;

        let place = dir::StaticTerm::Place {
            place: dir::Place::Space(dir::Space::Shared),
        };
        let place = self.check.inference.push_term(StaticTerm::Literal(place));
        let form = self.check.inference.push_term(FormTerm::Placed {
            place: place.into(),
        });
        let term = TypeTerm::Form {
            form,
            payload: self.allocate_node_type_operand(target_type)?.into(),
        };

        self.bind_node_type(id, term)?;

        Ok(())
    }

    /// Walk one must type expression.
    ///
    /// Example:
    /// ```ds
    /// T!
    /// ```
    fn walk_must_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        self.walk_type_expression(target_type, self.tree.get(target_type))?;

        let source = self.allocate_node_type_operand(target_type)?;
        let null = self
            .check
            .inference
            .push_term(TypeTerm::Literal(TypeLiteralTerm::Null));
        let without_null = self.check.inference.push_term(TypeOperationTerm::Exclude {
            source,
            target: null.into(),
        });
        let without_null = self
            .check
            .inference
            .push_term(TypeTerm::Operation(without_null));
        let undefined = self
            .check
            .inference
            .push_term(TypeTerm::Literal(TypeLiteralTerm::Undefined));
        let without_undefined = self.check.inference.push_term(TypeOperationTerm::Exclude {
            source: without_null.into(),
            target: undefined.into(),
        });
        let term = TypeTerm::Operation(without_undefined);

        self.bind_node_type(id, term)?;

        Ok(())
    }

    /// Walk one not type expression.
    ///
    /// Example:
    /// ```ds
    /// !T
    /// ```
    fn walk_not_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        let target = self.create_static_argument_variable(target_type)?;
        let left = target.into();
        let right = self
            .check
            .inference
            .push_term(StaticTerm::Literal(dir::StaticTerm::ScalarLiteral {
                value: dir::ScalarLiteral::Boolean(true),
            }))
            .into();
        let value = StaticTerm::Equal {
            left,
            right,
            is_negated: true,
        };

        self.bind_static_value_type(id, value)?;

        Ok(())
    }

    /// Walk one keyof type expression.
    ///
    /// Example:
    /// ```ds
    /// keyof T
    /// ```
    fn walk_keyof_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        self.walk_type_expression(target_type, self.tree.get(target_type))?;

        let target = self.allocate_node_type_operand(target_type)?;
        let operation = self
            .check
            .inference
            .push_term(TypeOperationTerm::KeyOf { target });
        let term = TypeTerm::Operation(operation);

        self.bind_node_type(id, term)?;

        Ok(())
    }

    /// Walk one typeof value type expression.
    ///
    /// Example:
    /// ```ds
    /// typeof value
    /// ```
    fn walk_typeof_value_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // check type query operand in type context
        let before_value = self.fork_flow();

        self.walk_expression(value, self.tree.get(value))?;
        self.restore_flow(before_value);

        let ty = self.allocate_node_type_operand(value)?;

        self.bind_node_type_operand(id, ty)?;

        Ok(())
    }

    /// Walk one union type expression.
    ///
    /// Example:
    /// ```ds
    /// A | B
    /// ```
    fn walk_union_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        elements: &[dir::LocalNodeId<dir::TypeExpression>],
    ) -> CompilerResult<()> {
        // walk union elements
        for element in elements {
            self.walk_type_expression(*element, self.tree.get(*element))?;
        }

        let mut element_terms = Vec::with_capacity(elements.len());

        // collect union operands
        for element in elements {
            element_terms.push(self.allocate_node_type_operand(*element)?.into());
        }

        let term = TypeTerm::Union {
            elements: element_terms,
        };

        self.bind_node_type(id, term)?;

        Ok(())
    }

    /// Walk one intersection type expression.
    ///
    /// Example:
    /// ```ds
    /// A & B
    /// ```
    fn walk_intersection_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        elements: &[dir::LocalNodeId<dir::TypeExpression>],
    ) -> CompilerResult<()> {
        // walk intersection elements
        for element in elements {
            self.walk_type_expression(*element, self.tree.get(*element))?;
        }

        let mut element_terms = Vec::with_capacity(elements.len());

        // collect intersection operands
        for element in elements {
            element_terms.push(self.allocate_node_type_operand(*element)?.into());
        }

        let term = TypeTerm::Intersection {
            elements: element_terms,
        };

        self.bind_node_type(id, term)?;

        Ok(())
    }

    /// Walk one conditional type expression.
    ///
    /// Example:
    /// ```ds
    /// T extends U ? X : Y
    /// ```
    fn walk_conditional_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        left: dir::LocalNodeId<dir::TypeExpression>,
        extends_type: dir::LocalNodeId<dir::TypeExpression>,
        then_type: dir::LocalNodeId<dir::TypeExpression>,
        else_type: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        self.walk_type_expression(left, self.tree.get(left))?;
        self.walk_type_expression(extends_type, self.tree.get(extends_type))?;
        self.walk_type_expression(then_type, self.tree.get(then_type))?;
        self.walk_type_expression(else_type, self.tree.get(else_type))?;

        let left_variable = self.allocate_node_type_operand(left)?;
        let right_variable = self.allocate_node_type_operand(extends_type)?;
        let then_variable = self.allocate_node_type_operand(then_type)?;
        let else_variable = self.allocate_node_type_operand(else_type)?;
        let operation = self
            .check
            .inference
            .push_term(TypeOperationTerm::Conditional {
                left: left_variable,
                right: right_variable,
                then_type: then_variable,
                else_type: else_variable,
            });
        let term = TypeTerm::Operation(operation);

        self.bind_node_type(id, term)?;

        Ok(())
    }

    /// Walk one extends type expression.
    ///
    /// Example:
    /// ```ds
    /// T extends U
    /// ```
    fn walk_extends_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        left: dir::LocalNodeId<dir::TypeExpression>,
        right: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        self.walk_type_relation_expression(id, TypeRelation::Extends, left, right)?;

        Ok(())
    }

    /// Walk one implements type expression.
    ///
    /// Example:
    /// ```ds
    /// T implements U
    /// ```
    fn walk_implements_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        left: dir::LocalNodeId<dir::TypeExpression>,
        right: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        self.walk_type_relation_expression(id, TypeRelation::Implements, left, right)?;

        Ok(())
    }

    /// Walk one type relation expression.
    ///
    /// Example:
    /// ```ds
    /// T extends U
    /// ```
    fn walk_type_relation_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        relation: TypeRelation,
        left: dir::LocalNodeId<dir::TypeExpression>,
        right: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        self.walk_type_expression(left, self.tree.get(left))?;
        self.walk_type_expression(right, self.tree.get(right))?;

        let value = StaticTerm::TypeRelation {
            relation,
            left: self.allocate_node_type_operand(left)?,
            right: self.allocate_node_type_operand(right)?,
        };

        self.bind_static_value_type(id, value)?;

        Ok(())
    }

    /// Walk one mapped type expression.
    ///
    /// Example:
    /// ```ds
    /// { [K in keyof T]: T[K] }
    /// ```
    fn walk_mapped_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        parameter: dir::LocalNodeId<dir::TypeMappedParameter>,
        readonly: dir::MappedTypeModifier,
        optional: dir::MappedTypeModifier,
        value: Option<dir::LocalNodeId<dir::TypeExpression>>,
    ) -> CompilerResult<()> {
        self.walk_type_mapped_parameter(parameter, self.tree.get(parameter))?;

        if let Some(value) = value {
            self.walk_type_expression(value, self.tree.get(value))?;
        }

        if let Some(term) = self.lower_mapped_type_term(parameter, readonly, optional, value)? {
            self.bind_node_type(id, term)?;
        }

        Ok(())
    }

    /// Walk one index type expression.
    ///
    /// Example:
    /// ```ds
    /// T[K]
    /// ```
    fn walk_index_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        left: dir::LocalNodeId<dir::TypeExpression>,
        index: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        self.walk_type_expression(left, self.tree.get(left))?;
        self.walk_type_expression(index, self.tree.get(index))?;

        let left_variable = self.allocate_node_type_operand(left)?;
        let index_variable = self.allocate_node_type_operand(index)?;
        let operation = self.check.inference.push_term(TypeOperationTerm::Index {
            left: left_variable,
            index: index_variable,
        });
        let term = TypeTerm::Operation(operation);

        self.bind_node_type(id, term)?;

        Ok(())
    }

    /// Walk one template literal type expression.
    ///
    /// Example:
    /// ```ds
    /// `get${Name}`
    /// ```
    fn walk_template_literal_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        strings: &[dir::StringId],
        spans: &[dir::LocalNodeId<dir::TypeExpression>],
    ) -> CompilerResult<()> {
        // walk template spans
        for span in spans {
            self.walk_type_expression(*span, self.tree.get(*span))?;
        }

        let mut span_variables = Vec::with_capacity(spans.len());

        // collect template span operands
        for span in spans {
            span_variables.push(self.allocate_node_type_operand(*span)?);
        }

        let operation = self
            .check
            .inference
            .push_term(TypeOperationTerm::TemplateLiteral {
                strings: strings.to_vec(),
                spans: span_variables,
            });
        let term = TypeTerm::Operation(operation);

        self.bind_node_type(id, term)?;

        Ok(())
    }

    /// Walk one anonymous inference hole type expression.
    ///
    /// Example:
    /// ```ds
    /// _
    /// ```
    fn walk_hole_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        constraint: Option<dir::LocalNodeId<dir::TypeExpression>>,
    ) -> CompilerResult<()> {
        if let Some(constraint) = constraint {
            self.walk_type_expression(constraint, self.tree.get(constraint))?;
        }
        let origin = Origin::Node(id.into_global_any(self.module));
        let variable = self.check.create_type_variable(self.module, origin);

        self.bind_node_type_operand(id, variable.into())?;

        Ok(())
    }

    /// Walk one infer type expression.
    ///
    /// Example:
    /// ```ds
    /// infer T extends U
    /// ```
    fn walk_infer_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        name: Option<dir::StringId>,
        constraint: Option<dir::LocalNodeId<dir::TypeExpression>>,
    ) -> CompilerResult<()> {
        if let Some(constraint) = constraint {
            self.walk_type_expression(constraint, self.tree.get(constraint))?;
        }

        let constraint_variable = constraint
            .map(|constraint| self.allocate_node_type_operand(constraint))
            .transpose()?;
        let operation = self.check.inference.push_term(TypeOperationTerm::Infer {
            name,
            constraint: constraint_variable,
        });
        let term = TypeTerm::Operation(operation);

        self.bind_node_type(id, term)?;

        Ok(())
    }

    /// Walk one type member.
    ///
    /// Example:
    /// ```ds
    /// method(value: T): U
    /// ```
    pub(in crate::check) fn walk_type_member(
        &mut self,
        id: dir::LocalNodeId<dir::TypeMember>,
        type_member: &dir::TypeMember,
    ) -> CompilerResult<()> {
        // enter static owner guard
        let static_receiver = self.type_member_static_guard_receiver(self.module, id)?;
        let Some(_guard) = self.enter_static_guard_for(id.into_any(), static_receiver)? else {
            return Ok(());
        };

        match type_member {
            // field: T
            dir::TypeMember::Field {
                key, declared_type, ..
            } => {
                if declared_type.is_none() {
                    self.check
                        .report_missing_type_annotation(self.module, id.into_any());
                }

                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(*declared_type, self.tree.get(*declared_type))?;
                }

                if let Some(declared_type) = declared_type {
                    if key.direct_static_key().is_some() {
                        if let Some(symbol) = self
                            .check
                            .module(self.module)
                            .declaration_symbol(id.into_any())
                        {
                            let declared_type = self.allocate_node_type_operand(*declared_type)?;
                            let condition = self.active_static_guard();

                            self.bind_symbol_type_operand(symbol, declared_type, condition)?;
                        }
                    }
                }

                // check computed type member key in declaration context
                if let dir::Key::Expression(key) = key {
                    let before_key = self.fork_flow();

                    self.walk_expression(*key, self.tree.get(*key))?;
                    self.restore_flow(before_key);
                }
            }
            // method(): T
            dir::TypeMember::Method {
                key,
                signature,
                body,
                is_static,
                ..
            } => {
                // check computed type member key in declaration context
                if let dir::Key::Expression(key) = key {
                    let before_key = self.fork_flow();

                    self.walk_expression(*key, self.tree.get(*key))?;
                    self.restore_flow(before_key);
                }

                let symbol = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any());

                // walk signature before reading its term inputs
                self.walk_function_signature(signature)?;
                let receiver = self.bind_type_member_receiver(id, signature, *is_static)?;
                let result = self.allocate_function_result_operand(
                    self.module,
                    id.into_any(),
                    signature,
                    *body,
                )?;

                // bind type member method symbol
                if let Some(symbol) = symbol {
                    let receiver_type = if *is_static {
                        None
                    } else {
                        receiver.map(|receiver| receiver.ty)
                    };
                    let term =
                        self.lower_function_signature_term(signature, receiver_type, result)?;
                    let condition = self.active_static_guard();

                    let operand = self
                        .check
                        .inference
                        .push_term(TypeTerm::Function(term))
                        .into();

                    if let Some(receiver) = receiver {
                        self.check.push_generic_induction_root(symbol, receiver.ty);
                    }
                    self.check.push_generic_induction_root(symbol, operand);
                    self.bind_symbol_type_operand(symbol, operand, condition)?;
                }

                // walk method body after its result operand exists
                if let Some(body) = body
                    && let Some(symbol) = symbol
                    && let Some(result) = result
                {
                    self.walk_function_body(symbol, signature, *body, result, receiver)?;
                }
            }
            // (...): T
            dir::TypeMember::CallSignature { signature } => {
                self.walk_function_type(signature)?;

                let return_type = signature
                    .return_type
                    .map(|return_type| self.allocate_node_type_operand(return_type))
                    .transpose()?;
                let term = self.lower_function_type_term(signature, return_type)?;
                self.bind_node_type(id, TypeTerm::Function(term))?;
            }
            // new (...): T
            dir::TypeMember::ConstructSignature { signature } => {
                self.walk_constructor_type(signature)?;

                let return_type = signature
                    .return_type
                    .map(|return_type| self.allocate_node_type_operand(return_type))
                    .transpose()?;
                let term = self.lower_constructor_type_term(signature, return_type)?;
                self.bind_node_type(id, TypeTerm::Function(term))?;
            }
            // [key: K]: V
            dir::TypeMember::IndexSignature {
                key_type,
                value_type,
                ..
            } => {
                self.walk_type_expression(*key_type, self.tree.get(*key_type))?;
                self.walk_type_expression(*value_type, self.tree.get(*value_type))?;
            }
            // type Item = T
            dir::TypeMember::AssociatedType {
                generic_parameters,
                where_clauses,
                constraint,
                value,
                ..
            } => {
                for generic_parameter in generic_parameters {
                    self.walk_generic_parameter(
                        *generic_parameter,
                        self.tree.get(*generic_parameter),
                    )?;
                }

                for where_clause in where_clauses {
                    self.walk_where_clause(*where_clause, self.tree.get(*where_clause))?;
                }

                if let Some(constraint) = constraint {
                    self.walk_type_expression(*constraint, self.tree.get(*constraint))?;
                }

                if let Some(value) = value {
                    self.walk_type_expression(*value, self.tree.get(*value))?;
                }

                if let (Some(symbol), Some(value)) = (
                    self.check
                        .module(self.module)
                        .declaration_symbol(id.into_any()),
                    value,
                ) {
                    let condition = self.active_static_guard();
                    let value = self.allocate_node_type_operand(*value)?;

                    self.bind_symbol_type_operand(symbol, value, condition.clone())?;

                    if let Some(constraint) = constraint {
                        let constraint = self.allocate_node_type_operand(*constraint)?;
                        let origin = Origin::Symbol(symbol);

                        self.check.relate_type(
                            origin,
                            TypeRelation::Extends,
                            value,
                            constraint,
                            condition,
                        );
                    }
                }
            }
            // const item: T = value
            dir::TypeMember::AssociatedConst {
                declared_type,
                value,
                ..
            } => {
                if value.is_some() && declared_type.is_none() {
                    self.check
                        .report_missing_type_annotation(self.module, id.into_any());
                }

                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(*declared_type, self.tree.get(*declared_type))?;
                }

                if let Some(value) = value {
                    self.walk_static_expression(*value)?;
                }

                if let Some(symbol) = self
                    .check
                    .module(self.module)
                    .declaration_symbol(id.into_any())
                {
                    // associated const type lives in type space
                    if let Some(declared_type) = declared_type {
                        let declared_type = self.allocate_node_type_operand(*declared_type)?;
                        let condition = self.active_static_guard();

                        self.bind_symbol_type_operand(symbol, declared_type, condition)?;
                    }

                    // associated const value lives in static space
                    if let Some(value) = value {
                        let variable = self.allocate_symbol_static_variable(symbol)?;
                        let condition = self.active_static_guard();
                        let value =
                            self.allocate_static_expression_variable(*value, condition.clone())?;
                        let origin = self.check.variable(variable).source;

                        self.check.equate_static(origin, variable, value, condition);
                    }
                }
            }
            // ignore damaged syntax
            dir::TypeMember::Error => {}
        }

        Ok(())
    }

    /// Return the receiver visible to decorators on one type member.
    fn type_member_static_guard_receiver(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::TypeMember>,
    ) -> CompilerResult<Option<ReceiverCapture>> {
        let Some(symbol) = self
            .check
            .module(module)
            .implicit_receiver_symbol(id.into_any())
        else {
            return Ok(None);
        };
        let Some((owner, ty)) = self.type_member_receiver_context(module, id)? else {
            return Ok(None);
        };

        Ok(Some(ReceiverCapture { symbol, owner, ty }))
    }

    /// Walk one mapped type parameter.
    ///
    /// Example:
    /// ```ds
    /// K in keyof T as Remap<K>
    /// ```
    pub(in crate::check) fn walk_type_mapped_parameter(
        &mut self,
        id: dir::LocalNodeId<dir::TypeMappedParameter>,
        type_mapped_parameter: &dir::TypeMappedParameter,
    ) -> CompilerResult<()> {
        // enter static owner guard
        let Some(_guard) = self.enter_static_guard_for(id.into_any(), None)? else {
            return Ok(());
        };

        self.walk_type_expression(
            type_mapped_parameter.source_type,
            self.tree.get(type_mapped_parameter.source_type),
        )?;

        if let Some(key_remap) = type_mapped_parameter.key_remap {
            self.walk_type_expression(key_remap, self.tree.get(key_remap))?;
        }

        Ok(())
    }

    /// Walk one tuple element.
    ///
    /// Example:
    /// ```ds
    /// label?: T
    /// ```
    pub(in crate::check) fn walk_tuple_element(
        &mut self,
        id: dir::LocalNodeId<dir::TupleElement>,
        tuple_element: &dir::TupleElement,
    ) -> CompilerResult<()> {
        // enter static owner guard
        let Some(_guard) = self.enter_static_guard_for(id.into_any(), None)? else {
            return Ok(());
        };

        match tuple_element {
            // [T]
            dir::TupleElement::Element { value, .. } => {
                self.walk_type_expression(*value, self.tree.get(*value))?;
            }
            // [...T]
            dir::TupleElement::Spread { value, .. } => {
                self.walk_type_expression(*value, self.tree.get(*value))?;
            }
            // ignore damaged syntax
            dir::TupleElement::Error => {}
        }

        Ok(())
    }

    /// Walk one type-space function declaration.
    ///
    /// Example:
    /// ```ds
    /// (value: T) => U
    /// ```
    fn walk_function_type(
        &mut self,
        declaration: &dir::FunctionTypeExpression,
    ) -> CompilerResult<()> {
        for parameter in &declaration.generic_parameters {
            self.walk_generic_parameter(*parameter, self.tree.get(*parameter))?;
        }

        if let Some(parameter) = declaration.this_parameter {
            self.walk_parameter(parameter, self.tree.get(parameter), true)?;
        }

        for parameter in &declaration.parameters {
            self.walk_parameter(*parameter, self.tree.get(*parameter), true)?;
        }

        if let Some(return_type) = declaration.return_type {
            self.walk_type_expression(return_type, self.tree.get(return_type))?;
        }

        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(*where_clause, self.tree.get(*where_clause))?;
        }

        Ok(())
    }

    /// Walk one type-space constructor declaration.
    ///
    /// Example:
    /// ```ds
    /// new (value: T) => Box<T>
    /// ```
    fn walk_constructor_type(&mut self, declaration: &dir::ConstructorType) -> CompilerResult<()> {
        for parameter in &declaration.generic_parameters {
            self.walk_generic_parameter(*parameter, self.tree.get(*parameter))?;
        }

        for parameter in &declaration.parameters {
            self.walk_parameter(*parameter, self.tree.get(*parameter), true)?;
        }

        if let Some(return_type) = declaration.return_type {
            self.walk_type_expression(return_type, self.tree.get(return_type))?;
        }

        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(*where_clause, self.tree.get(*where_clause))?;
        }

        Ok(())
    }

    /// Bind the receiver visible inside one type member method.
    ///
    /// Example:
    /// ```ds
    /// method(this: Box): number
    /// ```
    fn bind_type_member_receiver(
        &mut self,
        id: dir::LocalNodeId<dir::TypeMember>,
        signature: &dir::FunctionSignature,
        is_static: bool,
    ) -> CompilerResult<Option<ReceiverCapture>> {
        let owner = self.type_member_receiver_owner(self.module, id);

        // bind explicit `this` parameters before implicit receivers
        if let Some(parameter) = signature.this_parameter {
            let receiver = self.bind_this_parameter_receiver(parameter, owner)?;

            return Ok(Some(receiver));
        }

        if is_static {
            return Ok(None);
        }
        let Some(symbol) = self
            .check
            .module(self.module)
            .implicit_receiver_symbol(id.into_any())
        else {
            return Ok(None);
        };
        let Some((owner, ty)) = self.type_member_receiver_context(self.module, id)? else {
            return Ok(None);
        };
        if self
            .check
            .module(self.module)
            .profile
            .flags
            .no_implicit_receivers
        {
            self.check
                .report_implicit_receiver(self.module, id.into_any());
        }

        Ok(Some(ReceiverCapture { symbol, owner, ty }))
    }

    /// Return the contextual type used for an implicit type member receiver.
    fn type_member_receiver_context(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::TypeMember>,
    ) -> CompilerResult<Option<(Option<dir::GlobalSymbolId>, TypeOperand)>> {
        if let Some(object) = self.enclosing_object_type_expression(module, id) {
            let ty = self.allocate_node_type_operand(object)?;

            return Ok(Some((None, ty)));
        }

        let Some(declaration) = self.enclosing_declaration(module, id.into_any()) else {
            return Ok(None);
        };
        let Some(owner) = self
            .check
            .module(module)
            .declaration_symbol(declaration.into_any())
        else {
            return Ok(None);
        };
        let ty = self.allocate_symbol_type_operand(owner)?;

        Ok(Some((Some(owner), ty)))
    }

    /// Return the nominal owner that provides one type member receiver.
    fn type_member_receiver_owner(
        &self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::TypeMember>,
    ) -> Option<dir::GlobalSymbolId> {
        if self.enclosing_object_type_expression(module, id).is_some() {
            return None;
        }

        let declaration = self.enclosing_declaration(module, id.into_any())?;

        self.check
            .module(module)
            .declaration_symbol(declaration.into_any())
    }

    /// Return the enclosing object type expression.
    fn enclosing_object_type_expression(
        &self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::TypeMember>,
    ) -> Option<dir::LocalNodeId<dir::TypeExpression>> {
        let view = self.check.module(module).view();
        let parent = view.get_parent_for(id)?;
        if parent.ty != dir::NodeType::TypeExpression {
            return None;
        }

        let expression = dir::LocalNodeId::<dir::TypeExpression>::new(parent.id);
        if matches!(view.get(expression), dir::TypeExpression::Object { .. }) {
            Some(expression)
        } else {
            None
        }
    }

    /// Return the nearest enclosing declaration.
    fn enclosing_declaration(
        &self,
        module: ModuleId,
        id: dir::LocalNodeIdAny,
    ) -> Option<dir::LocalNodeId<dir::Declaration>> {
        let view = self.check.module(module).view();
        let mut current = view.get_parent(id.id);

        while let Some(node) = current {
            if node.ty == dir::NodeType::Declaration {
                return Some(dir::LocalNodeId::<dir::Declaration>::new(node.id));
            }

            current = view.get_parent(node.id);
        }

        None
    }

    /// Lower one string mapping reference.
    ///
    /// Example:
    /// ```ds
    /// Uppercase<Name>
    /// ```
    fn lower_string_mapping_term(
        &mut self,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(mapping) = self.string_mapping_for_path(self.module, path) else {
            return Ok(None);
        };
        let arguments = self.walk_generic_arguments(generic_arguments)?;
        let Some(argument) = arguments
            .first()
            .and_then(|argument| argument.type_operand())
        else {
            return Ok(Some(TypeTerm::Literal(TypeLiteralTerm::Error)));
        };
        let operation = self
            .check
            .inference
            .push_term(TypeOperationTerm::StringMapping { mapping, argument });

        Ok(Some(TypeTerm::Operation(operation)))
    }

    /// Return the string mapping named by one path.
    fn string_mapping_for_path(
        &self,
        module: ModuleId,
        path: &dir::Path,
    ) -> Option<dir::StringMapping> {
        let [name] = path.segments.as_slice() else {
            return None;
        };
        let name = self.check.module(module).strings.get(*name);

        dir::StringMapping::try_from(name).ok()
    }

    /// Return one compact scalar range type term.
    ///
    /// Example:
    /// ```ds
    /// 0..10
    /// ```
    fn lower_range_type_term(
        &self,
        start: Option<dir::LocalNodeId<dir::TypeExpression>>,
        end: Option<dir::LocalNodeId<dir::TypeExpression>>,
        end_kind: dir::RangeEnd,
    ) -> Option<TypeTerm> {
        let start = match start {
            Some(start) => Some(self.lower_scalar_range_bound(start)?),
            None => None,
        };
        let end = match end {
            Some(end) => Some(self.lower_scalar_range_bound(end)?),
            None => None,
        };

        Some(TypeTerm::Range {
            start,
            end,
            is_inclusive: end_kind == dir::RangeEnd::Inclusive,
        })
    }

    /// Return one scalar literal range bound.
    ///
    /// Example:
    /// ```ds
    /// 10
    /// ```
    fn lower_scalar_range_bound(
        &self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> Option<dir::ScalarLiteral> {
        match self.tree.get(id) {
            // 0
            dir::TypeExpression::ScalarLiteral { value } => Some(value.clone()),
            // not a compact scalar range bound
            _ => None,
        }
    }

    /// Return one tuple type term.
    ///
    /// Example:
    /// ```ds
    /// [name: string, age?: number]
    /// ```
    fn lower_tuple_type_term(
        &mut self,
        elements: &[dir::LocalNodeId<dir::TupleElement>],
        form: dir::TupleForm,
    ) -> CompilerResult<TypeTerm> {
        let mut element_terms = Vec::with_capacity(elements.len());

        // collect tuple element operands
        for element in elements {
            if let Some(element) = self.lower_tuple_element_term(*element)? {
                element_terms.push(element);
            }
        }

        Ok(TypeTerm::Tuple {
            form,
            elements: element_terms,
        })
    }

    /// Return one tuple element term from tuple type syntax.
    ///
    /// Example:
    /// ```ds
    /// ...items
    /// ```
    fn lower_tuple_element_term(
        &mut self,
        id: dir::LocalNodeId<dir::TupleElement>,
    ) -> CompilerResult<Option<TupleElement>> {
        let element = match self.tree.get(id) {
            // [T]
            dir::TupleElement::Element {
                label,
                value,
                is_optional,
                is_readonly,
            } => TupleElement {
                label: *label,
                ty: self.allocate_node_type_operand(*value)?.into(),
                is_optional: *is_optional,
                is_readonly: *is_readonly,
                is_rest: false,
            },
            // [...T]
            dir::TupleElement::Spread { label, value } => TupleElement {
                label: *label,
                ty: self.allocate_node_type_operand(*value)?.into(),
                is_optional: false,
                is_readonly: false,
                is_rest: true,
            },
            // ignore damaged syntax
            dir::TupleElement::Error => return Ok(None),
        };

        Ok(Some(element))
    }

    /// Return shape member terms from object type syntax.
    ///
    /// Example:
    /// ```ds
    /// { name: string, read(): string }
    /// ```
    fn lower_shape_member_terms(
        &mut self,
        members: &[dir::LocalNodeId<dir::TypeMember>],
    ) -> CompilerResult<SmallVec<[ShapeMember; 2]>> {
        let mut member_terms = SmallVec::new();

        // collect object member operands
        for member in members {
            if let Some(member) = self.lower_shape_member_term(*member)? {
                member_terms.push(member);
            }
        }

        Ok(member_terms)
    }

    /// Return one shape member term from object type syntax.
    ///
    /// Example:
    /// ```ds
    /// name?: string
    /// ```
    fn lower_shape_member_term(
        &mut self,
        id: dir::LocalNodeId<dir::TypeMember>,
    ) -> CompilerResult<Option<ShapeMember>> {
        let member = match self.tree.get(id) {
            // field: T
            dir::TypeMember::Field {
                key,
                declared_type,
                is_static: false,
                is_optional,
                is_readonly,
            } => {
                let Some(key) = key.static_key(self.tree) else {
                    return Ok(None);
                };
                let ty = if let Some(declared_type) = declared_type {
                    self.allocate_node_type_operand(*declared_type)?
                } else {
                    self.allocate_node_type_operand(id)?
                };

                ShapeMember::Field {
                    key,
                    ty: ty.into(),
                    is_optional: *is_optional,
                    is_readonly: *is_readonly,
                }
            }
            // method(): T
            dir::TypeMember::Method {
                key,
                is_static: false,
                is_optional,
                ..
            } => {
                let Some(key) = key.static_key(self.tree) else {
                    return Ok(None);
                };
                let Some(symbol) = self.check.module(self.module).declaration_symbol(id.into_any()) else {
                    return Ok(None);
                };
                let ty = self.allocate_symbol_type_operand(symbol)?;

                ShapeMember::Field {
                    key,
                    ty: ty.into(),
                    is_optional: *is_optional,
                    is_readonly: false,
                }
            }
            // (...): T
            dir::TypeMember::CallSignature { .. } => ShapeMember::CallSignature {
                ty: self.allocate_node_type_operand(id)?.into(),
            },
            // new (...): T
            dir::TypeMember::ConstructSignature { .. } => ShapeMember::ConstructSignature {
                ty: self.allocate_node_type_operand(id)?.into(),
            },
            // [key: K]: V
            dir::TypeMember::IndexSignature {
                name,
                key_type,
                value_type,
                is_optional,
                is_readonly,
            } => {
                ShapeMember::IndexSignature {
                    name: *name,
                    key_type: self.allocate_node_type_operand(*key_type)?.into(),
                    value_type: self.allocate_node_type_operand(*value_type)?.into(),
                    is_optional: *is_optional,
                    is_readonly: *is_readonly,
                }
            }
            // static field: T
            dir::TypeMember::Field {
                is_static: true, ..
            }
            // static method(): T
            | dir::TypeMember::Method {
                is_static: true, ..
            }
            // type Item = T
            | dir::TypeMember::AssociatedType { .. }
            // const item: T = value
            | dir::TypeMember::AssociatedConst { .. }
            // ignore damaged syntax
            | dir::TypeMember::Error => return Ok(None),
        };

        Ok(Some(member))
    }

    /// Return one mapped type operation term.
    ///
    /// Example:
    /// ```ds
    /// { [K in keyof T]: T[K] }
    /// ```
    fn lower_mapped_type_term(
        &mut self,
        parameter: dir::LocalNodeId<dir::TypeMappedParameter>,
        readonly: dir::MappedTypeModifier,
        optional: dir::MappedTypeModifier,
        value: Option<dir::LocalNodeId<dir::TypeExpression>>,
    ) -> CompilerResult<Option<TypeTerm>> {
        let mapped_parameter = self.tree.get(parameter);
        let Some(symbol) = self
            .check
            .module(self.module)
            .declaration_symbol(parameter.into_any())
        else {
            return Ok(None);
        };
        let Some(value) = value else {
            return Ok(None);
        };
        let parameter = MappedParameter {
            name: mapped_parameter.name,
            symbol,
            constraint: self.allocate_node_type_operand(mapped_parameter.source_type)?,
            key_remap: mapped_parameter
                .key_remap
                .map(|key_remap| self.allocate_node_type_operand(key_remap))
                .transpose()?,
        };
        let modifiers = dir::MappedTypeModifiers { readonly, optional };

        let value = self.allocate_node_type_operand(value)?;
        let operation = self.check.inference.push_term(TypeOperationTerm::Mapped {
            parameter,
            modifiers,
            value,
        });

        Ok(Some(TypeTerm::Operation(operation)))
    }

    /// Bind one type expression to the singleton type of a static computation.
    fn bind_static_value_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        term: StaticTerm,
    ) -> CompilerResult<()> {
        let node = id.into_global_any(self.module);
        let origin = Origin::Node(node);
        let variable = self.check.create_static_variable(self.module, origin);
        let condition = self.active_static_guard();
        let term = self.check.inference.push_term(term);

        self.check.equate_static(origin, variable, term, condition);

        let term = TypeTerm::StaticValue {
            value: StaticOperand::Variable(variable),
        };

        self.bind_node_type(id, term)?;

        Ok(())
    }
}
