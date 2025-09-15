use crate::{
    Argument, ArrayLiteral, Block, Break, Call, Cast, Coalesce, Continue, Defer, Enum, EnumField,
    Expression, For, Function, If, Implement, Index, Let, Loop, Match, MatchCase, Module, NodeTree,
    NodeVisitor, Parameter, Pattern, PatternField, RangeLiteral, Return, ScalarLiteral, Statement,
    Struct, StructField, StructLiteral, Trait, Try, Tuple, TupleField, TupleLiteral, Type, Union,
    UnionField, Use, UseClause, UseItem, While, With, WithClause,
};

// ----------------------------------------------------------------------------
// Groupings
// ----------------------------------------------------------------------------

/// Walk the Block's children.
pub fn walk_block<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, block: &Block) {
    for statement_id in &block.statements {
        let statement = tree.get(*statement_id);
        visitor.visit_statement(tree, *statement_id, statement);
    }
}

/// Walk the Statement's children.
pub fn walk_statement<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    statement: &Statement,
) {
    match statement {
        Statement::Expression(node) => visitor.visit_expression(tree, *node, tree.get(*node)),
        Statement::Module(node) => visitor.visit_module(tree, *node, tree.get(*node)),
        Statement::Struct(node) => visitor.visit_struct(tree, *node, tree.get(*node)),
        Statement::Enum(node) => visitor.visit_enum(tree, *node, tree.get(*node)),
        Statement::Union(node) => visitor.visit_union(tree, *node, tree.get(*node)),
        Statement::Trait(node) => visitor.visit_trait(tree, *node, tree.get(*node)),
        Statement::Implement(node) => visitor.visit_implement(tree, *node, tree.get(*node)),
        Statement::Function(node) => visitor.visit_function(tree, *node, tree.get(*node)),
        Statement::With(node) => visitor.visit_with(tree, *node, tree.get(*node)),
        Statement::Use(node) => visitor.visit_use(tree, *node, tree.get(*node)),
    }
}

/// Walk the Expression's children.
pub fn walk_expression<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    expression: &Expression,
) {
    match expression {
        Expression::Module(node) => visitor.visit_module(tree, *node, tree.get(*node)),
        Expression::Struct(node) => visitor.visit_struct(tree, *node, tree.get(*node)),
        Expression::Enum(node) => visitor.visit_enum(tree, *node, tree.get(*node)),
        Expression::Union(node) => visitor.visit_union(tree, *node, tree.get(*node)),
        Expression::Trait(node) => visitor.visit_trait(tree, *node, tree.get(*node)),
        Expression::Implement(node) => visitor.visit_implement(tree, *node, tree.get(*node)),
        Expression::Function(node) => visitor.visit_function(tree, *node, tree.get(*node)),

        Expression::Let(node) => visitor.visit_let(tree, *node, tree.get(*node)),
        Expression::Block(node) => visitor.visit_block(tree, *node, tree.get(*node)),
        Expression::If(node) => visitor.visit_if(tree, *node, tree.get(*node)),
        Expression::While(node) => visitor.visit_while(tree, *node, tree.get(*node)),
        Expression::For(node) => visitor.visit_for(tree, *node, tree.get(*node)),
        Expression::Loop(node) => visitor.visit_loop(tree, *node, tree.get(*node)),
        Expression::Break(node) => visitor.visit_break(tree, *node, tree.get(*node)),
        Expression::Continue(node) => visitor.visit_continue(tree, *node, tree.get(*node)),
        Expression::Defer(node) => visitor.visit_defer(tree, *node, tree.get(*node)),
        Expression::Return(node) => visitor.visit_return(tree, *node, tree.get(*node)),
        Expression::Try(node) => visitor.visit_try(tree, *node, tree.get(*node)),
        Expression::Match(node) => visitor.visit_match(tree, *node, tree.get(*node)),

        Expression::Path(_) => {}
        Expression::ScalarLiteral(node) => {
            visitor.visit_scalar_literal(tree, *node, tree.get(*node))
        }
        Expression::RangeLiteral(node) => visitor.visit_range_literal(tree, *node, tree.get(*node)),
        Expression::ArrayLiteral(node) => visitor.visit_array_literal(tree, *node, tree.get(*node)),
        Expression::TupleLiteral(node) => visitor.visit_tuple_literal(tree, *node, tree.get(*node)),
        Expression::StructLiteral(node) => {
            visitor.visit_struct_literal(tree, *node, tree.get(*node))
        }

        Expression::Unary { operator: _, right } => {
            visitor.visit_expression(tree, *right, tree.get(*right));
        }
        Expression::Reference {
            mutability: _,
            right,
        } => {
            visitor.visit_expression(tree, *right, tree.get(*right));
        }
        Expression::Binary {
            left,
            operator: _,
            right,
        } => {
            visitor.visit_expression(tree, *left, tree.get(*left));
            visitor.visit_expression(tree, *right, tree.get(*right));
        }
        Expression::Assign {
            left,
            operator: _,
            right,
        } => {
            visitor.visit_expression(tree, *left, tree.get(*left));
            visitor.visit_expression(tree, *right, tree.get(*right));
        }
        Expression::Member { receiver, path: _ } => {
            visitor.visit_expression(tree, *receiver, tree.get(*receiver));
        }
        Expression::Index(node) => visitor.visit_index(tree, *node, tree.get(*node)),
        Expression::Unwrap(node) => visitor.visit_expression(tree, *node, tree.get(*node)),
        Expression::Call(node) => visitor.visit_call(tree, *node, tree.get(*node)),
        Expression::Cast(node) => visitor.visit_cast(tree, *node, tree.get(*node)),
        Expression::Coalesce(node) => visitor.visit_coalesce(tree, *node, tree.get(*node)),

        Expression::Error(_) => {}
    }
}

