use crate::{
    Annotation, Argument, ArrayLiteral, Blank, Block, Break, Call, Comment, Continue, Decorator,
    Defer, Doc, Enum, EnumField, Expression, FieldLiteral, For, Function, If, Implement, Index,
    Let, Loop, Match, MatchCase, Module, NodeId, NodeTree, NodeType, NodeVisitor, Parameter,
    Pattern, PatternField, RangeLiteral, Return, ScalarLiteral, Struct, StructField, StructLiteral,
    Tag, Trait, Try, TupleLiteral, TupleLiteralField, TypeLiteral, Union, UnionField, Use,
    UseClause, UseItem, While, With, WithClause,
};

// ----------------------------------------------------------------------------
// Groupings
// ----------------------------------------------------------------------------

/// Walk the Block.
pub fn walk_block<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Block>,
    block: &Block,
) {
    visitor.visit_any(tree, NodeType::Block, id.id);
    for expression_id in &block.expressions {
        let expression = tree.get(*expression_id);
        visitor.visit_expression(tree, *expression_id, expression);
    }
}

/// Walk the Expression.
pub fn walk_expression<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Expression>,
    expression: &Expression,
) {
    visitor.visit_any(tree, NodeType::Expression, id.id);
    match expression {
        Expression::Module(node) => visitor.visit_module(tree, *node, tree.get(*node)),
        Expression::Struct(node) => visitor.visit_struct(tree, *node, tree.get(*node)),
        Expression::Enum(node) => visitor.visit_enum(tree, *node, tree.get(*node)),
        Expression::Union(node) => visitor.visit_union(tree, *node, tree.get(*node)),
        Expression::Trait(node) => visitor.visit_trait(tree, *node, tree.get(*node)),
        Expression::Implement(node) => visitor.visit_implement(tree, *node, tree.get(*node)),
        Expression::Function(node) => visitor.visit_function(tree, *node, tree.get(*node)),
        Expression::Block(node) => visitor.visit_block(tree, *node, tree.get(*node)),

        Expression::With(node) => visitor.visit_with(tree, *node, tree.get(*node)),
        Expression::Use(node) => visitor.visit_use(tree, *node, tree.get(*node)),
        Expression::Let(node) => visitor.visit_let(tree, *node, tree.get(*node)),
        Expression::If(node) => visitor.visit_if(tree, *node, tree.get(*node)),
        Expression::While(node) => visitor.visit_while(tree, *node, tree.get(*node)),
        Expression::For(node) => visitor.visit_for(tree, *node, tree.get(*node)),
        Expression::Loop(node) => visitor.visit_loop(tree, *node, tree.get(*node)),
        Expression::Try(node) => visitor.visit_try(tree, *node, tree.get(*node)),
        Expression::Match(node) => visitor.visit_match(tree, *node, tree.get(*node)),
        Expression::Break(node) => visitor.visit_break(tree, *node, tree.get(*node)),
        Expression::Continue(node) => visitor.visit_continue(tree, *node, tree.get(*node)),
        Expression::Defer(node) => visitor.visit_defer(tree, *node, tree.get(*node)),
        Expression::Return(node) => visitor.visit_return(tree, *node, tree.get(*node)),

        Expression::Path {
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
        Expression::ScalarLiteral(node) => {
            visitor.visit_scalar_literal(tree, *node, tree.get(*node))
        }
        Expression::TypeLiteral(node) => visitor.visit_type_literal(tree, *node, tree.get(*node)),
        Expression::RangeLiteral(node) => visitor.visit_range_literal(tree, *node, tree.get(*node)),
        Expression::TupleLiteral(node) => visitor.visit_tuple_literal(tree, *node, tree.get(*node)),
        Expression::ArrayLiteral(node) => visitor.visit_array_literal(tree, *node, tree.get(*node)),
        Expression::StructLiteral(node) => {
            visitor.visit_struct_literal(tree, *node, tree.get(*node))
        }

        Expression::Parenthesized { expression } => {
            visitor.visit_expression(tree, *expression, tree.get(*expression));
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
        Expression::Call(node) => visitor.visit_call(tree, *node, tree.get(*node)),
        Expression::Maybe(node) => visitor.visit_expression(tree, *node, tree.get(*node)),
        Expression::Must(node) => visitor.visit_expression(tree, *node, tree.get(*node)),

        Expression::Error => {}
    }
}

// ----------------------------------------------------------------------------
// Declarations
// ----------------------------------------------------------------------------

/// Walk the Module.
pub fn walk_module<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Module>,
    module: &Module,
) {
    visitor.visit_any(tree, NodeType::Module, id.id);
    for expression_id in &module.expressions {
        let expression = tree.get(*expression_id);
        visitor.visit_expression(tree, *expression_id, expression);
    }
}

/// Walk the Struct.
pub fn walk_struct<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Struct>,
    struct_node: &Struct,
) {
    visitor.visit_any(tree, NodeType::Struct, id.id);
    if let Some(super_types) = &struct_node.super_types {
        for type_id in super_types {
            let type_node = tree.get(*type_id);
            visitor.visit_expression(tree, *type_id, type_node);
        }
    }

    if let Some(representation_type) = &struct_node.representation_type {
        let type_node = tree.get(*representation_type);
        visitor.visit_expression(tree, *representation_type, type_node);
    }

    if let Some(static_parameters) = &struct_node.static_parameters {
        for parameter_id in static_parameters {
            let parameter = tree.get(*parameter_id);
            visitor.visit_parameter(tree, *parameter_id, parameter);
        }
    }

    if let Some(with) = &struct_node.with {
        let with_node = tree.get(*with);
        visitor.visit_with(tree, *with, with_node);
    }

    for field_id in &struct_node.fields {
        let field = tree.get(*field_id);
        visitor.visit_struct_field(tree, *field_id, field);
    }

    for expression_id in &struct_node.expressions {
        let expression = tree.get(*expression_id);
        visitor.visit_expression(tree, *expression_id, expression);
    }
}

