use crate::{
    Annotation, Argument, Block, Declaration, DeclarationDescriptor, Declarator, DependencyItem,
    EnumField, Expression, FunctionSignature, Generics, Heritage, Key, LocalNodeId, Member,
    NodeTree, NodeType, NodeVisitor, Parameter, Pattern, PatternField, Property, Statement,
    SwitchCase, TemplateLiteral, Type, TypeField,
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
        NodeType::Block => {
            let block = tree.blocks.get(local_idx);
            walk_block(visitor, tree, LocalNodeId::new(node_id), block);
        }
        NodeType::Statement => {
            let statement = tree.statements.get(local_idx);
            walk_statement(visitor, tree, LocalNodeId::new(node_id), statement);
        }
        NodeType::Expression => {
            let expression = tree.expressions.get(local_idx);
            walk_expression(visitor, tree, LocalNodeId::new(node_id), expression);
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
        NodeType::Type => {
            let ty = tree.types.get(local_idx);
            walk_type(visitor, tree, LocalNodeId::new(node_id), ty);
        }
        NodeType::TypeField => {
            let attribute = tree.type_fields.get(local_idx);
            walk_type_field(visitor, tree, LocalNodeId::new(node_id), attribute);
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

// ----------------------------------------------------------------------------
// Groupings
// ----------------------------------------------------------------------------

/// Walk a block.
pub fn walk_block<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
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
    tree: &NodeTree,
    id: LocalNodeId<Statement>,
    statement: &Statement,
) {
    visitor.visit_any(tree, NodeType::Statement, id.id);

    match statement {
        Statement::Import {
            kind: _,
            target: _,
            items,
            arguments,
        } => {
            for item_id in items {
                let item = tree.get(*item_id);
                visitor.visit_dependency_item(tree, *item_id, item);
            }
            if let Some(arguments) = arguments {
                for argument_id in arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_argument(tree, *argument_id, argument);
                }
            }
        }
        Statement::Export {
            kind: _,
            target: _,
            items,
        } => {
            for item_id in items {
                let item = tree.get(*item_id);
                visitor.visit_dependency_item(tree, *item_id, item);
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
            descriptor: _,
            mutability: _,
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
        Statement::For {
            initialization,
            condition,
            increment,
            body,
        } => {
            if let Some(initialization) = initialization {
                let initialization_expr = tree.get(*initialization);
                visitor.visit_expression(tree, *initialization, initialization_expr);
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
            name: _,
            iterator,
            body,
        } => {
            let iterator_expr = tree.get(*iterator);
            visitor.visit_expression(tree, *iterator, iterator_expr);
            let body_block = tree.get(*body);
            visitor.visit_block(tree, *body, body_block);
        }
        Statement::ForOf {
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
        Statement::Try {
            try_block,
            catch_pattern,
            catch_block,
            finally_block,
        } => {
            let try_block_node = tree.get(*try_block);
            visitor.visit_block(tree, *try_block, try_block_node);
            if let Some(pattern) = catch_pattern {
                let pattern_node = tree.get(*pattern);
                visitor.visit_pattern(tree, *pattern, pattern_node);
            }
            let catch_block_node = tree.get(*catch_block);
            visitor.visit_block(tree, *catch_block, catch_block_node);
            if let Some(finally_block) = finally_block {
                let finally_block_node = tree.get(*finally_block);
                visitor.visit_block(tree, *finally_block, finally_block_node);
            }
        }
        Statement::Await { value } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }
        Statement::Yield { value } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
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

/// Walk the Generics.
fn walk_generics<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, generics: &Generics) {
    if let Some(static_parameters) = generics.static_parameters.as_ref() {
        for parameter_id in static_parameters {
            let parameter = tree.get(*parameter_id);
            visitor.visit_parameter(tree, *parameter_id, parameter);
        }
    }
}

/// Walk the Heritage.
fn walk_heritage<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, heritage: &Heritage) {
    if let Some(extends_types) = heritage.extends_types.as_ref() {
        for extends_type_id in extends_types {
            let extends_type = tree.get(*extends_type_id);
            visitor.visit_type(tree, *extends_type_id, extends_type);
        }
    }
    if let Some(implements_types) = heritage.implements_types.as_ref() {
        for implements_type_id in implements_types {
            let implements_type = tree.get(*implements_type_id);
            visitor.visit_type(tree, *implements_type_id, implements_type);
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
    for parameter_id in signature.dynamic_parameters.iter() {
        let parameter = tree.get(*parameter_id);
        visitor.visit_parameter(tree, *parameter_id, parameter);
    }
    if let Some(return_type) = signature.return_type {
        let return_type_node = tree.get(return_type);
        visitor.visit_type(tree, return_type, return_type_node);
    }
}

/// Walk an expression.
pub fn walk_expression<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
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
            let body_expr = tree.get(*body);
            visitor.visit_expression(tree, *body, body_expr);
        }
        Expression::Path {
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
                visitor.visit_expression(tree, *element_id, element);
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
        Expression::TypeUnary { operator: _, right } => {
            let expression_node = tree.get(*right);
            visitor.visit_expression(tree, *right, expression_node);
        }
        Expression::Unary { operator: _, right } => {
            let expression_node = tree.get(*right);
            visitor.visit_expression(tree, *right, expression_node);
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
            let left_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expr);
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
        Expression::Member {
            left,
            name: _,
            static_arguments,
        } => {
            let left_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expr);
            if let Some(arguments) = static_arguments {
                for argument_id in arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_argument(tree, *argument_id, argument);
                }
            }
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
        Expression::Call {
            position: _,
            left,
            static_arguments,
            dynamic_arguments,
        } => {
            let left_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expr);
            if let Some(arguments) = static_arguments {
                for argument_id in arguments {
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
            left,
            static_arguments,
            dynamic_arguments,
        } => {
            let left_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expr);
            if let Some(arguments) = static_arguments {
                for argument_id in arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_argument(tree, *argument_id, argument);
                }
            }
            for argument_id in dynamic_arguments {
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

/// Walk a declaration.
pub fn walk_declaration<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Declaration>,
    declaration: &Declaration,
) {
    visitor.visit_any(tree, NodeType::Declaration, id.id);

    match declaration {
        Declaration::Namespace {
            descriptor,
            statements,
        } => {
            walk_declaration_descriptor(visitor, tree, descriptor);
            for statement_id in statements {
                let statement = tree.get(*statement_id);
                visitor.visit_statement(tree, *statement_id, statement);
            }
        }
        Declaration::Type {
            descriptor,
            static_parameters,
            value,
        } => {
            walk_declaration_descriptor(visitor, tree, descriptor);
            if let Some(parameters) = static_parameters {
                for parameter_id in parameters {
                    let parameter = tree.get(*parameter_id);
                    visitor.visit_parameter(tree, *parameter_id, parameter);
                }
            }
            let ty = tree.get(*value);
            visitor.visit_type(tree, *value, ty);
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
        Declaration::Interface {
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
        Declaration::Enum { descriptor, fields } => {
            walk_declaration_descriptor(visitor, tree, descriptor);
            for field_id in fields {
                let field = tree.get(*field_id);
                visitor.visit_enum_field(tree, *field_id, field);
            }
        }
        Declaration::Function {
            descriptor,
            signature,
            body,
        } => {
            walk_declaration_descriptor(visitor, tree, descriptor);
            walk_function_signature(visitor, tree, signature);
            if let Some(body) = body {
                let body_block = tree.get(*body);
                visitor.visit_block(tree, *body, body_block);
            }
        }
    }
}

/// Walk the Key.
pub fn walk_key<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, key: &Key) {
    match key {
        Key::Name(_) => {}
        Key::Expression(expression) => {
            let expression_expr = tree.get(*expression);
            visitor.visit_expression(tree, *expression, expression_expr);
        }
        Key::NamedExpression { name: _, key } => {
            let key_expr = tree.get(*key);
            visitor.visit_expression(tree, *key, key_expr);
        }
    }
}

/// Walk a declarator.
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
        visitor.visit_type(tree, *ty_id, ty_node);
    }
    if let Some(value_id) = value {
        let value_node = tree.get(*value_id);
        visitor.visit_expression(tree, *value_id, value_node);
    }
}

/// Walk a field.
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
            if let Some(block) = body {
                let body_expression = tree.get(*block);
                visitor.visit_expression(tree, *block, body_expression);
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
    tree: &NodeTree,
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
            if let Some(block) = body {
                let body_expression = tree.get(*block);
                visitor.visit_expression(tree, *block, body_expression);
            }
        }
        Member::StaticBlock { body } => {
            let body_expr = tree.get(*body);
            visitor.visit_expression(tree, *body, body_expr);
        }
    }
}

/// Walk an enum field.
pub fn walk_enum_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
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
    tree: &NodeTree,
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
    tree: &NodeTree,
    id: LocalNodeId<SwitchCase>,
    switch_case: &SwitchCase,
) {
    visitor.visit_any(tree, NodeType::SwitchCase, id.id);

    let value_expr = tree.get(switch_case.value);
    visitor.visit_expression(tree, switch_case.value, value_expr);
    let body_block = tree.get(switch_case.body);
    visitor.visit_block(tree, switch_case.body, body_block);
}

/// Walk a parameter.
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
                let ty_node = tree.get(*ty);
                visitor.visit_type(tree, *ty, ty_node);
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
                visitor.visit_type(tree, *ty, ty_node);
            }
            if let Some(default) = default {
                let default_expr = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expr);
            }
        }
        Parameter::Variadic {
            modifiers: _,
            name: _,
            ty,
        } => {
            if let Some(ty) = ty {
                let ty_node = tree.get(*ty);
                visitor.visit_type(tree, *ty, ty_node);
            }
        }
    }
}

