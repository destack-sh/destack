use crate::{
    Annotation, Argument, ArrayElement, AssignPattern, AssignPatternField, Block, CatchClause,
    ClassDeclaration, Declaration, Declarator, DependencyItem, EnumDeclaration, EnumField,
    Expression, FunctionDeclaration, FunctionSignature, GenericParameter, GlobalDeclaration,
    InterfaceDeclaration, Key, LocalNodeId, LocalNodeIdAny, Member, NodeType, NodeVisitor,
    Parameter, Pattern, PatternField, Property, Statement, SwitchCase, TemplateLiteral, Tree,
    TupleElement, TypeDeclaration, TypeExpression, TypeMember,
};

/// Walk any node.
pub fn walk_any<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    node_type: NodeType,
    node_id: u32,
) {
    let local_idx = tree.local_id_for_node_id(node_id);
    match node_type {
        NodeType::Block => {
            let block = tree.blocks.get(local_idx);
            walk_block(visitor, tree, LocalNodeId::new(node_id), block);
        }
        NodeType::CatchClause => {
            let catch_clause = tree.catch_clauses.get(local_idx);
            walk_catch_clause(visitor, tree, LocalNodeId::new(node_id), catch_clause);
        }
        NodeType::Statement => {
            let statement = tree.statements.get(local_idx);
            walk_statement(visitor, tree, LocalNodeId::new(node_id), statement);
        }
        NodeType::Expression => {
            let expression = tree.expressions.get(local_idx);
            walk_expression(visitor, tree, LocalNodeId::new(node_id), expression);
        }
        NodeType::ArrayElement => {
            let array_element = tree.array_elements.get(local_idx);
            walk_array_element(visitor, tree, LocalNodeId::new(node_id), array_element);
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
        NodeType::Member => {
            let member = tree.members.get(local_idx);
            walk_member(visitor, tree, LocalNodeId::new(node_id), member);
        }
        NodeType::TypeExpression => {
            let type_expression = tree.type_expressions.get(local_idx);
            walk_type_expression(visitor, tree, LocalNodeId::new(node_id), type_expression);
        }
        NodeType::TupleElement => {
            let tuple_element = tree.tuple_elements.get(local_idx);
            walk_tuple_element(visitor, tree, LocalNodeId::new(node_id), tuple_element);
        }
        NodeType::TypeMember => {
            let attribute = tree.type_members.get(local_idx);
            walk_type_member(visitor, tree, LocalNodeId::new(node_id), attribute);
        }
        NodeType::EnumField => {
            let field = tree.enum_fields.get(local_idx);
            walk_enum_field(visitor, tree, LocalNodeId::new(node_id), field);
        }
        NodeType::DependencyItem => {
            let dependency_item = tree.dependency_items.get(local_idx);
            walk_dependency_item(visitor, tree, LocalNodeId::new(node_id), dependency_item);
        }
        NodeType::SwitchCase => {
            let case = tree.switch_cases.get(local_idx);
            walk_switch_case(visitor, tree, LocalNodeId::new(node_id), case);
        }
        NodeType::Pattern => {
            let pattern = tree.patterns.get(local_idx);
            walk_pattern(visitor, tree, LocalNodeId::new(node_id), pattern);
        }
        NodeType::PatternField => {
            let field = tree.pattern_fields.get(local_idx);
            walk_pattern_field(visitor, tree, LocalNodeId::new(node_id), field);
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
        NodeType::GenericParameter => {
            let parameter = tree.generic_parameters.get(local_idx);
            walk_generic_parameter(visitor, tree, LocalNodeId::new(node_id), parameter);
        }
        NodeType::Parameter => {
            let parameter = tree.parameters.get(local_idx);
            walk_parameter(visitor, tree, LocalNodeId::new(node_id), parameter);
        }
        NodeType::Argument => {
            let argument = tree.arguments.get(local_idx);
            walk_argument(visitor, tree, LocalNodeId::new(node_id), argument);
        }
        NodeType::Annotation => {
            let annotation = tree.annotations.get(local_idx);
            walk_annotation(visitor, tree, LocalNodeId::new(node_id), annotation);
        }
    }
}

/// Visit one root node through the visitor entry points.
pub fn walk_root<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &Tree, root: &LocalNodeIdAny) {
    match root.ty {
        NodeType::Block => {
            let block_id = LocalNodeId::<Block>::new(root.id);
            let block = tree.get(block_id);
            visitor.visit_block(tree, block_id, block);
        }
        NodeType::CatchClause => {
            let catch_clause_id = LocalNodeId::<CatchClause>::new(root.id);
            let catch_clause = tree.get(catch_clause_id);
            visitor.visit_catch_clause(tree, catch_clause_id, catch_clause);
        }
        NodeType::Statement => {
            let statement_id = LocalNodeId::<Statement>::new(root.id);
            let statement = tree.get(statement_id);
            visitor.visit_statement(tree, statement_id, statement);
        }
        NodeType::Expression => {
            let expression_id = LocalNodeId::<Expression>::new(root.id);
            let expression = tree.get(expression_id);
            visitor.visit_expression(tree, expression_id, expression);
        }
        NodeType::ArrayElement => {
            let array_element_id = LocalNodeId::<ArrayElement>::new(root.id);
            let array_element = tree.get(array_element_id);
            visitor.visit_array_element(tree, array_element_id, array_element);
        }
        NodeType::Declaration => {
            let declaration_id = LocalNodeId::<Declaration>::new(root.id);
            let declaration = tree.get(declaration_id);
            visitor.visit_declaration(tree, declaration_id, declaration);
        }
        NodeType::Declarator => {
            let declarator_id = LocalNodeId::<Declarator>::new(root.id);
            let declarator = tree.get(declarator_id);
            visitor.visit_declarator(tree, declarator_id, declarator);
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
        NodeType::TypeExpression => {
            let type_expression_id = LocalNodeId::<TypeExpression>::new(root.id);
            let type_expression = tree.get(type_expression_id);
            visitor.visit_type_expression(tree, type_expression_id, type_expression);
        }
        NodeType::TupleElement => {
            let tuple_element_id = LocalNodeId::<TupleElement>::new(root.id);
            let tuple_element = tree.get(tuple_element_id);
            visitor.visit_tuple_element(tree, tuple_element_id, tuple_element);
        }
        NodeType::TypeMember => {
            let type_field_id = LocalNodeId::<TypeMember>::new(root.id);
            let type_field = tree.get(type_field_id);
            visitor.visit_type_member(tree, type_field_id, type_field);
        }
        NodeType::EnumField => {
            let enum_field_id = LocalNodeId::<EnumField>::new(root.id);
            let enum_field = tree.get(enum_field_id);
            visitor.visit_enum_field(tree, enum_field_id, enum_field);
        }
        NodeType::DependencyItem => {
            let dependency_item_id = LocalNodeId::<DependencyItem>::new(root.id);
            let dependency_item = tree.get(dependency_item_id);
            visitor.visit_dependency_item(tree, dependency_item_id, dependency_item);
        }
        NodeType::SwitchCase => {
            let switch_case_id = LocalNodeId::<SwitchCase>::new(root.id);
            let switch_case = tree.get(switch_case_id);
            visitor.visit_switch_case(tree, switch_case_id, switch_case);
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
        NodeType::GenericParameter => {
            let parameter_id = LocalNodeId::<GenericParameter>::new(root.id);
            let parameter = tree.get(parameter_id);
            visitor.visit_generic_parameter(tree, parameter_id, parameter);
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
        NodeType::Annotation => {
            let annotation_id = LocalNodeId::<Annotation>::new(root.id);
            let annotation = tree.get(annotation_id);
            visitor.visit_annotation(tree, annotation_id, annotation);
        }
    }
}

/// Walk one root list through the visitor entry points.
pub fn walk_roots<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &Tree, roots: &[LocalNodeIdAny]) {
    for root in roots {
        walk_root(visitor, tree, root);
    }
}

// ----------------------------------------------------------------------------
// Groupings
// ----------------------------------------------------------------------------

/// Walk a block.
pub fn walk_block<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Block>,
    block: &Block,
) {
    visitor.visit_any(tree, NodeType::Block, id.id);

    for statement_id in &block.statements {
        let statement = tree.get(*statement_id);
        visitor.visit_statement(tree, *statement_id, statement);
    }
}

/// Walk a statement.
pub fn walk_statement<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Statement>,
    statement: &Statement,
) {
    visitor.visit_any(tree, NodeType::Statement, id.id);

    match statement {
        Statement::Import {
            form: _,
            target: _,
            target_module: _,
            items,
            attributes,
        } => {
            if let Some(items) = items {
                for item_id in items {
                    let item = tree.get(*item_id);
                    visitor.visit_dependency_item(tree, *item_id, item);
                }
            }
            if let Some(attributes) = attributes {
                for property_id in &attributes.properties {
                    let property = tree.get(*property_id);
                    visitor.visit_property(tree, *property_id, property);
                }
            }
        }
        Statement::Export {
            form: _,
            target: _,
            target_module: _,
            items,
            attributes,
        } => {
            for item_id in items {
                let item = tree.get(*item_id);
                visitor.visit_dependency_item(tree, *item_id, item);
            }
            if let Some(attributes) = attributes {
                for property_id in &attributes.properties {
                    let property = tree.get(*property_id);
                    visitor.visit_property(tree, *property_id, property);
                }
            }
        }
        Statement::ExportValue { value } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }
        Statement::Declaration { declaration } => {
            let declaration_node = tree.get(*declaration);
            visitor.visit_declaration(tree, *declaration, declaration_node);
        }
        Statement::Block { block } => {
            let block_node = tree.get(*block);
            visitor.visit_block(tree, *block, block_node);
        }
        Statement::Labelled { label: _, body } => {
            let body_node = tree.get(*body);
            visitor.visit_statement(tree, *body, body_node);
        }
        Statement::Let {
            export: _,
            is_ambient: _,
            mutability: _,
            declarators,
        } => {
            for declarator_id in declarators {
                let declarator = tree.get(*declarator_id);
                visitor.visit_declarator(tree, *declarator_id, declarator);
            }
        }
        Statement::Using {
            asynchrony: _,
            export: _,
            is_ambient: _,
            declarators,
        } => {
            for declarator_id in declarators {
                let declarator = tree.get(*declarator_id);
                visitor.visit_declarator(tree, *declarator_id, declarator);
            }
        }
        Statement::Var {
            export: _,
            is_ambient: _,
            declarators,
        } => {
            for declarator_id in declarators {
                let declarator = tree.get(*declarator_id);
                visitor.visit_declarator(tree, *declarator_id, declarator);
            }
        }
        Statement::Assign {
            left,
            operator: _,
            right,
        } => {
            let left_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expr);
            let right_expr = tree.get(*right);
            visitor.visit_expression(tree, *right, right_expr);
        }
        Statement::Expression { expression } => {
            let expression_node = tree.get(*expression);
            visitor.visit_expression(tree, *expression, expression_node);
        }
        Statement::If {
            condition,
            then_block,
            else_block,
        } => {
            let condition_expr = tree.get(*condition);
            visitor.visit_expression(tree, *condition, condition_expr);
            let then_block_node = tree.get(*then_block);
            visitor.visit_block(tree, *then_block, then_block_node);
            if let Some(else_block) = else_block {
                let else_block_node = tree.get(*else_block);
                visitor.visit_block(tree, *else_block, else_block_node);
            }
        }
        Statement::While { condition, body } => {
            let condition_expr = tree.get(*condition);
            visitor.visit_expression(tree, *condition, condition_expr);
            let body_block = tree.get(*body);
            visitor.visit_block(tree, *body, body_block);
        }
        Statement::DoWhile { body, condition } => {
            let body_block = tree.get(*body);
            visitor.visit_block(tree, *body, body_block);
            let condition_expr = tree.get(*condition);
            visitor.visit_expression(tree, *condition, condition_expr);
        }
        Statement::For {
            initialization,
            condition,
            increment,
            body,
        } => {
            if let Some(initialization) = initialization {
                match initialization {
                    crate::ForInitialization::Expression(initialization) => {
                        let initialization_expr = tree.get(*initialization);
                        visitor.visit_expression(tree, *initialization, initialization_expr);
                    }
                    crate::ForInitialization::Declaration {
                        keyword: _,
                        declarators,
                    } => {
                        for declarator_id in declarators {
                            let declarator = tree.get(*declarator_id);
                            visitor.visit_declarator(tree, *declarator_id, declarator);
                        }
                    }
                }
            }
            if let Some(condition) = condition {
                let condition_expr = tree.get(*condition);
                visitor.visit_expression(tree, *condition, condition_expr);
            }
            if let Some(increment) = increment {
                let increment_expr = tree.get(*increment);
                visitor.visit_expression(tree, *increment, increment_expr);
            }
            let body_block = tree.get(*body);
            visitor.visit_block(tree, *body, body_block);
        }
        Statement::ForIn {
            keyword: _,
            pattern,
            iterator,
            body,
        } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, pattern_node);
            let iterator_expr = tree.get(*iterator);
            visitor.visit_expression(tree, *iterator, iterator_expr);
            let body_block = tree.get(*body);
            visitor.visit_block(tree, *body, body_block);
        }
        Statement::ForOf {
            asynchrony: _,
            keyword: _,
            pattern,
            iterator,
            body,
        } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, pattern_node);
            let iterator_expr = tree.get(*iterator);
            visitor.visit_expression(tree, *iterator, iterator_expr);
            let body_block = tree.get(*body);
            visitor.visit_block(tree, *body, body_block);
        }
        Statement::Switch { value, cases } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);

            for switch_case_id in cases {
                let switch_case = tree.get(*switch_case_id);
                visitor.visit_switch_case(tree, *switch_case_id, switch_case);
            }
        }
        Statement::Try {
            try_block,
            catch_clause,
            finally_block,
        } => {
            let try_block_node = tree.get(*try_block);
            visitor.visit_block(tree, *try_block, try_block_node);
            if let Some(catch_clause) = catch_clause {
                let catch_clause_node = tree.get(*catch_clause);
                visitor.visit_catch_clause(tree, *catch_clause, catch_clause_node);
            }
            if let Some(finally_block) = finally_block {
                let finally_block_node = tree.get(*finally_block);
                visitor.visit_block(tree, *finally_block, finally_block_node);
            }
        }
        Statement::Throw { value } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }
        Statement::Continue { label: _ } => {}
        Statement::Break { label: _ } => {}
        Statement::Return { value } => {
            if let Some(value) = value {
                let value_expr = tree.get(*value);
                visitor.visit_expression(tree, *value, value_expr);
            }
        }
        Statement::Debugger => {}
    }
}

