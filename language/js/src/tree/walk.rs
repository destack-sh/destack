use crate::{
    Annotation, Argument, ArrayElement, ArrowFunctionBody, AssignPattern, AssignPatternField,
    Block, CatchClause, ClassDeclaration, Declaration, Declarator, DependencyItem, Expression,
    ForInitialization, FunctionDeclaration, FunctionSignature, Key, LocalNodeId, LocalNodeIdAny,
    Member, NodeType, NodeVisitor, Parameter, Pattern, PatternField, Property, Statement,
    SwitchCase, TemplateLiteral, Tree,
};

/// Walk one root through visitor entry points.
pub fn walk_root<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &Tree, root: &LocalNodeIdAny) {
    match root.ty {
        NodeType::Block => visit_block(visitor, tree, LocalNodeId::new(root.id)),
        NodeType::CatchClause => visit_catch_clause(visitor, tree, LocalNodeId::new(root.id)),
        NodeType::Statement => visit_statement(visitor, tree, LocalNodeId::new(root.id)),
        NodeType::Expression => visit_expression(visitor, tree, LocalNodeId::new(root.id)),
        NodeType::ArrayElement => visit_array_element(visitor, tree, LocalNodeId::new(root.id)),
        NodeType::Declaration => visit_declaration(visitor, tree, LocalNodeId::new(root.id)),
        NodeType::Declarator => visit_declarator(visitor, tree, LocalNodeId::new(root.id)),
        NodeType::Property => visit_property(visitor, tree, LocalNodeId::new(root.id)),
        NodeType::Member => visit_member(visitor, tree, LocalNodeId::new(root.id)),
        NodeType::DependencyItem => visit_dependency_item(visitor, tree, LocalNodeId::new(root.id)),
        NodeType::SwitchCase => visit_switch_case(visitor, tree, LocalNodeId::new(root.id)),
        NodeType::Pattern => visit_pattern(visitor, tree, LocalNodeId::new(root.id)),
        NodeType::PatternField => visit_pattern_field(visitor, tree, LocalNodeId::new(root.id)),
        NodeType::AssignPattern => visit_assign_pattern(visitor, tree, LocalNodeId::new(root.id)),
        NodeType::AssignPatternField => {
            visit_assign_pattern_field(visitor, tree, LocalNodeId::new(root.id));
        }
        NodeType::Parameter => visit_parameter(visitor, tree, LocalNodeId::new(root.id)),
        NodeType::Argument => visit_argument(visitor, tree, LocalNodeId::new(root.id)),
        NodeType::Annotation => visit_annotation(visitor, tree, LocalNodeId::new(root.id)),
    }
}

/// Walk one root list through visitor entry points.
pub fn walk_roots<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &Tree, roots: &[LocalNodeIdAny]) {
    for root in roots {
        walk_root(visitor, tree, root);
    }
}

/// Walk one block.
pub fn walk_block<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Block>,
    block: &Block,
) {
    visitor.visit_any(tree, NodeType::Block, id.id);

    for statement in &block.statements {
        visit_statement(visitor, tree, *statement);
    }
}