/// Walk the StructField.
pub fn walk_struct_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<StructField>,
    field: &StructField,
) {
    visitor.visit_any(tree, NodeType::StructField, id.id);
    let type_node = tree.get(field.r#type);
    visitor.visit_expression(tree, field.r#type, type_node);

    if let Some(default) = &field.default {
        let expression = tree.get(*default);
        visitor.visit_expression(tree, *default, expression);
    }
}

/// Walk the Enum.
pub fn walk_enum<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Enum>,
    enum_node: &Enum,
) {
    visitor.visit_any(tree, NodeType::Enum, id.id);
    if let Some(type_node) = &enum_node.tag_type {
        let type_ref = tree.get(*type_node);
        visitor.visit_expression(tree, *type_node, type_ref);
    }

    if let Some(super_types) = &enum_node.super_types {
        for type_id in super_types {
            let type_node = tree.get(*type_id);
            visitor.visit_expression(tree, *type_id, type_node);
        }
    }

    if let Some(with) = &enum_node.with {
        let with_node = tree.get(*with);
        visitor.visit_with(tree, *with, with_node);
    }

    for field_id in &enum_node.fields {
        let field = tree.get(*field_id);
        visitor.visit_enum_field(tree, *field_id, field);
    }

    for expression_id in &enum_node.expressions {
        let expression = tree.get(*expression_id);
        visitor.visit_expression(tree, *expression_id, expression);
    }
}

/// Walk the EnumField.
pub fn walk_enum_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<EnumField>,
    field: &EnumField,
) {
    visitor.visit_any(tree, NodeType::EnumField, id.id);
    if let Some(value) = &field.value {
        let expression = tree.get(*value);
        visitor.visit_expression(tree, *value, expression);
    }
}

/// Walk the Union.
pub fn walk_union<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Union>,
    union_node: &Union,
) {
    visitor.visit_any(tree, NodeType::Union, id.id);
    if let Some(tag_type) = &union_node.tag_type {
        let type_node = tree.get(*tag_type);
        visitor.visit_expression(tree, *tag_type, type_node);
    }

    if let Some(representation_type) = &union_node.representation_type {
        let type_node = tree.get(*representation_type);
        visitor.visit_expression(tree, *representation_type, type_node);
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
            visitor.visit_expression(tree, *type_id, type_node);
        }
    }

    if let Some(with) = &union_node.with {
        let with_node = tree.get(*with);
        visitor.visit_with(tree, *with, with_node);
    }

    for field_id in &union_node.fields {
        let field = tree.get(*field_id);
        visitor.visit_union_field(tree, *field_id, field);
    }

    for expression_id in &union_node.expressions {
        let expression = tree.get(*expression_id);
        visitor.visit_expression(tree, *expression_id, expression);
    }
}

/// Walk the UnionField.
pub fn walk_union_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<UnionField>,
    field: &UnionField,
) {
    visitor.visit_any(tree, NodeType::UnionField, id.id);
    if let Some(type_node) = &field.r#type {
        let type_ref = tree.get(*type_node);
        visitor.visit_expression(tree, *type_node, type_ref);
    }
    if let Some(value) = &field.value {
        let expression = tree.get(*value);
        visitor.visit_expression(tree, *value, expression);
    }
}

