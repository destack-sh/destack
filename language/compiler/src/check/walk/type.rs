use destack_dir as dir;
use dir::NodeVisitor as _;

use crate::check::{
    CheckModuleState, ConstraintOrigin, FormTerm, MappedParameterTerm, MemberTerm, ReceiverCapture,
    ShapeMemberTerm, StaticTerm, TupleElementTerm, TypeLiteralTerm, TypeOperationTerm,
    TypeRelation, TypeTerm, VariableId, VariableKind,
};

impl CheckModuleState {
    /// Walk one type expression.
    pub(in crate::check) fn walk_type_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeExpression>,
        type_expression: &dir::TypeExpression,
    ) {
        // apply static owner guards
        if !self.static_allows(tree, id.into_any()) {
            return;
        }

        self.visit_any(tree, dir::NodeType::TypeExpression, id.id);

        match type_expression {
            // (T)
            dir::TypeExpression::Parenthesized { expression } => {
                let ty = self.intern_local_type_variable(*expression);

                self.define_type_expression_type(id, TypeTerm::Variable(ty));
                self.walk_type_expression(tree, *expression, tree.get(*expression));
            }
            // 1
            dir::TypeExpression::ScalarLiteral { value } => {
                let term = TypeTerm::Literal(TypeLiteralTerm::Scalar(value.clone()));

                self.define_type_expression_type(id, term);
            }
            // null
            dir::TypeExpression::Literal { value } => {
                let ty = dir::Type::from(value.clone());
                if let Some(literal) = TypeLiteralTerm::from_type(&ty) {
                    self.define_type_expression_type(id, TypeTerm::Literal(literal));
                }
            }
            // intrinsic
            dir::TypeExpression::Intrinsic => {
                self.define_type_expression_type(id, TypeTerm::Intrinsic);
            }
            // const
            dir::TypeExpression::Const => {
                self.define_type_expression_type(id, TypeTerm::ConstAssertion);
            }
            // this
            dir::TypeExpression::This => {
                self.define_type_expression_type(id, TypeTerm::This);
            }
            // (T, U)
            dir::TypeExpression::Tuple { elements } => {
                let term = self.build_tuple_type_term(elements, dir::TupleForm::Tuple, tree);

                self.define_type_expression_type(id, term);

                for element in elements {
                    self.walk_tuple_element(tree, *element, tree.get(*element));
                }
            }
            // [T, U]
            dir::TypeExpression::ArrayTuple { elements } => {
                let term = self.build_tuple_type_term(elements, dir::TupleForm::Array, tree);

                self.define_type_expression_type(id, term);

                for element in elements {
                    self.walk_tuple_element(tree, *element, tree.get(*element));
                }
            }
            // T[]
            dir::TypeExpression::Array { element } => {
                let term = TypeTerm::Array {
                    element: self.intern_local_type_variable(*element),
                };

                self.define_type_expression_type(id, term);
                self.walk_type_expression(tree, *element, tree.get(*element));
            }
            // [T]
            dir::TypeExpression::Slice { element } => {
                let term = TypeTerm::Slice {
                    element: self.intern_local_type_variable(*element),
                    is_readonly: false,
                };

                self.define_type_expression_type(id, term);
                self.walk_type_expression(tree, *element, tree.get(*element));
            }
            // [T; N]
            dir::TypeExpression::FixedArray { element, length } => {
                let term = TypeTerm::FixedArray {
                    element: self.intern_local_type_variable(*element),
                    length: self.define_static_expression_variable(*length),
                    is_readonly: false,
                };

                self.define_type_expression_type(id, term);
                self.walk_type_expression(tree, *element, tree.get(*element));

                // check fixed array length in type context
                let before_length = self.checkpoint_flow();

                self.walk_expression(tree, *length, tree.get(*length));
                self.restore_flow(before_length);
            }
            // { name: T }
            dir::TypeExpression::Object { members } => {
                let term = TypeTerm::Shape {
                    members: self.build_shape_member_terms(members, tree),
                };

                self.define_type_expression_type(id, term);

                for member in members {
                    self.walk_type_member(tree, *member, tree.get(*member));
                }
            }
            // struct User {}
            dir::TypeExpression::Declaration { declaration } => {
                if let Some(symbol) = self.declaration_symbol((*declaration).into_any()) {
                    let term = TypeTerm::Variable(self.intern_symbol_type_variable(symbol));

                    self.define_type_expression_type(id, term);
                }

                self.walk_declaration(tree, *declaration, tree.get(*declaration));
            }
            // (value: T) => U
            dir::TypeExpression::FunctionTypeDeclaration(declaration) => {
                let return_type = declaration
                    .return_type
                    .map(|return_type| self.intern_local_type_variable(return_type));
                let term = self.build_function_type_declaration_term(declaration, return_type, tree);

                self.define_type_expression_type(id, TypeTerm::Function(term));
                self.walk_function_type_declaration(tree, declaration);
            }
            // new (value: T) => U
            dir::TypeExpression::ConstructorTypeDeclaration(declaration) => {
                let return_type = declaration
                    .return_type
                    .map(|return_type| self.intern_local_type_variable(return_type));
                let term = self.build_constructor_type_declaration_term(declaration, return_type, tree);

                self.define_type_expression_type(id, TypeTerm::Function(term));
                self.walk_constructor_type_declaration(tree, declaration);
            }
            // T
            dir::TypeExpression::Reference {
                path,
                generic_arguments,
            } => {
                if let Some(term) =
                    self.resolve_reference_type_expression(id, path, generic_arguments, tree)
                {
                    self.define_type_expression_type(id, term);
                }

                for argument in generic_arguments {
                    self.walk_generic_argument(tree, *argument, tree.get(*argument));
                }
            }
            // T.Item
            dir::TypeExpression::Member {
                left,
                name,
                generic_arguments,
            } => {
                let arguments = self.build_generic_arguments(generic_arguments, tree);
                let term = TypeTerm::Member(MemberTerm {
                    source: Some(id.into_global_any(self.input.module_id)),
                    owner: self.intern_local_type_variable(*left),
                    key: dir::StaticKey::Name(*name),
                    arguments,
                });

                self.define_type_expression_type(id, term);
                self.walk_type_expression(tree, *left, tree.get(*left));

                for argument in generic_arguments {
                    self.walk_generic_argument(tree, *argument, tree.get(*argument));
                }
            }
            // 0..10
            dir::TypeExpression::Range {
                start,
                end,
                end_kind,
            } => {
                if let Some(term) = self.build_range_type_term(*start, *end, *end_kind, tree) {
                    self.define_type_expression_type(id, term);
                }

                if let Some(start) = start {
                    self.walk_type_expression(tree, *start, tree.get(*start));
                }
                if let Some(end) = end {
                    self.walk_type_expression(tree, *end, tree.get(*end));
                }
            }
            // readonly T
            dir::TypeExpression::Readonly { target_type } => {
                let term = TypeTerm::Form {
                    form: FormTerm::Readonly,
                    payload: self.intern_local_type_variable(*target_type),
                };

                self.define_type_expression_type(id, term);
                self.walk_type_expression(tree, *target_type, tree.get(*target_type));
            }
            // ^T
            dir::TypeExpression::OwnedOf { target_type, .. } => {
                let term = TypeTerm::Form {
                    form: FormTerm::Owned,
                    payload: self.intern_local_type_variable(*target_type),
                };

                self.define_type_expression_type(id, term);
                self.walk_type_expression(tree, *target_type, tree.get(*target_type));
            }
            // &T
            dir::TypeExpression::BorrowedOf {
                mutability,
                target_type,
                ..
            } => {
                let origin = ConstraintOrigin::Node(id.into_global_any(self.input.module_id));
                let access = mutability
                    .map(dir::Mutability::access)
                    .unwrap_or(dir::Access::Mutable);
                let access = self.define_static_literal(origin, dir::StaticTerm::Access { access });
                let lifetime = self.allocate_anonymous_variable(VariableKind::Static, origin);
                let term = TypeTerm::Form {
                    form: FormTerm::Borrowed { lifetime, access },
                    payload: self.intern_local_type_variable(*target_type),
                };

                self.define_type_expression_type(id, term);
                self.walk_type_expression(tree, *target_type, tree.get(*target_type));
            }
            // *T
            dir::TypeExpression::PointerOf { target_type, .. } => {
                let term = TypeTerm::Form {
                    form: FormTerm::Raw,
                    payload: self.intern_local_type_variable(*target_type),
                };

                self.define_type_expression_type(id, term);
                self.walk_type_expression(tree, *target_type, tree.get(*target_type));
            }
            // local T
            dir::TypeExpression::Local { target_type } => {
                let origin = ConstraintOrigin::Node(id.into_global_any(self.input.module_id));
                let place = dir::StaticTerm::Place {
                    place: dir::Place::Space(dir::Space::Local),
                };
                let place = self.define_static_literal(origin, place);
                let term = TypeTerm::Form {
                    form: FormTerm::Placed { place },
                    payload: self.intern_local_type_variable(*target_type),
                };

                self.define_type_expression_type(id, term);
                self.walk_type_expression(tree, *target_type, tree.get(*target_type));
            }
            // shared T
            dir::TypeExpression::Shared { target_type } => {
                let origin = ConstraintOrigin::Node(id.into_global_any(self.input.module_id));
                let place = dir::StaticTerm::Place {
                    place: dir::Place::Space(dir::Space::Shared),
                };
                let place = self.define_static_literal(origin, place);
                let term = TypeTerm::Form {
                    form: FormTerm::Placed { place },
                    payload: self.intern_local_type_variable(*target_type),
                };

                self.define_type_expression_type(id, term);
                self.walk_type_expression(tree, *target_type, tree.get(*target_type));
            }
            // T!
            dir::TypeExpression::Must { target_type }
            // !T
            | dir::TypeExpression::Not { target_type } => {
                self.define_materialized_type_expression(id);
                self.walk_type_expression(tree, *target_type, tree.get(*target_type));
            }
            // keyof T
            dir::TypeExpression::KeyOf { target_type } => {
                let term = TypeTerm::Operation(TypeOperationTerm::KeyOf {
                    target: self.intern_local_type_variable(*target_type),
                });

                self.define_type_expression_type(id, term);
                self.walk_type_expression(tree, *target_type, tree.get(*target_type));
            }
            // typeof value
            dir::TypeExpression::TypeOfValue { value } => {
                let ty = self.intern_local_type_variable(*value);

                self.define_type_expression_type(id, TypeTerm::Variable(ty));

                // check type query operand in type context
                let before_value = self.checkpoint_flow();

                self.walk_expression(tree, *value, tree.get(*value));
                self.restore_flow(before_value);
            }
            // T | U
            dir::TypeExpression::Union { elements } => {
                let term = TypeTerm::Union {
                    elements: elements
                        .iter()
                        .map(|element| self.intern_local_type_variable(*element))
                        .collect(),
                };

                self.define_type_expression_type(id, term);

                for element in elements {
                    self.walk_type_expression(tree, *element, tree.get(*element));
                }
            }
            // T & U
            dir::TypeExpression::Intersection { elements } => {
                let term = TypeTerm::Intersection {
                    elements: elements
                        .iter()
                        .map(|element| self.intern_local_type_variable(*element))
                        .collect(),
                };

                self.define_type_expression_type(id, term);

                for element in elements {
                    self.walk_type_expression(tree, *element, tree.get(*element));
                }
            }
            // T extends U ? X : Y
            dir::TypeExpression::Conditional {
                left,
                extends_type,
                then_type,
                else_type,
            } => {
                let term = TypeTerm::Operation(TypeOperationTerm::Conditional {
                    left: self.intern_local_type_variable(*left),
                    right: self.intern_local_type_variable(*extends_type),
                    then_type: self.intern_local_type_variable(*then_type),
                    else_type: self.intern_local_type_variable(*else_type),
                });

                self.define_type_expression_type(id, term);
                self.walk_type_expression(tree, *left, tree.get(*left));
                self.walk_type_expression(tree, *extends_type, tree.get(*extends_type));
                self.walk_type_expression(tree, *then_type, tree.get(*then_type));
                self.walk_type_expression(tree, *else_type, tree.get(*else_type));
            }
            // T extends U
            dir::TypeExpression::Extends { left, right }
            // T implements U
            | dir::TypeExpression::Implements { left, right } => {
                self.define_materialized_type_expression(id);
                self.walk_type_expression(tree, *left, tree.get(*left));
                self.walk_type_expression(tree, *right, tree.get(*right));
            }
            // { [K in keyof T]: T[K] }
            dir::TypeExpression::Mapped {
                parameter,
                readonly,
                optional,
                value,
            } => {
                if let Some(term) =
                    self.build_mapped_type_term(*parameter, *readonly, *optional, *value, tree)
                {
                    self.define_type_expression_type(id, term);
                }

                self.walk_type_mapped_parameter(tree, *parameter, tree.get(*parameter));

                if let Some(value) = value {
                    self.walk_type_expression(tree, *value, tree.get(*value));
                }
            }
            // T[K]
            dir::TypeExpression::Index { left, index } => {
                let term = TypeTerm::Operation(TypeOperationTerm::Index {
                    left: self.intern_local_type_variable(*left),
                    index: self.intern_local_type_variable(*index),
                });

                self.define_type_expression_type(id, term);
                self.walk_type_expression(tree, *left, tree.get(*left));
                self.walk_type_expression(tree, *index, tree.get(*index));
            }
            // `get${Name}`
            dir::TypeExpression::TemplateLiteral { strings, spans } => {
                let term = TypeTerm::Operation(TypeOperationTerm::TemplateLiteral {
                    strings: strings.clone(),
                    spans: spans
                        .iter()
                        .map(|span| self.intern_local_type_variable(*span))
                        .collect(),
                });

                self.define_type_expression_type(id, term);

                for span in spans {
                    self.walk_type_expression(tree, *span, tree.get(*span));
                }
            }
            // infer T extends U
            dir::TypeExpression::Infer {
                name, constraint, ..
            } => {
                let term = TypeTerm::Operation(TypeOperationTerm::Infer {
                    name: *name,
                    constraint: constraint
                        .map(|constraint| self.intern_local_type_variable(constraint)),
                });

                self.define_type_expression_type(id, term);

                if let Some(constraint) = constraint {
                    self.walk_type_expression(tree, *constraint, tree.get(*constraint));
                }
            }
            // value is T
            dir::TypeExpression::Predicate {
                asserts,
                subject,
                target: Some(target),
            } => {
                if let Some(subject) = self.predicate_subject(id, *subject) {
                    let term = TypeTerm::Predicate {
                        asserts: *asserts,
                        subject,
                        target: Some(self.intern_local_type_variable(*target)),
                    };

                    self.define_type_expression_type(id, term);
                }

                self.walk_type_expression(tree, *target, tree.get(*target));
            }
            // asserts value
            dir::TypeExpression::Predicate {
                asserts,
                subject,
                target: None,
            } => {
                if let Some(subject) = self.predicate_subject(id, *subject) {
                    let term = TypeTerm::Predicate {
                        asserts: *asserts,
                        subject,
                        target: None,
                    };

                    self.define_type_expression_type(id, term);
                }
            }
            // ignore damaged syntax
            dir::TypeExpression::Missing => {}
            // ignore damaged syntax
            dir::TypeExpression::Error => {}
        }
    }

    /// Walk one type member.
    pub(in crate::check) fn walk_type_member(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeMember>,
        type_member: &dir::TypeMember,
    ) {
        // apply static owner guards
        if !self.static_allows(tree, id.into_any()) {
            return;
        }

        self.visit_any(tree, dir::NodeType::TypeMember, id.id);

        match type_member {
            // field: T
            dir::TypeMember::Field {
                key, declared_type, ..
            } => {
                if declared_type.is_none() {
                    self.report_missing_type_annotation(id.into_any());
                }

                if let Some(declared_type) = declared_type {
                    if key.direct_static_key().is_some() {
                        if let Some(symbol) = self.declaration_symbol(id.into_any()) {
                            let variable = self.intern_symbol_type_variable(symbol);
                            let declared_type = self.intern_local_type_variable(*declared_type);

                            self.define_type(variable, TypeTerm::Variable(declared_type));
                        }
                    }
                }

                // check computed type member key in declaration context
                if let dir::Key::Expression(key) = key {
                    let before_key = self.checkpoint_flow();

                    self.walk_expression(tree, *key, tree.get(*key));
                    self.restore_flow(before_key);
                }

                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(tree, *declared_type, tree.get(*declared_type));
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
                    let before_key = self.checkpoint_flow();

                    self.walk_expression(tree, *key, tree.get(*key));
                    self.restore_flow(before_key);
                }

                let receiver = self.type_member_receiver(tree, id, signature, *is_static);
                let symbol = self.declaration_symbol(id.into_any());
                if let Some(symbol) = symbol {
                    let variable = self.intern_symbol_type_variable(symbol);
                    let return_type =
                        self.intern_signature_return_type_variable(id.into_any(), signature, *body);
                    let mut term = self.build_function_signature_term(signature, return_type, tree);
                    if term.this_parameter.is_none() && !*is_static {
                        term.this_parameter = receiver.map(|receiver| receiver.ty);
                    }

                    self.define_type(variable, TypeTerm::Function(term));
                }

                self.walk_function_signature(tree, signature);

                let Some(body) = body else {
                    return;
                };
                let Some(symbol) = symbol else {
                    return;
                };
                let Some(return_type) = self.intern_signature_return_type_variable(
                    id.into_any(),
                    signature,
                    Some(*body),
                ) else {
                    return;
                };

                self.walk_function_body(tree, symbol, signature, *body, return_type, receiver);
            }
            // (...): T
            dir::TypeMember::CallSignature { signature } => {
                self.walk_function_type_declaration(tree, signature);

                let return_type = signature
                    .return_type
                    .map(|return_type| self.intern_local_type_variable(return_type));
                let term = self.build_function_type_declaration_term(signature, return_type, tree);
                let variable = self.intern_local_type_variable(id);

                self.define_type(variable, TypeTerm::Function(term));
            }
            // new (...): T
            dir::TypeMember::ConstructSignature { signature } => {
                self.walk_constructor_type_declaration(tree, signature);

                let return_type = signature
                    .return_type
                    .map(|return_type| self.intern_local_type_variable(return_type));
                let term =
                    self.build_constructor_type_declaration_term(signature, return_type, tree);
                let variable = self.intern_local_type_variable(id);

                self.define_type(variable, TypeTerm::Function(term));
            }
            // [key: K]: V
            dir::TypeMember::IndexSignature {
                key_type,
                value_type,
                ..
            } => {
                self.walk_type_expression(tree, *key_type, tree.get(*key_type));
                self.walk_type_expression(tree, *value_type, tree.get(*value_type));
            }
            // type Item = T
            dir::TypeMember::AssociatedType {
                generic_parameters,
                where_clauses,
                constraint,
                value,
                ..
            } => {
                if let Some(value) = value {
                    if let Some(symbol) = self.declaration_symbol(id.into_any()) {
                        let variable = self.intern_symbol_type_variable(symbol);
                        let value = self.intern_local_type_variable(*value);

                        self.define_type(variable, TypeTerm::Variable(value));
                    }
                }

                for generic_parameter in generic_parameters {
                    self.walk_generic_parameter(
                        tree,
                        *generic_parameter,
                        tree.get(*generic_parameter),
                    );
                }

                for where_clause in where_clauses {
                    self.walk_where_clause(tree, *where_clause, tree.get(*where_clause));
                }

                if let Some(constraint) = constraint {
                    self.walk_type_expression(tree, *constraint, tree.get(*constraint));
                }

                if let Some(value) = value {
                    self.walk_type_expression(tree, *value, tree.get(*value));
                }
            }
            // const item: T = value
            dir::TypeMember::AssociatedConst {
                declared_type,
                value,
                ..
            } => {
                if value.is_some() && declared_type.is_none() {
                    self.report_missing_type_annotation(id.into_any());
                }

                if let Some(symbol) = self.declaration_symbol(id.into_any()) {
                    // associated const type lives in type space
                    if let Some(declared_type) = declared_type {
                        let variable = self.intern_symbol_type_variable(symbol);
                        let declared_type = self.intern_local_type_variable(*declared_type);

                        self.define_type(variable, TypeTerm::Variable(declared_type));
                    }

                    // associated const value lives in static space
                    if let Some(value) = value {
                        let variable = self.intern_symbol_static_variable(symbol);
                        let value = self.define_static_expression_variable(*value);

                        self.define_static(variable, StaticTerm::Variable(value));
                    }
                }

                // defaults must fit the declared associated const type
                if let (Some(declared_type), Some(value)) = (declared_type, value) {
                    let origin =
                        ConstraintOrigin::Node((*value).into_global_any(self.input.module_id));
                    let value = self.intern_local_type_variable(*value);
                    let declared_type = self.intern_local_type_variable(*declared_type);

                    self.constrain_type(origin, TypeRelation::Assignable, value, declared_type);
                }

                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(tree, *declared_type, tree.get(*declared_type));
                }

                if let Some(value) = value {
                    // check associated const value in declaration context
                    let before_value = self.checkpoint_flow();

                    self.walk_expression(tree, *value, tree.get(*value));
                    self.restore_flow(before_value);
                }
            }
            // ignore damaged syntax
            dir::TypeMember::Error => {}
        }
    }

    /// Walk one mapped type parameter.
    pub(in crate::check) fn walk_type_mapped_parameter(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeMappedParameter>,
        type_mapped_parameter: &dir::TypeMappedParameter,
    ) {
        // apply static owner guards
        if !self.static_allows(tree, id.into_any()) {
            return;
        }

        self.visit_any(tree, dir::NodeType::TypeMappedParameter, id.id);

        self.walk_type_expression(
            tree,
            type_mapped_parameter.source_type,
            tree.get(type_mapped_parameter.source_type),
        );

        if let Some(key_remap) = type_mapped_parameter.key_remap {
            self.walk_type_expression(tree, key_remap, tree.get(key_remap));
        }
    }

    /// Walk one tuple element.
    pub(in crate::check) fn walk_tuple_element(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TupleElement>,
        tuple_element: &dir::TupleElement,
    ) {
        // apply static owner guards
        if !self.static_allows(tree, id.into_any()) {
            return;
        }

        self.visit_any(tree, dir::NodeType::TupleElement, id.id);

        match tuple_element {
            // [T]
            dir::TupleElement::Element { value, .. } => {
                self.walk_type_expression(tree, *value, tree.get(*value));
            }
            // [...T]
            dir::TupleElement::Spread { value, .. } => {
                self.walk_type_expression(tree, *value, tree.get(*value));
            }
            // ignore damaged syntax
            dir::TupleElement::Error => {}
        }
    }

    /// Walk one type-space function declaration.
    fn walk_function_type_declaration(
        &mut self,
        tree: &dir::Tree,
        declaration: &dir::FunctionTypeDeclaration,
    ) {
        for parameter in &declaration.generic_parameters {
            self.walk_generic_parameter(tree, *parameter, tree.get(*parameter));
        }

        if let Some(parameter) = declaration.this_parameter {
            self.walk_parameter(tree, parameter, tree.get(parameter), true);
        }

        for parameter in &declaration.parameters {
            self.walk_parameter(tree, *parameter, tree.get(*parameter), true);
        }

        if let Some(return_type) = declaration.return_type {
            self.walk_type_expression(tree, return_type, tree.get(return_type));
        }

        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(tree, *where_clause, tree.get(*where_clause));
        }
    }

    /// Walk one type-space constructor declaration.
    fn walk_constructor_type_declaration(
        &mut self,
        tree: &dir::Tree,
        declaration: &dir::ConstructorTypeDeclaration,
    ) {
        for parameter in &declaration.generic_parameters {
            self.walk_generic_parameter(tree, *parameter, tree.get(*parameter));
        }

        for parameter in &declaration.parameters {
            self.walk_parameter(tree, *parameter, tree.get(*parameter), true);
        }

        if let Some(return_type) = declaration.return_type {
            self.walk_type_expression(tree, return_type, tree.get(return_type));
        }

        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(tree, *where_clause, tree.get(*where_clause));
        }
    }

    /// Return the receiver visible inside one type-surface method.
    fn type_member_receiver(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeMember>,
        signature: &dir::FunctionSignature,
        is_static: bool,
    ) -> Option<ReceiverCapture> {
        let owner = self.type_member_receiver_owner(id);
        if let Some(receiver) = self.function_receiver(signature.this_parameter, owner, tree) {
            return Some(receiver);
        }
        if is_static {
            return None;
        }
        let Some(symbol) = self.implicit_receiver_symbol(id.into_any()) else {
            return None;
        };
        let Some((owner, ty)) = self.type_member_receiver_type(id) else {
            return None;
        };

        Some(ReceiverCapture { symbol, owner, ty })
    }

    /// Return the contextual type used for an implicit type member receiver.
    fn type_member_receiver_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeMember>,
    ) -> Option<(Option<dir::GlobalSymbolId>, VariableId)> {
        if let Some(object) = self.enclosing_object_type_expression(id) {
            let ty = self.intern_local_type_variable(object);

            return Some((None, ty));
        }

        let declaration = self.enclosing_declaration(id.into_any())?;
        let Some(owner) = self.declaration_symbol(declaration.into_any()) else {
            return None;
        };
        let ty = self.intern_symbol_type_variable(owner);

        Some((Some(owner), ty))
    }

    /// Return the nominal owner that provides one type member receiver.
    fn type_member_receiver_owner(
        &self,
        id: dir::LocalNodeId<dir::TypeMember>,
    ) -> Option<dir::GlobalSymbolId> {
        if self.enclosing_object_type_expression(id).is_some() {
            return None;
        }

        let declaration = self.enclosing_declaration(id.into_any())?;

        self.declaration_symbol(declaration.into_any())
    }

    /// Return the enclosing object type expression.
    fn enclosing_object_type_expression(
        &self,
        id: dir::LocalNodeId<dir::TypeMember>,
    ) -> Option<dir::LocalNodeId<dir::TypeExpression>> {
        let view = self.input.view();
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
        id: dir::LocalNodeIdAny,
    ) -> Option<dir::LocalNodeId<dir::Declaration>> {
        let view = self.input.view();
        let mut current = view.get_parent(id.id);

        while let Some(node) = current {
            if node.ty == dir::NodeType::Declaration {
                return Some(dir::LocalNodeId::<dir::Declaration>::new(node.id));
            }

            current = view.get_parent(node.id);
        }

        None
    }

    /// Define one type expression output type.
    fn define_type_expression_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        term: TypeTerm,
    ) {
        let variable = self.intern_local_type_variable(id);

        self.define_type(variable, term);
    }

    /// Define one type expression from the imported or prechecked type table.
    fn define_materialized_type_expression(&mut self, id: dir::LocalNodeId<dir::TypeExpression>) {
        let source = id.into_global_any(self.input.module_id);
        let Some(type_id) = self.input.type_table().get_node_type_id(source) else {
            return;
        };
        let materialized = self.materialize_type_id(type_id.into_global(self.input.module_id));

        self.define_type_expression_type(id, TypeTerm::Variable(materialized));
    }

    /// Return one compact scalar range type term.
    fn build_range_type_term(
        &self,
        start: Option<dir::LocalNodeId<dir::TypeExpression>>,
        end: Option<dir::LocalNodeId<dir::TypeExpression>>,
        end_kind: dir::RangeEnd,
        tree: &dir::Tree,
    ) -> Option<TypeTerm> {
        let start = match start {
            Some(start) => Some(self.scalar_range_bound(start, tree)?),
            None => None,
        };
        let end = match end {
            Some(end) => Some(self.scalar_range_bound(end, tree)?),
            None => None,
        };

        Some(TypeTerm::Range {
            start,
            end,
            is_inclusive: end_kind == dir::RangeEnd::Inclusive,
        })
    }

    /// Return one scalar literal range bound.
    fn scalar_range_bound(
        &self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        tree: &dir::Tree,
    ) -> Option<dir::ScalarLiteral> {
        match tree.get(id) {
            // 0
            dir::TypeExpression::ScalarLiteral { value } => Some(value.clone()),
            // not a compact scalar range bound
            _ => None,
        }
    }

    /// Return one semantic predicate subject.
    fn predicate_subject(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        subject: dir::TypePredicateSubject,
    ) -> Option<dir::PredicateSubject> {
        match subject {
            // value is T
            dir::TypePredicateSubject::Identifier(name) => {
                let symbol = self.require_name(id.into_any(), name, dir::SymbolSpace::Value)?;
                let source = id.into_global_any(self.input.module_id);

                self.capture_symbol_reference(symbol);
                self.record_name_resolution(source, symbol);

                Some(dir::PredicateSubject::Symbol(symbol))
            }
            // this is T
            dir::TypePredicateSubject::This => Some(dir::PredicateSubject::This),
        }
    }

    /// Return one tuple type term.
    fn build_tuple_type_term(
        &mut self,
        elements: &[dir::LocalNodeId<dir::TupleElement>],
        form: dir::TupleForm,
        tree: &dir::Tree,
    ) -> TypeTerm {
        let elements = elements
            .iter()
            .filter_map(|element| self.build_tuple_element_term(*element, tree))
            .collect();

        TypeTerm::Tuple {
            form,
            elements,
            is_readonly: false,
        }
    }

    /// Return one tuple element term from tuple type syntax.
    fn build_tuple_element_term(
        &mut self,
        id: dir::LocalNodeId<dir::TupleElement>,
        tree: &dir::Tree,
    ) -> Option<TupleElementTerm> {
        let element = match tree.get(id) {
            // [T]
            dir::TupleElement::Element {
                label,
                value,
                is_optional,
                is_readonly,
            } => TupleElementTerm {
                label: *label,
                ty: self.intern_local_type_variable(*value),
                is_optional: *is_optional,
                is_readonly: *is_readonly,
                is_rest: false,
            },
            // [...T]
            dir::TupleElement::Spread { label, value } => TupleElementTerm {
                label: *label,
                ty: self.intern_local_type_variable(*value),
                is_optional: false,
                is_readonly: false,
                is_rest: true,
            },
            // ignore damaged syntax
            dir::TupleElement::Error => return None,
        };

        Some(element)
    }

    /// Return shape member terms from object type syntax.
    fn build_shape_member_terms(
        &mut self,
        members: &[dir::LocalNodeId<dir::TypeMember>],
        tree: &dir::Tree,
    ) -> Vec<ShapeMemberTerm> {
        members
            .iter()
            .filter_map(|member| self.build_shape_member_term(*member, tree))
            .collect()
    }

    /// Return one shape member term from object type syntax.
    fn build_shape_member_term(
        &mut self,
        id: dir::LocalNodeId<dir::TypeMember>,
        tree: &dir::Tree,
    ) -> Option<ShapeMemberTerm> {
        let member = match tree.get(id) {
            // field: T
            dir::TypeMember::Field {
                key,
                declared_type,
                is_static: false,
                is_optional,
                is_readonly,
            } => {
                let key = key.static_key(tree)?;
                let ty = if let Some(declared_type) = declared_type {
                    self.intern_local_type_variable(*declared_type)
                } else {
                    self.intern_local_type_variable(id)
                };

                ShapeMemberTerm::Field {
                    key,
                    ty,
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
                let key = key.static_key(tree)?;
                let Some(symbol) = self.declaration_symbol(id.into_any()) else {
                    return None;
                };
                let ty = self.intern_symbol_type_variable(symbol);

                ShapeMemberTerm::Field {
                    key,
                    ty,
                    is_optional: *is_optional,
                    is_readonly: false,
                }
            }
            // (...): T
            dir::TypeMember::CallSignature { .. } => ShapeMemberTerm::CallSignature {
                ty: self.intern_local_type_variable(id),
            },
            // new (...): T
            dir::TypeMember::ConstructSignature { .. } => ShapeMemberTerm::ConstructSignature {
                ty: self.intern_local_type_variable(id),
            },
            // [key: K]: V
            dir::TypeMember::IndexSignature {
                name,
                key_type,
                value_type,
                is_optional,
                is_readonly,
            } => ShapeMemberTerm::IndexSignature {
                name: *name,
                key_type: self.intern_local_type_variable(*key_type),
                value_type: self.intern_local_type_variable(*value_type),
                is_optional: *is_optional,
                is_readonly: *is_readonly,
            },
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
            | dir::TypeMember::Error => return None,
        };

        Some(member)
    }

    /// Return one mapped type operation term.
    fn build_mapped_type_term(
        &mut self,
        parameter: dir::LocalNodeId<dir::TypeMappedParameter>,
        readonly: dir::MappedTypeModifier,
        optional: dir::MappedTypeModifier,
        value: Option<dir::LocalNodeId<dir::TypeExpression>>,
        tree: &dir::Tree,
    ) -> Option<TypeTerm> {
        let mapped_parameter = tree.get(parameter);
        let symbol = self.declaration_symbol(parameter.into_any())?;
        let value = value?;
        let parameter = MappedParameterTerm {
            name: mapped_parameter.name,
            symbol,
            constraint: self.intern_local_type_variable(mapped_parameter.source_type),
            key_remap: mapped_parameter
                .key_remap
                .map(|key_remap| self.intern_local_type_variable(key_remap)),
        };
        let modifiers = dir::MappedTypeModifiers { readonly, optional };

        Some(TypeTerm::Operation(TypeOperationTerm::Mapped {
            parameter,
            modifiers,
            value: self.intern_local_type_variable(value),
        }))
    }
}
