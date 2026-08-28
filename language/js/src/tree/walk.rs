use crate::{
    Argument, ArrayAssignPatternField, ArrayElement, ArrayPatternField, ArrowFunctionBody,
    AssignPattern, Block, CatchClause, ClassDeclaration, ClassElementName, Declaration, Declarator,
    ExportSpecifier, Expression, ForInitialization, FunctionDeclaration, FunctionSignature,
    ImportAttribute, ImportAttributeName, ImportClause, ImportSpecifier, IterationTarget,
    LocalNodeId, Member, ModuleExportName, NodeType, NodeVisitor, ObjectAssignPatternField,
    ObjectPatternField, Parameter, Pattern, Place, Property, PropertyName, ReExportSpecifier,
    Statement, SwitchCase, TemplateLiteral, Tree,
};

/// Walk one root through visitor entry points.
pub fn walk_root<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    root: LocalNodeId<Statement>,
) {
    visit_statement(visitor, tree, root);
}

/// Walk one root list through visitor entry points.
pub fn walk_roots<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    roots: &[LocalNodeId<Statement>],
) {
    for root in roots.iter().copied() {
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
            clause, attributes, ..
        } => {
            if let Some(clause) = clause {
                walk_import_clause(visitor, tree, clause);
            }
            if let Some(attributes) = attributes {
                for attribute in attributes {
                    visit_import_attribute(visitor, tree, *attribute);
                }
            }
        }
        Statement::Export { specifiers } => {
            for specifier in specifiers {
                visit_export_specifier(visitor, tree, *specifier);
            }
        }
        Statement::ReExport {
            specifiers,
            attributes,
            ..
        } => {
            for specifier in specifiers {
                visit_re_export_specifier(visitor, tree, *specifier);
            }
            if let Some(attributes) = attributes {
                for attribute in attributes {
                    visit_import_attribute(visitor, tree, *attribute);
                }
            }
        }
        Statement::ExportAll {
            exported,
            attributes,
            ..
        } => {
            if let Some(exported) = exported {
                walk_module_export_name(visitor, exported);
            }
            if let Some(attributes) = attributes {
                for attribute in attributes {
                    visit_import_attribute(visitor, tree, *attribute);
                }
            }
        }
        Statement::ExportDefault { value } => visit_expression(visitor, tree, *value),
        Statement::Declaration { declaration, .. } => {
            visit_declaration(visitor, tree, *declaration);
        }
        Statement::Block { block } => visit_block(visitor, tree, *block),
        Statement::Labelled { label, body } => {
            visitor.visit_identifier(*label);
            visit_statement(visitor, tree, *body);
        }
        Statement::Let { declarators, .. }
        | Statement::Var { declarators, .. }
        | Statement::Using { declarators, .. } => {
            for declarator in declarators {
                visit_declarator(visitor, tree, *declarator);
            }
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
            target,
            iterator,
            body,
        }
        | Statement::ForOf {
            target,
            iterator,
            body,
            ..
        } => {
            walk_iteration_target(visitor, tree, target);
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
        Statement::Continue { label } | Statement::Break { label } => {
            if let Some(label) = label {
                visitor.visit_identifier(*label);
            }
        }
        Statement::Debugger => {}
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
        Expression::Parenthesized { expression } | Expression::Await { value: expression } => {
            visit_expression(visitor, tree, *expression);
        }
        Expression::Member {
            object, property, ..
        } => {
            visit_expression(visitor, tree, *object);
            visitor.visit_identifier_name(*property);
        }
        Expression::PrivateMember { object, property } => {
            visit_expression(visitor, tree, *object);
            visitor.visit_identifier(*property);
        }
        Expression::Unary { right, .. } => visit_expression(visitor, tree, *right),
        Expression::Update { place, .. } => visit_place(visitor, tree, *place),
        Expression::Binary { left, right, .. } => {
            visit_expression(visitor, tree, *left);
            visit_expression(visitor, tree, *right);
        }
        Expression::AssignBinary { left, right, .. } => {
            visit_place(visitor, tree, *left);
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
        Expression::ImportCall { specifier, options } => {
            visit_expression(visitor, tree, *specifier);
            if let Some(options) = options {
                visit_expression(visitor, tree, *options);
            }
        }
        Expression::Yield { value, .. } => {
            if let Some(value) = value {
                visit_expression(visitor, tree, *value);
            }
        }
        Expression::ArrowFunction {
            parameters,
            rest,
            body,
            ..
        } => {
            for parameter in parameters {
                visit_parameter(visitor, tree, *parameter);
            }
            if let Some(rest) = rest {
                visit_pattern(visitor, tree, *rest);
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
        Expression::Identifier { identifier } => visitor.visit_identifier(*identifier),
        Expression::PrivateIn { identifier, object } => {
            visitor.visit_identifier(*identifier);
            visit_expression(visitor, tree, *object);
        }
        Expression::ImportMeta
        | Expression::This
        | Expression::Super
        | Expression::Literal { .. } => {}
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
            name,
            extends_expression,
            members,
        }) => {
            if let Some(name) = name {
                visitor.visit_identifier(*name);
            }
            if let Some(extends) = extends_expression {
                visit_expression(visitor, tree, *extends);
            }
            for member in members {
                visit_member(visitor, tree, *member);
            }
        }
        Declaration::Function(FunctionDeclaration {
            name,
            signature,
            body,
        }) => {
            if let Some(name) = name {
                visitor.visit_identifier(*name);
            }
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
            walk_property_name(visitor, tree, key);
            visit_expression(visitor, tree, *value);
        }
        Property::Method {
            key,
            signature,
            body,
            ..
        } => {
            walk_property_name(visitor, tree, key);
            walk_signature(visitor, tree, signature);
            visit_block(visitor, tree, *body);
        }
        Property::Getter { key, body } => {
            walk_property_name(visitor, tree, key);
            visit_block(visitor, tree, *body);
        }
        Property::Setter {
            key,
            parameter,
            body,
        } => {
            walk_property_name(visitor, tree, key);
            visit_parameter(visitor, tree, *parameter);
            visit_block(visitor, tree, *body);
        }
        Property::Shorthand { value } => visitor.visit_identifier(*value),
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
            walk_class_element_name(visitor, tree, key);
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
            walk_class_element_name(visitor, tree, key);
            walk_signature(visitor, tree, signature);
            visit_block(visitor, tree, *body);
        }
        Member::Getter { key, body, .. } => {
            walk_class_element_name(visitor, tree, key);
            visit_block(visitor, tree, *body);
        }
        Member::Setter {
            key,
            parameter,
            body,
            ..
        } => {
            walk_class_element_name(visitor, tree, key);
            visit_parameter(visitor, tree, *parameter);
            visit_block(visitor, tree, *body);
        }
        Member::Constructor {
            parameters,
            rest,
            body,
        } => {
            for parameter in parameters {
                visit_parameter(visitor, tree, *parameter);
            }
            if let Some(rest) = rest {
                visit_pattern(visitor, tree, *rest);
            }
            visit_block(visitor, tree, *body);
        }
        Member::StaticBlock { body } => visit_block(visitor, tree, *body),
    }
}