/// Walk the Trait.
pub fn walk_trait<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Trait>,
    trait_node: &Trait,
) {
    visitor.visit_any(tree, NodeType::Trait, id.id);
    if let Some(super_types) = &trait_node.super_types {
        for type_id in super_types {
            let type_node = tree.get(*type_id);
            visitor.visit_expression(tree, *type_id, type_node);
        }
    }

    if let Some(static_parameters) = &trait_node.static_parameters {
        for parameter_id in static_parameters {
            let parameter = tree.get(*parameter_id);
            visitor.visit_parameter(tree, *parameter_id, parameter);
        }
    }

    if let Some(with) = &trait_node.with {
        let with_node = tree.get(*with);
        visitor.visit_with(tree, *with, with_node);
    }

    for expression_id in &trait_node.expressions {
        let expression = tree.get(*expression_id);
        visitor.visit_expression(tree, *expression_id, expression);
    }
}

/// Walk the Implement.
pub fn walk_implement<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Implement>,
    implement: &Implement,
) {
    visitor.visit_any(tree, NodeType::Implement, id.id);
    if let Some(static_arguments) = &implement.static_arguments {
        for argument_id in static_arguments {
            let argument = tree.get(*argument_id);
            visitor.visit_argument(tree, *argument_id, argument);
        }
    }

    let receiver = tree.get(implement.receiver);
    visitor.visit_expression(tree, implement.receiver, receiver);

    if let Some(for_trait) = &implement.for_trait {
        let trait_type = tree.get(*for_trait);
        visitor.visit_expression(tree, *for_trait, trait_type);
    }

    if let Some(with) = &implement.with {
        let with_node = tree.get(*with);
        visitor.visit_with(tree, *with, with_node);
    }

    for expression_id in &implement.expressions {
        let expression = tree.get(*expression_id);
        visitor.visit_expression(tree, *expression_id, expression);
    }
}

/// Walk the Function.
pub fn walk_function<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Function>,
    function: &Function,
) {
    visitor.visit_any(tree, NodeType::Function, id.id);
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
        visitor.visit_expression(tree, *return_type, type_node);
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

/// Walk the Let.
pub fn walk_let<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Let>,
    let_node: &Let,
) {
    visitor.visit_any(tree, NodeType::Let, id.id);
    let pattern = tree.get(let_node.pattern);
    visitor.visit_pattern(tree, let_node.pattern, pattern);

    if let Some(type_node) = &let_node.r#type {
        let type_ref = tree.get(*type_node);
        visitor.visit_expression(tree, *type_node, type_ref);
    }

    if let Some(value) = &let_node.value {
        let expression = tree.get(*value);
        visitor.visit_expression(tree, *value, expression);
    }
}

// ----------------------------------------------------------------------------
// Context
// ----------------------------------------------------------------------------

/// Walk the With.
pub fn walk_with<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<With>,
    with: &With,
) {
    visitor.visit_any(tree, NodeType::With, id.id);
    for clause_id in &with.clauses {
        let clause = tree.get(*clause_id);
        visitor.visit_with_clause(tree, *clause_id, clause);
    }
}

/// Walk the WithClause.
pub fn walk_with_clause<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<WithClause>,
    with_clause: &WithClause,
) {
    visitor.visit_any(tree, NodeType::WithClause, id.id);
    match with_clause {
        WithClause::Declaration { target } => {
            let target_type = tree.get(*target);
            visitor.visit_expression(tree, *target, target_type);
        }
        WithClause::Assertion { target, assertion } => {
            let target_type = tree.get(*target);
            visitor.visit_expression(tree, *target, target_type);
            let assertion_type = tree.get(*assertion);
            visitor.visit_expression(tree, *assertion, assertion_type);
        }
    }
}

/// Walk the Use.
pub fn walk_use<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Use>,
    use_node: &Use,
) {
    visitor.visit_any(tree, NodeType::Use, id.id);
    for clause_id in &use_node.clauses {
        let clause = tree.get(*clause_id);
        visitor.visit_use_clause(tree, *clause_id, clause);
    }

    if let Some(body) = &use_node.body {
        let block = tree.get(*body);
        visitor.visit_block(tree, *body, block);
    }
}

