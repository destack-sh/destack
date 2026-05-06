use destack_ast as ast;
use destack_core::StringId;
use destack_source::Span;

use super::{
    AstQueryContext, enclosing_missing_expression, enclosing_spans_at_cursor,
    previous_significant_token, span_owns_cursor, token_text,
    tokens_between_offsets_include_statement_boundary,
};

/// The structural context for one expression slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExpressionSlotPosition {
    /// The slot expects one constructable value for `new`.
    Constructor,
    /// The slot expects a value expression.
    Value,
    /// The slot expects a type expression.
    Type,
}

/// The structural owner for one expression slot.
#[derive(Debug, Clone, Copy)]
enum ExpressionSlotOwner {
    /// The slot expects one constructable value for `new`.
    Constructor,
    /// The slot is the value side of one declarator.
    DeclaratorValue(ast::LocalNodeId<ast::Declarator>),
    /// The slot expects a value expression.
    Value,
    /// The slot expects a type expression.
    Type,
}

/// Return whether one assign pattern contains the expression.
fn assign_pattern_contains_expression(
    tree: &ast::Tree,
    assign_pattern_id: ast::LocalNodeId<ast::AssignPattern>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let assign_pattern = tree.get(assign_pattern_id);

    match assign_pattern {
        ast::AssignPattern::Expression { value } => *value == expression_id,
        ast::AssignPattern::Assign { pattern, value } => {
            assign_pattern_contains_expression(tree, *pattern, expression_id)
                || *value == expression_id
        }
        ast::AssignPattern::Array { fields } | ast::AssignPattern::Object { fields } => {
            fields.iter().any(|field_id| {
                assign_pattern_field_contains_expression(tree, *field_id, expression_id)
            })
        }
    }
}

/// Return whether one assign pattern field contains the expression.
fn assign_pattern_field_contains_expression(
    tree: &ast::Tree,
    assign_pattern_field_id: ast::LocalNodeId<ast::AssignPatternField>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let assign_pattern_field = tree.get(assign_pattern_field_id);

    match assign_pattern_field {
        ast::AssignPatternField::Named { pattern, .. } => pattern.is_some_and(|pattern_id| {
            assign_pattern_contains_expression(tree, pattern_id, expression_id)
        }),
        ast::AssignPatternField::Computed { key, pattern } => {
            *key == expression_id
                || assign_pattern_contains_expression(tree, *pattern, expression_id)
        }
        ast::AssignPatternField::Positional { pattern } => {
            assign_pattern_contains_expression(tree, *pattern, expression_id)
        }
        ast::AssignPatternField::Spread { pattern } => pattern.is_some_and(|pattern_id| {
            assign_pattern_contains_expression(tree, pattern_id, expression_id)
        }),
        ast::AssignPatternField::Elision => false,
    }
}

/// Resolve the structural context for the innermost expression slot at the cursor.
pub(crate) fn expression_slot_position(
    ast: AstQueryContext<'_>,
    source: &str,
    offset: u32,
) -> Option<ExpressionSlotPosition> {
    let owner = expression_slot_owner(ast, source, offset)?;

    // collapse the richer slot owner to the public type/value classification
    match owner {
        ExpressionSlotOwner::Constructor => Some(ExpressionSlotPosition::Constructor),
        ExpressionSlotOwner::DeclaratorValue(_) | ExpressionSlotOwner::Value => {
            Some(ExpressionSlotPosition::Value)
        }
        ExpressionSlotOwner::Type => Some(ExpressionSlotPosition::Type),
    }
}

/// Collect the binding names excluded from completion inside one initializer.
pub(crate) fn current_initializer_binding_names(
    ast: AstQueryContext<'_>,
    source: &str,
    offset: u32,
) -> Vec<StringId> {
    let ast_tree = ast.tree();

    // missing initializer slots should resolve through the recovered missing node first
    if let Some(declarator_id) = current_initializer_declarator(ast, source, offset) {
        let declarator = ast_tree.get(declarator_id);
        let mut names = Vec::new();
        collect_pattern_binding_names(ast_tree, declarator.pattern, &mut names);
        return names;
    }

    Vec::new()
}