// ----------------------------------------------------------------------------
// Declarations
// ----------------------------------------------------------------------------

/// Walk the Module's children.
pub fn walk_module<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, module: &Module) {
    for statement_id in &module.statements {
        let statement = tree.get(*statement_id);
        visitor.visit_statement(tree, *statement_id, statement);
    }
}

/// Walk the Struct's children.
pub fn walk_struct<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    struct_node: &Struct,
) {
    if let Some(super_types) = &struct_node.super_types {
        for type_id in super_types {
            let type_node = tree.get(*type_id);
            visitor.visit_type(tree, *type_id, type_node);
        }
    }

    if let Some(representation_type) = &struct_node.representation_type {
        let type_node = tree.get(*representation_type);
        visitor.visit_type(tree, *representation_type, type_node);
    }

    if let Some(static_parameters) = &struct_node.static_parameters {
        for parameter_id in static_parameters {
            let parameter = tree.get(*parameter_id);
            visitor.visit_parameter(tree, *parameter_id, parameter);
        }
    }

    for field_id in &struct_node.fields {
        let field = tree.get(*field_id);
        visitor.visit_struct_field(tree, *field_id, field);
    }

    for statement_id in &struct_node.statements {
        let statement = tree.get(*statement_id);
        visitor.visit_statement(tree, *statement_id, statement);
    }
}

/// Walk the StructField's children.
pub fn walk_struct_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    field: &StructField,
) {
    let type_node = tree.get(field.r#type);
    visitor.visit_type(tree, field.r#type, type_node);

    if let Some(default) = &field.default {
        let expression = tree.get(*default);
        visitor.visit_expression(tree, *default, expression);
    }
}

/// Walk the Enum's children.
pub fn walk_enum<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, enum_node: &Enum) {
    if let Some(type_node) = &enum_node.r#type {
        let type_ref = tree.get(*type_node);
        visitor.visit_type(tree, *type_node, type_ref);
    }

    if let Some(super_types) = &enum_node.super_types {
        for type_id in super_types {
            let type_node = tree.get(*type_id);
            visitor.visit_type(tree, *type_id, type_node);
        }
    }

    for field_id in &enum_node.fields {
        let field = tree.get(*field_id);
        visitor.visit_enum_field(tree, *field_id, field);
    }

    for statement_id in &enum_node.statements {
        let statement = tree.get(*statement_id);
        visitor.visit_statement(tree, *statement_id, statement);
    }
}

/// Walk the EnumField's children.
pub fn walk_enum_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    field: &EnumField,
) {
    if let Some(value) = &field.value {
        let expression = tree.get(*value);
        visitor.visit_expression(tree, *value, expression);
    }
}