/// Walk the UseClause.
pub fn walk_use_clause<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<UseClause>,
    use_clause: &UseClause,
) {
    visitor.visit_any(tree, NodeType::UseClause, id.id);
    let target = tree.get(use_clause.target);
    visitor.visit_expression(tree, use_clause.target, target);

    if let Some(items) = &use_clause.items {
        for item_id in items {
            let item = tree.get(*item_id);
            visitor.visit_use_item(tree, *item_id, item);
        }
    }
}

/// Walk the UseItem.
pub fn walk_use_item<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    _tree: &NodeTree,
    id: NodeId<UseItem>,
    _use_item: &UseItem,
) {
    visitor.visit_any(_tree, NodeType::UseItem, id.id);
    // UseItem has no child nodes to visit (only StringId fields)
}

// ----------------------------------------------------------------------------
// Control
// ----------------------------------------------------------------------------

/// Walk the If.
pub fn walk_if<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<If>,
    if_node: &If,
) {
    visitor.visit_any(tree, NodeType::If, id.id);
    match if_node {
        If::If {
            runtime: _,
            condition,
            then_block,
        } => {
            let condition_expr = tree.get(*condition);
            visitor.visit_expression(tree, *condition, condition_expr);
            let then_block_node = tree.get(*then_block);
            visitor.visit_block(tree, *then_block, then_block_node);
        }
        If::IfElse {
            runtime: _,
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
            runtime: _,
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

/// Walk the While.
pub fn walk_while<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<While>,
    while_node: &While,
) {
    visitor.visit_any(tree, NodeType::While, id.id);
    let condition = tree.get(while_node.condition);
    visitor.visit_expression(tree, while_node.condition, condition);
    let body = tree.get(while_node.body);
    visitor.visit_block(tree, while_node.body, body);
}

/// Walk the For.
pub fn walk_for<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<For>,
    for_node: &For,
) {
    visitor.visit_any(tree, NodeType::For, id.id);
    let pattern = tree.get(for_node.pattern);
    visitor.visit_pattern(tree, for_node.pattern, pattern);
    let iterator = tree.get(for_node.iterator);
    visitor.visit_expression(tree, for_node.iterator, iterator);
    let body = tree.get(for_node.body);
    visitor.visit_block(tree, for_node.body, body);
}

/// Walk the Loop.
pub fn walk_loop<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Loop>,
    loop_node: &Loop,
) {
    visitor.visit_any(tree, NodeType::Loop, id.id);
    let body = tree.get(loop_node.body);
    visitor.visit_block(tree, loop_node.body, body);
}

/// Walk the Break.
pub fn walk_break<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Break>,
    break_node: &Break,
) {
    visitor.visit_any(tree, NodeType::Break, id.id);
    if let Some(value) = &break_node.value {
        let value_expr = tree.get(*value);
        visitor.visit_expression(tree, *value, value_expr);
    }
}

/// Walk the Continue.
pub fn walk_continue<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Continue>,
    _continue_node: &Continue,
) {
    visitor.visit_any(tree, NodeType::Continue, id.id);
    // Continue has no child nodes to visit (only optional StringId)
}

/// Walk the Defer.
pub fn walk_defer<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Defer>,
    defer_node: &Defer,
) {
    visitor.visit_any(tree, NodeType::Defer, id.id);
    match defer_node {
        Defer::Expression(expr_id) => {
            let expression = tree.get(*expr_id);
            visitor.visit_expression(tree, *expr_id, expression);
        }
        Defer::Block(block_id) => {
            let block = tree.get(*block_id);
            visitor.visit_block(tree, *block_id, block);
        }
        Defer::Catch(match_id) => {
            let match_node = tree.get(*match_id);
            visitor.visit_match(tree, *match_id, match_node);
        }
    }
}

/// Walk the Return.
pub fn walk_return<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Return>,
    return_node: &Return,
) {
    visitor.visit_any(tree, NodeType::Return, id.id);
    if let Some(value) = &return_node.value {
        let value_expr = tree.get(*value);
        visitor.visit_expression(tree, *value, value_expr);
    }
}

/// Walk the Match.
pub fn walk_match<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Match>,
    match_node: &Match,
) {
    visitor.visit_any(tree, NodeType::Match, id.id);
    let value = tree.get(match_node.value);
    visitor.visit_expression(tree, match_node.value, value);

    for case_id in &match_node.cases {
        let case = tree.get(*case_id);
        visitor.visit_match_case(tree, *case_id, case);
    }
}

