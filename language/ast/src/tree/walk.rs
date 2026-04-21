use crate::{
    Argument, AssignPattern, AssignPatternField, Block, Declaration, Declarator, Decorator,
    DependencyItem, EnumField, Expression, ForEachBinding, FunctionSignature, GenericArgument,
    GenericParameter, IfCondition, ImportAliasTarget, ImportTarget, Key, LocalNodeId,
    LocalNodeIdAny, MatchCase, MatchSelector, Member, NodeTree, NodeType, NodeVisitor, Parameter,
    Pattern, PatternField, Property, TemplateLiteral, TupleElement, TypeExpression, TypeMember,
    WhereClause,
};

/// Walk any node.
pub fn walk_any<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    node_type: NodeType,
    node_id: u32,
) {
    let local_idx = tree.node_index_by_node_id[node_id as usize].local_id();
    match node_type {
        // --------------------------------------------------------------------
        // Groupings
        // --------------------------------------------------------------------
        NodeType::Expression => {
            let expression = tree.expressions.get(local_idx);
            walk_expression(visitor, tree, LocalNodeId::new(node_id), expression);
        }
        NodeType::Block => {
            let block = tree.blocks.get(local_idx);
            walk_block(visitor, tree, LocalNodeId::new(node_id), block);
        }
        // --------------------------------------------------------------------
        // Declarations
        // --------------------------------------------------------------------
        NodeType::Declaration => {
            let declaration = tree.declarations.get(local_idx);
            walk_declaration(visitor, tree, LocalNodeId::new(node_id), declaration);
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
        // --------------------------------------------------------------------
        // Context
        // --------------------------------------------------------------------
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
        // --------------------------------------------------------------------
        // Bindings
        // --------------------------------------------------------------------
        NodeType::Parameter => {
            let parameter = tree.parameters.get(local_idx);
            walk_parameter(visitor, tree, LocalNodeId::new(node_id), parameter);
        }
        NodeType::Argument => {
            let argument = tree.arguments.get(local_idx);
            walk_argument(visitor, tree, LocalNodeId::new(node_id), argument);
        }
        NodeType::GenericArgument => {
            let type_argument = tree.generic_arguments.get(local_idx);
            walk_generic_argument(visitor, tree, LocalNodeId::new(node_id), type_argument);
        }
        NodeType::TupleElement => {
            let tuple_element = tree.tuple_elements.get(local_idx);
            walk_tuple_element(visitor, tree, LocalNodeId::new(node_id), tuple_element);
        }
        // --------------------------------------------------------------------
        // Matching
        // --------------------------------------------------------------------
        NodeType::Pattern => {
            let pattern = tree.patterns.get(local_idx);
            walk_pattern(visitor, tree, LocalNodeId::new(node_id), pattern);
        }
        NodeType::PatternField => {
            let pattern_field = tree.pattern_fields.get(local_idx);
            walk_pattern_field(visitor, tree, LocalNodeId::new(node_id), pattern_field);
        }
        NodeType::AssignPattern => {
            let assign_pattern = tree.assign_patterns.get(local_idx);
            walk_assign_pattern(visitor, tree, LocalNodeId::new(node_id), assign_pattern);
        }
        NodeType::AssignPatternField => {
            let assign_pattern_field = tree.assign_pattern_fields.get(local_idx);
            walk_assign_pattern_field(
                visitor,
                tree,
                LocalNodeId::new(node_id),
                assign_pattern_field,
            );
        }
        NodeType::MatchCase => {
            let match_case = tree.match_cases.get(local_idx);
            walk_match_case(visitor, tree, LocalNodeId::new(node_id), match_case);
        }
        NodeType::Declarator => {
            let declarator = tree.declarators.get(local_idx);
            walk_declarator(visitor, tree, LocalNodeId::new(node_id), declarator);
        }
        NodeType::Decorator => {
            let decorator = tree.decorators.get(local_idx);
            walk_decorator(visitor, tree, LocalNodeId::new(node_id), decorator);
        }
        NodeType::TypeExpression => {
            let type_expression = tree.type_expressions.get(local_idx);
            walk_type_expression(visitor, tree, LocalNodeId::new(node_id), type_expression);
        }
    }
}

/// Walk one root node through the visitor entry points.
pub fn walk_root<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, root: &LocalNodeIdAny) {
    match root.ty {
        NodeType::Expression => {
            let expression_id = LocalNodeId::<Expression>::new(root.id);
            let expression = tree.get(expression_id);
            visitor.visit_expression(tree, expression_id, expression);
        }
        NodeType::Block => {
            let block_id = LocalNodeId::<Block>::new(root.id);
            let block = tree.get(block_id);
            visitor.visit_block(tree, block_id, block);
        }
        NodeType::Declaration => {
            let declaration_id = LocalNodeId::<Declaration>::new(root.id);
            let declaration = tree.get(declaration_id);
            visitor.visit_declaration(tree, declaration_id, declaration);
        }
        NodeType::Property => {
            let property_id = LocalNodeId::<Property>::new(root.id);
            let property = tree.get(property_id);
            visitor.visit_property(tree, property_id, property);
        }
        NodeType::TypeMember => {
            let type_member_id = LocalNodeId::<TypeMember>::new(root.id);
            let type_member = tree.get(type_member_id);
            visitor.visit_type_member(tree, type_member_id, type_member);
        }
        NodeType::Member => {
            let member_id = LocalNodeId::<Member>::new(root.id);
            let member = tree.get(member_id);
            visitor.visit_member(tree, member_id, member);
        }
        NodeType::EnumField => {
            let enum_field_id = LocalNodeId::<EnumField>::new(root.id);
            let enum_field = tree.get(enum_field_id);
            visitor.visit_enum_field(tree, enum_field_id, enum_field);
        }
        NodeType::WhereClause => {
            let where_clause_id = LocalNodeId::<WhereClause>::new(root.id);
            let where_clause = tree.get(where_clause_id);
            visitor.visit_where_clause(tree, where_clause_id, where_clause);
        }
        NodeType::DependencyItem => {
            let dependency_item_id = LocalNodeId::<DependencyItem>::new(root.id);
            let dependency_item = tree.get(dependency_item_id);
            visitor.visit_dependency_item(tree, dependency_item_id, dependency_item);
        }
        NodeType::GenericParameter => {
            let generic_parameter_id = LocalNodeId::<GenericParameter>::new(root.id);
            let generic_parameter = tree.get(generic_parameter_id);
            visitor.visit_generic_parameter(tree, generic_parameter_id, generic_parameter);
        }
        NodeType::Parameter => {
            let parameter_id = LocalNodeId::<Parameter>::new(root.id);
            let parameter = tree.get(parameter_id);
            visitor.visit_parameter(tree, parameter_id, parameter);
        }
        NodeType::Argument => {
            let argument_id = LocalNodeId::<Argument>::new(root.id);
            let argument = tree.get(argument_id);
            visitor.visit_argument(tree, argument_id, argument);
        }
        NodeType::GenericArgument => {
            let type_argument_id = LocalNodeId::<GenericArgument>::new(root.id);
            let type_argument = tree.get(type_argument_id);
            visitor.visit_generic_argument(tree, type_argument_id, type_argument);
        }
        NodeType::TupleElement => {
            let tuple_element_id = LocalNodeId::<TupleElement>::new(root.id);
            let tuple_element = tree.get(tuple_element_id);
            visitor.visit_tuple_element(tree, tuple_element_id, tuple_element);
        }
        NodeType::MatchCase => {
            let match_case_id = LocalNodeId::<MatchCase>::new(root.id);
            let match_case = tree.get(match_case_id);
            visitor.visit_match_case(tree, match_case_id, match_case);
        }
        NodeType::Pattern => {
            let pattern_id = LocalNodeId::<Pattern>::new(root.id);
            let pattern = tree.get(pattern_id);
            visitor.visit_pattern(tree, pattern_id, pattern);
        }
        NodeType::PatternField => {
            let pattern_field_id = LocalNodeId::<PatternField>::new(root.id);
            let pattern_field = tree.get(pattern_field_id);
            visitor.visit_pattern_field(tree, pattern_field_id, pattern_field);
        }
        NodeType::AssignPattern => {
            let assign_pattern_id = LocalNodeId::<AssignPattern>::new(root.id);
            let assign_pattern = tree.get(assign_pattern_id);
            visitor.visit_assign_pattern(tree, assign_pattern_id, assign_pattern);
        }
        NodeType::AssignPatternField => {
            let assign_pattern_field_id = LocalNodeId::<AssignPatternField>::new(root.id);
            let assign_pattern_field = tree.get(assign_pattern_field_id);
            visitor.visit_assign_pattern_field(tree, assign_pattern_field_id, assign_pattern_field);
        }
        NodeType::Declarator => {
            let declarator_id = LocalNodeId::<Declarator>::new(root.id);
            let declarator = tree.get(declarator_id);
            visitor.visit_declarator(tree, declarator_id, declarator);
        }
        NodeType::Decorator => {
            let decorator_id = LocalNodeId::<Decorator>::new(root.id);
            let decorator = tree.get(decorator_id);
            visitor.visit_decorator(tree, decorator_id, decorator);
        }
        NodeType::TypeExpression => {
            let type_expression_id = LocalNodeId::<TypeExpression>::new(root.id);
            let type_expression = tree.get(type_expression_id);
            visitor.visit_type_expression(tree, type_expression_id, type_expression);
        }
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
        TypeExpression::ScalarLiteral { .. } => {}
        TypeExpression::Literal { .. } => {}
        TypeExpression::Intrinsic => {}
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
        TypeExpression::Const => {}
        TypeExpression::This => {}
        TypeExpression::Import {
            target,
            arguments,
            qualifier: _,
            generic_arguments,
        } => {
            let target_expression = tree.get(*target);
            visitor.visit_expression(tree, *target, target_expression);

            for argument_id in arguments {
                let argument = tree.get(*argument_id);
                visitor.visit_argument(tree, *argument_id, argument);
            }

            for argument_id in generic_arguments {
                let argument = tree.get(*argument_id);
                visitor.visit_generic_argument(tree, *argument_id, argument);
            }
        }
        TypeExpression::Readonly { target_type }
        | TypeExpression::KeyOf { target_type }
        | TypeExpression::Must { target_type }
        | TypeExpression::AsComptime { target_type }
        | TypeExpression::Not { target_type }
        | TypeExpression::ValueOf {
            mutability: _,
            variance: _,
            target_type,
        }
        | TypeExpression::ReferenceOf {
            mutability: _,
            variance: _,
            target_type,
        }
        | TypeExpression::PointerOf {
            mutability: _,
            target_type,
        } => {
            let target_type_node = tree.get(*target_type);
            visitor.visit_type_expression(tree, *target_type, target_type_node);
        }
        TypeExpression::TypeOfValue { value } => {
            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
        }
        TypeExpression::Union { elements } | TypeExpression::Intersection { elements } => {
            for element_id in elements {
                let element = tree.get(*element_id);
                visitor.visit_type_expression(tree, *element_id, element);
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
        TypeExpression::Index { left, index } => {
            let left_node = tree.get(*left);
            visitor.visit_type_expression(tree, *left, left_node);

            let index_node = tree.get(*index);
            visitor.visit_type_expression(tree, *index, index_node);
        }
        TypeExpression::TemplateLiteral { strings: _, spans } => {
            for span_id in spans {
                let span = tree.get(*span_id);
                visitor.visit_type_expression(tree, *span_id, span);
            }
        }
        TypeExpression::Infer {
            name: _,
            constraint,
        } => {
            if let Some(constraint_id) = constraint {
                let constraint_node = tree.get(*constraint_id);
                visitor.visit_type_expression(tree, *constraint_id, constraint_node);
            }
        }
        TypeExpression::Predicate {
            asserts: _,
            subject: _,
            target,
        } => {
            if let Some(target_id) = target {
                let target_node = tree.get(*target_id);
                visitor.visit_type_expression(tree, *target_id, target_node);
            }
        }
        TypeExpression::Missing | TypeExpression::Error => {}
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
        } => {
            walk_key(visitor, tree, key);

            if let Some(declared_type) = declared_type {
                let declared_type_node = tree.get(*declared_type);
                visitor.visit_type_expression(tree, *declared_type, declared_type_node);
            }
        }
        TypeMember::Method {
            is_optional: _,
            key,
            signature,
            body,
        } => {
            if let Some(key) = key {
                walk_key(visitor, tree, key);
            }

            walk_function_signature(visitor, tree, signature);

            if let Some(body_id) = body {
                let expression = tree.get(*body_id);
                visitor.visit_expression(tree, *body_id, expression);
            }
        }
        TypeMember::IndexSignature {
            is_optional: _,
            is_readonly: _,
            name: _,
            key_type,
            value_type,
        } => {
            let key_type_node = tree.get(*key_type);
            visitor.visit_type_expression(tree, *key_type, key_type_node);

            let value_type_node = tree.get(*value_type);
            visitor.visit_type_expression(tree, *value_type, value_type_node);
        }
        TypeMember::Embed { value } => {
            let value_node = tree.get(*value);
            visitor.visit_type_expression(tree, *value, value_node);
        }
        TypeMember::AssociatedType {
            name: _,
            generic_parameters,
            where_clauses,
            constraint,
            value,
        } => {
            for parameter_id in generic_parameters {
                let parameter = tree.get(*parameter_id);
                visitor.visit_generic_parameter(tree, *parameter_id, parameter);
            }

            for where_clause_id in where_clauses {
                let where_clause = tree.get(*where_clause_id);
                visitor.visit_where_clause(tree, *where_clause_id, where_clause);
            }

            if let Some(constraint_id) = constraint {
                let constraint_node = tree.get(*constraint_id);
                visitor.visit_type_expression(tree, *constraint_id, constraint_node);
            }

            if let Some(value_id) = value {
                let value_node = tree.get(*value_id);
                visitor.visit_type_expression(tree, *value_id, value_node);
            }
        }
        TypeMember::AssociatedConst {
            name: _,
            declared_type,
            value,
        } => {
            if let Some(declared_type_id) = declared_type {
                let declared_type_node = tree.get(*declared_type_id);
                visitor.visit_type_expression(tree, *declared_type_id, declared_type_node);
            }

            if let Some(value_id) = value {
                let value_node = tree.get(*value_id);
                visitor.visit_expression(tree, *value_id, value_node);
            }
        }
        TypeMember::Error => {}
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
            let value_type = tree.get(*value);
            visitor.visit_type_expression(tree, *value, value_type);
        }
        GenericArgument::Value { value } => {
            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
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
            let value_type = tree.get(*value);
            visitor.visit_type_expression(tree, *value, value_type);
        }
        TupleElement::Error => {}
    }
}