/// Walk one generic parameter list.
fn walk_generic_parameters<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    generic_parameters: &[LocalNodeId<GenericParameter>],
) {
    for parameter_id in generic_parameters {
        let parameter = tree.get(*parameter_id);
        visitor.visit_generic_parameter(tree, *parameter_id, parameter);
    }
}

/// Walk the FunctionSignature.
fn walk_function_signature<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    signature: &FunctionSignature,
) {
    walk_generic_parameters(visitor, tree, &signature.generic_parameters);

    if let Some(this_parameter_id) = signature.this_parameter {
        let this_parameter = tree.get(this_parameter_id);
        visitor.visit_parameter(tree, this_parameter_id, this_parameter);
    }

    for parameter_id in signature.parameters.iter() {
        let parameter = tree.get(*parameter_id);
        visitor.visit_parameter(tree, *parameter_id, parameter);
    }
    if let Some(return_type) = signature.return_type {
        let return_type_node = tree.get(return_type);
        visitor.visit_type_expression(tree, return_type, return_type_node);
    }
}

/// Walk an expression.
pub fn walk_expression<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Expression>,
    expression: &Expression,
) {
    visitor.visit_any(tree, NodeType::Expression, id.id);

    match expression {
        Expression::Declaration { declaration } => {
            let declaration_node = tree.get(*declaration);
            visitor.visit_declaration(tree, *declaration, declaration_node);
        }
        Expression::ArrowFunction { signature, body } => {
            walk_function_signature(visitor, tree, signature);

            match body {
                crate::ArrowFunctionBody::Expression(body) => {
                    let body_expr = tree.get(*body);
                    visitor.visit_expression(tree, *body, body_expr);
                }
                crate::ArrowFunctionBody::Block(body) => {
                    let body_block = tree.get(*body);
                    visitor.visit_block(tree, *body, body_block);
                }
            }
        }
        Expression::Path {
            path: _,
            generic_arguments,
        } => {
            for type_id in generic_arguments {
                let ty = tree.get(*type_id);
                visitor.visit_type_expression(tree, *type_id, ty);
            }
        }
        Expression::ImportMeta => {}
        Expression::This => {}
        Expression::Super => {}
        Expression::PrivateIdentifier { name: _ } => {}
        Expression::ScalarLiteral { value: _ } => {}
        Expression::TemplateLiteral { value } => match value {
            TemplateLiteral::String { template: _ } => {}
            TemplateLiteral::TaggedString {
                tag: _,
                template: _,
            } => {}
            TemplateLiteral::InterpolatedString {
                template: _,
                expressions,
            } => {
                for expression_id in expressions {
                    let expression_node = tree.get(*expression_id);
                    visitor.visit_expression(tree, *expression_id, expression_node);
                }
            }
            TemplateLiteral::TaggedInterpolatedString {
                tag: _,
                template: _,
                expressions,
            } => {
                for expression_id in expressions {
                    let expression_node = tree.get(*expression_id);
                    visitor.visit_expression(tree, *expression_id, expression_node);
                }
            }
        },
        Expression::ArrayLiteral { elements } => {
            for element_id in elements {
                let element = tree.get(*element_id);
                visitor.visit_array_element(tree, *element_id, element);
            }
        }
        Expression::SequenceExpression { expressions } => {
            for expr_id in expressions {
                let expr = tree.get(*expr_id);
                visitor.visit_expression(tree, *expr_id, expr);
            }
        }
        Expression::ObjectLiteral { properties } => {
            for property_id in properties {
                let property = tree.get(*property_id);
                visitor.visit_property(tree, *property_id, property);
            }
        }
        Expression::Parenthesized { expression } => {
            let expression_node = tree.get(*expression);
            visitor.visit_expression(tree, *expression, expression_node);
        }
        Expression::As {
            expression,
            target_type,
        }
        | Expression::Satisfies {
            expression,
            target_type,
        } => {
            let expression_node = tree.get(*expression);
            visitor.visit_expression(tree, *expression, expression_node);

            let target_type_node = tree.get(*target_type);
            visitor.visit_type_expression(tree, *target_type, target_type_node);
        }
        Expression::InstanceOf { value, target } => {
            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);

            let target_expression = tree.get(*target);
            visitor.visit_expression(tree, *target, target_expression);
        }
        Expression::Await { value } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }
        Expression::Yield {
            is_delegate: _,
            value,
        } => {
            if let Some(value) = value {
                let value_expr = tree.get(*value);
                visitor.visit_expression(tree, *value, value_expr);
            }
        }
        Expression::Unary { operator: _, right } => {
            let expression_node = tree.get(*right);
            visitor.visit_expression(tree, *right, expression_node);
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
        Expression::Assign { left, right } => {
            let left_pattern = tree.get(*left);
            visitor.visit_assign_pattern(tree, *left, left_pattern);
            let right_expr = tree.get(*right);
            visitor.visit_expression(tree, *right, right_expr);
        }
        Expression::AssignBinary {
            left,
            operator: _,
            right,
        } => {
            let left_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expr);
            let right_expr = tree.get(*right);
            visitor.visit_expression(tree, *right, right_expr);
        }
        Expression::Maybe { position: _, left } => {
            let left_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expr);
        }
        Expression::Must { position: _, left } => {
            let left_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expr);
        }
        Expression::Member { left, name: _ } => {
            let left_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expr);
        }
        Expression::PrivateMember { left, name: _ } => {
            let left_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expr);
        }
        Expression::Index {
            position: _,
            left,
            right,
        } => {
            let left_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expr);
            let right_expr = tree.get(*right);
            visitor.visit_expression(tree, *right, right_expr);
        }
        Expression::Instantiation {
            left,
            generic_arguments,
        } => {
            let left_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expr);

            for type_id in generic_arguments {
                let ty = tree.get(*type_id);
                visitor.visit_type_expression(tree, *type_id, ty);
            }
        }
        Expression::Call {
            position: _,
            left,
            generic_arguments,
            arguments,
        } => {
            let left_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expr);

            for type_id in generic_arguments {
                let ty = tree.get(*type_id);
                visitor.visit_type_expression(tree, *type_id, ty);
            }

            for argument_id in arguments {
                let argument = tree.get(*argument_id);
                visitor.visit_argument(tree, *argument_id, argument);
            }
        }
        Expression::ImportCall {
            target,
            target_module: _,
            arguments,
        } => {
            let target_expression = tree.get(*target);
            visitor.visit_expression(tree, *target, target_expression);
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
            let left_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expr);

            for type_id in generic_arguments {
                let ty = tree.get(*type_id);
                visitor.visit_type_expression(tree, *type_id, ty);
            }

            for argument_id in arguments {
                let argument = tree.get(*argument_id);
                visitor.visit_argument(tree, *argument_id, argument);
            }
        }
        Expression::IfTernary {
            condition,
            then_expression,
            else_expression,
        } => {
            let condition_expr = tree.get(*condition);
            visitor.visit_expression(tree, *condition, condition_expr);
            let then_expr = tree.get(*then_expression);
            visitor.visit_expression(tree, *then_expression, then_expr);
            if let Some(else_expression) = else_expression {
                let else_expr = tree.get(*else_expression);
                visitor.visit_expression(tree, *else_expression, else_expr);
            }
        }
        Expression::Missing => {}
        Expression::Stub => {}
        Expression::Error => {}
    }
}