/// Walk the Try.
pub fn walk_try<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Try>,
    try_node: &Try,
) {
    visitor.visit_any(tree, NodeType::Try, id.id);
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

/// Walk the Parameter.
pub fn walk_parameter<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Parameter>,
    parameter: &Parameter,
) {
    visitor.visit_any(tree, NodeType::Parameter, id.id);
    if let Some(type_node) = &parameter.r#type {
        let type_ref = tree.get(*type_node);
        visitor.visit_expression(tree, *type_node, type_ref);
    }

    if let Some(default) = &parameter.default {
        let default_expr = tree.get(*default);
        visitor.visit_expression(tree, *default, default_expr);
    }
}

/// Walk the Argument.
pub fn walk_argument<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Argument>,
    argument: &Argument,
) {
    visitor.visit_any(tree, NodeType::Argument, id.id);
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

/// Walk the ScalarLiteral.
pub fn walk_scalar_literal<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<ScalarLiteral>,
    _scalar_literal: &ScalarLiteral,
) {
    visitor.visit_any(tree, NodeType::ScalarLiteral, id.id);
    // ScalarLiteral has no child nodes to visit
}

/// Walk the TypeLiteral.
pub fn walk_type_literal<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<TypeLiteral>,
    _type_literal: &TypeLiteral,
) {
    visitor.visit_any(tree, NodeType::TypeLiteral, id.id);
    // TypeLiteral has no child nodes to visit
}

/// Walk the RangeLiteral.
pub fn walk_range_literal<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<RangeLiteral>,
    range_literal: &RangeLiteral,
) {
    visitor.visit_any(tree, NodeType::RangeLiteral, id.id);
    let start_expr = tree.get(range_literal.start);
    visitor.visit_expression(tree, range_literal.start, start_expr);
    let end_expr = tree.get(range_literal.end);
    visitor.visit_expression(tree, range_literal.end, end_expr);
}

/// Walk the ArrayLiteral.
pub fn walk_array_literal<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<ArrayLiteral>,
    array_literal: &ArrayLiteral,
) {
    visitor.visit_any(tree, NodeType::ArrayLiteral, id.id);
    match array_literal {
        ArrayLiteral::Fixed { elements } => {
            for element_id in elements {
                let element = tree.get(*element_id);
                visitor.visit_expression(tree, *element_id, element);
            }
        }
    }
}

/// Walk the TupleLiteral.
pub fn walk_tuple_literal<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<TupleLiteral>,
    tuple_literal: &TupleLiteral,
) {
    visitor.visit_any(tree, NodeType::TupleLiteral, id.id);
    for element_id in &tuple_literal.elements {
        let element = tree.get(*element_id);
        visitor.visit_tuple_literal_field(tree, *element_id, element);
    }
}

/// Walk the TupleLiteralField.
pub fn walk_tuple_literal_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<TupleLiteralField>,
    tuple_literal_field: &TupleLiteralField,
) {
    visitor.visit_any(tree, NodeType::TupleLiteralField, id.id);
    match tuple_literal_field {
        TupleLiteralField::Named { name: _, value } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }
        TupleLiteralField::Positional { value } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }
    }
}

/// Walk the StructLiteral.
pub fn walk_struct_literal<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<StructLiteral>,
    struct_literal: &StructLiteral,
) {
    visitor.visit_any(tree, NodeType::StructLiteral, id.id);
    let type_node = tree.get(struct_literal.r#type);
    visitor.visit_expression(tree, struct_literal.r#type, type_node);
}

/// Walk the FieldLiteral.
pub fn walk_field_literal<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<FieldLiteral>,
    _field_literal: &FieldLiteral,
) {
    visitor.visit_any(tree, NodeType::FieldLiteral, id.id);
}

/// Walk the Index.
pub fn walk_index<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Index>,
    index: &Index,
) {
    visitor.visit_any(tree, NodeType::Index, id.id);
    match index {
        Index::Declarative { receiver } => {
            visitor.visit_expression(tree, *receiver, tree.get(*receiver));
        }
        Index::Explicit { receiver, index } => {
            visitor.visit_expression(tree, *receiver, tree.get(*receiver));
            visitor.visit_expression(tree, *index, tree.get(*index));
        }
        Index::Member { receiver, index: _ } => {
            visitor.visit_expression(tree, *receiver, tree.get(*receiver));
        }
    }
}

// ----------------------------------------------------------------------------
// Calls
// ----------------------------------------------------------------------------