/// Walk the Union's children.
pub fn walk_union<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, union_node: &Union) {
    if let Some(tag_type) = &union_node.tag_type {
        let type_node = tree.get(*tag_type);
        visitor.visit_type(tree, *tag_type, type_node);
    }

    if let Some(representation_type) = &union_node.representation_type {
        let type_node = tree.get(*representation_type);
        visitor.visit_type(tree, *representation_type, type_node);
    }

    if let Some(static_parameters) = &union_node.static_parameters {
        for parameter_id in static_parameters {
            let parameter = tree.get(*parameter_id);
            visitor.visit_parameter(tree, *parameter_id, parameter);
        }
    }

    if let Some(super_types) = &union_node.super_types {
        for type_id in super_types {
            let type_node = tree.get(*type_id);
            visitor.visit_type(tree, *type_id, type_node);
        }
    }

    for field_id in &union_node.fields {
        let field = tree.get(*field_id);
        visitor.visit_union_field(tree, *field_id, field);
    }

    for statement_id in &union_node.statements {
        let statement = tree.get(*statement_id);
        visitor.visit_statement(tree, *statement_id, statement);
    }
}

/// Walk the UnionField's children.
pub fn walk_union_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    field: &UnionField,
) {
    if let Some(type_node) = &field.r#type {
        let type_ref = tree.get(*type_node);
        visitor.visit_type(tree, *type_node, type_ref);
    }

    if let Some(value) = &field.value {
        let expression = tree.get(*value);
        visitor.visit_expression(tree, *value, expression);
    }
}

/// Walk the Trait's children.
pub fn walk_trait<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, trait_node: &Trait) {
    if let Some(super_types) = &trait_node.super_types {
        for type_id in super_types {
            let type_node = tree.get(*type_id);
            visitor.visit_type(tree, *type_id, type_node);
        }
    }

    if let Some(static_parameters) = &trait_node.static_parameters {
        for parameter_id in static_parameters {
            let parameter = tree.get(*parameter_id);
            visitor.visit_parameter(tree, *parameter_id, parameter);
        }
    }

    for with_id in &trait_node.withs {
        let with_node = tree.get(*with_id);
        visitor.visit_with(tree, *with_id, with_node);
    }

    for statement_id in &trait_node.statements {
        let statement = tree.get(*statement_id);
        visitor.visit_statement(tree, *statement_id, statement);
    }
}

/// Walk the Implement's children.
pub fn walk_implement<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    implement: &Implement,
) {
    if let Some(static_arguments) = &implement.static_arguments {
        for argument_id in static_arguments {
            let argument = tree.get(*argument_id);
            visitor.visit_argument(tree, *argument_id, argument);
        }
    }

    let receiver = tree.get(implement.receiver);
    visitor.visit_type(tree, implement.receiver, receiver);

    if let Some(for_trait) = &implement.for_trait {
        let trait_type = tree.get(*for_trait);
        visitor.visit_type(tree, *for_trait, trait_type);
    }

    for statement_id in &implement.statements {
        let statement = tree.get(*statement_id);
        visitor.visit_statement(tree, *statement_id, statement);
    }
}

/// Walk the Function's children.
pub fn walk_function<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    function: &Function,
) {
    if let Some(static_parameters) = &function.static_parameters {
        for parameter_id in static_parameters {
            let parameter = tree.get(*parameter_id);
            visitor.visit_parameter(tree, *parameter_id, parameter);
        }
    }

    for parameter_id in &function.dynamic_parameters {
        let parameter = tree.get(*parameter_id);
        visitor.visit_parameter(tree, *parameter_id, parameter);
    }

    if let Some(return_type) = &function.return_type {
        let type_node = tree.get(*return_type);
        visitor.visit_type(tree, *return_type, type_node);
    }

    if let Some(with) = &function.with {
        let with_node = tree.get(*with);
        visitor.visit_with(tree, *with, with_node);
    }

    if let Some(body) = &function.body {
        let block = tree.get(*body);
        visitor.visit_block(tree, *body, block);
    }
}