/// Walk one root list through the visitor entry points.
pub fn walk_roots<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    roots: &[LocalNodeIdAny],
) {
    for root in roots {
        walk_root(visitor, tree, root);
    }
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

/// Walk the Expression.
pub fn walk_expression<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Expression>,
    expression: &Expression,
) {
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

        Expression::Labelled { label: _, body } => {
            let body_expr = tree.get(*body);
            visitor.visit_expression(tree, *body, body_expr);
        }

        Expression::Import {
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

        Expression::Export {
            kind: _,
            target: _,
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
            kind: _,
            mutability: _,
            export: _,
            ambient: _,
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

        Expression::If {
            kind: _,
            condition,
            then_expression,
            else_expression,
        } => {
            match condition {
                IfCondition::Expression { condition } => {
                    let cond_expr = tree.get(*condition);
                    visitor.visit_expression(tree, *condition, cond_expr);
                }
                IfCondition::Let { declarator, .. } => {
                    let declarator_node = tree.get(*declarator);
                    visitor.visit_declarator(tree, *declarator, declarator_node);
                }
            }
            let then_expression_node = tree.get(*then_expression);
            visitor.visit_expression(tree, *then_expression, then_expression_node);
            if let Some(else_expression_id) = else_expression {
                let else_expression_node = tree.get(*else_expression_id);
                visitor.visit_expression(tree, *else_expression_id, else_expression_node);
            }
        }

        Expression::While {
            kind: _,
            condition,
            body,
        } => {
            let cond_expr = tree.get(*condition);
            visitor.visit_expression(tree, *condition, cond_expr);
            let body_block = tree.get(*body);
            visitor.visit_block(tree, *body, body_block);
        }

        Expression::ForEach {
            asynchrony: _,
            kind: _,
            binding,
            iterator,
            body,
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
            let iterator_expr = tree.get(*iterator);
            visitor.visit_expression(tree, *iterator, iterator_expr);
            let body_block = tree.get(*body);
            visitor.visit_block(tree, *body, body_block);
        }

        Expression::For {
            initialization,
            condition,
            increment,
            body,
        } => {
            if let Some(initialization_id) = initialization {
                let initialization_expr = tree.get(*initialization_id);
                visitor.visit_expression(tree, *initialization_id, initialization_expr);
            }
            if let Some(condition_id) = condition {
                let condition_expr = tree.get(*condition_id);
                visitor.visit_expression(tree, *condition_id, condition_expr);
            }
            if let Some(increment_id) = increment {
                let increment_expr = tree.get(*increment_id);
                visitor.visit_expression(tree, *increment_id, increment_expr);
            }
            let body_block = tree.get(*body);
            visitor.visit_block(tree, *body, body_block);
        }

        Expression::Loop { body } => {
            let body_block = tree.get(*body);
            visitor.visit_block(tree, *body, body_block);
        }

        Expression::Try {
            try_expression,
            catch_pattern,
            catch_ty,
            catch_expression,
            finally_expression,
        } => {
            let try_expr_node = tree.get(*try_expression);
            visitor.visit_expression(tree, *try_expression, try_expr_node);
            if let Some(catch_pattern_id) = catch_pattern {
                let catch_pattern_node = tree.get(*catch_pattern_id);
                visitor.visit_pattern(tree, *catch_pattern_id, catch_pattern_node);
            }
            if let Some(catch_ty_id) = catch_ty {
                let catch_ty_node = tree.get(*catch_ty_id);
                visitor.visit_type_expression(tree, *catch_ty_id, catch_ty_node);
            }
            if let Some(catch_id) = catch_expression {
                let catch_expr = tree.get(*catch_id);
                visitor.visit_expression(tree, *catch_id, catch_expr);
            }
            if let Some(finally_id) = finally_expression {
                let finally_expr = tree.get(*finally_id);
                visitor.visit_expression(tree, *finally_id, finally_expr);
            }
        }

        Expression::Match {
            kind: _,
            value,
            cases,
        } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
            for case_id in cases {
                let case = tree.get(*case_id);
                visitor.visit_match_case(tree, *case_id, case);
            }
        }

        Expression::Break { label: _, value } => {
            if let Some(value_id) = value {
                let value_expr = tree.get(*value_id);
                visitor.visit_expression(tree, *value_id, value_expr);
            }
        }

        Expression::Continue { label: _ } => {}

        Expression::Await { expression } => {
            let expression_node = tree.get(*expression);
            visitor.visit_expression(tree, *expression, expression_node);
        }

        Expression::AwaitMaybe { expression } => {
            let expression_node = tree.get(*expression);
            visitor.visit_expression(tree, *expression, expression_node);
        }

        Expression::Yield {
            cardinality: _,
            value,
        } => {
            if let Some(value_id) = value {
                let value_node = tree.get(*value_id);
                visitor.visit_expression(tree, *value_id, value_node);
            }
        }

        Expression::Throw { value } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }

        Expression::Return { value } => {
            if let Some(value_id) = value {
                let value_expr = tree.get(*value_id);
                visitor.visit_expression(tree, *value_id, value_expr);
            }
        }

        Expression::Identifier { name: _ } => {}

        Expression::QualifiedReference {
            path: _,
            generic_arguments,
        } => {
            for argument_id in generic_arguments {
                let argument = tree.get(*argument_id);
                visitor.visit_generic_argument(tree, *argument_id, argument);
            }
        }

        Expression::PrivateIdentifier { name: _ } => {}

        Expression::This | Expression::Super | Expression::ImportMeta | Expression::NewTarget => {}

        Expression::ScalarLiteral(_) => {
            // no child nodes to visit
        }

        Expression::TemplateExpression { value } => match value {
            TemplateLiteral::String { .. } => {}
            TemplateLiteral::InterpolatedString { arguments, .. } => {
                for argument_id in arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_argument(tree, *argument_id, argument);
                }
            }
        },
        Expression::TaggedTemplateExpression {
            tag,
            generic_arguments,
            value,
        } => {
            let tag_expr = tree.get(*tag);
            visitor.visit_expression(tree, *tag, tag_expr);

            for argument_id in generic_arguments {
                let argument = tree.get(*argument_id);
                visitor.visit_generic_argument(tree, *argument_id, argument);
            }

            match value {
                TemplateLiteral::String { .. } => {}
                TemplateLiteral::InterpolatedString { arguments, .. } => {
                    for argument_id in arguments {
                        let argument = tree.get(*argument_id);
                        visitor.visit_argument(tree, *argument_id, argument);
                    }
                }
            }
        }

        Expression::ArrayExpression { elements } => {
            for argument_id in elements {
                let argument = tree.get(*argument_id);
                visitor.visit_argument(tree, *argument_id, argument);
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
            if let Some(type_id) = ty {
                let type_expr = tree.get(*type_id);
                visitor.visit_type_expression(tree, *type_id, type_expr);
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
                let left_expr = tree.get(*left_id);
                visitor.visit_expression(tree, *left_id, left_expr);
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

        Expression::Parenthesized { expression } => {
            let expr = tree.get(*expression);
            visitor.visit_expression(tree, *expression, expr);
        }

        Expression::Type { value } => {
            let value_node = tree.get(*value);
            visitor.visit_type_expression(tree, *value, value_node);
        }

        Expression::Comptime { body } => {
            let body_expr = tree.get(*body);
            visitor.visit_expression(tree, *body, body_expr);
        }

        Expression::As {
            expression,
            target_type,
        } => {
            let expression_node = tree.get(*expression);
            visitor.visit_expression(tree, *expression, expression_node);

            let target_type_node = tree.get(*target_type);
            visitor.visit_type_expression(tree, *target_type, target_type_node);
        }

        Expression::Satisfies {
            expression,
            target_type,
        } => {
            let expression_node = tree.get(*expression);
            visitor.visit_expression(tree, *expression, expression_node);

            let target_type_node = tree.get(*target_type);
            visitor.visit_type_expression(tree, *target_type, target_type_node);
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

        Expression::Unary { operator: _, right } => {
            let right_expr = tree.get(*right);
            visitor.visit_expression(tree, *right, right_expr);
        }

        Expression::ValueOf {
            mutability: _,
            variance: _,
            right,
        } => {
            let right_expr = tree.get(*right);
            visitor.visit_expression(tree, *right, right_expr);
        }

        Expression::ReferenceOf {
            mutability: _,
            variance: _,
            right,
        } => {
            let right_expr = tree.get(*right);
            visitor.visit_expression(tree, *right, right_expr);
        }

        Expression::PointerOf {
            mutability: _,
            right,
        } => {
            let right_expr = tree.get(*right);
            visitor.visit_expression(tree, *right, right_expr);
        }

        Expression::Member {
            left: receiver,
            name: _,
        } => {
            let receiver_expr = tree.get(*receiver);
            visitor.visit_expression(tree, *receiver, receiver_expr);
        }

        Expression::PrivateMember {
            left: receiver,
            name: _,
        } => {
            let receiver_expr = tree.get(*receiver);
            visitor.visit_expression(tree, *receiver, receiver_expr);
        }

        Expression::Index {
            position: _,
            left: receiver,
            index,
        } => {
            let receiver_expr = tree.get(*receiver);
            visitor.visit_expression(tree, *receiver, receiver_expr);
            if let Some(index_id) = index {
                let index_expr = tree.get(*index_id);
                visitor.visit_expression(tree, *index_id, index_expr);
            }
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
            position: _,
            left,
            generic_arguments,
            arguments,
        } => {
            let receiver_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, receiver_expr);
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
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }

        Expression::Maybe { position: _, left } => {
            let left_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expr);
        }

        Expression::Must { position: _, left } => {
            let left_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expr);
        }

        Expression::Binary {
            left,
            operator: _,
            right,
        } => {
            let left_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expr);
            let right_expr = tree.get(*right);
            visitor.visit_expression(tree, *right, right_expr);
        }

        Expression::Assign {
            left,
            operator: _,
            right,
        } => {
            let left_pattern = tree.get(*left);
            visitor.visit_assign_pattern(tree, *left, left_pattern);
            let right_expr = tree.get(*right);
            visitor.visit_expression(tree, *right, right_expr);
        }

        Expression::Debugger => {}
        Expression::Missing => {}
        Expression::Stub => {}
        Expression::Error => {}
    }
}