/// Walk the Call.
pub fn walk_call<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Call>,
    call: &Call,
) {
    visitor.visit_any(tree, NodeType::Call, id.id);
    let receiver = tree.get(call.receiver);
    visitor.visit_expression(tree, call.receiver, receiver);

    for arg_id in &call.dynamic_arguments {
        let arg = tree.get(*arg_id);
        visitor.visit_argument(tree, *arg_id, arg);
    }
}

// ----------------------------------------------------------------------------
// Patterns
// ----------------------------------------------------------------------------

/// Walk the Pattern.
pub fn walk_pattern<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Pattern>,
    pattern: &Pattern,
) {
    visitor.visit_any(tree, NodeType::Pattern, id.id);
    match pattern {
        Pattern::Wildcard => {
            // no child nodes to visit
        }
        Pattern::Rest => {
            // no child nodes to visit
        }
        Pattern::Maybe(unwrap) => {
            let unwrap_pattern = tree.get(*unwrap);
            visitor.visit_pattern(tree, *unwrap, unwrap_pattern);
        }
        Pattern::Reference {
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
        Pattern::Binding { name: _ } => {
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
            visitor.visit_expression(tree, *r#type, type_node);
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

/// Walk the PatternField.
pub fn walk_pattern_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<PatternField>,
    pattern_field: &PatternField,
) {
    visitor.visit_any(tree, NodeType::PatternField, id.id);
    match pattern_field {
        PatternField::Named {
            name: _,
            pattern,
            mutability: _,
        } => {
            if let Some(pattern_id) = pattern {
                let pattern_node = tree.get(*pattern_id);
                visitor.visit_pattern(tree, *pattern_id, pattern_node);
            }
        }
        PatternField::NamedAlias {
            name: _,
            alias: _,
            mutability: _,
        } => {
            // no child nodes to visit
        }
        PatternField::Positional { pattern } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, pattern_node);
        }
    }
}

/// Walk the MatchCase.
pub fn walk_match_case<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<MatchCase>,
    match_case: &MatchCase,
) {
    visitor.visit_any(tree, NodeType::MatchCase, id.id);
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

// ----------------------------------------------------------------------------
// Annotations
// ----------------------------------------------------------------------------

/// Walk the Annotation.
pub fn walk_annotation<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Annotation>,
    annotation: &Annotation,
) {
    visitor.visit_any(tree, NodeType::Annotation, id.id);
    match annotation {
        Annotation::Blank { node, .. } => {
            visitor.visit_blank(tree, *node, tree.get(*node));
        }
        Annotation::Doc { node, .. } => {
            visitor.visit_doc(tree, *node, tree.get(*node));
        }
        Annotation::Comment { node, .. } => {
            visitor.visit_comment(tree, *node, tree.get(*node));
        }
        Annotation::Tag { node, .. } => {
            visitor.visit_tag(tree, *node, tree.get(*node));
        }
        Annotation::Decorator { node, .. } => {
            visitor.visit_decorator(tree, *node, tree.get(*node));
        }
    }
}

/// Walk the Blank.
pub fn walk_blank<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Blank>,
    _blank: &Blank,
) {
    visitor.visit_any(tree, NodeType::Blank, id.id);
}

/// Walk the Doc.
pub fn walk_doc<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Doc>,
    _doc: &Doc,
) {
    visitor.visit_any(tree, NodeType::Doc, id.id);
}

/// Walk the Comment.
pub fn walk_comment<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Comment>,
    _comment: &Comment,
) {
    visitor.visit_any(tree, NodeType::Comment, id.id);
}

/// Walk the Tag.
pub fn walk_tag<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Tag>,
    tag: &Tag,
) {
    visitor.visit_any(tree, NodeType::Tag, id.id);
    if let Some(arguments) = &tag.arguments {
        for argument in arguments {
            let argument_node = tree.get(*argument);
            visitor.visit_argument(tree, *argument, argument_node);
        }
    }
}

/// Walk the Decorator.
pub fn walk_decorator<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Decorator>,
    decorator: &Decorator,
) {
    visitor.visit_any(tree, NodeType::Decorator, id.id);
    if let Some(arguments) = &decorator.arguments {
        for argument in arguments {
            let argument_node = tree.get(*argument);
            visitor.visit_argument(tree, *argument, argument_node);
        }
    }
}

// ----------------------------------------------------------------------------
// Traversal functions
// ----------------------------------------------------------------------------