/// Walk a declaration.
pub fn walk_declaration<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Declaration>,
    declaration: &Declaration,
) {
    visitor.visit_any(tree, NodeType::Declaration, id.id);

    match declaration {
        Declaration::Global(GlobalDeclaration {
            is_ambient: _,
            statements,
        }) => {
            for statement_id in statements {
                let statement = tree.get(*statement_id);
                visitor.visit_statement(tree, *statement_id, statement);
            }
        }
        Declaration::Type(TypeDeclaration {
            name: _,
            export: _,
            is_ambient: _,
            generic_parameters,
            value,
        }) => {
            walk_generic_parameters(visitor, tree, generic_parameters);
            let ty = tree.get(*value);
            visitor.visit_type_expression(tree, *value, ty);
        }
        Declaration::Class(ClassDeclaration {
            name: _,
            export: _,
            is_ambient: _,
            is_abstract: _,
            generic_parameters,
            extends_expression,
            extends_generic_arguments,
            implements_types,
            members,
        }) => {
            walk_generic_parameters(visitor, tree, generic_parameters);

            if let Some(extends_expression_id) = extends_expression {
                let extends_expression = tree.get(*extends_expression_id);
                visitor.visit_expression(tree, *extends_expression_id, extends_expression);
            }

            for extends_generic_argument_id in extends_generic_arguments {
                let extends_generic_argument = tree.get(*extends_generic_argument_id);
                visitor.visit_type_expression(
                    tree,
                    *extends_generic_argument_id,
                    extends_generic_argument,
                );
            }

            for implements_type_id in implements_types {
                let implements_type = tree.get(*implements_type_id);
                visitor.visit_type_expression(tree, *implements_type_id, implements_type);
            }

            for member_id in members {
                let member = tree.get(*member_id);
                visitor.visit_member(tree, *member_id, member);
            }
        }
        Declaration::Interface(InterfaceDeclaration {
            name: _,
            export: _,
            is_ambient: _,
            generic_parameters,
            extends,
            members,
        }) => {
            walk_generic_parameters(visitor, tree, generic_parameters);

            for heritage in extends {
                let expression = tree.get(heritage.expression);
                visitor.visit_expression(tree, heritage.expression, expression);

                for type_argument_id in &heritage.type_arguments {
                    let type_argument = tree.get(*type_argument_id);
                    visitor.visit_type_expression(tree, *type_argument_id, type_argument);
                }
            }

            for member_id in members {
                let member = tree.get(*member_id);
                visitor.visit_type_member(tree, *member_id, member);
            }
        }
        Declaration::Enum(EnumDeclaration {
            name: _,
            export: _,
            is_ambient: _,
            fields,
        }) => {
            for field_id in fields {
                let field = tree.get(*field_id);
                visitor.visit_enum_field(tree, *field_id, field);
            }
        }
        Declaration::Function(FunctionDeclaration {
            name: _,
            export: _,
            is_ambient: _,
            is_abstract: _,
            signature,
            body,
        }) => {
            walk_function_signature(visitor, tree, signature);
            if let Some(body) = body {
                let body_block = tree.get(*body);
                visitor.visit_block(tree, *body, body_block);
            }
        }
    }
}