/// Walk one statement.
pub fn walk_statement<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Statement>,
    statement: &Statement,
) {
    visitor.visit_any(tree, NodeType::Statement, id.id);

    match statement {
        Statement::Import {
            items, attributes, ..
        } => {
            if let Some(items) = items {
                for item in items {
                    visit_dependency_item(visitor, tree, *item);
                }
            }
            if let Some(attributes) = attributes {
                for property in &attributes.properties {
                    visit_property(visitor, tree, *property);
                }
            }
        }
        Statement::Export {
            items, attributes, ..
        } => {
            for item in items {
                visit_dependency_item(visitor, tree, *item);
            }
            if let Some(attributes) = attributes {
                for property in &attributes.properties {
                    visit_property(visitor, tree, *property);
                }
            }
        }
        Statement::ExportDefault { value } => visit_expression(visitor, tree, *value),
        Statement::Declaration { declaration } => visit_declaration(visitor, tree, *declaration),
        Statement::Block { block } => visit_block(visitor, tree, *block),
        Statement::Labelled { body, .. } => visit_statement(visitor, tree, *body),
        Statement::Let { declarators, .. }
        | Statement::Var { declarators, .. }
        | Statement::Using { declarators, .. } => {
            for declarator in declarators {
                visit_declarator(visitor, tree, *declarator);
            }
        }
        Statement::Assign { left, right, .. } => {
            visit_expression(visitor, tree, *left);
            visit_expression(visitor, tree, *right);
        }
        Statement::Expression { expression } => visit_expression(visitor, tree, *expression),
        Statement::If {
            condition,
            then_block,
            else_block,
        } => {
            visit_expression(visitor, tree, *condition);
            visit_block(visitor, tree, *then_block);
            if let Some(else_block) = else_block {
                visit_block(visitor, tree, *else_block);
            }
        }
        Statement::While { condition, body } | Statement::DoWhile { condition, body } => {
            visit_expression(visitor, tree, *condition);
            visit_block(visitor, tree, *body);
        }
        Statement::For {
            initialization,
            condition,
            increment,
            body,
        } => {
            if let Some(initialization) = initialization {
                walk_for_initialization(visitor, tree, initialization);
            }
            if let Some(condition) = condition {
                visit_expression(visitor, tree, *condition);
            }
            if let Some(increment) = increment {
                visit_expression(visitor, tree, *increment);
            }
            visit_block(visitor, tree, *body);
        }
        Statement::ForIn {
            pattern,
            iterator,
            body,
            ..
        }
        | Statement::ForOf {
            pattern,
            iterator,
            body,
            ..
        } => {
            visit_pattern(visitor, tree, *pattern);
            visit_expression(visitor, tree, *iterator);
            visit_block(visitor, tree, *body);
        }
        Statement::Switch { value, cases } => {
            visit_expression(visitor, tree, *value);
            for case in cases {
                visit_switch_case(visitor, tree, *case);
            }
        }
        Statement::Try {
            try_block,
            catch_clause,
            finally_block,
        } => {
            visit_block(visitor, tree, *try_block);
            if let Some(catch_clause) = catch_clause {
                visit_catch_clause(visitor, tree, *catch_clause);
            }
            if let Some(finally_block) = finally_block {
                visit_block(visitor, tree, *finally_block);
            }
        }
        Statement::Throw { value } => visit_expression(visitor, tree, *value),
        Statement::Return { value } => {
            if let Some(value) = value {
                visit_expression(visitor, tree, *value);
            }
        }
        Statement::Continue { .. } | Statement::Break { .. } | Statement::Debugger => {}
    }
}

/// Walk one expression.
pub fn walk_expression<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Expression>,
    expression: &Expression,
) {
    visitor.visit_any(tree, NodeType::Expression, id.id);

    match expression {
        Expression::Declaration { declaration } => visit_declaration(visitor, tree, *declaration),
        Expression::TemplateLiteral { value } => walk_template(visitor, tree, value),
        Expression::ArrayLiteral { elements } => {
            for element in elements {
                visit_array_element(visitor, tree, *element);
            }
        }
        Expression::SequenceExpression { expressions } => {
            for expression in expressions {
                visit_expression(visitor, tree, *expression);
            }
        }
        Expression::ObjectLiteral { properties } => {
            for property in properties {
                visit_property(visitor, tree, *property);
            }
        }
        Expression::Parenthesized { expression }
        | Expression::Await { value: expression }
        | Expression::Member {
            left: expression, ..
        }
        | Expression::PrivateMember {
            left: expression, ..
        } => visit_expression(visitor, tree, *expression),
        Expression::InstanceOf { value, target } => {
            visit_expression(visitor, tree, *value);
            visit_expression(visitor, tree, *target);
        }
        Expression::Unary { right, .. } => visit_expression(visitor, tree, *right),
        Expression::Binary { left, right, .. } | Expression::AssignBinary { left, right, .. } => {
            visit_expression(visitor, tree, *left);
            visit_expression(visitor, tree, *right);
        }
        Expression::Assign { left, right } => {
            visit_assign_pattern(visitor, tree, *left);
            visit_expression(visitor, tree, *right);
        }
        Expression::Index { left, right, .. } => {
            visit_expression(visitor, tree, *left);
            visit_expression(visitor, tree, *right);
        }
        Expression::Call {
            left, arguments, ..
        }
        | Expression::New {
            left, arguments, ..
        } => {
            visit_expression(visitor, tree, *left);
            for argument in arguments {
                visit_argument(visitor, tree, *argument);
            }
        }
        Expression::ImportCall {
            target, arguments, ..
        } => {
            visit_expression(visitor, tree, *target);
            for argument in arguments {
                visit_argument(visitor, tree, *argument);
            }
        }
        Expression::Yield { value, .. } => {
            if let Some(value) = value {
                visit_expression(visitor, tree, *value);
            }
        }
        Expression::ArrowFunction {
            parameters, body, ..
        } => {
            for parameter in parameters {
                visit_parameter(visitor, tree, *parameter);
            }
            match body {
                ArrowFunctionBody::Expression(expression) => {
                    visit_expression(visitor, tree, *expression);
                }
                ArrowFunctionBody::Block(block) => visit_block(visitor, tree, *block),
            }
        }
        Expression::IfTernary {
            condition,
            then_expression,
            else_expression,
        } => {
            visit_expression(visitor, tree, *condition);
            visit_expression(visitor, tree, *then_expression);
            visit_expression(visitor, tree, *else_expression);
        }
        Expression::Path { .. }
        | Expression::ImportMeta
        | Expression::This
        | Expression::Super
        | Expression::PrivateIdentifier { .. }
        | Expression::ScalarLiteral { .. }
        | Expression::Error => {}
    }
}