/// Walk one import specifier.
pub fn walk_import_specifier<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<ImportSpecifier>,
    specifier: &ImportSpecifier,
) {
    visitor.visit_any(tree, NodeType::ImportSpecifier, id.id);
    walk_module_export_name(visitor, &specifier.imported);
    visitor.visit_identifier(specifier.local);
}

/// Walk one export specifier.
pub fn walk_export_specifier<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<ExportSpecifier>,
    specifier: &ExportSpecifier,
) {
    visitor.visit_any(tree, NodeType::ExportSpecifier, id.id);
    visitor.visit_identifier(specifier.local);
    walk_module_export_name(visitor, &specifier.exported);
}

/// Walk one re-export specifier.
pub fn walk_re_export_specifier<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<ReExportSpecifier>,
    specifier: &ReExportSpecifier,
) {
    visitor.visit_any(tree, NodeType::ReExportSpecifier, id.id);
    walk_module_export_name(visitor, &specifier.imported);
    walk_module_export_name(visitor, &specifier.exported);
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
    for statement in &case.body {
        visit_statement(visitor, tree, *statement);
    }
}

/// Walk one import attribute.
pub fn walk_import_attribute<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<ImportAttribute>,
    attribute: &ImportAttribute,
) {
    visitor.visit_any(tree, NodeType::ImportAttribute, id.id);
    if let ImportAttributeName::Identifier(name) = attribute.name {
        visitor.visit_identifier_name(name);
    }
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
        Parameter::Named { name, default } => {
            visitor.visit_identifier(*name);
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
        Pattern::Array { fields, rest } => {
            for field in fields {
                visit_array_pattern_field(visitor, tree, *field);
            }
            if let Some(rest) = rest {
                visit_pattern(visitor, tree, *rest);
            }
        }
        Pattern::Object { fields, rest } => {
            for field in fields {
                visit_object_pattern_field(visitor, tree, *field);
            }
            if let Some(rest) = rest {
                visitor.visit_identifier(*rest);
            }
        }
        Pattern::Binding { identifier } => visitor.visit_identifier(*identifier),
    }
}

