use crate::{
    Argument, Block, Declaration, Declarator, Decorator, DependencyItem, EnumField, Expression,
    ForEachBinding, FunctionSignature, GenericArgument, GenericParameter, IfCondition,
    ImportAliasTarget, ImportTarget, Key, LocalNodeId, MatchCase, MatchSelector, Member, NodeTree,
    NodeType, NodeVisitor, Parameter, Pattern, PatternField, Property, TemplateLiteral,
    TupleElement, TypeExpression, TypeMember, WhereClause,
};

/// Walk any node.
pub fn walk_any<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    node_type: NodeType,
    node_id: u32,
) {
    let local_idx = tree.local_id_by_node_id[node_id as usize];
    match node_type {
        NodeType::Expression => {
            let expression = tree.expressions.get(local_idx);
            walk_expression(visitor, tree, LocalNodeId::new(node_id), expression);
        }
        NodeType::Block => {
            let block = tree.blocks.get(local_idx);
            walk_block(visitor, tree, LocalNodeId::new(node_id), block);
        }
        NodeType::Declaration => {
            let declaration = tree.declarations.get(local_idx);
            walk_declaration(visitor, tree, LocalNodeId::new(node_id), declaration);
        }
        NodeType::Declarator => {
            let declarator = tree.declarators.get(local_idx);
            walk_declarator(visitor, tree, LocalNodeId::new(node_id), declarator);
        }
        NodeType::Property => {
            let property = tree.properties.get(local_idx);
            walk_property(visitor, tree, LocalNodeId::new(node_id), property);
        }
        NodeType::TypeMember => {
            let type_member = tree.type_members.get(local_idx);
            walk_type_member(visitor, tree, LocalNodeId::new(node_id), type_member);
        }
        NodeType::Member => {
            let member = tree.members.get(local_idx);
            walk_member(visitor, tree, LocalNodeId::new(node_id), member);
        }
        NodeType::EnumField => {
            let enum_field = tree.enum_fields.get(local_idx);
            walk_enum_field(visitor, tree, LocalNodeId::new(node_id), enum_field);
        }
        NodeType::WhereClause => {
            let where_clause = tree.where_clauses.get(local_idx);
            walk_where_clause(visitor, tree, LocalNodeId::new(node_id), where_clause);
        }
        NodeType::DependencyItem => {
            let dependency_item = tree.dependency_items.get(local_idx);
            walk_dependency_item(visitor, tree, LocalNodeId::new(node_id), dependency_item);
        }
        NodeType::GenericParameter => {
            let generic_parameter = tree.generic_parameters.get(local_idx);
            walk_generic_parameter(visitor, tree, LocalNodeId::new(node_id), generic_parameter);
        }
        NodeType::Parameter => {
            let parameter = tree.parameters.get(local_idx);
            walk_parameter(visitor, tree, LocalNodeId::new(node_id), parameter);
        }
        NodeType::GenericArgument => {
            let generic_argument = tree.generic_arguments.get(local_idx);
            walk_generic_argument(visitor, tree, LocalNodeId::new(node_id), generic_argument);
        }
        NodeType::TupleElement => {
            let tuple_element = tree.tuple_elements.get(local_idx);
            walk_tuple_element(visitor, tree, LocalNodeId::new(node_id), tuple_element);
        }
        NodeType::Argument => {
            let argument = tree.arguments.get(local_idx);
            walk_argument(visitor, tree, LocalNodeId::new(node_id), argument);
        }
        NodeType::TypeExpression => {
            let type_expression = tree.type_expressions.get(local_idx);
            walk_type_expression(visitor, tree, LocalNodeId::new(node_id), type_expression);
        }
        NodeType::MatchCase => {
            let match_case = tree.match_cases.get(local_idx);
            walk_match_case(visitor, tree, LocalNodeId::new(node_id), match_case);
        }
        NodeType::Pattern => {
            let pattern = tree.patterns.get(local_idx);
            walk_pattern(visitor, tree, LocalNodeId::new(node_id), pattern);
        }
        NodeType::PatternField => {
            let pattern_field = tree.pattern_fields.get(local_idx);
            walk_pattern_field(visitor, tree, LocalNodeId::new(node_id), pattern_field);
        }
        NodeType::Decorator => {
            let decorator = tree.decorators.get(local_idx);
            walk_decorator(visitor, tree, LocalNodeId::new(node_id), decorator);
        }
    }
}

/// Walk the FunctionSignature.
fn walk_function_signature<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    signature: &FunctionSignature,
) {
    for generic_parameter_id in &signature.generic_parameters {
        let generic_parameter = tree.get(*generic_parameter_id);
        visitor.visit_generic_parameter(tree, *generic_parameter_id, generic_parameter);
    }
    for where_clause_id in &signature.where_clauses {
        let where_clause = tree.get(*where_clause_id);
        visitor.visit_where_clause(tree, *where_clause_id, where_clause);
    }
    if let Some(this_parameter) = signature.this_parameter {
        let parameter = tree.get(this_parameter);
        visitor.visit_parameter(tree, this_parameter, parameter);
    }
    for parameter_id in &signature.parameters {
        let parameter = tree.get(*parameter_id);
        visitor.visit_parameter(tree, *parameter_id, parameter);
    }
    if let Some(return_type) = signature.return_type {
        let return_type_node = tree.get(return_type);
        visitor.visit_type_expression(tree, return_type, return_type_node);
    }
}

/// Walk the GenericParameter.
pub fn walk_generic_parameter<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<GenericParameter>,
    generic_parameter: &GenericParameter,
) {
    visitor.visit_any(tree, NodeType::GenericParameter, id.id);

    match generic_parameter {
        GenericParameter::Type {
            constraint,
            default,
            ..
        } => {
            if let Some(constraint) = constraint {
                let constraint_node = tree.get(*constraint);
                visitor.visit_type_expression(tree, *constraint, constraint_node);
            }
            if let Some(default) = default {
                let default_node = tree.get(*default);
                visitor.visit_type_expression(tree, *default, default_node);
            }
        }
        GenericParameter::Value {
            declared_type,
            default,
            ..
        } => {
            if let Some(declared_type) = declared_type {
                let declared_type_node = tree.get(*declared_type);
                visitor.visit_type_expression(tree, *declared_type, declared_type_node);
            }
            if let Some(default) = default {
                let default_node = tree.get(*default);
                visitor.visit_expression(tree, *default, default_node);
            }
        }
        GenericParameter::Error { .. } => {}
    }
}

/// Walk the GenericArgument.
pub fn walk_generic_argument<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<GenericArgument>,
    generic_argument: &GenericArgument,
) {
    visitor.visit_any(tree, NodeType::GenericArgument, id.id);

    match generic_argument {
        GenericArgument::Type { value } => {
            let value_node = tree.get(*value);
            visitor.visit_type_expression(tree, *value, value_node);
        }
        GenericArgument::Value { value } => {
            let value_node = tree.get(*value);
            visitor.visit_expression(tree, *value, value_node);
        }
        GenericArgument::Error => {}
    }
}