/// Walk the FunctionSignature.
pub fn walk_generic_parameter<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<GenericParameter>,
    generic_parameter: &GenericParameter,
) {
    visitor.visit_any(tree, NodeType::GenericParameter, id.id);

    match generic_parameter {
        GenericParameter::Type {
            name: _,
            variance: _,
            constraint,
            default,
        } => {
            if let Some(constraint) = constraint {
                let constraint_expression = tree.get(*constraint);
                visitor.visit_type_expression(tree, *constraint, constraint_expression);
            }

            if let Some(default) = default {
                let default_expression = tree.get(*default);
                visitor.visit_type_expression(tree, *default, default_expression);
            }
        }
        GenericParameter::Value {
            name: _,
            declared_type,
            default,
            is_comptime: _,
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
        GenericParameter::Error => {}
    }
}

/// Walk the FunctionSignature.
fn walk_function_signature<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    signature: &FunctionSignature,
) {
    for parameter_id in signature.generic_parameters.iter() {
        let parameter = tree.get(*parameter_id);
        visitor.visit_generic_parameter(tree, *parameter_id, parameter);
    }

    for clause_id in signature.where_clauses.iter() {
        let clause = tree.get(*clause_id);
        visitor.visit_where_clause(tree, *clause_id, clause);
    }

    if let Some(this_parameter_id) = signature.this_parameter {
        let this_parameter = tree.get(this_parameter_id);
        visitor.visit_parameter(tree, this_parameter_id, this_parameter);
    }

    for parameter_id in signature.parameters.iter() {
        let parameter = tree.get(*parameter_id);
        visitor.visit_parameter(tree, *parameter_id, parameter);
    }

    if let Some(return_type) = signature.return_type {
        let return_type_expression = tree.get(return_type);
        visitor.visit_type_expression(tree, return_type, return_type_expression);
    }
}