/// Walk the Key.
pub fn walk_key<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &Tree, key: &Key) {
    match key {
        Key::Name(_) => {}
        Key::Private(_) => {}
        Key::Expression(expression) => {
            let expression_expr = tree.get(*expression);
            visitor.visit_expression(tree, *expression, expression_expr);
        }
        Key::NamedExpression { name: _, key } => {
            let key_expr = tree.get(*key);
            visitor.visit_type_expression(tree, *key, key_expr);
        }
    }
}

/// Walk a declarator.
pub fn walk_declarator<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
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

/// Walk a field.
pub fn walk_property<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Property>,
    property: &Property,
) {
    visitor.visit_any(tree, NodeType::Property, id.id);

    match property {
        Property::Field {
            modifiers: _,
            key,
            value,
            is_shorthand: _,
        } => {
            walk_key(visitor, tree, key);

            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
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
            if let Some(block) = body {
                let body_block = tree.get(*block);
                visitor.visit_block(tree, *block, body_block);
            }
        }
        Property::Spread {
            modifiers: _,
            value,
        } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }
    }
}

/// Walk a member.
pub fn walk_member<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Member>,
    member: &Member,
) {
    visitor.visit_any(tree, NodeType::Member, id.id);

    match member {
        Member::Field {
            modifiers: _,
            key,
            value,
            default,
        } => {
            walk_key(visitor, tree, key);

            if let Some(value) = value {
                let value_expr = tree.get(*value);
                visitor.visit_type_expression(tree, *value, value_expr);
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
            if let Some(block) = body {
                let body_block = tree.get(*block);
                visitor.visit_block(tree, *block, body_block);
            }
        }
        Member::StaticBlock { body } => {
            let body_block = tree.get(*body);
            visitor.visit_block(tree, *body, body_block);
        }
    }
}

/// Walk an enum field.
pub fn walk_enum_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<EnumField>,
    field: &EnumField,
) {
    visitor.visit_any(tree, NodeType::EnumField, id.id);

    if let Some(value) = field.value {
        let value_expr = tree.get(value);
        visitor.visit_expression(tree, value, value_expr);
    }
}