/// Walk the TupleElement.
pub fn walk_tuple_element<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<TupleElement>,
    tuple_element: &TupleElement,
) {
    visitor.visit_any(tree, NodeType::TupleElement, id.id);

    match tuple_element {
        TupleElement::Element { value, .. } | TupleElement::Spread { value, .. } => {
            let value_node = tree.get(*value);
            visitor.visit_type_expression(tree, *value, value_node);
        }
        TupleElement::Error => {}
    }
}

/// Walk the TypeExpression.
pub fn walk_type_expression<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<TypeExpression>,
    type_expression: &TypeExpression,
) {
    visitor.visit_any(tree, NodeType::TypeExpression, id.id);

    match type_expression {
        TypeExpression::Parenthesized { expression } => {
            let expression_node = tree.get(*expression);
            visitor.visit_type_expression(tree, *expression, expression_node);
        }
        TypeExpression::ScalarLiteral { .. }
        | TypeExpression::Literal { .. }
        | TypeExpression::Intrinsic
        | TypeExpression::Const
        | TypeExpression::This
        | TypeExpression::Missing
        | TypeExpression::Error => {}
        TypeExpression::Tuple { elements } => {
            for element_id in elements {
                let element = tree.get(*element_id);
                visitor.visit_tuple_element(tree, *element_id, element);
            }
        }
        TypeExpression::Array { element } => {
            let element_node = tree.get(*element);
            visitor.visit_type_expression(tree, *element, element_node);
        }
        TypeExpression::Object { members } => {
            for member_id in members {
                let member = tree.get(*member_id);
                visitor.visit_type_member(tree, *member_id, member);
            }
        }
        TypeExpression::Declaration { declaration } => {
            let declaration_node = tree.get(*declaration);
            visitor.visit_declaration(tree, *declaration, declaration_node);
        }
        TypeExpression::Reference {
            path: _,
            generic_arguments,
            space_order: _,
        }
        | TypeExpression::LocalReference {
            path: _,
            generic_arguments,
            target_symbol: _,
        }
        | TypeExpression::ModuleReference {
            path: _,
            generic_arguments,
            target_symbol: _,
        }
        | TypeExpression::GlobalReference {
            path: _,
            generic_arguments,
            target_symbol: _,
        } => {
            for argument_id in generic_arguments {
                let argument = tree.get(*argument_id);
                visitor.visit_generic_argument(tree, *argument_id, argument);
            }
        }
        TypeExpression::Member {
            left,
            name: _,
            generic_arguments,
        } => {
            let left_node = tree.get(*left);
            visitor.visit_type_expression(tree, *left, left_node);

            for argument_id in generic_arguments {
                let argument = tree.get(*argument_id);
                visitor.visit_generic_argument(tree, *argument_id, argument);
            }
        }
        TypeExpression::Import {
            target,
            arguments,
            qualifier: _,
            generic_arguments,
        } => {
            let target_node = tree.get(*target);
            visitor.visit_expression(tree, *target, target_node);

            for argument_id in arguments {
                let argument = tree.get(*argument_id);
                visitor.visit_argument(tree, *argument_id, argument);
            }

            for argument_id in generic_arguments {
                let argument = tree.get(*argument_id);
                visitor.visit_generic_argument(tree, *argument_id, argument);
            }
        }
        TypeExpression::Readonly {
            target_type: right, ..
        }
        | TypeExpression::KeyOf {
            target_type: right, ..
        }
        | TypeExpression::Must {
            target_type: right, ..
        }
        | TypeExpression::AsComptime {
            target_type: right, ..
        }
        | TypeExpression::Not {
            target_type: right, ..
        }
        | TypeExpression::ValueOf {
            target_type: right, ..
        }
        | TypeExpression::ReferenceOf {
            target_type: right, ..
        }
        | TypeExpression::PointerOf {
            target_type: right, ..
        } => {
            let right_node = tree.get(*right);
            visitor.visit_type_expression(tree, *right, right_node);
        }
        TypeExpression::TypeOfValue { value } => {
            let value_node = tree.get(*value);
            visitor.visit_expression(tree, *value, value_node);
        }
        TypeExpression::Index { left, index: right } => {
            let left_node = tree.get(*left);
            visitor.visit_type_expression(tree, *left, left_node);

            let right_node = tree.get(*right);
            visitor.visit_type_expression(tree, *right, right_node);
        }
        TypeExpression::Union { elements } | TypeExpression::Intersection { elements } => {
            for element_id in elements {
                let element_node = tree.get(*element_id);
                visitor.visit_type_expression(tree, *element_id, element_node);
            }
        }
        TypeExpression::Conditional {
            left,
            extends_type,
            then_type,
            else_type,
        } => {
            let left_node = tree.get(*left);
            visitor.visit_type_expression(tree, *left, left_node);

            let extends_type_node = tree.get(*extends_type);
            visitor.visit_type_expression(tree, *extends_type, extends_type_node);

            let then_node = tree.get(*then_type);
            visitor.visit_type_expression(tree, *then_type, then_node);

            let else_node = tree.get(*else_type);
            visitor.visit_type_expression(tree, *else_type, else_node);
        }
        TypeExpression::Mapped {
            parameter,
            readonly: _,
            optional: _,
            value,
        } => {
            let source_type_node = tree.get(parameter.source_type);
            visitor.visit_type_expression(tree, parameter.source_type, source_type_node);

            if let Some(key_remap) = parameter.key_remap {
                let key_remap_node = tree.get(key_remap);
                visitor.visit_type_expression(tree, key_remap, key_remap_node);
            }

            let value_node = tree.get(*value);
            visitor.visit_type_expression(tree, *value, value_node);
        }
        TypeExpression::TemplateLiteral { strings: _, spans } => {
            for span_id in spans {
                let span_node = tree.get(*span_id);
                visitor.visit_type_expression(tree, *span_id, span_node);
            }
        }
        TypeExpression::Infer { constraint, .. } => {
            if let Some(constraint) = constraint {
                let constraint_node = tree.get(*constraint);
                visitor.visit_type_expression(tree, *constraint, constraint_node);
            }
        }
        TypeExpression::Predicate { target, .. } => {
            if let Some(target) = target {
                let target_node = tree.get(*target);
                visitor.visit_type_expression(tree, *target, target_node);
            }
        }
    }
}