/// Walk the Let's children.
pub fn walk_let<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, let_node: &Let) {
    let pattern = tree.get(let_node.pattern);
    visitor.visit_pattern(tree, let_node.pattern, pattern);

    if let Some(type_node) = &let_node.r#type {
        let type_ref = tree.get(*type_node);
        visitor.visit_type(tree, *type_node, type_ref);
    }

    if let Some(value) = &let_node.value {
        let expression = tree.get(*value);
        visitor.visit_expression(tree, *value, expression);
    }
}

/// Walk the Tuple's children.
pub fn walk_tuple<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, tuple: &Tuple) {
    for element_id in &tuple.elements {
        let element = tree.get(*element_id);
        visitor.visit_tuple_field(tree, *element_id, element);
    }
}

/// Walk the TupleField's children.
pub fn walk_tuple_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    field: &TupleField,
) {
    match field {
        TupleField::Named { name: _, r#type } => {
            let type_node = tree.get(*r#type);
            visitor.visit_type(tree, *r#type, type_node);
        }
        TupleField::Positional { r#type } => {
            let type_node = tree.get(*r#type);
            visitor.visit_type(tree, *r#type, type_node);
        }
    }
}

/// Walk the Type's children.
pub fn walk_type<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, type_node: &Type) {
    match type_node {
        Type::Infer => {}
        Type::Maybe(inner) => {
            let inner_type = tree.get(*inner);
            visitor.visit_type(tree, *inner, inner_type);
        }
        Type::Not(inner) => {
            let inner_type = tree.get(*inner);
            visitor.visit_type(tree, *inner, inner_type);
        }
        Type::Never => {}
        Type::Self_ => {}
        Type::Primitive(_) => {}
        Type::Path {
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
        Type::Pointer {
            mutability: _,
            target,
        } => {
            let target_type = tree.get(*target);
            visitor.visit_type(tree, *target, target_type);
        }
        Type::Virtual(inner) => {
            let inner_type = tree.get(*inner);
            visitor.visit_type(tree, *inner, inner_type);
        }
        Type::Variadic(inner) => {
            let inner_type = tree.get(*inner);
            visitor.visit_type(tree, *inner, inner_type);
        }
        Type::Array { element, count } => {
            let element_type = tree.get(*element);
            visitor.visit_type(tree, *element, element_type);
            let count_expr = tree.get(*count);
            visitor.visit_expression(tree, *count, count_expr);
        }
        Type::Slice { element } => {
            let element_type = tree.get(*element);
            visitor.visit_type(tree, *element, element_type);
        }
        Type::Tuple(tuple_id) => {
            let tuple = tree.get(*tuple_id);
            visitor.visit_tuple(tree, *tuple_id, tuple);
        }
        Type::Struct(struct_id) => {
            let struct_node = tree.get(*struct_id);
            visitor.visit_struct(tree, *struct_id, struct_node);
        }
        Type::Enum(enum_id) => {
            let enum_node = tree.get(*enum_id);
            visitor.visit_enum(tree, *enum_id, enum_node);
        }
        Type::Union(union_id) => {
            let union_node = tree.get(*union_id);
            visitor.visit_union(tree, *union_id, union_node);
        }
        Type::Function(function_id) => {
            let function = tree.get(*function_id);
            visitor.visit_function(tree, *function_id, function);
        }
    }
}

// ----------------------------------------------------------------------------
// Context
// ----------------------------------------------------------------------------

/// Walk the With's children.
pub fn walk_with<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, with: &With) {
    for clause_id in &with.clauses {
        let clause = tree.get(*clause_id);
        visitor.visit_with_clause(tree, *clause_id, clause);
    }
}

/// Walk the WithClause's children.
pub fn walk_with_clause<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    with_clause: &WithClause,
) {
    match with_clause {
        WithClause::Declaration { target, alias: _ } => {
            let target_type = tree.get(*target);
            visitor.visit_type(tree, *target, target_type);
        }
        WithClause::Assertion { target, assertion } => {
            let target_type = tree.get(*target);
            visitor.visit_type(tree, *target, target_type);
            let assertion_type = tree.get(*assertion);
            visitor.visit_type(tree, *assertion, assertion_type);
        }
    }
}

/// Walk the Use's children.
pub fn walk_use<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, use_node: &Use) {
    for clause_id in &use_node.clauses {
        let clause = tree.get(*clause_id);
        visitor.visit_use_clause(tree, *clause_id, clause);
    }

    if let Some(body) = &use_node.body {
        let block = tree.get(*body);
        visitor.visit_block(tree, *body, block);
    }
}

/// Walk the UseClause's children.
pub fn walk_use_clause<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    use_clause: &UseClause,
) {
    let target = tree.get(use_clause.target);
    visitor.visit_expression(tree, use_clause.target, target);

    if let Some(items) = &use_clause.items {
        for item_id in items {
            let item = tree.get(*item_id);
            visitor.visit_use_item(tree, *item_id, item);
        }
    }
}