/// Walk an argument.
pub fn walk_argument<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
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
        Argument::Dynamic { key, value } => {
            let key_expr = tree.get(*key);
            visitor.visit_expression(tree, *key, key_expr);
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }
    }
}

/// Walk a pattern.
pub fn walk_pattern<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Pattern>,
    pattern: &Pattern,
) {
    visitor.visit_any(tree, NodeType::Pattern, id.id);

    match pattern {
        Pattern::Binding {
            mutability: _,
            name: _,
        } => {}
        Pattern::Hole => {}
        Pattern::Array { elements } => {
            for element_id in elements {
                let element = tree.get(*element_id);
                visitor.visit_pattern(tree, *element_id, element);
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
    tree: &NodeTree,
    id: LocalNodeId<PatternField>,
    field: &PatternField,
) {
    visitor.visit_any(tree, NodeType::PatternField, id.id);

    match field {
        PatternField::Named {
            mutability: _,
            name: _,
            pattern,
            default,
        } => {
            if let Some(pattern) = pattern {
                let pattern_node = tree.get(*pattern);
                visitor.visit_pattern(tree, *pattern, pattern_node);
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
        PatternField::Positional { pattern } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, pattern_node);
        }
        PatternField::Spread {
            mutability: _,
            name: _,
        } => {
            // nothing to do
        }
        PatternField::Elision => {
            // nothing to do
        }
    }
}

/// Walk an annotation.
pub fn walk_annotation<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Annotation>,
    _annotation: &Annotation,
) {
    visitor.visit_any(tree, NodeType::Annotation, id.id);
}

/// Walk a type.
pub fn walk_type<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Type>,
    ty: &Type,
) {
    visitor.visit_any(tree, NodeType::Type, id.id);
    match ty {
        Type::Scalar(_) => {}
        Type::Expression(expression) => {
            let expression_expr = tree.get(*expression);
            visitor.visit_expression(tree, *expression, expression_expr);
        }
        Type::Path {
            path: _,
            static_arguments,
        } => {
            if let Some(static_arguments) = static_arguments {
                for argument_id in static_arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_argument(tree, *argument_id, argument);
                }
            }
        }

        Type::Unary { operator: _, right } => {
            let right_ty = tree.get(*right);
            visitor.visit_type(tree, *right, right_ty);
        }
        Type::Binary {
            left,
            operator: _,
            right,
        } => {
            let left_ty = tree.get(*left);
            visitor.visit_type(tree, *left, left_ty);
            let right_ty = tree.get(*right);
            visitor.visit_type(tree, *right, right_ty);
        }

        Type::Array { element } => {
            if let Some(element) = element {
                let element_ty = tree.get(*element);
                visitor.visit_type(tree, *element, element_ty);
            }
        }
        Type::Tuple { elements } => {
            for element_id in elements {
                let element_ty = tree.get(*element_id);
                visitor.visit_type(tree, *element_id, element_ty);
            }
        }
        Type::Object { properties } => {
            for property_id in properties {
                let property = tree.get(*property_id);
                visitor.visit_type_field(tree, *property_id, property);
            }
        }
        Type::Union { elements } => {
            for element_id in elements {
                let element_ty = tree.get(*element_id);
                visitor.visit_type(tree, *element_id, element_ty);
            }
        }
        Type::Intersection { elements } => {
            for element_id in elements {
                let element_ty = tree.get(*element_id);
                visitor.visit_type(tree, *element_id, element_ty);
            }
        }
        Type::Function { signature } => {
            walk_function_signature(visitor, tree, signature);
        }

        Type::Error => {}
    }
}

/// Walk a type field.
pub fn walk_type_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<TypeField>,
    attribute: &TypeField,
) {
    visitor.visit_any(tree, NodeType::TypeField, id.id);
    match attribute {
        TypeField::Field {
            modifiers: _,
            key,
            ty,
        } => {
            if let Some(key) = key {
                walk_key(visitor, tree, key);
            }
            let ty_ty = tree.get(*ty);
            visitor.visit_type(tree, *ty, ty_ty);
        }
        TypeField::Method {
            modifiers: _,
            key,
            signature,
        } => {
            if let Some(key) = key {
                walk_key(visitor, tree, key);
            }
            walk_function_signature(visitor, tree, signature);
        }
    }
}