/// Walk the TypeMember.
pub fn walk_type_member<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<TypeMember>,
    type_member: &TypeMember,
) {
    visitor.visit_any(tree, NodeType::TypeMember, id.id);

    match type_member {
        TypeMember::Field {
            is_optional: _,
            is_readonly: _,
            key,
            declared_type,
            symbol: _,
        } => {
            walk_key(visitor, tree, key);

            let declared_type_node = tree.get(*declared_type);
            visitor.visit_type_expression(tree, *declared_type, declared_type_node);
        }
        TypeMember::Method {
            is_optional: _,
            key,
            signature,
            body,
            symbol: _,
        } => {
            if let Some(key) = key {
                walk_key(visitor, tree, key);
            }

            walk_function_signature(visitor, tree, signature);

            if let Some(body) = body {
                let body_node = tree.get(*body);
                visitor.visit_expression(tree, *body, body_node);
            }
        }
        TypeMember::IndexSignature {
            is_optional: _,
            is_readonly: _,
            name: _,
            key_type,
            value_type,
            symbol: _,
        } => {
            let key_type_node = tree.get(*key_type);
            visitor.visit_type_expression(tree, *key_type, key_type_node);

            let value_type_node = tree.get(*value_type);
            visitor.visit_type_expression(tree, *value_type, value_type_node);
        }
        TypeMember::Embed { value, symbol: _ } => {
            let value_node = tree.get(*value);
            visitor.visit_type_expression(tree, *value, value_node);
        }
        TypeMember::AssociatedType {
            name: _,
            generic_parameters,
            where_clauses,
            constraint,
            value,
            symbol: _,
        } => {
            for generic_parameter_id in generic_parameters {
                let generic_parameter = tree.get(*generic_parameter_id);
                visitor.visit_generic_parameter(tree, *generic_parameter_id, generic_parameter);
            }

            for where_clause_id in where_clauses {
                let where_clause = tree.get(*where_clause_id);
                visitor.visit_where_clause(tree, *where_clause_id, where_clause);
            }

            if let Some(constraint) = constraint {
                let constraint_node = tree.get(*constraint);
                visitor.visit_type_expression(tree, *constraint, constraint_node);
            }

            if let Some(value) = value {
                let value_node = tree.get(*value);
                visitor.visit_type_expression(tree, *value, value_node);
            }
        }
        TypeMember::AssociatedConst {
            name: _,
            declared_type,
            value,
            symbol: _,
        } => {
            if let Some(declared_type) = declared_type {
                let declared_type_node = tree.get(*declared_type);
                visitor.visit_type_expression(tree, *declared_type, declared_type_node);
            }

            if let Some(value) = value {
                let value_node = tree.get(*value);
                visitor.visit_expression(tree, *value, value_node);
            }
        }
        TypeMember::Error { symbol: _ } => {}
    }
}