/// Walk the Declaration and visit all child nodes.
pub fn walk_declaration<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Declaration>,
    declaration: &Declaration,
) {
    visitor.visit_any(tree, NodeType::Declaration, id.id);

    match declaration {
        Declaration::Global(declaration) => {
            for expression_id in &declaration.expressions {
                let expression = tree.get(*expression_id);
                visitor.visit_expression(tree, *expression_id, expression);
            }
        }
        Declaration::Namespace(declaration) => {
            for parameter_id in &declaration.generic_parameters {
                let parameter = tree.get(*parameter_id);
                visitor.visit_generic_parameter(tree, *parameter_id, parameter);
            }
            for where_clause_id in &declaration.where_clauses {
                let where_clause = tree.get(*where_clause_id);
                visitor.visit_where_clause(tree, *where_clause_id, where_clause);
            }
            for statement_id in &declaration.expressions {
                let statement = tree.get(*statement_id);
                visitor.visit_expression(tree, *statement_id, statement);
            }
        }
        Declaration::Type(declaration) => {
            for parameter_id in &declaration.generic_parameters {
                let parameter = tree.get(*parameter_id);
                visitor.visit_generic_parameter(tree, *parameter_id, parameter);
            }
            for where_clause_id in &declaration.where_clauses {
                let where_clause = tree.get(*where_clause_id);
                visitor.visit_where_clause(tree, *where_clause_id, where_clause);
            }
            let value_expr = tree.get(declaration.value);
            visitor.visit_type_expression(tree, declaration.value, value_expr);
        }
        Declaration::ImportAlias(declaration) => {
            if let ImportAliasTarget::Path { path: _ } = &declaration.target {
                // no child nodes
            }
        }
        Declaration::Struct(declaration) => {
            for parameter_id in &declaration.generic_parameters {
                let parameter = tree.get(*parameter_id);
                visitor.visit_generic_parameter(tree, *parameter_id, parameter);
            }
            for where_clause_id in &declaration.where_clauses {
                let where_clause = tree.get(*where_clause_id);
                visitor.visit_where_clause(tree, *where_clause_id, where_clause);
            }
            for expression_id in &declaration.implements_types {
                let expression = tree.get(*expression_id);
                visitor.visit_type_expression(tree, *expression_id, expression);
            }
            for expression_id in &declaration.embedded_types {
                let expression = tree.get(*expression_id);
                visitor.visit_type_expression(tree, *expression_id, expression);
            }
            for member_id in &declaration.members {
                let member = tree.get(*member_id);
                visitor.visit_member(tree, *member_id, member);
            }
        }
        Declaration::Class(declaration) => {
            for parameter_id in &declaration.generic_parameters {
                let parameter = tree.get(*parameter_id);
                visitor.visit_generic_parameter(tree, *parameter_id, parameter);
            }
            for where_clause_id in &declaration.where_clauses {
                let where_clause = tree.get(*where_clause_id);
                visitor.visit_where_clause(tree, *where_clause_id, where_clause);
            }
            if let Some(extends_expression_id) = declaration.extends_expression {
                let extends_expression = tree.get(extends_expression_id);
                visitor.visit_expression(tree, extends_expression_id, extends_expression);
            }
            for expression_id in &declaration.implements_types {
                let expression = tree.get(*expression_id);
                visitor.visit_type_expression(tree, *expression_id, expression);
            }
            for member_id in &declaration.members {
                let member = tree.get(*member_id);
                visitor.visit_member(tree, *member_id, member);
            }
        }
        Declaration::Enum(declaration) => {
            for parameter_id in &declaration.generic_parameters {
                let parameter = tree.get(*parameter_id);
                visitor.visit_generic_parameter(tree, *parameter_id, parameter);
            }
            for where_clause_id in &declaration.where_clauses {
                let where_clause = tree.get(*where_clause_id);
                visitor.visit_where_clause(tree, *where_clause_id, where_clause);
            }
            for expression_id in &declaration.implements_types {
                let expression = tree.get(*expression_id);
                visitor.visit_type_expression(tree, *expression_id, expression);
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
            for parameter_id in &declaration.generic_parameters {
                let parameter = tree.get(*parameter_id);
                visitor.visit_generic_parameter(tree, *parameter_id, parameter);
            }
            for where_clause_id in &declaration.where_clauses {
                let where_clause = tree.get(*where_clause_id);
                visitor.visit_where_clause(tree, *where_clause_id, where_clause);
            }
            for expression_id in &declaration.extends_types {
                let expression = tree.get(*expression_id);
                visitor.visit_type_expression(tree, *expression_id, expression);
            }
            for member_id in &declaration.members {
                let member = tree.get(*member_id);
                visitor.visit_type_member(tree, *member_id, member);
            }
        }
        Declaration::Extension(declaration) => {
            for parameter_id in &declaration.generic_parameters {
                let parameter = tree.get(*parameter_id);
                visitor.visit_generic_parameter(tree, *parameter_id, parameter);
            }
            for where_clause_id in &declaration.where_clauses {
                let where_clause = tree.get(*where_clause_id);
                visitor.visit_where_clause(tree, *where_clause_id, where_clause);
            }
            let target_type_expression = tree.get(declaration.target_type);
            visitor.visit_type_expression(tree, declaration.target_type, target_type_expression);
            for expression_id in &declaration.implements_types {
                let expression = tree.get(*expression_id);
                visitor.visit_type_expression(tree, *expression_id, expression);
            }
            for member_id in &declaration.members {
                let member = tree.get(*member_id);
                visitor.visit_member(tree, *member_id, member);
            }
        }
        Declaration::Function(declaration) => {
            walk_function_signature(visitor, tree, &declaration.signature);
            if let Some(body_id) = declaration.body {
                let expression = tree.get(body_id);
                visitor.visit_expression(tree, body_id, expression);
            }
        }
    }
}

/// Walk a Key.
pub fn walk_key<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, key: &Key) {
    match key {
        Key::Name(_name) => {
            // nothing to do
        }
        Key::Private(_name) => {
            // nothing to do
        }
        Key::Expression(dynamic_key) => {
            let dynamic_key_expr = tree.get(*dynamic_key);
            visitor.visit_expression(tree, *dynamic_key, dynamic_key_expr);
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
        Property::Field { key, value } => {
            walk_key(visitor, tree, key);

            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }
        Property::Method {
            key,
            signature,
            body,
        } => {
            if let Some(key) = key {
                walk_key(visitor, tree, key);
            }
            walk_function_signature(visitor, tree, signature);

            if let Some(body) = body {
                let expression = tree.get(*body);
                visitor.visit_expression(tree, *body, expression);
            }
        }
        Property::Spread { value } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }
        Property::Error => {}
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
        } => {
            for parameter_id in generic_parameters {
                let parameter = tree.get(*parameter_id);
                visitor.visit_generic_parameter(tree, *parameter_id, parameter);
            }
            for where_clause_id in where_clauses {
                let where_clause = tree.get(*where_clause_id);
                visitor.visit_where_clause(tree, *where_clause_id, where_clause);
            }
            if let Some(constraint) = constraint {
                let constraint_expression = tree.get(*constraint);
                visitor.visit_type_expression(tree, *constraint, constraint_expression);
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
        } => {
            if let Some(declared_type) = declared_type {
                let declared_type_expression = tree.get(*declared_type);
                visitor.visit_type_expression(tree, *declared_type, declared_type_expression);
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
        } => {
            walk_key(visitor, tree, key);
            if let Some(declared_type) = declared_type {
                let declared_type_expression = tree.get(*declared_type);
                visitor.visit_type_expression(tree, *declared_type, declared_type_expression);
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
        } => {
            if let Some(key) = key {
                walk_key(visitor, tree, key);
            }
            walk_function_signature(visitor, tree, signature);
            if let Some(body_id) = body {
                let expression = tree.get(*body_id);
                visitor.visit_expression(tree, *body_id, expression);
            }
        }
        Member::Embed { value, .. } => {
            let value_expr = tree.get(*value);
            visitor.visit_type_expression(tree, *value, value_expr);
        }
        Member::StaticBlock { body, .. } => {
            let body_expr = tree.get(*body);
            visitor.visit_expression(tree, *body, body_expr);
        }
        Member::ComptimeBlock { body, .. } => {
            let body_expr = tree.get(*body);
            visitor.visit_expression(tree, *body, body_expr);
        }
        Member::Error => {}
    }
}

/// Walk the EnumField.
pub fn walk_enum_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<EnumField>,
    field: &EnumField,
) {
    visitor.visit_any(tree, NodeType::EnumField, id.id);
    if let Some(value) = &field.value {
        let expression = tree.get(*value);
        visitor.visit_expression(tree, *value, expression);
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

/// Walk the UseClause.
/// Walk the DependencyItem.
pub fn walk_dependency_item<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<DependencyItem>,
    dependency_item: &DependencyItem,
) {
    visitor.visit_any(tree, NodeType::DependencyItem, id.id);
    if let DependencyItem::Item {
        value: Some(value), ..
    } = dependency_item
    {
        let value_expr = tree.get(*value);
        visitor.visit_expression(tree, *value, value_expr);
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
        } => {
            if let Some(declared_type) = declared_type {
                let type_node = tree.get(*declared_type);
                visitor.visit_type_expression(tree, *declared_type, type_node);
            }
            if let Some(default) = default {
                let default_expr = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expr);
            }
        }
        Parameter::Pattern {
            pattern,
            is_optional: _,
            declared_type,
            default,
        } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, pattern_node);
            if let Some(declared_type) = declared_type {
                let type_node = tree.get(*declared_type);
                visitor.visit_type_expression(tree, *declared_type, type_node);
            }
            if let Some(default) = default {
                let default_expr = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expr);
            }
        }
        Parameter::VariadicNamed {
            name: _,
            visibility: _,
            is_readonly: _,
            declared_type,
        } => {
            if let Some(declared_type) = declared_type {
                let type_node = tree.get(*declared_type);
                visitor.visit_type_expression(tree, *declared_type, type_node);
            }
        }
        Parameter::VariadicPattern {
            pattern,
            declared_type,
        } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, pattern_node);
            if let Some(declared_type) = declared_type {
                let type_node = tree.get(*declared_type);
                visitor.visit_type_expression(tree, *declared_type, type_node);
            }
        }
        Parameter::Error => {}
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
        Argument::Named { name: _, value }
        | Argument::Labeled { label: _, value }
        | Argument::Positional { value }
        | Argument::Spread { label: _, value } => {
            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
        }
        Argument::Error => {}
    }
}