/// Walk a dependency item.
pub fn walk_dependency_item<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<DependencyItem>,
    dependency_item: &DependencyItem,
) {
    visitor.visit_any(tree, NodeType::DependencyItem, id.id);
    if let Some(value) = &dependency_item.value {
        let value_expr = tree.get(*value);
        visitor.visit_expression(tree, *value, value_expr);
    }
}

/// Walk a switch case.
pub fn walk_switch_case<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<SwitchCase>,
    switch_case: &SwitchCase,
) {
    visitor.visit_any(tree, NodeType::SwitchCase, id.id);

    if let Some(value) = switch_case.value {
        let value_expr = tree.get(value);
        visitor.visit_expression(tree, value, value_expr);
    }

    let body_block = tree.get(switch_case.body);
    visitor.visit_block(tree, switch_case.body, body_block);
}

/// Walk a catch clause.
pub fn walk_catch_clause<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<CatchClause>,
    catch_clause: &CatchClause,
) {
    visitor.visit_any(tree, NodeType::CatchClause, id.id);

    if let Some(pattern) = catch_clause.pattern {
        let pattern_node = tree.get(pattern);
        visitor.visit_pattern(tree, pattern, pattern_node);
    }

    let body_block = tree.get(catch_clause.body);
    visitor.visit_block(tree, catch_clause.body, body_block);
}