/// Walk the Expression.
pub fn walk_expression<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Expression>,
    expression: &Expression,
) {
    destack_core::ensure_sufficient_stack(|| {
        visitor.visit_any(tree, NodeType::Expression, id.id);
        match expression {
            Expression::Declaration(declaration_id) => {
                let declaration = tree.get(*declaration_id);
                visitor.visit_declaration(tree, *declaration_id, declaration);
            }
            Expression::Block(block_id) => {
                let block = tree.get(*block_id);
                visitor.visit_block(tree, *block_id, block);
            }
            Expression::Labelled {
                label: _,
                body,
                symbol: _,
            } => {
                let body_expr = tree.get(*body);
                visitor.visit_expression(tree, *body, body_expr);
            }
            Expression::UnresolvedImport {
                source: _,
                kind: _,
                target,
                items,
                attributes: _,
                arguments,
            } => {
                if let ImportTarget::Expression { target } = target {
                    let target_expression = tree.get(*target);
                    visitor.visit_expression(tree, *target, target_expression);
                }
                if let Some(items) = items {
                    for item_id in items {
                        let item = tree.get(*item_id);
                        visitor.visit_dependency_item(tree, *item_id, item);
                    }
                }
                if let Some(arguments) = arguments {
                    for argument_id in arguments {
                        let argument = tree.get(*argument_id);
                        visitor.visit_argument(tree, *argument_id, argument);
                    }
                }
            }
            Expression::Import {
                source: _,
                kind: _,
                target: _,
                target_module: _,
                items,
                attributes: _,
                arguments,
            } => {
                if let Some(items) = items {
                    for item_id in items {
                        let item = tree.get(*item_id);
                        visitor.visit_dependency_item(tree, *item_id, item);
                    }
                }
                if let Some(arguments) = arguments {
                    for argument_id in arguments {
                        let argument = tree.get(*argument_id);
                        visitor.visit_argument(tree, *argument_id, argument);
                    }
                }
            }
            Expression::UnresolvedReExport {
                target: _,
                kind: _,
                items,
                attributes: _,
            } => {
                for item_id in items {
                    let item = tree.get(*item_id);
                    visitor.visit_dependency_item(tree, *item_id, item);
                }
            }
            Expression::ReExport {
                target: _,
                target_module: _,
                kind: _,
                items,
                attributes: _,
            } => {
                for item_id in items {
                    let item = tree.get(*item_id);
                    visitor.visit_dependency_item(tree, *item_id, item);
                }
            }
            Expression::Export {
                kind: _,
                items,
                attributes: _,
            } => {
                for item_id in items {
                    let item = tree.get(*item_id);
                    visitor.visit_dependency_item(tree, *item_id, item);
                }
            }
            Expression::ExportNamespace { name: _ } => {}
            Expression::Let {
                export: _,
                ambient: _,
                mutability: _,
                declarators,
            } => {
                for declarator_id in declarators {
                    let declarator = tree.get(*declarator_id);
                    visitor.visit_declarator(tree, *declarator_id, declarator);
                }
            }
            Expression::Using {
                asynchrony: _,
                export: _,
                ambient: _,
                declarators,
            } => {
                for declarator_id in declarators {
                    let declarator = tree.get(*declarator_id);
                    visitor.visit_declarator(tree, *declarator_id, declarator);
                }
            }
            Expression::As {
                operator: _,
                source: _,
                expression,
                target_type,
            } => {
                let expression_node = tree.get(*expression);
                visitor.visit_expression(tree, *expression, expression_node);
                let target_expression = tree.get(*target_type);
                visitor.visit_type_expression(tree, *target_type, target_expression);
            }
            Expression::Satisfies {
                expression,
                target_type,
            } => {
                let expression_node = tree.get(*expression);
                visitor.visit_expression(tree, *expression, expression_node);
                let target_expression = tree.get(*target_type);
                visitor.visit_type_expression(tree, *target_type, target_expression);
            }
            Expression::Is { value, target_type } => {
                let value_expression = tree.get(*value);
                visitor.visit_expression(tree, *value, value_expression);

                let target_type_expression = tree.get(*target_type);
                visitor.visit_type_expression(tree, *target_type, target_type_expression);
            }
            Expression::InstanceOf { value, target } => {
                let value_expression = tree.get(*value);
                visitor.visit_expression(tree, *value, value_expression);

                let target_expression = tree.get(*target);
                visitor.visit_expression(tree, *target, target_expression);
            }
            Expression::Unary { operator: _, right }
            | Expression::ValueOf {
                mutability: _,
                variance: _,
                right,
            }
            | Expression::ReferenceOf {
                mutability: _,
                variance: _,
                right,
            }
            | Expression::PointerOf {
                mutability: _,
                right,
            } => {
                let right_expression = tree.get(*right);
                visitor.visit_expression(tree, *right, right_expression);
            }
            Expression::Binary {
                left,
                operator: _,
                right,
            }
            | Expression::Assign { left, right }
            | Expression::AssignBinary {
                left,
                operator: _,
                right,
            } => {
                let left_expression = tree.get(*left);
                visitor.visit_expression(tree, *left, left_expression);
                let right_expression = tree.get(*right);
                visitor.visit_expression(tree, *right, right_expression);
            }
            Expression::Member { left, name: _ } => {
                let left_expression = tree.get(*left);
                visitor.visit_expression(tree, *left, left_expression);
            }
            Expression::PrivateMember { left, name: _ } => {
                let left_expression = tree.get(*left);
                visitor.visit_expression(tree, *left, left_expression);
            }
            Expression::Instantiation {
                left,
                generic_arguments,
            } => {
                let left_expression = tree.get(*left);
                visitor.visit_expression(tree, *left, left_expression);
                for argument_id in generic_arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_generic_argument(tree, *argument_id, argument);
                }
            }
            Expression::Call {
                left,
                generic_arguments,
                arguments,
            } => {
                let left_expression = tree.get(*left);
                visitor.visit_expression(tree, *left, left_expression);
                for argument_id in generic_arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_generic_argument(tree, *argument_id, argument);
                }
                for argument_id in arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_argument(tree, *argument_id, argument);
                }
            }
            Expression::New {
                left,
                generic_arguments,
                arguments,
            } => {
                let left_expression = tree.get(*left);
                visitor.visit_expression(tree, *left, left_expression);

                for argument_id in generic_arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_generic_argument(tree, *argument_id, argument);
                }
                for argument_id in arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_argument(tree, *argument_id, argument);
                }
            }
            Expression::Delete { value } => {
                let value_expression = tree.get(*value);
                visitor.visit_expression(tree, *value, value_expression);
            }
            Expression::Index { left, right } => {
                let left_expression = tree.get(*left);
                visitor.visit_expression(tree, *left, left_expression);
                if let Some(right_id) = right {
                    let right_expression = tree.get(*right_id);
                    visitor.visit_expression(tree, *right_id, right_expression);
                }
            }
            Expression::Maybe { left } => {
                let left_expression = tree.get(*left);
                visitor.visit_expression(tree, *left, left_expression);
            }
            Expression::Must { left } => {
                let left_expression = tree.get(*left);
                visitor.visit_expression(tree, *left, left_expression);
            }
            Expression::UnresolvedPath {
                path: _,
                generic_arguments,
                space_order: _,
            } => {
                for argument_id in generic_arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_generic_argument(tree, *argument_id, argument);
                }
            }
            Expression::LocalReference {
                path: _,
                generic_arguments,
                target_symbol: _,
            }
            | Expression::ModuleReference {
                path: _,
                generic_arguments,
                target_symbol: _,
            }
            | Expression::GlobalReference {
                path: _,
                generic_arguments,
                target_symbol: _,
            } => {
                for argument_id in generic_arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_generic_argument(tree, *argument_id, argument);
                }
            }
            Expression::PrivateIdentifier { name: _ }
            | Expression::ImportMeta
            | Expression::NewTarget
            | Expression::This
            | Expression::Super => {
                // nothing to do
            }

            Expression::Type {
                value,
                resolved_type: _,
            } => {
                let value_expression = tree.get(*value);
                visitor.visit_type_expression(tree, *value, value_expression);
            }
            Expression::ScalarLiteral { value: _ } => {
                // nothing to do
            }
            Expression::TemplateExpression { value } => {
                match value {
                    TemplateLiteral::String { .. } => {
                        // nothing to do
                    }
                    TemplateLiteral::InterpolatedString { arguments, .. } => {
                        for argument_id in arguments {
                            let argument = tree.get(*argument_id);
                            visitor.visit_argument(tree, *argument_id, argument);
                        }
                    }
                }
            }
            Expression::TaggedTemplateExpression {
                tag,
                generic_arguments,
                value,
            } => {
                let tag_expression = tree.get(*tag);
                visitor.visit_expression(tree, *tag, tag_expression);

                for argument_id in generic_arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_generic_argument(tree, *argument_id, argument);
                }

                match value {
                    TemplateLiteral::String { .. } => {
                        // nothing to do
                    }
                    TemplateLiteral::InterpolatedString { arguments, .. } => {
                        for argument_id in arguments {
                            let argument = tree.get(*argument_id);
                            visitor.visit_argument(tree, *argument_id, argument);
                        }
                    }
                }
            }
            Expression::TypeLiteral { value: _ } => {
                // nothing to do
            }
            Expression::ArrayExpression { elements } => {
                for element_id in elements {
                    let element = tree.get(*element_id);
                    visitor.visit_argument(tree, *element_id, element);
                }
            }
            Expression::TupleExpression { elements } => {
                for argument_id in elements {
                    let argument = tree.get(*argument_id);
                    visitor.visit_argument(tree, *argument_id, argument);
                }
            }
            Expression::SequenceExpression { expressions } => {
                for expr_id in expressions {
                    let expr = tree.get(*expr_id);
                    visitor.visit_expression(tree, *expr_id, expr);
                }
            }
            Expression::ObjectExpression { ty, properties } => {
                if let Some(ty) = ty {
                    let ty_node = tree.get(*ty);
                    visitor.visit_type_expression(tree, *ty, ty_node);
                }
                for property_id in properties {
                    let property = tree.get(*property_id);
                    visitor.visit_property(tree, *property_id, property);
                }
            }
            Expression::TreeExpression {
                left,
                generic_arguments,
                arguments,
                elements,
            } => {
                if let Some(left_id) = left {
                    let left_expression = tree.get(*left_id);
                    visitor.visit_expression(tree, *left_id, left_expression);
                }

                for argument_id in generic_arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_generic_argument(tree, *argument_id, argument);
                }

                if let Some(arguments) = arguments {
                    for argument_id in arguments {
                        let argument = tree.get(*argument_id);
                        visitor.visit_argument(tree, *argument_id, argument);
                    }
                }
                if let Some(elements) = elements {
                    for element_id in elements {
                        let element = tree.get(*element_id);
                        visitor.visit_argument(tree, *element_id, element);
                    }
                }
            }
            Expression::TaggedScalarExpression { ty, value } => {
                let ty_node = tree.get(*ty);
                visitor.visit_type_expression(tree, *ty, ty_node);
                let value_node = tree.get(*value);
                visitor.visit_expression(tree, *value, value_node);
            }
            Expression::TaggedTupleExpression { ty, elements } => {
                let ty_node = tree.get(*ty);
                visitor.visit_type_expression(tree, *ty, ty_node);
                for argument_id in elements {
                    let argument = tree.get(*argument_id);
                    visitor.visit_argument(tree, *argument_id, argument);
                }
            }
            Expression::TaggedObjectExpression { ty, properties } => {
                let ty_node = tree.get(*ty);
                visitor.visit_type_expression(tree, *ty, ty_node);
                for property_id in properties {
                    let property = tree.get(*property_id);
                    visitor.visit_property(tree, *property_id, property);
                }
            }
            Expression::Parenthesized { expression } => {
                let expression_node = tree.get(*expression);
                visitor.visit_expression(tree, *expression, expression_node);
            }

            Expression::If {
                kind: _,
                condition,
                then_expression,
                else_expression,
            } => {
                match condition {
                    IfCondition::Expression { condition } => {
                        let condition_expression = tree.get(*condition);
                        visitor.visit_expression(tree, *condition, condition_expression);
                    }
                    IfCondition::Let { declarator, .. } => {
                        let declarator_node = tree.get(*declarator);
                        visitor.visit_declarator(tree, *declarator, declarator_node);
                    }
                }
                let then_expression_node = tree.get(*then_expression);
                visitor.visit_expression(tree, *then_expression, then_expression_node);
                if let Some(else_expression_id) = else_expression {
                    let else_expression = tree.get(*else_expression_id);
                    visitor.visit_expression(tree, *else_expression_id, else_expression);
                }
            }
            Expression::Loop {
                kind: _,
                condition,
                body,
                scope: _,
                symbol: _,
            } => {
                if let Some(condition_id) = condition {
                    let condition_expression = tree.get(*condition_id);
                    visitor.visit_expression(tree, *condition_id, condition_expression);
                }
                let body_block = tree.get(*body);
                visitor.visit_block(tree, *body, body_block);
            }
            Expression::ForEach {
                asynchrony: _,
                kind: _,
                binding,
                iterator,
                body,
                scope: _,
                symbol: _,
            } => {
                match binding {
                    ForEachBinding::Pattern { pattern, .. } => {
                        let pattern_node = tree.get(*pattern);
                        visitor.visit_pattern(tree, *pattern, pattern_node);
                    }
                    ForEachBinding::Using {
                        asynchrony: _,
                        pattern,
                    } => {
                        let pattern_node = tree.get(*pattern);
                        visitor.visit_pattern(tree, *pattern, pattern_node);
                    }
                }
                let iterator_expression = tree.get(*iterator);
                visitor.visit_expression(tree, *iterator, iterator_expression);
                let body_block = tree.get(*body);
                visitor.visit_block(tree, *body, body_block);
            }
            Expression::For {
                initialization,
                condition,
                increment,
                body,
                scope: _,
                symbol: _,
            } => {
                if let Some(initialization_id) = initialization {
                    let initialization_expression = tree.get(*initialization_id);
                    visitor.visit_expression(tree, *initialization_id, initialization_expression);
                }
                if let Some(condition_id) = condition {
                    let condition_expression = tree.get(*condition_id);
                    visitor.visit_expression(tree, *condition_id, condition_expression);
                }
                if let Some(increment_id) = increment {
                    let increment_expression = tree.get(*increment_id);
                    visitor.visit_expression(tree, *increment_id, increment_expression);
                }
                let body_block = tree.get(*body);
                visitor.visit_block(tree, *body, body_block);
            }
            Expression::Try {
                try_expression,
                catch_pattern,
                catch_ty,
                catch_expression,
                finally_expression,
                scope: _,
                symbol: _,
            } => {
                let try_expression_node = tree.get(*try_expression);
                visitor.visit_expression(tree, *try_expression, try_expression_node);
                if let Some(catch_pattern_id) = catch_pattern {
                    let catch_pattern_node = tree.get(*catch_pattern_id);
                    visitor.visit_pattern(tree, *catch_pattern_id, catch_pattern_node);
                }
                if let Some(catch_ty_id) = catch_ty {
                    let catch_ty_node = tree.get(*catch_ty_id);
                    visitor.visit_type_expression(tree, *catch_ty_id, catch_ty_node);
                }
                if let Some(catch_expression_id) = catch_expression {
                    let catch_expression_node = tree.get(*catch_expression_id);
                    visitor.visit_expression(tree, *catch_expression_id, catch_expression_node);
                }
                if let Some(finally_expression_id) = finally_expression {
                    let finally_expression_node = tree.get(*finally_expression_id);
                    visitor.visit_expression(tree, *finally_expression_id, finally_expression_node);
                }
            }
            Expression::Match {
                kind: _,
                value,
                cases,
                source: _,
                scope: _,
                symbol: _,
            } => {
                let value_expression = tree.get(*value);
                visitor.visit_expression(tree, *value, value_expression);
                for case_id in cases {
                    let case = tree.get(*case_id);
                    visitor.visit_match_case(tree, *case_id, case);
                }
            }
            Expression::Break {
                target: _,
                target_symbol: _,
                value,
            }
            | Expression::UnresolvedBreak { target: _, value } => {
                if let Some(value_id) = value {
                    let value_expression = tree.get(*value_id);
                    visitor.visit_expression(tree, *value_id, value_expression);
                }
            }
            Expression::Continue {
                target: _,
                target_symbol: _,
            }
            | Expression::UnresolvedContinue { target: _ } => {}
            Expression::Await { expression } => {
                let expression_node = tree.get(*expression);
                visitor.visit_expression(tree, *expression, expression_node);
            }
            Expression::AwaitMaybe { expression } => {
                let expression_node = tree.get(*expression);
                visitor.visit_expression(tree, *expression, expression_node);
            }
            Expression::Comptime { body } => {
                let body_expr = tree.get(*body);
                visitor.visit_expression(tree, *body, body_expr);
            }
            Expression::Yield {
                cardinality: _,
                value,
            } => {
                if let Some(value_id) = value {
                    let value_expression = tree.get(*value_id);
                    visitor.visit_expression(tree, *value_id, value_expression);
                }
            }
            Expression::Throw { value } => {
                let value_expression = tree.get(*value);
                visitor.visit_expression(tree, *value, value_expression);
            }
            Expression::Return { value } => {
                if let Some(value_id) = value {
                    let value_expression = tree.get(*value_id);
                    visitor.visit_expression(tree, *value_id, value_expression);
                }
            }
            Expression::Debugger => {}
            Expression::Missing => {}
            Expression::Stub => {}
            Expression::Error => {}
        }
    });
}