/// Walk one declaration.
pub fn walk_declaration<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Declaration>,
    declaration: &Declaration,
) {
    visitor.visit_any(tree, NodeType::Declaration, id.id);

    match declaration {
        Declaration::Class(ClassDeclaration {
            extends_expression,
            members,
            ..
        }) => {
            if let Some(extends) = extends_expression {
                visit_expression(visitor, tree, *extends);
            }
            for member in members {
                visit_member(visitor, tree, *member);
            }
        }
        Declaration::Function(FunctionDeclaration {
            signature, body, ..
        }) => {
            walk_signature(visitor, tree, signature);
            visit_block(visitor, tree, *body);
        }
    }
}

/// Walk one declarator.
pub fn walk_declarator<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Declarator>,
    declarator: &Declarator,
) {
    visitor.visit_any(tree, NodeType::Declarator, id.id);
    visit_pattern(visitor, tree, declarator.pattern);
    if let Some(value) = declarator.value {
        visit_expression(visitor, tree, value);
    }
}

/// Walk one property.
pub fn walk_property<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Property>,
    property: &Property,
) {
    visitor.visit_any(tree, NodeType::Property, id.id);

    match property {
        Property::Field { key, value, .. } => {
            walk_key(visitor, tree, key);
            visit_expression(visitor, tree, *value);
        }
        Property::Method {
            key,
            signature,
            body,
            ..
        } => {
            walk_key(visitor, tree, key);
            walk_signature(visitor, tree, signature);
            visit_block(visitor, tree, *body);
        }
        Property::Spread { value } => visit_expression(visitor, tree, *value),
    }
}

/// Walk one class member.
pub fn walk_member<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Member>,
    member: &Member,
) {
    visitor.visit_any(tree, NodeType::Member, id.id);

    match member {
        Member::Field { key, default, .. } => {
            walk_key(visitor, tree, key);
            if let Some(default) = default {
                visit_expression(visitor, tree, *default);
            }
        }
        Member::Method {
            key,
            signature,
            body,
            ..
        } => {
            walk_key(visitor, tree, key);
            walk_signature(visitor, tree, signature);
            visit_block(visitor, tree, *body);
        }
        Member::Constructor { parameters, body } => {
            for parameter in parameters {
                visit_parameter(visitor, tree, *parameter);
            }
            visit_block(visitor, tree, *body);
        }
        Member::StaticBlock { body } => visit_block(visitor, tree, *body),
    }
}

/// Walk one dependency item.
pub fn walk_dependency_item<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<DependencyItem>,
    _item: &DependencyItem,
) {
    visitor.visit_any(tree, NodeType::DependencyItem, id.id);
}

/// Walk one switch case.
pub fn walk_switch_case<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<SwitchCase>,
    case: &SwitchCase,
) {
    visitor.visit_any(tree, NodeType::SwitchCase, id.id);
    if let Some(value) = case.value {
        visit_expression(visitor, tree, value);
    }
    visit_block(visitor, tree, case.body);
}