/// Walk the UseItem's children.
pub fn walk_use_item<V: NodeVisitor + ?Sized>(
    _visitor: &mut V,
    _tree: &NodeTree,
    _use_item: &UseItem,
) {
    // UseItem has no child nodes to visit (only StringId fields)
}

// ----------------------------------------------------------------------------
// Control
// ----------------------------------------------------------------------------

/// Walk the If's children.
pub fn walk_if<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, if_node: &If) {
    match if_node {
        If::If {
            condition,
            then_block,
        } => {
            let condition_expr = tree.get(*condition);
            visitor.visit_expression(tree, *condition, condition_expr);
            let then_block_node = tree.get(*then_block);
            visitor.visit_block(tree, *then_block, then_block_node);
        }
        If::IfElse {
            condition,
            then_block,
            else_block,
        } => {
            let condition_expr = tree.get(*condition);
            visitor.visit_expression(tree, *condition, condition_expr);
            let then_block_node = tree.get(*then_block);
            visitor.visit_block(tree, *then_block, then_block_node);
            let else_block_node = tree.get(*else_block);
            visitor.visit_block(tree, *else_block, else_block_node);
        }
        If::IfElseIf {
            condition,
            then_block,
            else_if,
        } => {
            let condition_expr = tree.get(*condition);
            visitor.visit_expression(tree, *condition, condition_expr);
            let then_block_node = tree.get(*then_block);
            visitor.visit_block(tree, *then_block, then_block_node);
            let else_if_node = tree.get(*else_if);
            visitor.visit_if(tree, *else_if, else_if_node);
        }
    }
}

/// Walk the While's children.
pub fn walk_while<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, while_node: &While) {
    let condition = tree.get(while_node.condition);
    visitor.visit_expression(tree, while_node.condition, condition);
    let body = tree.get(while_node.body);
    visitor.visit_block(tree, while_node.body, body);
}

/// Walk the For's children.
pub fn walk_for<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, for_node: &For) {
    let pattern = tree.get(for_node.pattern);
    visitor.visit_pattern(tree, for_node.pattern, pattern);
    let iterator = tree.get(for_node.iterator);
    visitor.visit_expression(tree, for_node.iterator, iterator);
    let body = tree.get(for_node.body);
    visitor.visit_block(tree, for_node.body, body);
}

/// Walk the Loop's children.
pub fn walk_loop<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, loop_node: &Loop) {
    let body = tree.get(loop_node.body);
    visitor.visit_block(tree, loop_node.body, body);
}

/// Walk the Break's children.
pub fn walk_break<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, break_node: &Break) {
    if let Some(value) = &break_node.value {
        let value_expr = tree.get(*value);
        visitor.visit_expression(tree, *value, value_expr);
    }
}

/// Walk the Continue's children.
pub fn walk_continue<V: NodeVisitor + ?Sized>(
    _visitor: &mut V,
    _tree: &NodeTree,
    _continue_node: &Continue,
) {
    // Continue has no child nodes to visit (only optional StringId)
}

/// Walk the Defer's children.
pub fn walk_defer<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, defer_node: &Defer) {
    match defer_node {
        Defer::Expression(expr_id) => {
            let expression = tree.get(*expr_id);
            visitor.visit_expression(tree, *expr_id, expression);
        }
        Defer::Block(block_id) => {
            let block = tree.get(*block_id);
            visitor.visit_block(tree, *block_id, block);
        }
    }
}

/// Walk the Return's children.
pub fn walk_return<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    return_node: &Return,
) {
    if let Some(value) = &return_node.value {
        let value_expr = tree.get(*value);
        visitor.visit_expression(tree, *value, value_expr);
    }
}