/// Walk the Block.
pub fn walk_block<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Block>,
    block: &Block,
) {
    visitor.visit_any(tree, NodeType::Block, id.id);
    for expression_id in block.iter_expressions() {
        let expression = tree.get(expression_id);
        visitor.visit_expression(tree, expression_id, expression);
    }
}

/// Walk the Declaration.
pub fn walk_declaration<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Declaration>,
    declaration: &Declaration,
) {
    destack_core::ensure_sufficient_stack(|| {
        visitor.visit_any(tree, NodeType::Declaration, id.id);
        match declaration {
            Declaration::Global(declaration) => {
                for expression_id in &declaration.expressions {
                    let expression = tree.get(*expression_id);
                    visitor.visit_expression(tree, *expression_id, expression);
                }
            }
            Declaration::Namespace(declaration) => {
                for generic_parameter_id in &declaration.generic_parameters {
                    let generic_parameter = tree.get(*generic_parameter_id);
                    visitor.visit_generic_parameter(tree, *generic_parameter_id, generic_parameter);
                }
                for where_clause_id in &declaration.where_clauses {
                    let where_clause = tree.get(*where_clause_id);
                    visitor.visit_where_clause(tree, *where_clause_id, where_clause);
                }
                for expression_id in &declaration.expressions {
                    let expression = tree.get(*expression_id);
                    visitor.visit_expression(tree, *expression_id, expression);
                }
            }
            Declaration::Type(declaration) => {
                for generic_parameter_id in &declaration.generic_parameters {
                    let generic_parameter = tree.get(*generic_parameter_id);
                    visitor.visit_generic_parameter(tree, *generic_parameter_id, generic_parameter);
                }
                for where_clause_id in &declaration.where_clauses {
                    let where_clause = tree.get(*where_clause_id);
                    visitor.visit_where_clause(tree, *where_clause_id, where_clause);
                }
                let value_expression = tree.get(declaration.value);
                visitor.visit_type_expression(tree, declaration.value, value_expression);
            }
            Declaration::ImportAlias(declaration) => {
                if let ImportAliasTarget::Path { path: _ } = &declaration.target {
                    // no child nodes to visit
                }
            }
            Declaration::Struct(declaration) => {
                for generic_parameter_id in &declaration.generic_parameters {
                    let generic_parameter = tree.get(*generic_parameter_id);
                    visitor.visit_generic_parameter(tree, *generic_parameter_id, generic_parameter);
                }
                for where_clause_id in &declaration.where_clauses {
                    let where_clause = tree.get(*where_clause_id);
                    visitor.visit_where_clause(tree, *where_clause_id, where_clause);
                }
                for implements_type_id in &declaration.implements_types {
                    let implements_type = tree.get(*implements_type_id);
                    visitor.visit_type_expression(tree, *implements_type_id, implements_type);
                }
                for embedded_type_id in &declaration.embedded_types {
                    let embedded_type = tree.get(*embedded_type_id);
                    visitor.visit_type_expression(tree, *embedded_type_id, embedded_type);
                }
                for member_id in &declaration.members {
                    let member = tree.get(*member_id);
                    visitor.visit_member(tree, *member_id, member);
                }
            }
            Declaration::Class(declaration) => {
                for generic_parameter_id in &declaration.generic_parameters {
                    let generic_parameter = tree.get(*generic_parameter_id);
                    visitor.visit_generic_parameter(tree, *generic_parameter_id, generic_parameter);
                }
                for where_clause_id in &declaration.where_clauses {
                    let where_clause = tree.get(*where_clause_id);
                    visitor.visit_where_clause(tree, *where_clause_id, where_clause);
                }
                if let Some(extends_expression_id) = declaration.extends_expression {
                    let extends_expression = tree.get(extends_expression_id);
                    visitor.visit_expression(tree, extends_expression_id, extends_expression);
                }
                for implements_type_id in &declaration.implements_types {
                    let implements_type = tree.get(*implements_type_id);
                    visitor.visit_type_expression(tree, *implements_type_id, implements_type);
                }
                for member_id in &declaration.members {
                    let member = tree.get(*member_id);
                    visitor.visit_member(tree, *member_id, member);
                }
            }
            Declaration::Enum(declaration) => {
                for generic_parameter_id in &declaration.generic_parameters {
                    let generic_parameter = tree.get(*generic_parameter_id);
                    visitor.visit_generic_parameter(tree, *generic_parameter_id, generic_parameter);
                }
                for where_clause_id in &declaration.where_clauses {
                    let where_clause = tree.get(*where_clause_id);
                    visitor.visit_where_clause(tree, *where_clause_id, where_clause);
                }
                for implements_type_id in &declaration.implements_types {
                    let implements_type = tree.get(*implements_type_id);
                    visitor.visit_type_expression(tree, *implements_type_id, implements_type);
                }
                for field_id in &declaration.fields {
                    let field = tree.get(*field_id);
                    visitor.visit_enum_field(tree, *field_id, field);
                }
                for member_id in &declaration.members {
                    let member = tree.get(*member_id);
                    visitor.visit_member(tree, *member_id, member);
                }
            }
            Declaration::Interface(declaration) => {
                for generic_parameter_id in &declaration.generic_parameters {
                    let generic_parameter = tree.get(*generic_parameter_id);
                    visitor.visit_generic_parameter(tree, *generic_parameter_id, generic_parameter);
                }
                for where_clause_id in &declaration.where_clauses {
                    let where_clause = tree.get(*where_clause_id);
                    visitor.visit_where_clause(tree, *where_clause_id, where_clause);
                }
                for extends_type_id in &declaration.extends_types {
                    let extends_type = tree.get(*extends_type_id);
                    visitor.visit_type_expression(tree, *extends_type_id, extends_type);
                }
                for member_id in &declaration.members {
                    let member = tree.get(*member_id);
                    visitor.visit_type_member(tree, *member_id, member);
                }
            }
            Declaration::Extension(declaration) => {
                for generic_parameter_id in &declaration.generic_parameters {
                    let generic_parameter = tree.get(*generic_parameter_id);
                    visitor.visit_generic_parameter(tree, *generic_parameter_id, generic_parameter);
                }
                for where_clause_id in &declaration.where_clauses {
                    let where_clause = tree.get(*where_clause_id);
                    visitor.visit_where_clause(tree, *where_clause_id, where_clause);
                }
                let target_type_expr = tree.get(declaration.target_type);
                visitor.visit_type_expression(tree, declaration.target_type, target_type_expr);
                for implements_type_id in &declaration.implements_types {
                    let implements_type = tree.get(*implements_type_id);
                    visitor.visit_type_expression(tree, *implements_type_id, implements_type);
                }
                for member_id in &declaration.members {
                    let member = tree.get(*member_id);
                    visitor.visit_member(tree, *member_id, member);
                }
            }
            Declaration::Function(declaration) => {
                walk_function_signature(visitor, tree, &declaration.signature);
                if let Some(body) = declaration.body {
                    let body_expression = tree.get(body);
                    visitor.visit_expression(tree, body, body_expression);
                }
            }
        }
    });
}

