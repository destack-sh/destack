use crate::{
    Annotation, Argument, Block, Declaration, DeclarationDescriptor, Declarator, Decorator,
    DependencyItem, EnumField, Expression, ForEachBinding, FunctionSignature, Generics, Heritage,
    IfCondition, ImportAliasTarget, ImportTarget, Key, LocalNodeId, LocalNodeIdAny, MatchCase,
    MatchSelector, Member, NodeTree, NodeType, NodeVisitor, Parameter, Pattern, PatternField,
    Property, TemplateLiteral, WhereClause,
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
        NodeType::MatchCase => {
            let match_case = tree.match_cases.get(local_idx);
            walk_match_case(visitor, tree, LocalNodeId::new(node_id), match_case);
        }
        NodeType::Declarator => {
            let declarator = tree.declarators.get(local_idx);
            walk_declarator(visitor, tree, LocalNodeId::new(node_id), declarator);
        }
        // --------------------------------------------------------------------
        // Annotations
        // --------------------------------------------------------------------
        NodeType::Annotation => {
            let annotation = tree.annotations.get(local_idx);
            walk_annotation(visitor, tree, LocalNodeId::new(node_id), annotation);
        }
        NodeType::Decorator => {
            let decorator = tree.decorators.get(local_idx);
            walk_decorator(visitor, tree, LocalNodeId::new(node_id), decorator);
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
        NodeType::Declarator => {
            let declarator_id = LocalNodeId::<Declarator>::new(root.id);
            let declarator = tree.get(declarator_id);
            visitor.visit_declarator(tree, declarator_id, declarator);
        }
        NodeType::Annotation => {
            let annotation_id = LocalNodeId::<Annotation>::new(root.id);
            let annotation = tree.get(annotation_id);
            visitor.visit_annotation(tree, annotation_id, annotation);
        }
        NodeType::Decorator => {
            let decorator_id = LocalNodeId::<Decorator>::new(root.id);
            let decorator = tree.get(decorator_id);
            visitor.visit_decorator(tree, decorator_id, decorator);
        }
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
            descriptor: _,
            declarators,
        } => {
            for declarator_id in declarators {
                let declarator = tree.get(*declarator_id);
                visitor.visit_declarator(tree, *declarator_id, declarator);
            }
        }
        Expression::Using {
            asynchrony: _,
            descriptor: _,
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
                visitor.visit_expression(tree, *catch_ty_id, catch_ty_node);
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
            static_arguments,
        } => {
            if let Some(arguments) = static_arguments {
                for argument_id in arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_argument(tree, *argument_id, argument);
                }
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
        Expression::TaggedTemplateExpression { tag, value } => {
            let tag_expr = tree.get(*tag);
            visitor.visit_expression(tree, *tag, tag_expr);
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

        Expression::TypeLiteral(_) => {
            // no child nodes to visit
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
                visitor.visit_expression(tree, *type_id, type_expr);
            }
            for property_id in properties {
                let property = tree.get(*property_id);
                visitor.visit_property(tree, *property_id, property);
            }
        }

        Expression::TreeExpression {
            left,
            arguments,
            elements,
        } => {
            if let Some(left_id) = left {
                let left_expr = tree.get(*left_id);
                visitor.visit_expression(tree, *left_id, left_expr);
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

        Expression::Comptime { body } => {
            let body_expr = tree.get(*body);
            visitor.visit_expression(tree, *body, body_expr);
        }

        Expression::Unary { operator: _, right } => {
            let right_expr = tree.get(*right);
            visitor.visit_expression(tree, *right, right_expr);
        }

        Expression::TypeUnary { operator: _, right } => {
            let right_expr = tree.get(*right);
            visitor.visit_expression(tree, *right, right_expr);
        }

        Expression::As {
            expression,
            type_annotation,
        } => {
            let expression_node = tree.get(*expression);
            visitor.visit_expression(tree, *expression, expression_node);

            let type_annotation_node = tree.get(*type_annotation);
            visitor.visit_expression(tree, *type_annotation, type_annotation_node);
        }

        Expression::Satisfies {
            expression,
            type_annotation,
        } => {
            let expression_node = tree.get(*expression);
            visitor.visit_expression(tree, *expression, expression_node);

            let type_annotation_node = tree.get(*type_annotation);
            visitor.visit_expression(tree, *type_annotation, type_annotation_node);
        }

        Expression::TypeAssertion {
            type_annotation,
            expression,
        } => {
            let type_annotation_node = tree.get(*type_annotation);
            visitor.visit_expression(tree, *type_annotation, type_annotation_node);

            let expression_node = tree.get(*expression);
            visitor.visit_expression(tree, *expression, expression_node);
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
            static_arguments,
        } => {
            let receiver_expr = tree.get(*receiver);
            visitor.visit_expression(tree, *receiver, receiver_expr);
            if let Some(static_arguments) = static_arguments {
                for argument_id in static_arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_argument(tree, *argument_id, argument);
                }
            }
        }

        Expression::PrivateMember {
            left: receiver,
            name: _,
            static_arguments,
        } => {
            let receiver_expr = tree.get(*receiver);
            visitor.visit_expression(tree, *receiver, receiver_expr);
            if let Some(static_arguments) = static_arguments {
                for argument_id in static_arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_argument(tree, *argument_id, argument);
                }
            }
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
            static_arguments,
        } => {
            let left_expression = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expression);
            for argument_id in static_arguments {
                let argument = tree.get(*argument_id);
                visitor.visit_argument(tree, *argument_id, argument);
            }
        }

        Expression::Call {
            position: _,
            left,
            static_arguments,
            dynamic_arguments,
        } => {
            let receiver_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, receiver_expr);
            if let Some(static_arguments) = static_arguments {
                for argument_id in static_arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_argument(tree, *argument_id, argument);
                }
            }
            for argument_id in dynamic_arguments {
                let argument = tree.get(*argument_id);
                visitor.visit_argument(tree, *argument_id, argument);
            }
        }

        Expression::New {
            left: _,
            static_arguments,
            dynamic_arguments,
        } => {
            if let Some(static_arguments) = static_arguments {
                for argument_id in static_arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_argument(tree, *argument_id, argument);
                }
            }
            for argument_id in dynamic_arguments {
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

        Expression::TypeBinary {
            left,
            operator: _,
            right,
        } => {
            let left_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expr);
            let right_expr = tree.get(*right);
            visitor.visit_expression(tree, *right, right_expr);
        }

        Expression::TypeConditional {
            left,
            right,
            then_type,
            else_type,
        } => {
            let left_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expr);
            let right_expr = tree.get(*right);
            visitor.visit_expression(tree, *right, right_expr);
            let then_expr = tree.get(*then_type);
            visitor.visit_expression(tree, *then_type, then_expr);
            let else_expr = tree.get(*else_type);
            visitor.visit_expression(tree, *else_type, else_expr);
        }

        Expression::TypeMapped {
            parameter,
            modifiers: _,
            value,
        } => {
            let constraint_expr = tree.get(parameter.constraint);
            visitor.visit_expression(tree, parameter.constraint, constraint_expr);
            if let Some(key_remap) = parameter.key_remap {
                let key_expr = tree.get(key_remap);
                visitor.visit_expression(tree, key_remap, key_expr);
            }
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }

        Expression::TypeIndex { left, index } => {
            let left_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expr);
            let index_expr = tree.get(*index);
            visitor.visit_expression(tree, *index, index_expr);
        }

        Expression::TypeTemplateLiteral { strings: _, spans } => {
            for span_id in spans {
                let span_expr = tree.get(*span_id);
                visitor.visit_expression(tree, *span_id, span_expr);
            }
        }

        Expression::TypeImport {
            target: _,
            arguments,
            qualifier: _,
            static_arguments,
        } => {
            for argument_id in arguments {
                let argument = tree.get(*argument_id);
                visitor.visit_argument(tree, *argument_id, argument);
            }
            if let Some(static_arguments) = static_arguments {
                for argument_id in static_arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_argument(tree, *argument_id, argument);
                }
            }
        }

        Expression::TypeInfer {
            name: _,
            constraint,
        } => {
            if let Some(constraint_id) = constraint {
                let constraint_expr = tree.get(*constraint_id);
                visitor.visit_expression(tree, *constraint_id, constraint_expr);
            }
        }

        Expression::TypePredicate {
            asserts: _,
            subject: _,
            target,
        } => {
            if let Some(target_id) = target {
                let target_expr = tree.get(*target_id);
                visitor.visit_expression(tree, *target_id, target_expr);
            }
        }

        Expression::Assign {
            left,
            operator: _,
            right,
        } => {
            let left_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expr);
            let right_expr = tree.get(*right);
            visitor.visit_expression(tree, *right, right_expr);
        }

        Expression::Debugger => {}
        Expression::Missing => {}
        Expression::Stub => {}
        Expression::Error => {}
    }
}