/// Walk one tuple element.
pub fn walk_tuple_element<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<TupleElement>,
    tuple_element: &TupleElement,
) {
    visitor.visit_any(tree, NodeType::TupleElement, id.id);

    let ty = tree.get(tuple_element.ty);
    visitor.visit_type_expression(tree, tuple_element.ty, ty);
}

/// Walk one generic parameter.
pub fn walk_generic_parameter<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<GenericParameter>,
    parameter: &GenericParameter,
) {
    visitor.visit_any(tree, NodeType::GenericParameter, id.id);

    match parameter {
        GenericParameter::Type {
            modifiers: _,
            name: _,
            constraint,
            default,
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
    }
}

/// Walk a parameter.
pub fn walk_parameter<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
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
                let ty_node = tree.get(*ty);
                visitor.visit_type_expression(tree, *ty, ty_node);
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
                let ty_node = tree.get(*ty);
                visitor.visit_type_expression(tree, *ty, ty_node);
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
                let ty_node = tree.get(*ty);
                visitor.visit_type_expression(tree, *ty, ty_node);
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
                let ty_node = tree.get(*ty);
                visitor.visit_type_expression(tree, *ty, ty_node);
            }
        }
    }
}

/// Walk an argument.
pub fn walk_argument<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Argument>,
    argument: &Argument,
) {
    visitor.visit_any(tree, NodeType::Argument, id.id);

    match argument {
        Argument::Positional { value } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }
        Argument::Spread { value } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }
    }
}