/// Walk one catch clause.
pub fn walk_catch_clause<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<CatchClause>,
    clause: &CatchClause,
) {
    visitor.visit_any(tree, NodeType::CatchClause, id.id);
    if let Some(pattern) = clause.pattern {
        visit_pattern(visitor, tree, pattern);
    }
    visit_block(visitor, tree, clause.body);
}

/// Walk one parameter.
pub fn walk_parameter<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Parameter>,
    parameter: &Parameter,
) {
    visitor.visit_any(tree, NodeType::Parameter, id.id);

    match parameter {
        Parameter::Named { default, .. } => {
            if let Some(default) = default {
                visit_expression(visitor, tree, *default);
            }
        }
        Parameter::Pattern { pattern, default } => {
            visit_pattern(visitor, tree, *pattern);
            if let Some(default) = default {
                visit_expression(visitor, tree, *default);
            }
        }
        Parameter::VariadicNamed { .. } => {}
        Parameter::VariadicPattern { pattern } => visit_pattern(visitor, tree, *pattern),
    }
}

/// Walk one argument.
pub fn walk_argument<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Argument>,
    argument: &Argument,
) {
    visitor.visit_any(tree, NodeType::Argument, id.id);
    let value = match argument {
        Argument::Positional { value } | Argument::Spread { value } => *value,
    };
    visit_expression(visitor, tree, value);
}

/// Walk one array element.
pub fn walk_array_element<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<ArrayElement>,
    element: &ArrayElement,
) {
    visitor.visit_any(tree, NodeType::ArrayElement, id.id);
    match element {
        ArrayElement::Expression { value } | ArrayElement::Spread { value } => {
            visit_expression(visitor, tree, *value);
        }
        ArrayElement::Elision => {}
    }
}

/// Walk one binding pattern.
pub fn walk_pattern<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Pattern>,
    pattern: &Pattern,
) {
    visitor.visit_any(tree, NodeType::Pattern, id.id);

    match pattern {
        Pattern::Assign { pattern, value } => {
            visit_pattern(visitor, tree, *pattern);
            visit_expression(visitor, tree, *value);
        }
        Pattern::Array { fields } | Pattern::Object { fields } => {
            for field in fields {
                visit_pattern_field(visitor, tree, *field);
            }
        }
        Pattern::Binding { .. } | Pattern::Hole => {}
    }
}

/// Walk one binding pattern field.
pub fn walk_pattern_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<PatternField>,
    field: &PatternField,
) {
    visitor.visit_any(tree, NodeType::PatternField, id.id);

    match field {
        PatternField::Named { pattern, .. } | PatternField::Spread { pattern } => {
            visit_pattern(visitor, tree, *pattern);
        }
        PatternField::Shorthand { value, .. } => {
            if let Some(value) = value {
                visit_expression(visitor, tree, *value);
            }
        }
        PatternField::Computed { key, pattern } => {
            visit_expression(visitor, tree, *key);
            visit_pattern(visitor, tree, *pattern);
        }
        PatternField::Positional { pattern } => visit_pattern(visitor, tree, *pattern),
        PatternField::Elision => {}
    }
}

/// Walk one assignment pattern.
pub fn walk_assign_pattern<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<AssignPattern>,
    pattern: &AssignPattern,
) {
    visitor.visit_any(tree, NodeType::AssignPattern, id.id);

    match pattern {
        AssignPattern::Expression { value } => visit_expression(visitor, tree, *value),
        AssignPattern::Assign { pattern, value } => {
            visit_assign_pattern(visitor, tree, *pattern);
            visit_expression(visitor, tree, *value);
        }
        AssignPattern::Array { fields } | AssignPattern::Object { fields } => {
            for field in fields {
                visit_assign_pattern_field(visitor, tree, *field);
            }
        }
    }
}