/// Walk the Declarator.
pub fn walk_declarator<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Declarator>,
    declarator: &Declarator,
) {
    visitor.visit_any(tree, NodeType::Declarator, id.id);
    let Declarator { pattern, ty, value } = declarator;
    let pattern_node = tree.get(*pattern);
    visitor.visit_pattern(tree, *pattern, pattern_node);
    if let Some(ty_id) = ty {
        let ty_node = tree.get(*ty_id);
        visitor.visit_type_expression(tree, *ty_id, ty_node);
    }
    if let Some(value_id) = value {
        let value_node = tree.get(*value_id);
        visitor.visit_expression(tree, *value_id, value_node);
    }
}

/// Walk the Key.
pub fn walk_key<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, key: &Key) {
    match key {
        Key::Name(_) | Key::Private(_) => {}
        Key::Expression(expression) => {
            let expression_expr = tree.get(*expression);
            visitor.visit_expression(tree, *expression, expression_expr);
        }
    }
}

/// Walk the Property.
pub fn walk_property<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Property>,
    property: &Property,
) {
    visitor.visit_any(tree, NodeType::Property, id.id);
    match property {
        Property::Field {
            key,
            value,
            symbol: _,
        } => {
            walk_key(visitor, tree, key);

            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }
        Property::Method {
            key,
            signature,
            body,
            symbol: _,
        } => {
            if let Some(key) = key {
                walk_key(visitor, tree, key);
            }
            walk_function_signature(visitor, tree, signature);

            if let Some(body) = body {
                let body_expr = tree.get(*body);
                visitor.visit_expression(tree, *body, body_expr);
            }
        }
        Property::Spread { value, symbol: _ } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }
        Property::Error { symbol: _ } => {}
    }
}