/// Walk one array binding field.
pub fn walk_array_pattern_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<ArrayPatternField>,
    field: &ArrayPatternField,
) {
    visitor.visit_any(tree, NodeType::ArrayPatternField, id.id);

    match field {
        ArrayPatternField::Positional { pattern, default } => {
            visit_pattern(visitor, tree, *pattern);
            if let Some(default) = default {
                visit_expression(visitor, tree, *default);
            }
        }
        ArrayPatternField::Elision => {}
    }
}

/// Walk one object binding field.
pub fn walk_object_pattern_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<ObjectPatternField>,
    field: &ObjectPatternField,
) {
    visitor.visit_any(tree, NodeType::ObjectPatternField, id.id);

    match field {
        ObjectPatternField::Named {
            name,
            pattern,
            default,
        } => {
            walk_property_name(visitor, tree, name);
            visit_pattern(visitor, tree, *pattern);
            if let Some(default) = default {
                visit_expression(visitor, tree, *default);
            }
        }
        ObjectPatternField::Shorthand {
            identifier,
            default,
        } => {
            visitor.visit_identifier(*identifier);
            if let Some(default) = default {
                visit_expression(visitor, tree, *default);
            }
        }
    }
}

/// Walk one writable place.
pub fn walk_place<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Place>,
    place: &Place,
) {
    visitor.visit_any(tree, NodeType::Place, id.id);

    match place {
        Place::Identifier { identifier } => visitor.visit_identifier(*identifier),
        Place::Member {
            object, property, ..
        } => {
            visit_expression(visitor, tree, *object);
            visitor.visit_identifier_name(*property);
        }
        Place::PrivateMember { object, property } => {
            visit_expression(visitor, tree, *object);
            visitor.visit_identifier(*property);
        }
        Place::Index { object, key } => {
            visit_expression(visitor, tree, *object);
            visit_expression(visitor, tree, *key);
        }
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
        AssignPattern::Place { place } => visit_place(visitor, tree, *place),
        AssignPattern::Array { fields, rest } => {
            for field in fields {
                visit_array_assign_pattern_field(visitor, tree, *field);
            }
            if let Some(rest) = rest {
                visit_assign_pattern(visitor, tree, *rest);
            }
        }
        AssignPattern::Object { fields, rest } => {
            for field in fields {
                visit_object_assign_pattern_field(visitor, tree, *field);
            }
            if let Some(rest) = rest {
                visit_place(visitor, tree, *rest);
            }
        }
    }
}

/// Walk one array assignment field.
pub fn walk_array_assign_pattern_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<ArrayAssignPatternField>,
    field: &ArrayAssignPatternField,
) {
    visitor.visit_any(tree, NodeType::ArrayAssignPatternField, id.id);

    match field {
        ArrayAssignPatternField::Positional { pattern, default } => {
            visit_assign_pattern(visitor, tree, *pattern);
            if let Some(default) = default {
                visit_expression(visitor, tree, *default);
            }
        }
        ArrayAssignPatternField::Elision => {}
    }
}