/// Walk one assignment pattern field.
pub fn walk_assign_pattern_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<AssignPatternField>,
    field: &AssignPatternField,
) {
    visitor.visit_any(tree, NodeType::AssignPatternField, id.id);

    match field {
        AssignPatternField::Named { pattern, .. } | AssignPatternField::Spread { pattern } => {
            visit_assign_pattern(visitor, tree, *pattern);
        }
        AssignPatternField::Shorthand { value, .. } => {
            if let Some(value) = value {
                visit_expression(visitor, tree, *value);
            }
        }
        AssignPatternField::Computed { key, pattern } => {
            visit_expression(visitor, tree, *key);
            visit_assign_pattern(visitor, tree, *pattern);
        }
        AssignPatternField::Positional { pattern } => {
            visit_assign_pattern(visitor, tree, *pattern);
        }
        AssignPatternField::Elision => {}
    }
}

/// Walk one annotation.
pub fn walk_annotation<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Annotation>,
    _annotation: &Annotation,
) {
    visitor.visit_any(tree, NodeType::Annotation, id.id);
}

/// Walk one key.
fn walk_key<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &Tree, key: &Key) {
    if let Key::Expression(expression) = key {
        visit_expression(visitor, tree, *expression);
    }
}

/// Walk one function signature.
fn walk_signature<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    signature: &FunctionSignature,
) {
    for parameter in &signature.parameters {
        visit_parameter(visitor, tree, *parameter);
    }
}

/// Walk one template literal.
fn walk_template<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &Tree, value: &TemplateLiteral) {
    match value {
        TemplateLiteral::InterpolatedString { expressions, .. }
        | TemplateLiteral::TaggedInterpolatedString { expressions, .. } => {
            for expression in expressions {
                visit_expression(visitor, tree, *expression);
            }
        }
        TemplateLiteral::String { .. } | TemplateLiteral::TaggedString { .. } => {}
    }
}

/// Walk one for initializer.
fn walk_for_initialization<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    initialization: &ForInitialization,
) {
    match initialization {
        ForInitialization::Expression(expression) => visit_expression(visitor, tree, *expression),
        ForInitialization::Declaration { declarators, .. } => {
            for declarator in declarators {
                visit_declarator(visitor, tree, *declarator);
            }
        }
    }
}

fn visit_block<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &Tree, id: LocalNodeId<Block>) {
    visitor.visit_block(tree, id, tree.get(id));
}

fn visit_catch_clause<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<CatchClause>,
) {
    visitor.visit_catch_clause(tree, id, tree.get(id));
}

fn visit_statement<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Statement>,
) {
    visitor.visit_statement(tree, id, tree.get(id));
}

fn visit_expression<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Expression>,
) {
    visitor.visit_expression(tree, id, tree.get(id));
}

fn visit_array_element<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<ArrayElement>,
) {
    visitor.visit_array_element(tree, id, tree.get(id));
}

fn visit_declaration<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Declaration>,
) {
    visitor.visit_declaration(tree, id, tree.get(id));
}

fn visit_declarator<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Declarator>,
) {
    visitor.visit_declarator(tree, id, tree.get(id));
}

fn visit_property<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Property>,
) {
    visitor.visit_property(tree, id, tree.get(id));
}

fn visit_member<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &Tree, id: LocalNodeId<Member>) {
    visitor.visit_member(tree, id, tree.get(id));
}

fn visit_dependency_item<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<DependencyItem>,
) {
    visitor.visit_dependency_item(tree, id, tree.get(id));
}

fn visit_switch_case<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<SwitchCase>,
) {
    visitor.visit_switch_case(tree, id, tree.get(id));
}

fn visit_parameter<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Parameter>,
) {
    visitor.visit_parameter(tree, id, tree.get(id));
}

fn visit_argument<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Argument>,
) {
    visitor.visit_argument(tree, id, tree.get(id));
}

fn visit_pattern<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &Tree, id: LocalNodeId<Pattern>) {
    visitor.visit_pattern(tree, id, tree.get(id));
}

fn visit_pattern_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<PatternField>,
) {
    visitor.visit_pattern_field(tree, id, tree.get(id));
}

fn visit_assign_pattern<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<AssignPattern>,
) {
    visitor.visit_assign_pattern(tree, id, tree.get(id));
}

fn visit_assign_pattern_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<AssignPatternField>,
) {
    visitor.visit_assign_pattern_field(tree, id, tree.get(id));
}

fn visit_annotation<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Annotation>,
) {
    visitor.visit_annotation(tree, id, tree.get(id));
}