// ----------------------------------------------------------------------------
// Patterns
// ----------------------------------------------------------------------------

/// Walk the Pattern.
pub fn walk_pattern<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Pattern>,
    pattern: &Pattern,
) {
    visitor.visit_any(tree, NodeType::Pattern, id.id);
    match pattern {
        Pattern::Wildcard => {
            // no child nodes to visit
        }
        Pattern::Must(unwrap) => {
            let unwrap_pattern = tree.get(*unwrap);
            visitor.visit_pattern(tree, *unwrap, unwrap_pattern);
        }
        Pattern::Assign { pattern, value } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, pattern_node);

            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
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
            let type_node = tree.get(*ty);
            visitor.visit_type_expression(tree, *ty, type_node);
            for field_id in fields {
                let field = tree.get(*field_id);
                visitor.visit_pattern_field(tree, *field_id, field);
            }
        }
        Pattern::Union { patterns } => {
            for pattern_id in patterns {
                let pattern = tree.get(*pattern_id);
                visitor.visit_pattern(tree, *pattern_id, pattern);
            }
        }
    }
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
            is_shorthand: _,
            pattern,
        } => {
            if let Some(pattern_id) = pattern {
                let pattern_node = tree.get(*pattern_id);
                visitor.visit_pattern(tree, *pattern_id, pattern_node);
            }
        }
        PatternField::Computed {
            mutability: _,
            key,
            pattern,
        } => {
            let key_expr = tree.get(*key);
            visitor.visit_expression(tree, *key, key_expr);
            let pattern_node = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, pattern_node);
        }
        PatternField::Positional { pattern } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, pattern_node);
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
            // nothing to visit
        }
    }
}