/// Walk the Match's children.
pub fn walk_match<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, match_node: &Match) {
    let value = tree.get(match_node.value);
    visitor.visit_expression(tree, match_node.value, value);

    for case_id in &match_node.cases {
        let case = tree.get(*case_id);
        visitor.visit_match_case(tree, *case_id, case);
    }
}

/// Walk the Try's children.
pub fn walk_try<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, try_node: &Try) {
    match try_node {
        Try::Expression { try_expression } => {
            let expression = tree.get(*try_expression);
            visitor.visit_expression(tree, *try_expression, expression);
        }
        Try::Block { try_block } => {
            let block = tree.get(*try_block);
            visitor.visit_block(tree, *try_block, block);
        }
        Try::BlockWithCatch {
            try_block,
            catch_match,
        } => {
            let block = tree.get(*try_block);
            visitor.visit_block(tree, *try_block, block);
            let match_node = tree.get(*catch_match);
            visitor.visit_match(tree, *catch_match, match_node);
        }
    }
}

/// Walk the Parameter's children.
pub fn walk_parameter<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    parameter: &Parameter,
) {
    if let Some(type_node) = &parameter.r#type {
        let type_ref = tree.get(*type_node);
        visitor.visit_type(tree, *type_node, type_ref);
    }

    if let Some(default) = &parameter.default {
        let default_expr = tree.get(*default);
        visitor.visit_expression(tree, *default, default_expr);
    }
}

/// Walk the Argument's children.
pub fn walk_argument<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    argument: &Argument,
) {
    match argument {
        Argument::Named { name: _, value } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }
        Argument::NamedShorthand { name: _ } => {
            // no child nodes to visit
        }
        Argument::Positional { value } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }
    }
}

// ----------------------------------------------------------------------------
// Literals
// ----------------------------------------------------------------------------

/// Walk the ScalarLiteral's children.
pub fn walk_scalar_literal<V: NodeVisitor + ?Sized>(
    _visitor: &mut V,
    _tree: &NodeTree,
    _scalar_literal: &ScalarLiteral,
) {
    // ScalarLiteral has no child nodes to visit
}

/// Walk the RangeLiteral's children.
pub fn walk_range_literal<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    range_literal: &RangeLiteral,
) {
    if let Some(start) = &range_literal.start {
        let start_expr = tree.get(*start);
        visitor.visit_expression(tree, *start, start_expr);
    }
    if let Some(end) = &range_literal.end {
        let end_expr = tree.get(*end);
        visitor.visit_expression(tree, *end, end_expr);
    }
}

/// Walk the ArrayLiteral's children.
pub fn walk_array_literal<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    array_literal: &ArrayLiteral,
) {
    match array_literal {
        ArrayLiteral::Fixed { elements } => {
            for element_id in elements {
                let element = tree.get(*element_id);
                visitor.visit_expression(tree, *element_id, element);
            }
        }
    }
}

/// Walk the TupleLiteral's children.
pub fn walk_tuple_literal<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    tuple_literal: &TupleLiteral,
) {
    for element_id in &tuple_literal.elements {
        let element = tree.get(*element_id);
        visitor.visit_expression(tree, *element_id, element);
    }
}

/// Walk the StructLiteral's children.
pub fn walk_struct_literal<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    struct_literal: &StructLiteral,
) {
    let type_node = tree.get(struct_literal.r#type);
    visitor.visit_type(tree, struct_literal.r#type, type_node);
}

/// Walk the Index's children.
pub fn walk_index<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, index: &Index) {
    let receiver = tree.get(index.receiver);
    visitor.visit_expression(tree, index.receiver, receiver);
    let index_expr = tree.get(index.index);
    visitor.visit_expression(tree, index.index, index_expr);
}

// ----------------------------------------------------------------------------
// Calls
// ----------------------------------------------------------------------------

/// Walk the Call's children.
pub fn walk_call<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, call: &Call) {
    let receiver = tree.get(call.receiver);
    visitor.visit_expression(tree, call.receiver, receiver);

    if let Some(static_arguments) = &call.static_arguments {
        for arg_id in static_arguments {
            let arg = tree.get(*arg_id);
            visitor.visit_argument(tree, *arg_id, arg);
        }
    }

    for arg_id in &call.dynamic_arguments {
        let arg = tree.get(*arg_id);
        visitor.visit_argument(tree, *arg_id, arg);
    }
}