/// Walk the Member.
pub fn walk_member<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Member>,
    member: &Member,
) {
    visitor.visit_any(tree, NodeType::Member, id.id);
    match member {
        Member::AssociatedType {
            name: _,
            generic_parameters,
            where_clauses,
            constraint,
            value,
            visibility: _,
            ambient: _,
            is_abstract: _,
            is_override: _,
            is_static: _,
            symbol: _,
        } => {
            for generic_parameter_id in generic_parameters {
                let generic_parameter = tree.get(*generic_parameter_id);
                visitor.visit_generic_parameter(tree, *generic_parameter_id, generic_parameter);
            }
            for where_clause_id in where_clauses {
                let where_clause = tree.get(*where_clause_id);
                visitor.visit_where_clause(tree, *where_clause_id, where_clause);
            }
            if let Some(constraint) = constraint {
                let constraint_expr = tree.get(*constraint);
                visitor.visit_type_expression(tree, *constraint, constraint_expr);
            }
            if let Some(value) = value {
                let value_expr = tree.get(*value);
                visitor.visit_type_expression(tree, *value, value_expr);
            }
        }
        Member::AssociatedConst {
            name: _,
            declared_type,
            value,
            visibility: _,
            ambient: _,
            is_static: _,
            symbol: _,
        } => {
            if let Some(declared_type) = declared_type {
                let declared_type_expr = tree.get(*declared_type);
                visitor.visit_type_expression(tree, *declared_type, declared_type_expr);
            }
            if let Some(value) = value {
                let value_expr = tree.get(*value);
                visitor.visit_expression(tree, *value, value_expr);
            }
        }
        Member::Field {
            key,
            declared_type,
            default,
            is_optional: _,
            is_readonly: _,
            mutability: _,
            visibility: _,
            ambient: _,
            is_abstract: _,
            is_override: _,
            is_static: _,
            is_definite: _,
            is_accessor: _,
            is_comptime: _,
            symbol: _,
        } => {
            walk_key(visitor, tree, key);
            if let Some(declared_type) = declared_type {
                let declared_type_expr = tree.get(*declared_type);
                visitor.visit_type_expression(tree, *declared_type, declared_type_expr);
            }
            if let Some(default) = default {
                let default_expr = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expr);
            }
        }
        Member::Method {
            key,
            signature,
            body,
            visibility: _,
            ambient: _,
            is_abstract: _,
            is_override: _,
            is_static: _,
            is_accessor: _,
            is_comptime: _,
            symbol: _,
        } => {
            if let Some(key) = key {
                walk_key(visitor, tree, key);
            }
            walk_function_signature(visitor, tree, signature);
            if let Some(body) = body {
                let body_expr = tree.get(*body);
                visitor.visit_expression(tree, *body, body_expr);
            }
        }
        Member::Embed {
            value,
            visibility: _,
            ambient: _,
            is_static: _,
            symbol: _,
        } => {
            let value_expr = tree.get(*value);
            visitor.visit_type_expression(tree, *value, value_expr);
        }
        Member::StaticBlock { body, symbol: _ } => {
            let body_expr = tree.get(*body);
            visitor.visit_expression(tree, *body, body_expr);
        }
        Member::ComptimeBlock { body, symbol: _ } => {
            let body_expr = tree.get(*body);
            visitor.visit_expression(tree, *body, body_expr);
        }
        Member::Error { symbol: _ } => {}
    }
}

/// Walk the EnumField.
pub fn walk_enum_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<EnumField>,
    enum_field: &EnumField,
) {
    visitor.visit_any(tree, NodeType::EnumField, id.id);
    if let Some(value) = enum_field.value {
        let value_expression = tree.get(value);
        visitor.visit_expression(tree, value, value_expression);
    }
}

/// Walk the WhereClause.
pub fn walk_where_clause<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<WhereClause>,
    where_clause: &WhereClause,
) {
    visitor.visit_any(tree, NodeType::WhereClause, id.id);
    let right_expression = tree.get(where_clause.right);
    visitor.visit_type_expression(tree, where_clause.right, right_expression);
}

/// Walk the DependencyItem.
pub fn walk_dependency_item<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<DependencyItem>,
    dependency_item: &DependencyItem,
) {
    visitor.visit_any(tree, NodeType::DependencyItem, id.id);
    match dependency_item {
        DependencyItem::Error => {}
        DependencyItem::UnresolvedRemote {
            source: _,
            mode: _,
            kind: _,
            name: _,
            alias: _,
            target: _,
            target_module: _,
            symbol: _,
        } => {
            // nothing to do
        }
        DependencyItem::UnresolvedLocal {
            mode: _,
            kind: _,
            name: _,
            alias: _,
            symbol: _,
        } => {
            // nothing to do
        }
        DependencyItem::Value { value, .. } => {
            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
        }
        DependencyItem::Local {
            mode: _,
            kind: _,
            name: _,
            alias: _,
            symbol: _,
            target_symbol: _,
        } => {
            // nothing to do
        }
        DependencyItem::Remote {
            mode: _,
            kind: _,
            name: _,
            alias: _,
            target: _,
            target_module: _,
            symbol: _,
            target_symbol: _,
        } => {
            // nothing to do
        }
    }
}

/// Walk the Parameter.
pub fn walk_parameter<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Parameter>,
    parameter: &Parameter,
) {
    visitor.visit_any(tree, NodeType::Parameter, id.id);
    match parameter {
        Parameter::Named {
            name: _,
            visibility: _,
            is_readonly: _,
            is_optional: _,
            declared_type,
            default,
            symbol: _,
        } => {
            if let Some(declared_type) = declared_type {
                let declared_type_expression = tree.get(*declared_type);
                visitor.visit_type_expression(tree, *declared_type, declared_type_expression);
            }
            if let Some(default) = default {
                let default_expression = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expression);
            }
        }
        Parameter::Pattern {
            pattern,
            is_optional: _,
            declared_type,
            symbol: _,
            default,
        } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, pattern_node);
            if let Some(declared_type) = declared_type {
                let declared_type_expression = tree.get(*declared_type);
                visitor.visit_type_expression(tree, *declared_type, declared_type_expression);
            }
            if let Some(default) = default {
                let default_expression = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expression);
            }
        }
        Parameter::VariadicNamed {
            name: _,
            visibility: _,
            is_readonly: _,
            declared_type,
            symbol: _,
        } => {
            if let Some(declared_type) = declared_type {
                let declared_type_expression = tree.get(*declared_type);
                visitor.visit_type_expression(tree, *declared_type, declared_type_expression);
            }
        }
        Parameter::VariadicPattern {
            pattern,
            declared_type,
            symbol: _,
        } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, pattern_node);
            if let Some(declared_type) = declared_type {
                let declared_type_expression = tree.get(*declared_type);
                visitor.visit_type_expression(tree, *declared_type, declared_type_expression);
            }
        }
        Parameter::Error { symbol: _ } => {}
    }
}