/// Walk the DeclarationDescriptor.
fn walk_declaration_descriptor<V: NodeVisitor + ?Sized>(
    _visitor: &mut V,
    _tree: &NodeTree,
    _descriptor: &DeclarationDescriptor,
) {
    // nothing to do
}

/// Walk the Generics.
fn walk_generics<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, generics: &Generics) {
    if let Some(static_parameters) = generics.static_parameters.as_ref() {
        for parameter_id in static_parameters.iter() {
            let parameter = tree.get(*parameter_id);
            visitor.visit_parameter(tree, *parameter_id, parameter);
        }
    }
    if let Some(where_clauses) = generics.where_clauses.as_ref() {
        for clause_id in where_clauses.iter() {
            let clause = tree.get(*clause_id);
            visitor.visit_where_clause(tree, *clause_id, clause);
        }
    }
}

/// Walk the Heritage.
fn walk_heritage<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, heritage: &Heritage) {
    if let Some(extends_expressions) = heritage.extends_types.as_ref() {
        for extends_expression_id in extends_expressions.iter() {
            let extends_expression = tree.get(*extends_expression_id);
            visitor.visit_expression(tree, *extends_expression_id, extends_expression);
        }
    }
    if let Some(implements_expressions) = heritage.implements_types.as_ref() {
        for implements_expression_id in implements_expressions.iter() {
            let implements_expression = tree.get(*implements_expression_id);
            visitor.visit_expression(tree, *implements_expression_id, implements_expression);
        }
    }
}