/// Walk one array element.
pub fn walk_array_element<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<ArrayElement>,
    array_element: &ArrayElement,
) {
    visitor.visit_any(tree, NodeType::ArrayElement, id.id);

    match array_element {
        ArrayElement::Expression { value } | ArrayElement::Spread { value } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }
        ArrayElement::Elision => {}
    }
}

/// Walk a pattern.
pub fn walk_pattern<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Pattern>,
    pattern: &Pattern,
) {
    visitor.visit_any(tree, NodeType::Pattern, id.id);

    match pattern {
        Pattern::Binding {
            mutability: _,
            name: _,
        } => {}
        Pattern::Assign { pattern, value } => {
            let inner_pattern = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, inner_pattern);

            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
        }
        Pattern::Hole => {}
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
    }
}

/// Walk a pattern field.
pub fn walk_pattern_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<PatternField>,
    field: &PatternField,
) {
    visitor.visit_any(tree, NodeType::PatternField, id.id);

    match field {
        PatternField::Named {
            mutability: _,
            name: _,
            is_shorthand: _,
            pattern,
        } => {
            if let Some(pattern) = pattern {
                let pattern_node = tree.get(*pattern);
                visitor.visit_pattern(tree, *pattern, pattern_node);
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
            // nothing to do
        }
    }
}

/// Walk an assign pattern.
pub fn walk_assign_pattern<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
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

/// Walk an assign pattern field.
pub fn walk_assign_pattern_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
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
            // nothing to do
        }
    }
}

/// Walk an annotation.
pub fn walk_annotation<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Annotation>,
    _annotation: &Annotation,
) {
    visitor.visit_any(tree, NodeType::Annotation, id.id);
}

/// Walk a type expression.
pub fn walk_type_expression<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<TypeExpression>,
    ty: &TypeExpression,
) {
    visitor.visit_any(tree, NodeType::TypeExpression, id.id);
    match ty {
        TypeExpression::Scalar(_) => {}
        TypeExpression::This => {}
        TypeExpression::Path {
            path: _,
            generic_arguments,
        } => {
            for type_id in generic_arguments {
                let ty = tree.get(*type_id);
                visitor.visit_type_expression(tree, *type_id, ty);
            }
        }
        TypeExpression::Readonly { target_type }
        | TypeExpression::KeyOf { target_type }
        | TypeExpression::Must { target_type }
        | TypeExpression::Not { target_type } => {
            let target_type_node = tree.get(*target_type);
            visitor.visit_type_expression(tree, *target_type, target_type_node);
        }
        TypeExpression::Extends { left, right } | TypeExpression::Implements { left, right } => {
            let left_ty = tree.get(*left);
            visitor.visit_type_expression(tree, *left, left_ty);

            let right_ty = tree.get(*right);
            visitor.visit_type_expression(tree, *right, right_ty);
        }
        TypeExpression::Conditional {
            left,
            right,
            then_type,
            else_type,
        } => {
            let left_ty = tree.get(*left);
            visitor.visit_type_expression(tree, *left, left_ty);

            let right_ty = tree.get(*right);
            visitor.visit_type_expression(tree, *right, right_ty);

            let then_ty = tree.get(*then_type);
            visitor.visit_type_expression(tree, *then_type, then_ty);

            let else_ty = tree.get(*else_type);
            visitor.visit_type_expression(tree, *else_type, else_ty);
        }
        TypeExpression::Mapped {
            parameter,
            modifiers: _,
            value,
        } => {
            let source_type = tree.get(parameter.source_type);
            visitor.visit_type_expression(tree, parameter.source_type, source_type);

            if let Some(key_remap) = parameter.key_remap {
                let key_remap_ty = tree.get(key_remap);
                visitor.visit_type_expression(tree, key_remap, key_remap_ty);
            }

            if let Some(value) = value {
                let value_ty = tree.get(*value);
                visitor.visit_type_expression(tree, *value, value_ty);
            }
        }
        TypeExpression::Index { left, index } => {
            let left_ty = tree.get(*left);
            visitor.visit_type_expression(tree, *left, left_ty);

            let index_ty = tree.get(*index);
            visitor.visit_type_expression(tree, *index, index_ty);
        }
        TypeExpression::TemplateLiteral(template) => {
            for span_id in &template.spans {
                let span_ty = tree.get(*span_id);
                visitor.visit_type_expression(tree, *span_id, span_ty);
            }
        }
        TypeExpression::Import {
            target: _,
            qualifier: _,
            generic_arguments,
        } => {
            for type_id in generic_arguments {
                let ty = tree.get(*type_id);
                visitor.visit_type_expression(tree, *type_id, ty);
            }
        }
        TypeExpression::Infer {
            name: _,
            constraint,
        } => {
            if let Some(constraint) = constraint {
                let constraint_ty = tree.get(*constraint);
                visitor.visit_type_expression(tree, *constraint, constraint_ty);
            }
        }
        TypeExpression::Array { element } => {
            let element_ty = tree.get(*element);
            visitor.visit_type_expression(tree, *element, element_ty);
        }
        TypeExpression::Tuple { elements } => {
            for element_id in elements {
                let tuple_element = tree.get(*element_id);
                visitor.visit_tuple_element(tree, *element_id, tuple_element);
            }
        }
        TypeExpression::Object { members } => {
            for member_id in members {
                let member = tree.get(*member_id);
                visitor.visit_type_member(tree, *member_id, member);
            }
        }
        TypeExpression::Union { elements } => {
            for element_id in elements {
                let element_ty = tree.get(*element_id);
                visitor.visit_type_expression(tree, *element_id, element_ty);
            }
        }
        TypeExpression::Intersection { elements } => {
            for element_id in elements {
                let element_ty = tree.get(*element_id);
                visitor.visit_type_expression(tree, *element_id, element_ty);
            }
        }
        TypeExpression::FunctionTypeDeclaration(function) => {
            for generic_parameter_id in &function.generic_parameters {
                let generic_parameter = tree.get(*generic_parameter_id);
                visitor.visit_generic_parameter(tree, *generic_parameter_id, generic_parameter);
            }

            if let Some(this_parameter_id) = function.this_parameter {
                let this_parameter = tree.get(this_parameter_id);
                visitor.visit_parameter(tree, this_parameter_id, this_parameter);
            }

            for parameter_id in &function.parameters {
                let parameter = tree.get(*parameter_id);
                visitor.visit_parameter(tree, *parameter_id, parameter);
            }

            if let Some(return_type_id) = function.return_type {
                let return_type = tree.get(return_type_id);
                visitor.visit_type_expression(tree, return_type_id, return_type);
            }
        }
        TypeExpression::ConstructorTypeDeclaration(function) => {
            for generic_parameter_id in &function.generic_parameters {
                let generic_parameter = tree.get(*generic_parameter_id);
                visitor.visit_generic_parameter(tree, *generic_parameter_id, generic_parameter);
            }

            for parameter_id in &function.parameters {
                let parameter = tree.get(*parameter_id);
                visitor.visit_parameter(tree, *parameter_id, parameter);
            }

            if let Some(return_type_id) = function.return_type {
                let return_type = tree.get(return_type_id);
                visitor.visit_type_expression(tree, return_type_id, return_type);
            }
        }
        TypeExpression::Error => {}
    }
}