/// Walk the AssignPattern.
pub fn walk_assign_pattern<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<AssignPattern>,
    assign_pattern: &AssignPattern,
) {
    visitor.visit_any(tree, NodeType::AssignPattern, id.id);

    match assign_pattern {
        AssignPattern::Expression { value } => {
            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
        }
        AssignPattern::Assign { pattern, value } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_assign_pattern(tree, *pattern, pattern_node);

            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
        }
        AssignPattern::Array { fields } | AssignPattern::Object { fields } => {
            for field_id in fields {
                let field = tree.get(*field_id);
                visitor.visit_assign_pattern_field(tree, *field_id, field);
            }
        }
    }
}

/// Walk the AssignPatternField.
pub fn walk_assign_pattern_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<AssignPatternField>,
    assign_pattern_field: &AssignPatternField,
) {
    visitor.visit_any(tree, NodeType::AssignPatternField, id.id);

    match assign_pattern_field {
        AssignPatternField::Named {
            name: _,
            is_shorthand: _,
            pattern,
        } => {
            if let Some(pattern_id) = pattern {
                let pattern_node = tree.get(*pattern_id);
                visitor.visit_assign_pattern(tree, *pattern_id, pattern_node);
            }
        }
        AssignPatternField::Computed { key, pattern } => {
            let key_expression = tree.get(*key);
            visitor.visit_expression(tree, *key, key_expression);

            let pattern_node = tree.get(*pattern);
            visitor.visit_assign_pattern(tree, *pattern, pattern_node);
        }
        AssignPatternField::Positional { pattern } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_assign_pattern(tree, *pattern, pattern_node);
        }
        AssignPatternField::Spread { pattern } => {
            if let Some(pattern_id) = pattern {
                let pattern_node = tree.get(*pattern_id);
                visitor.visit_assign_pattern(tree, *pattern_id, pattern_node);
            }
        }
        AssignPatternField::Elision => {
            // no child nodes to visit
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
            if let Some(guard_expr) = guard {
                let guard_node = tree.get(*guard_expr);
                visitor.visit_expression(tree, *guard_expr, guard_node);
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
        MatchCase::Expression { selector, body } => {
            walk_match_selector(visitor, tree, selector);
            let body_expr = tree.get(*body);
            visitor.visit_expression(tree, *body, body_expr);
        }
        MatchCase::Block { selector, body } => {
            walk_match_selector(visitor, tree, selector);
            let body_block = tree.get(*body);
            visitor.visit_block(tree, *body, body_block);
        }
    }
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
    if let Some(type_id) = ty {
        let type_expr = tree.get(*type_id);
        visitor.visit_type_expression(tree, *type_id, type_expr);
    }
    if let Some(value_id) = value {
        let value_expr = tree.get(*value_id);
        visitor.visit_expression(tree, *value_id, value_expr);
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
    let expression = tree.get(decorator.expression);
    visitor.visit_expression(tree, decorator.expression, expression);
}