/// Walk the Cast's children.
pub fn walk_cast<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, cast: &Cast) {
    let receiver = tree.get(cast.receiver);
    visitor.visit_expression(tree, cast.receiver, receiver);
    let type_node = tree.get(cast.r#type);
    visitor.visit_type(tree, cast.r#type, type_node);
}

/// Walk the Coalesce's children.
pub fn walk_coalesce<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    coalesce: &Coalesce,
) {
    let receiver = tree.get(coalesce.receiver);
    visitor.visit_expression(tree, coalesce.receiver, receiver);
    let default = tree.get(coalesce.default);
    visitor.visit_expression(tree, coalesce.default, default);
}

// ----------------------------------------------------------------------------
// Patterns
// ----------------------------------------------------------------------------

/// Walk the Pattern's children.
pub fn walk_pattern<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, pattern: &Pattern) {
    match pattern {
        Pattern::Wildcard => {
            // no child nodes to visit
        }
        Pattern::Rest => {
            // no child nodes to visit
        }
        Pattern::Pointer {
            target,
            mutability: _,
        } => {
            let target_pattern = tree.get(*target);
            visitor.visit_pattern(tree, *target, target_pattern);
        }
        Pattern::Literal(literal_id) => {
            let literal = tree.get(*literal_id);
            visitor.visit_scalar_literal(tree, *literal_id, literal);
        }
        Pattern::Identifier(_) => {
            // no child nodes to visit
        }
        Pattern::Path(_) => {
            // no child nodes to visit
        }
        Pattern::Range {
            start,
            end,
            is_inclusive: _,
        } => {
            if let Some(start_pattern) = start {
                let start_node = tree.get(*start_pattern);
                visitor.visit_pattern(tree, *start_pattern, start_node);
            }
            if let Some(end_pattern) = end {
                let end_node = tree.get(*end_pattern);
                visitor.visit_pattern(tree, *end_pattern, end_node);
            }
        }
        Pattern::Tuple { path: _, fields } => {
            for field_id in fields {
                let field = tree.get(*field_id);
                visitor.visit_pattern_field(tree, *field_id, field);
            }
        }
        Pattern::Slice { fields } => {
            for field_id in fields {
                let field = tree.get(*field_id);
                visitor.visit_pattern_field(tree, *field_id, field);
            }
        }
        Pattern::Struct { r#type, fields } => {
            let type_node = tree.get(*r#type);
            visitor.visit_type(tree, *r#type, type_node);
            for field_id in fields {
                let field = tree.get(*field_id);
                visitor.visit_pattern_field(tree, *field_id, field);
            }
        }
        Pattern::Union { fields } => {
            for field_id in fields {
                let field = tree.get(*field_id);
                visitor.visit_pattern(tree, *field_id, field);
            }
        }
    }
}

/// Walk the PatternField's children.
pub fn walk_pattern_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    pattern_field: &PatternField,
) {
    match pattern_field {
        PatternField::Named { name: _, pattern } => {
            if let Some(pattern_id) = pattern {
                let pattern_node = tree.get(*pattern_id);
                visitor.visit_pattern(tree, *pattern_id, pattern_node);
            }
        }
        PatternField::NamedAlias { name: _, alias: _ } => {
            // no child nodes to visit
        }
        PatternField::Positional { pattern } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, pattern_node);
        }
    }
}

/// Walk the MatchCase's children.
pub fn walk_match_case<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    match_case: &MatchCase,
) {
    match match_case {
        MatchCase::Expression {
            pattern,
            body,
            guard,
        } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, pattern_node);
            let body_expr = tree.get(*body);
            visitor.visit_expression(tree, *body, body_expr);
            if let Some(guard_expr) = guard {
                let guard_node = tree.get(*guard_expr);
                visitor.visit_expression(tree, *guard_expr, guard_node);
            }
        }
        MatchCase::Block {
            pattern,
            body,
            guard,
        } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, pattern_node);
            let body_block = tree.get(*body);
            visitor.visit_block(tree, *body, body_block);
            if let Some(guard_expr) = guard {
                let guard_node = tree.get(*guard_expr);
                visitor.visit_expression(tree, *guard_expr, guard_node);
            }
        }
    }
}