/// Walk one type member.
pub fn walk_type_member<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<TypeMember>,
    attribute: &TypeMember,
) {
    visitor.visit_any(tree, NodeType::TypeMember, id.id);
    match attribute {
        TypeMember::Field {
            modifiers: _,
            key,
            ty,
        } => {
            walk_key(visitor, tree, key);

            let ty_ty = tree.get(*ty);
            visitor.visit_type_expression(tree, *ty, ty_ty);
        }
        TypeMember::Method {
            modifiers: _,
            key,
            signature,
        } => {
            walk_key(visitor, tree, key);
            walk_function_signature(visitor, tree, signature);
        }
        TypeMember::CallSignature {
            modifiers: _,
            signature,
        } => {
            for generic_parameter_id in &signature.generic_parameters {
                let generic_parameter = tree.get(*generic_parameter_id);
                visitor.visit_generic_parameter(tree, *generic_parameter_id, generic_parameter);
            }

            if let Some(this_parameter_id) = signature.this_parameter {
                let this_parameter = tree.get(this_parameter_id);
                visitor.visit_parameter(tree, this_parameter_id, this_parameter);
            }

            for parameter_id in &signature.parameters {
                let parameter = tree.get(*parameter_id);
                visitor.visit_parameter(tree, *parameter_id, parameter);
            }

            if let Some(return_type_id) = signature.return_type {
                let return_type = tree.get(return_type_id);
                visitor.visit_type_expression(tree, return_type_id, return_type);
            }
        }
        TypeMember::ConstructSignature {
            modifiers: _,
            signature,
        } => {
            for generic_parameter_id in &signature.generic_parameters {
                let generic_parameter = tree.get(*generic_parameter_id);
                visitor.visit_generic_parameter(tree, *generic_parameter_id, generic_parameter);
            }

            for parameter_id in &signature.parameters {
                let parameter = tree.get(*parameter_id);
                visitor.visit_parameter(tree, *parameter_id, parameter);
            }

            if let Some(return_type_id) = signature.return_type {
                let return_type = tree.get(return_type_id);
                visitor.visit_type_expression(tree, return_type_id, return_type);
            }
        }
        TypeMember::IndexSignature {
            modifiers: _,
            name: _,
            key_type,
            value_type,
        } => {
            let key_type_node = tree.get(*key_type);
            visitor.visit_type_expression(tree, *key_type, key_type_node);

            let value_type_node = tree.get(*value_type);
            visitor.visit_type_expression(tree, *value_type, value_type_node);
        }
    }
}