/// Walk the Argument.
pub fn walk_argument<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Argument>,
    argument: &Argument,
) {
    visitor.visit_any(tree, NodeType::Argument, id.id);
    match argument {
        Argument::Named { name: _, value, .. }
        | Argument::Labeled {
            label: _, value, ..
        }
        | Argument::Positional { value, .. }
        | Argument::Spread {
            label: _, value, ..
        }
        | Argument::Error { value } => {
            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
        }
    }
}

/// Walk the Pattern.
pub fn walk_pattern<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Pattern>,
    pattern: &Pattern,
) {
    destack_core::ensure_sufficient_stack(|| {
        visitor.visit_any(tree, NodeType::Pattern, id.id);
        match pattern {
            Pattern::Wildcard => {}
            Pattern::Must(inner) => {
                let inner_pattern = tree.get(*inner);
                visitor.visit_pattern(tree, *inner, inner_pattern);
            }
            Pattern::ReferenceOf {
                right,
                mutability: _,
            } => {
                let right_pattern = tree.get(*right);
                visitor.visit_pattern(tree, *right, right_pattern);
            }
            Pattern::ValueOf {
                right,
                mutability: _,
            } => {
                let right_pattern = tree.get(*right);
                visitor.visit_pattern(tree, *right, right_pattern);
            }
            Pattern::Binding {
                mutability: _,
                name: _,
                pattern,
                symbol: _,
            } => {
                if let Some(pattern_id) = pattern {
                    let pattern_node = tree.get(*pattern_id);
                    visitor.visit_pattern(tree, *pattern_id, pattern_node);
                }
            }
            Pattern::Expression { value } => {
                let value_expression = tree.get(*value);
                visitor.visit_expression(tree, *value, value_expression);
            }
            Pattern::TypeExpression { value } => {
                let value_expression = tree.get(*value);
                visitor.visit_type_expression(tree, *value, value_expression);
            }
            Pattern::Tuple { fields } => {
                for field_id in fields {
                    let field = tree.get(*field_id);
                    visitor.visit_pattern_field(tree, *field_id, field);
                }
            }
            Pattern::TaggedTuple { ty, fields } => {
                let ty_expression = tree.get(*ty);
                visitor.visit_type_expression(tree, *ty, ty_expression);
                for field_id in fields {
                    let field = tree.get(*field_id);
                    visitor.visit_pattern_field(tree, *field_id, field);
                }
            }
            Pattern::Array { fields } => {
                for field_id in fields {
                    let field = tree.get(*field_id);
                    visitor.visit_pattern_field(tree, *field_id, field);
                }
            }
            Pattern::Object { fields } => {
                for field_id in fields {
                    let field = tree.get(*field_id);
                    visitor.visit_pattern_field(tree, *field_id, field);
                }
            }
            Pattern::TaggedObject { ty, fields } => {
                let ty_expression = tree.get(*ty);
                visitor.visit_type_expression(tree, *ty, ty_expression);
                for field_id in fields {
                    let field = tree.get(*field_id);
                    visitor.visit_pattern_field(tree, *field_id, field);
                }
            }
            Pattern::Union { patterns } => {
                for pattern_id in patterns {
                    let union_pattern = tree.get(*pattern_id);
                    visitor.visit_pattern(tree, *pattern_id, union_pattern);
                }
            }
        }
    });
}

/// Walk the PatternField.
pub fn walk_pattern_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<PatternField>,
    pattern_field: &PatternField,
) {
    visitor.visit_any(tree, NodeType::PatternField, id.id);
    match pattern_field {
        PatternField::Named {
            mutability: _,
            name: _,
            pattern,
            default,
        } => {
            if let Some(pattern_id) = pattern {
                let pattern_node = tree.get(*pattern_id);
                visitor.visit_pattern(tree, *pattern_id, pattern_node);
            }
            if let Some(default) = default {
                let default_expression = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expression);
            }
        }
        PatternField::Computed {
            mutability: _,
            key,
            pattern,
            default,
        } => {
            let key_expression = tree.get(*key);
            visitor.visit_expression(tree, *key, key_expression);
            if let Some(pattern_id) = pattern {
                let pattern_node = tree.get(*pattern_id);
                visitor.visit_pattern(tree, *pattern_id, pattern_node);
            }
            if let Some(default) = default {
                let default_expression = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expression);
            }
        }
        PatternField::Alias {
            mutability: _,
            name: _,
            alias: _,
            symbol: _,
            default,
        } => {
            if let Some(default) = default {
                let default_expression = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expression);
            }
        }
        PatternField::Positional { pattern, default } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, pattern_node);
            if let Some(default) = default {
                let default_expression = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expression);
            }
        }
        PatternField::Spread {
            mutability: _,
            pattern,
        } => {
            if let Some(pattern_id) = pattern {
                let pattern_node = tree.get(*pattern_id);
                visitor.visit_pattern(tree, *pattern_id, pattern_node);
            }
        }
        PatternField::Elision => {
            // nothing to do
        }
    }
}

/// Walk the MatchSelector.
fn walk_match_selector<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    selector: &MatchSelector,
) {
    match selector {
        MatchSelector::Pattern { pattern, guard } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, pattern_node);
            if let Some(guard_id) = guard {
                let guard_expression = tree.get(*guard_id);
                visitor.visit_expression(tree, *guard_id, guard_expression);
            }
        }
        MatchSelector::Default => {}
    }
}

/// Walk the MatchCase.
pub fn walk_match_case<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<MatchCase>,
    match_case: &MatchCase,
) {
    visitor.visit_any(tree, NodeType::MatchCase, id.id);
    match match_case {
        MatchCase::Expression {
            selector,
            body,
            scope: _,
        } => {
            walk_match_selector(visitor, tree, selector);
            let body_expression = tree.get(*body);
            visitor.visit_expression(tree, *body, body_expression);
        }
        MatchCase::Block {
            selector,
            body,
            scope: _,
        } => {
            walk_match_selector(visitor, tree, selector);
            let body_block = tree.get(*body);
            visitor.visit_block(tree, *body, body_block);
        }
    }
}

/// Walk the Decorator.
pub fn walk_decorator<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Decorator>,
    decorator: &Decorator,
) {
    visitor.visit_any(tree, NodeType::Decorator, id.id);

    let expression_node = tree.get(decorator.expression);
    visitor.visit_expression(tree, decorator.expression, expression_node);
}