pub fn walk_any(visitor: &mut dyn NodeVisitor, tree: &NodeTree, node_type: NodeType, node_id: u32) {
    let local_idx = tree.local_id_by_node[node_id as usize];
    match node_type {
        // --------------------------------------------------------------------
        // Groupings
        // --------------------------------------------------------------------
        NodeType::Expression => {
            let expression = tree.expressions.get(local_idx);
            walk_expression(visitor, tree, NodeId::new(node_id), expression);
        }
        NodeType::Block => {
            let block = tree.blocks.get(local_idx);
            walk_block(visitor, tree, NodeId::new(node_id), block);
        }
        // --------------------------------------------------------------------
        // Declarations
        // --------------------------------------------------------------------
        NodeType::Module => {
            let module = tree.modules.get(local_idx);
            walk_module(visitor, tree, NodeId::new(node_id), module);
        }
        NodeType::Struct => {
            let struct_node = tree.structs.get(local_idx);
            walk_struct(visitor, tree, NodeId::new(node_id), struct_node);
        }
        NodeType::StructField => {
            let struct_field = tree.struct_fields.get(local_idx);
            walk_struct_field(visitor, tree, NodeId::new(node_id), struct_field);
        }
        NodeType::Enum => {
            let enum_node = tree.enums.get(local_idx);
            walk_enum(visitor, tree, NodeId::new(node_id), enum_node);
        }
        NodeType::EnumField => {
            let enum_field = tree.enum_fields.get(local_idx);
            walk_enum_field(visitor, tree, NodeId::new(node_id), enum_field);
        }
        NodeType::Union => {
            let union_node = tree.unions.get(local_idx);
            walk_union(visitor, tree, NodeId::new(node_id), union_node);
        }
        NodeType::UnionField => {
            let union_field = tree.union_fields.get(local_idx);
            walk_union_field(visitor, tree, NodeId::new(node_id), union_field);
        }
        NodeType::Trait => {
            let trait_node = tree.traits.get(local_idx);
            walk_trait(visitor, tree, NodeId::new(node_id), trait_node);
        }
        NodeType::Implement => {
            let implement = tree.implements.get(local_idx);
            walk_implement(visitor, tree, NodeId::new(node_id), implement);
        }
        NodeType::Function => {
            let function = tree.functions.get(local_idx);
            walk_function(visitor, tree, NodeId::new(node_id), function);
        }
        // --------------------------------------------------------------------
        // Context
        // --------------------------------------------------------------------
        NodeType::With => {
            let with = tree.withs.get(local_idx);
            walk_with(visitor, tree, NodeId::new(node_id), with);
        }
        NodeType::WithClause => {
            let with_clause = tree.with_clauses.get(local_idx);
            walk_with_clause(visitor, tree, NodeId::new(node_id), with_clause);
        }
        NodeType::Use => {
            let use_node = tree.uses.get(local_idx);
            walk_use(visitor, tree, NodeId::new(node_id), use_node);
        }
        NodeType::UseClause => {
            let use_clause = tree.use_clauses.get(local_idx);
            walk_use_clause(visitor, tree, NodeId::new(node_id), use_clause);
        }
        NodeType::UseItem => {
            let use_item = tree.use_items.get(local_idx);
            walk_use_item(visitor, tree, NodeId::new(node_id), use_item);
        }
        // --------------------------------------------------------------------
        // Control
        // --------------------------------------------------------------------
        NodeType::If => {
            let if_node = tree.ifs.get(local_idx);
            walk_if(visitor, tree, NodeId::new(node_id), if_node);
        }
        NodeType::While => {
            let while_node = tree.whiles.get(local_idx);
            walk_while(visitor, tree, NodeId::new(node_id), while_node);
        }
        NodeType::For => {
            let for_node = tree.fors.get(local_idx);
            walk_for(visitor, tree, NodeId::new(node_id), for_node);
        }
        NodeType::Loop => {
            let loop_node = tree.loops.get(local_idx);
            walk_loop(visitor, tree, NodeId::new(node_id), loop_node);
        }
        NodeType::Break => {
            let break_node = tree.breaks.get(local_idx);
            walk_break(visitor, tree, NodeId::new(node_id), break_node);
        }
        NodeType::Continue => {
            let continue_node = tree.continues.get(local_idx);
            walk_continue(visitor, tree, NodeId::new(node_id), continue_node);
        }
        NodeType::Defer => {
            let defer = tree.defers.get(local_idx);
            walk_defer(visitor, tree, NodeId::new(node_id), defer);
        }
        NodeType::Return => {
            let return_node = tree.returns.get(local_idx);
            walk_return(visitor, tree, NodeId::new(node_id), return_node);
        }
        NodeType::Try => {
            let try_node = tree.trys.get(local_idx);
            walk_try(visitor, tree, NodeId::new(node_id), try_node);
        }
        // --------------------------------------------------------------------
        // Bindings
        // --------------------------------------------------------------------
        NodeType::Let => {
            let let_node = tree.lets.get(local_idx);
            walk_let(visitor, tree, NodeId::new(node_id), let_node);
        }
        NodeType::Parameter => {
            let parameter = tree.parameters.get(local_idx);
            walk_parameter(visitor, tree, NodeId::new(node_id), parameter);
        }
        NodeType::Argument => {
            let argument = tree.arguments.get(local_idx);
            walk_argument(visitor, tree, NodeId::new(node_id), argument);
        }
        // Literals
        NodeType::ScalarLiteral => {
            let scalar_literal = tree.scalar_literals.get(local_idx);
            walk_scalar_literal(visitor, tree, NodeId::new(node_id), scalar_literal);
        }
        NodeType::TypeLiteral => {
            let type_literal = tree.type_literals.get(local_idx);
            walk_type_literal(visitor, tree, NodeId::new(node_id), type_literal);
        }
        NodeType::RangeLiteral => {
            let range_literal = tree.range_literals.get(local_idx);
            walk_range_literal(visitor, tree, NodeId::new(node_id), range_literal);
        }
        NodeType::TupleLiteral => {
            let tuple_literal = tree.tuple_literals.get(local_idx);
            walk_tuple_literal(visitor, tree, NodeId::new(node_id), tuple_literal);
        }
        NodeType::TupleLiteralField => {
            let tuple_literal_field = tree.tuple_literal_fields.get(local_idx);
            walk_tuple_literal_field(visitor, tree, NodeId::new(node_id), tuple_literal_field);
        }
        NodeType::ArrayLiteral => {
            let array_literal = tree.array_literals.get(local_idx);
            walk_array_literal(visitor, tree, NodeId::new(node_id), array_literal);
        }
        NodeType::StructLiteral => {
            let struct_literal = tree.struct_literals.get(local_idx);
            walk_struct_literal(visitor, tree, NodeId::new(node_id), struct_literal);
        }
        NodeType::FieldLiteral => {
            let field_literal = tree.field_literals.get(local_idx);
            walk_field_literal(visitor, tree, NodeId::new(node_id), field_literal);
        }
        // --------------------------------------------------------------------
        // Calls
        // --------------------------------------------------------------------
        NodeType::Index => {
            let index = tree.indexes.get(local_idx);
            walk_index(visitor, tree, NodeId::new(node_id), index);
        }
        NodeType::Call => {
            let call = tree.calls.get(local_idx);
            walk_call(visitor, tree, NodeId::new(node_id), call);
        }
        // --------------------------------------------------------------------
        // Matching
        // --------------------------------------------------------------------
        NodeType::Match => {
            let match_node = tree.matches.get(local_idx);
            walk_match(visitor, tree, NodeId::new(node_id), match_node);
        }
        NodeType::Pattern => {
            let pattern = tree.patterns.get(local_idx);
            walk_pattern(visitor, tree, NodeId::new(node_id), pattern);
        }
        NodeType::PatternField => {
            let pattern_field = tree.pattern_fields.get(local_idx);
            walk_pattern_field(visitor, tree, NodeId::new(node_id), pattern_field);
        }
        NodeType::MatchCase => {
            let match_case = tree.match_cases.get(local_idx);
            walk_match_case(visitor, tree, NodeId::new(node_id), match_case);
        }
        // --------------------------------------------------------------------
        // Annotations
        // --------------------------------------------------------------------
        NodeType::Annotation => {
            let annotation = tree.annotations.get(local_idx);
            walk_annotation(visitor, tree, NodeId::new(node_id), annotation);
        }
        NodeType::Blank => {
            let blank = tree.blanks.get(local_idx);
            walk_blank(visitor, tree, NodeId::new(node_id), blank);
        }
        NodeType::Doc => {
            let doc = tree.docs.get(local_idx);
            walk_doc(visitor, tree, NodeId::new(node_id), doc);
        }
        NodeType::Comment => {
            let comment = tree.comments.get(local_idx);
            walk_comment(visitor, tree, NodeId::new(node_id), comment);
        }
        NodeType::Tag => {
            let tag = tree.tags.get(local_idx);
            walk_tag(visitor, tree, NodeId::new(node_id), tag);
        }
        NodeType::Decorator => {
            let decorator = tree.decorators.get(local_idx);
            walk_decorator(visitor, tree, NodeId::new(node_id), decorator);
        }
    }
}