/// Walk the FunctionSignature.
fn walk_function_signature<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    signature: &FunctionSignature,
) {
    if let Some(generics) = signature.generics.as_ref() {
        walk_generics(visitor, tree, generics);
    }
    if let Some(this_parameter_id) = signature.this_parameter {
        let this_parameter = tree.get(this_parameter_id);
        visitor.visit_parameter(tree, this_parameter_id, this_parameter);
    }
    for parameter_id in signature.dynamic_parameters.iter() {
        let parameter = tree.get(*parameter_id);
        visitor.visit_parameter(tree, *parameter_id, parameter);
    }
    if let Some(return_type) = signature.return_type {
        let return_type_expression = tree.get(return_type);
        visitor.visit_expression(tree, return_type, return_type_expression);
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
        Declaration::Global {
            descriptor,
            expressions,
        } => {
            walk_declaration_descriptor(visitor, tree, descriptor);
            for expression_id in expressions {
                let expression = tree.get(*expression_id);
                visitor.visit_expression(tree, *expression_id, expression);
            }
        }
        Declaration::Namespace {
            descriptor,
            kind: _,
            generics,
            expressions: statements,
        } => {
            walk_declaration_descriptor(visitor, tree, descriptor);
            walk_generics(visitor, tree, generics);
            for statement_id in statements {
                let statement = tree.get(*statement_id);
                visitor.visit_expression(tree, *statement_id, statement);
            }
        }
        Declaration::Type {
            descriptor,
            kind: _,
            mutability: _,
            static_parameters,
            value,
        } => {
            walk_declaration_descriptor(visitor, tree, descriptor);
            if let Some(static_parameters) = static_parameters {
                for parameter_id in static_parameters {
                    let parameter = tree.get(*parameter_id);
                    visitor.visit_parameter(tree, *parameter_id, parameter);
                }
            }
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }
        Declaration::ImportAlias {
            descriptor,
            kind: _,
            target,
        } => {
            walk_declaration_descriptor(visitor, tree, descriptor);
            if let ImportAliasTarget::Path { value } = target {
                let value_expr = tree.get(*value);
                visitor.visit_expression(tree, *value, value_expr);
            }
        }
        Declaration::Struct {
            descriptor,
            generics,
            heritage,
            members,
        } => {
            walk_declaration_descriptor(visitor, tree, descriptor);
            walk_generics(visitor, tree, generics);
            walk_heritage(visitor, tree, heritage);
            for member_id in members {
                let member = tree.get(*member_id);
                visitor.visit_member(tree, *member_id, member);
            }
        }
        Declaration::Class {
            descriptor,
            generics,
            heritage,
            members,
        } => {
            walk_declaration_descriptor(visitor, tree, descriptor);
            walk_generics(visitor, tree, generics);
            walk_heritage(visitor, tree, heritage);
            for member_id in members {
                let member = tree.get(*member_id);
                visitor.visit_member(tree, *member_id, member);
            }
        }
        Declaration::Enum {
            descriptor,
            kind: _,
            generics,
            heritage,
            fields,
            members,
        } => {
            walk_declaration_descriptor(visitor, tree, descriptor);
            walk_generics(visitor, tree, generics);
            walk_heritage(visitor, tree, heritage);
            for field_id in fields {
                let field = tree.get(*field_id);
                visitor.visit_enum_field(tree, *field_id, field);
            }
            for member_id in members {
                let member = tree.get(*member_id);
                visitor.visit_member(tree, *member_id, member);
            }
        }
        Declaration::Interface {
            descriptor,
            kind: _,
            generics,
            heritage,
            members,
        } => {
            walk_declaration_descriptor(visitor, tree, descriptor);
            walk_generics(visitor, tree, generics);
            walk_heritage(visitor, tree, heritage);
            for member_id in members {
                let member = tree.get(*member_id);
                visitor.visit_member(tree, *member_id, member);
            }
        }
        Declaration::Extension {
            descriptor,
            generics,
            target_type,
            heritage,
            members,
        } => {
            walk_declaration_descriptor(visitor, tree, descriptor);
            walk_generics(visitor, tree, generics);
            let target_type_expression = tree.get(*target_type);
            visitor.visit_expression(tree, *target_type, target_type_expression);
            walk_heritage(visitor, tree, heritage);
            for member_id in members {
                let member = tree.get(*member_id);
                visitor.visit_member(tree, *member_id, member);
            }
        }
        Declaration::Function {
            descriptor,
            signature,
            body,
        } => {
            walk_declaration_descriptor(visitor, tree, descriptor);
            walk_function_signature(visitor, tree, signature);
            if let Some(body_id) = body {
                let expression = tree.get(*body_id);
                visitor.visit_expression(tree, *body_id, expression);
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
        Key::NamedExpression { name: _, key } => {
            let key_expr = tree.get(*key);
            visitor.visit_expression(tree, *key, key_expr);
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
            modifiers: _,
            key,
            value,
            default,
        } => {
            if let Some(key) = key {
                walk_key(visitor, tree, key);
            }
            if let Some(value) = value {
                let value_expr = tree.get(*value);
                visitor.visit_expression(tree, *value, value_expr);
            }
            if let Some(default) = default {
                let default_expr = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expr);
            }
        }
        Property::Method {
            modifiers: _,
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
        Property::Spread {
            modifiers: _,
            value,
        } => {
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
        Member::Type {
            modifiers: _,
            name: _,
            static_parameters,
            where_clauses,
            ty,
            value,
        } => {
            if let Some(static_parameters) = static_parameters {
                for parameter_id in static_parameters {
                    let parameter = tree.get(*parameter_id);
                    visitor.visit_parameter(tree, *parameter_id, parameter);
                }
            }
            if let Some(where_clauses) = where_clauses {
                for where_clause_id in where_clauses {
                    let where_clause = tree.get(*where_clause_id);
                    visitor.visit_where_clause(tree, *where_clause_id, where_clause);
                }
            }
            if let Some(ty) = ty {
                let ty_expr = tree.get(*ty);
                visitor.visit_expression(tree, *ty, ty_expr);
            }
            if let Some(value) = value {
                let value_expr = tree.get(*value);
                visitor.visit_expression(tree, *value, value_expr);
            }
        }
        Member::ComptimeConst {
            modifiers: _,
            name: _,
            ty,
            value,
        } => {
            if let Some(ty) = ty {
                let ty_expr = tree.get(*ty);
                visitor.visit_expression(tree, *ty, ty_expr);
            }
            if let Some(value) = value {
                let value_expr = tree.get(*value);
                visitor.visit_expression(tree, *value, value_expr);
            }
        }
        Member::Field {
            modifiers: _,
            key,
            value,
            default,
        } => {
            if let Some(key) = key {
                walk_key(visitor, tree, key);
            }
            if let Some(value) = value {
                let value_expr = tree.get(*value);
                visitor.visit_expression(tree, *value, value_expr);
            }
            if let Some(default) = default {
                let default_expr = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expr);
            }
        }
        Member::Method {
            modifiers: _,
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
        Member::Embed {
            modifiers: _,
            value,
        } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }
        Member::StaticBlock { modifiers: _, body } => {
            let body_expr = tree.get(*body);
            visitor.visit_expression(tree, *body, body_expr);
        }
        Member::ComptimeBlock { modifiers: _, body } => {
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
    visitor.visit_expression(tree, where_clause.right, right_expression);
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
            modifiers: _,
            name: _,
            ty,
            default,
        } => {
            if let Some(ty) = ty {
                let type_node = tree.get(*ty);
                visitor.visit_expression(tree, *ty, type_node);
            }
            if let Some(default) = default {
                let default_expr = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expr);
            }
        }
        Parameter::Pattern {
            modifiers: _,
            pattern,
            ty,
            default,
        } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, pattern_node);
            if let Some(ty) = ty {
                let type_node = tree.get(*ty);
                visitor.visit_expression(tree, *ty, type_node);
            }
            if let Some(default) = default {
                let default_expr = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expr);
            }
        }
        Parameter::VariadicNamed {
            modifiers: _,
            name: _,
            ty,
        } => {
            if let Some(ty) = ty {
                let type_node = tree.get(*ty);
                visitor.visit_expression(tree, *ty, type_node);
            }
        }
        Parameter::VariadicPattern {
            modifiers: _,
            pattern,
            ty,
        } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, pattern_node);
            if let Some(ty) = ty {
                let type_node = tree.get(*ty);
                visitor.visit_expression(tree, *ty, type_node);
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
        Argument::Named {
            modifiers: _,
            name: _,
            value,
        }
        | Argument::Labeled {
            modifiers: _,
            label: _,
            value,
        }
        | Argument::Positional {
            modifiers: _,
            value,
        }
        | Argument::Spread {
            modifiers: _,
            label: _,
            value,
        } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
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
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }
        Pattern::Tuple { fields } => {
            for field_id in fields {
                let field = tree.get(*field_id);
                visitor.visit_pattern_field(tree, *field_id, field);
            }
        }
        Pattern::TaggedTuple { ty, fields } => {
            let ty_expression = tree.get(*ty);
            visitor.visit_expression(tree, *ty, ty_expression);
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
            visitor.visit_expression(tree, *ty, type_node);
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
            pattern,
            default,
        } => {
            if let Some(pattern_id) = pattern {
                let pattern_node = tree.get(*pattern_id);
                visitor.visit_pattern(tree, *pattern_id, pattern_node);
            }
            if let Some(default) = default {
                let default_expr = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expr);
            }
        }
        PatternField::Computed {
            mutability: _,
            key,
            pattern,
            default,
        } => {
            let key_expr = tree.get(*key);
            visitor.visit_expression(tree, *key, key_expr);
            if let Some(pattern_id) = pattern {
                let pattern_node = tree.get(*pattern_id);
                visitor.visit_pattern(tree, *pattern_id, pattern_node);
            }
            if let Some(default) = default {
                let default_expr = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expr);
            }
        }
        PatternField::Alias {
            mutability: _,
            name: _,
            alias: _,
            default,
        } => {
            if let Some(default) = default {
                let default_expr = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expr);
            }
        }
        PatternField::Positional { pattern, default } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, pattern_node);
            if let Some(default) = default {
                let default_expr = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expr);
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
            // nothing to visit
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
        visitor.visit_expression(tree, *type_id, type_expr);
    }
    if let Some(value_id) = value {
        let value_expr = tree.get(*value_id);
        visitor.visit_expression(tree, *value_id, value_expr);
    }
}

// ----------------------------------------------------------------------------
// Annotations
// ----------------------------------------------------------------------------

/// Walk the Annotation.
pub fn walk_annotation<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Annotation>,
    annotation: &Annotation,
) {
    visitor.visit_any(tree, NodeType::Annotation, id.id);
    match annotation {
        Annotation::Decorator { node, position: _ } => {
            visitor.visit_decorator(tree, *node, tree.get(*node));
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
    let expression = tree.get(decorator.expression);
    visitor.visit_expression(tree, decorator.expression, expression);
}