/// Walk one object assignment field.
pub fn walk_object_assign_pattern_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<ObjectAssignPatternField>,
    field: &ObjectAssignPatternField,
) {
    visitor.visit_any(tree, NodeType::ObjectAssignPatternField, id.id);

    match field {
        ObjectAssignPatternField::Named {
            name,
            pattern,
            default,
        } => {
            walk_property_name(visitor, tree, name);
            visit_assign_pattern(visitor, tree, *pattern);
            if let Some(default) = default {
                visit_expression(visitor, tree, *default);
            }
        }
        ObjectAssignPatternField::Shorthand {
            identifier,
            default,
        } => {
            visitor.visit_identifier(*identifier);
            if let Some(default) = default {
                visit_expression(visitor, tree, *default);
            }
        }
    }
}

/// Walk one property name.
fn walk_property_name<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &Tree, name: &PropertyName) {
    match name {
        PropertyName::Identifier(name) => visitor.visit_identifier_name(*name),
        PropertyName::Computed(expression) => visit_expression(visitor, tree, *expression),
        PropertyName::String(_) => {}
    }
}

/// Walk one class element name.
fn walk_class_element_name<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    name: &ClassElementName,
) {
    match name {
        ClassElementName::Public(name) => walk_property_name(visitor, tree, name),
        ClassElementName::Private(identifier) => visitor.visit_identifier(*identifier),
    }
}

/// Walk one module export name.
fn walk_module_export_name<V: NodeVisitor + ?Sized>(visitor: &mut V, name: &ModuleExportName) {
    if let ModuleExportName::Identifier(name) = name {
        visitor.visit_identifier_name(*name);
    }
}

/// Walk one import clause.
fn walk_import_clause<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    clause: &ImportClause,
) {
    match clause {
        ImportClause::Default { local } => visitor.visit_identifier(*local),
        ImportClause::Namespace { default, local } => {
            if let Some(default) = default {
                visitor.visit_identifier(*default);
            }
            visitor.visit_identifier(*local);
        }
        ImportClause::Named {
            default,
            specifiers,
        } => {
            if let Some(default) = default {
                visitor.visit_identifier(*default);
            }
            for specifier in specifiers {
                visit_import_specifier(visitor, tree, *specifier);
            }
        }
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
    if let Some(rest) = signature.rest {
        visit_pattern(visitor, tree, rest);
    }
}

/// Walk one template literal.
fn walk_template<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &Tree, value: &TemplateLiteral) {
    if let Some(tag) = value.tag {
        visit_expression(visitor, tree, tag);
    }
    for substitution in &value.substitutions {
        visit_expression(visitor, tree, substitution.expression);
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

/// Walk one `for in` or `for of` target.
fn walk_iteration_target<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    target: &IterationTarget,
) {
    match target {
        IterationTarget::Binding { pattern, .. } => visit_pattern(visitor, tree, *pattern),
        IterationTarget::Assignment { pattern } => visit_assign_pattern(visitor, tree, *pattern),
        IterationTarget::Using { pattern, .. } => visit_pattern(visitor, tree, *pattern),
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

fn visit_import_specifier<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<ImportSpecifier>,
) {
    visitor.visit_import_specifier(tree, id, tree.get(id));
}

fn visit_export_specifier<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<ExportSpecifier>,
) {
    visitor.visit_export_specifier(tree, id, tree.get(id));
}

fn visit_re_export_specifier<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<ReExportSpecifier>,
) {
    visitor.visit_re_export_specifier(tree, id, tree.get(id));
}

fn visit_import_attribute<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<ImportAttribute>,
) {
    visitor.visit_import_attribute(tree, id, tree.get(id));
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

fn visit_array_pattern_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<ArrayPatternField>,
) {
    visitor.visit_array_pattern_field(tree, id, tree.get(id));
}

fn visit_object_pattern_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<ObjectPatternField>,
) {
    visitor.visit_object_pattern_field(tree, id, tree.get(id));
}

fn visit_place<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &Tree, id: LocalNodeId<Place>) {
    visitor.visit_place(tree, id, tree.get(id));
}

fn visit_assign_pattern<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<AssignPattern>,
) {
    visitor.visit_assign_pattern(tree, id, tree.get(id));
}

fn visit_array_assign_pattern_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<ArrayAssignPatternField>,
) {
    visitor.visit_array_assign_pattern_field(tree, id, tree.get(id));
}

fn visit_object_assign_pattern_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<ObjectAssignPatternField>,
) {
    visitor.visit_object_assign_pattern_field(tree, id, tree.get(id));
}