/// Check whether the cursor sits in a missing declarator initializer slot.
pub(crate) fn missing_declarator_value_at_cursor(
    ast: AstQueryContext<'_>,
    source: &str,
    offset: u32,
) -> bool {
    let ast_tree = ast.tree();
    let Some(declarator_id) = current_initializer_declarator(ast, source, offset) else {
        return false;
    };

    let declarator = ast_tree.get(declarator_id);
    let Some(value_id) = declarator.value else {
        return false;
    };

    matches!(ast_tree.get(value_id), ast::Expression::Missing)
}

/// Resolve the structural owner for the innermost expression slot at the cursor.
fn expression_slot_owner(
    ast: AstQueryContext<'_>,
    source: &str,
    offset: u32,
) -> Option<ExpressionSlotOwner> {
    if let Some(expr_id) = enclosing_missing_expression(ast, offset) {
        return expression_slot_owner_for_missing_node(ast.tree(), ast.parents(), expr_id);
    }

    open_expression_slot_owner(ast, source, offset)
}

/// Resolve the structural context for one missing expression node.
fn expression_slot_owner_for_missing_node(
    ast_tree: &ast::Tree,
    parents: &ast::NodeParentIndex,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> Option<ExpressionSlotOwner> {
    let parent_id = parents.get(expr_id)?;

    match ast_tree.get_node_type(parent_id) {
        ast::NodeType::Declarator => {
            let declarator_id = ast::LocalNodeId::<ast::Declarator>::new(parent_id);
            let declarator = ast_tree.get(declarator_id);

            if declarator.value == Some(expr_id) {
                return Some(ExpressionSlotOwner::DeclaratorValue(declarator_id));
            }

            None
        }
        ast::NodeType::Parameter => {
            let parameter = ast_tree.get(ast::LocalNodeId::<ast::Parameter>::new(parent_id));
            expression_slot_position_in_parameter(parameter, expr_id)
                .map(expression_slot_owner_from_position)
        }
        ast::NodeType::Argument => {
            let argument = ast_tree.get(ast::LocalNodeId::<ast::Argument>::new(parent_id));
            expression_slot_position_in_argument(argument, expr_id)
                .map(expression_slot_owner_from_position)
        }
        ast::NodeType::Property => {
            let property = ast_tree.get(ast::LocalNodeId::<ast::Property>::new(parent_id));
            expression_slot_position_in_property(property, expr_id)
                .map(expression_slot_owner_from_position)
        }
        ast::NodeType::Member => {
            let member = ast_tree.get(ast::LocalNodeId::<ast::Member>::new(parent_id));
            expression_slot_position_in_member(member, expr_id)
                .map(expression_slot_owner_from_position)
        }
        ast::NodeType::Expression => {
            let parent = ast_tree.get(ast::LocalNodeId::<ast::Expression>::new(parent_id));
            expression_slot_position_in_expression(ast_tree, parent, expr_id)
                .map(expression_slot_owner_from_position)
        }
        _ => None,
    }
}

/// Resolve the structural context for an explicit value slot without a missing node.
fn open_expression_slot_owner(
    ast: AstQueryContext<'_>,
    source: &str,
    offset: u32,
) -> Option<ExpressionSlotOwner> {
    let ast_tree = ast.tree();

    // walk enclosing expressions from inner to outer
    for enclosing in enclosing_spans_at_cursor(ast, offset) {
        if ast_tree.get_node_type(enclosing.idx) != ast::NodeType::Expression {
            continue;
        }

        // unwrap statement wrappers to the actual expression owner
        let expr_id = ast::LocalNodeId::<ast::Expression>::new(enclosing.idx);
        let (expr_id, expr) = unwrap_statement_ast_expression(ast_tree, expr_id);
        let expr_span = ast.source_map().get(expr_id.id);

        // return without a value still owns a value slot after the keyword
        if let ast::Expression::Return { value: None } = expr
            && cursor_is_after_expression_keyword(ast, source, offset, expr_span, "return")
        {
            return Some(ExpressionSlotOwner::Value);
        }

        // yield without a value still owns a value slot after the keyword
        if let ast::Expression::Yield { value: None, .. } = expr
            && cursor_is_after_expression_keyword(ast, source, offset, expr_span, "yield")
        {
            return Some(ExpressionSlotOwner::Value);
        }

        // bare `new` still owns one constructor slot after the keyword
        if let ast::Expression::New { left, .. } = expr
            && matches!(ast_tree.get(*left), ast::Expression::Missing)
            && cursor_is_after_expression_keyword(ast, source, offset, expr_span, "new")
        {
            return Some(ExpressionSlotOwner::Constructor);
        }
    }

    None
}

/// Unwrap statement expressions to the inner structural owner.
fn unwrap_statement_ast_expression(
    ast_tree: &ast::Tree,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> (ast::LocalNodeId<ast::Expression>, &ast::Expression) {
    let expr = ast_tree.get(expr_id);
    (expr_id, expr)
}

/// Convert one public type/value classification into a generic slot owner.
fn expression_slot_owner_from_position(position: ExpressionSlotPosition) -> ExpressionSlotOwner {
    match position {
        ExpressionSlotPosition::Constructor => ExpressionSlotOwner::Constructor,
        ExpressionSlotPosition::Value => ExpressionSlotOwner::Value,
        ExpressionSlotPosition::Type => ExpressionSlotOwner::Type,
    }
}

/// Check whether the cursor still belongs to one keyword-owned expression slot.
fn cursor_is_after_expression_keyword(
    ast: AstQueryContext<'_>,
    source: &str,
    offset: u32,
    expr_span: Span,
    keyword: &str,
) -> bool {
    let Some(token) = previous_significant_token(ast, offset) else {
        return false;
    };

    // require the keyword token inside the owning expression span
    if token.token.ty != ast::TokenType::Identifier {
        return false;
    }

    let Some(text) = token_text(source, token.span) else {
        return false;
    };
    if text != keyword {
        return false;
    }

    if token.span.start < expr_span.start || token.span.end > expr_span.end {
        return false;
    }

    // statement boundaries end the keyword-owned slot
    !tokens_between_offsets_include_statement_boundary(ast, token.span.end, offset)
}

/// Resolve the structural context for a parameter child.
fn expression_slot_position_in_parameter(
    parameter: &ast::Parameter,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> Option<ExpressionSlotPosition> {
    match parameter {
        ast::Parameter::Named { default, .. } | ast::Parameter::Pattern { default, .. } => {
            if *default == Some(expr_id) {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        ast::Parameter::VariadicNamed { .. } | ast::Parameter::VariadicPattern { .. } => None,
        ast::Parameter::Error => None,
    }
}

/// Resolve the structural context for an argument child.
fn expression_slot_position_in_argument(
    argument: &ast::Argument,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> Option<ExpressionSlotPosition> {
    match argument {
        ast::Argument::Named { value, .. }
        | ast::Argument::Labeled { value, .. }
        | ast::Argument::Positional { value, .. }
        | ast::Argument::Spread { value, .. } => {
            if *value == expr_id {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        ast::Argument::Error => None,
    }
}

/// Resolve the structural context for a property child.
fn expression_slot_position_in_property(
    property: &ast::Property,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> Option<ExpressionSlotPosition> {
    match property {
        ast::Property::Field { value, .. } => {
            if *value == expr_id {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        ast::Property::Method { body, .. } => {
            if *body == Some(expr_id) {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        ast::Property::Spread { value, .. } => {
            if *value == expr_id {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        ast::Property::Error => None,
    }
}

/// Resolve the structural context for a member child.
fn expression_slot_position_in_member(
    member: &ast::Member,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> Option<ExpressionSlotPosition> {
    match member {
        ast::Member::AssociatedType { .. } => None,
        ast::Member::AssociatedConst { value, .. } => {
            if *value == Some(expr_id) {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        ast::Member::Field { default, .. } => {
            if *default == Some(expr_id) {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        ast::Member::Method { body, .. } => {
            if *body == Some(expr_id) {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        ast::Member::Embed { .. } => None,
        ast::Member::StaticBlock { body, .. } | ast::Member::ComptimeBlock { body, .. } => {
            if *body == expr_id {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        ast::Member::Error => None,
    }
}

/// Resolve the structural context for an expression child.
fn expression_slot_position_in_expression(
    ast_tree: &ast::Tree,
    expression: &ast::Expression,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> Option<ExpressionSlotPosition> {
    match expression {
        ast::Expression::ObjectExpression { .. } => None,
        ast::Expression::Parenthesized { expression }
        | ast::Expression::Comptime { body: expression }
        | ast::Expression::Await { expression }
        | ast::Expression::Delete { value: expression }
        | ast::Expression::Unary {
            right: expression, ..
        }
        | ast::Expression::ValueOf {
            right: expression, ..
        }
        | ast::Expression::ReferenceOf {
            right: expression, ..
        }
        | ast::Expression::PointerOf {
            right: expression, ..
        } => {
            if *expression == expr_id {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        ast::Expression::SequenceExpression { expressions } => {
            if expressions.contains(&expr_id) {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        ast::Expression::Binary { left, right, .. } => {
            if *left == expr_id || *right == expr_id {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        ast::Expression::Assign { left, right, .. } => {
            if assign_pattern_contains_expression(ast_tree, *left, expr_id) || *right == expr_id {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        ast::Expression::Return { value } | ast::Expression::Yield { value, .. } => {
            if *value == Some(expr_id) {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        ast::Expression::Throw { value } => {
            if *value == expr_id {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        ast::Expression::Index { left, index, .. } => {
            if *left == expr_id || *index == Some(expr_id) {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        ast::Expression::Member { left, .. }
        | ast::Expression::PrivateMember { left, .. }
        | ast::Expression::Instantiation { left, .. }
        | ast::Expression::Call { left, .. } => {
            if *left == expr_id {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        ast::Expression::New { left, .. } => {
            if *left == expr_id {
                return Some(ExpressionSlotPosition::Constructor);
            }

            None
        }
        ast::Expression::As { expression, .. } | ast::Expression::Satisfies { expression, .. } => {
            if *expression == expr_id {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        _ => None,
    }
}

/// Resolve the declarator that owns the initializer slot at one cursor offset.
fn current_initializer_declarator(
    ast: AstQueryContext<'_>,
    source: &str,
    offset: u32,
) -> Option<ast::LocalNodeId<ast::Declarator>> {
    // prefer direct slot ownership when the cursor is in one declarator value slot
    if let Some(ExpressionSlotOwner::DeclaratorValue(declarator_id)) =
        expression_slot_owner(ast, source, offset)
    {
        return Some(declarator_id);
    }

    // otherwise fall back to initializer spans that still own the cursor
    initializer_declarator_at_cursor(ast, offset)
}

/// Resolve the declarator whose initializer span still owns the cursor.
fn initializer_declarator_at_cursor(
    ast: AstQueryContext<'_>,
    offset: u32,
) -> Option<ast::LocalNodeId<ast::Declarator>> {
    let ast_tree = ast.tree();
    let parents = ast.parents();

    // walk enclosing nodes and their declarator parents
    for enclosing in enclosing_spans_at_cursor(ast, offset) {
        for parent_id in
            std::iter::once(enclosing.idx).chain(parents.walk_parents_by_id(enclosing.idx))
        {
            if ast_tree.get_node_type(parent_id) != ast::NodeType::Declarator {
                continue;
            }

            let declarator_id = ast::LocalNodeId::<ast::Declarator>::new(parent_id);
            let declarator_span = ast.source_map().get(declarator_id.id);
            let declarator = ast_tree.get(declarator_id);
            let Some(value_id) = declarator.value else {
                continue;
            };

            // missing initializer gaps still belong to the declarator after `=`
            if matches!(ast_tree.get(value_id), ast::Expression::Missing)
                && cursor_is_after_initializer_assign(ast, declarator_span, offset)
            {
                return Some(declarator_id);
            }

            let value_span = ast.source_map().get(value_id.id);
            if !span_owns_cursor(value_span, offset) {
                continue;
            }

            return Some(declarator_id);
        }
    }

    None
}

/// Check whether the cursor still belongs to one declarator initializer gap.
fn cursor_is_after_initializer_assign(
    ast: AstQueryContext<'_>,
    declarator_span: Span,
    offset: u32,
) -> bool {
    let Some(token) = previous_significant_token(ast, offset) else {
        return false;
    };

    // the initializer gap starts after the owning `=`
    if token.token.ty != ast::TokenType::Assign {
        return false;
    }

    if token.span.start < declarator_span.start || token.span.end > declarator_span.end {
        return false;
    }

    token.span.end <= offset
}

/// Collect simple binding names from one pattern.
fn collect_pattern_binding_names(
    ast_tree: &ast::Tree,
    pattern_id: ast::LocalNodeId<ast::Pattern>,
    names: &mut Vec<StringId>,
) {
    let pattern = ast_tree.get(pattern_id);

    match pattern {
        ast::Pattern::Assign { pattern, .. } => {
            collect_pattern_binding_names(ast_tree, *pattern, names);
        }
        ast::Pattern::Binding {
            name,
            pattern: nested,
            ..
        } => {
            names.push(*name);

            if let Some(nested) = nested {
                collect_pattern_binding_names(ast_tree, *nested, names);
            }
        }
        ast::Pattern::Must(right)
        | ast::Pattern::ReferenceOf { right, .. }
        | ast::Pattern::ValueOf { right, .. } => {
            collect_pattern_binding_names(ast_tree, *right, names);
        }
        ast::Pattern::Tuple { fields }
        | ast::Pattern::Array { fields }
        | ast::Pattern::Object { fields }
        | ast::Pattern::TaggedTuple { fields, .. }
        | ast::Pattern::TaggedObject { fields, .. } => {
            for field_id in fields {
                let field = ast_tree.get(*field_id);

                match field {
                    ast::PatternField::Named { name, pattern, .. } => {
                        if let Some(pattern) = pattern {
                            collect_pattern_binding_names(ast_tree, *pattern, names);
                            continue;
                        }

                        names.push(name.string());
                    }
                    ast::PatternField::Computed { pattern, .. } => {
                        collect_pattern_binding_names(ast_tree, *pattern, names);
                    }
                    ast::PatternField::Positional { pattern, .. } => {
                        collect_pattern_binding_names(ast_tree, *pattern, names);
                    }
                    ast::PatternField::Spread { pattern, .. } => {
                        let Some(pattern) = pattern else {
                            continue;
                        };

                        collect_pattern_binding_names(ast_tree, *pattern, names);
                    }
                    ast::PatternField::Elision => {}
                }
            }
        }
        ast::Pattern::Union { patterns } => {
            for pattern_id in patterns {
                collect_pattern_binding_names(ast_tree, *pattern_id, names);
            }
        }
        ast::Pattern::Wildcard
        | ast::Pattern::Expression { .. }
        | ast::Pattern::TypeExpression { .. } => {}
    }
}
