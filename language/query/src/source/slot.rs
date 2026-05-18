use destack_core::StringId;
use destack_dir as dir;
use destack_source::Span;

use super::{
    DirQueryContext, enclosing_missing_expression, enclosing_spans_at_cursor,
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
    DeclaratorValue(dir::LocalNodeId<dir::Declarator>),
    /// The slot expects a value expression.
    Value,
    /// The slot expects a type expression.
    Type,
}

/// Return whether one assign pattern contains the expression.
fn assign_pattern_contains_expression(
    tree: &dir::Tree,
    assign_pattern_id: dir::LocalNodeId<dir::AssignPattern>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let assign_pattern = tree.get(assign_pattern_id);

    match assign_pattern {
        dir::AssignPattern::Expression { value } => *value == expression_id,
        dir::AssignPattern::Assign { pattern, value } => {
            assign_pattern_contains_expression(tree, *pattern, expression_id)
                || *value == expression_id
        }
        dir::AssignPattern::Sequence { fields } | dir::AssignPattern::Object { fields } => {
            fields.iter().any(|field_id| {
                assign_pattern_field_contains_expression(tree, *field_id, expression_id)
            })
        }
    }
}

/// Return whether one assign pattern field contains the expression.
fn assign_pattern_field_contains_expression(
    tree: &dir::Tree,
    assign_pattern_field_id: dir::LocalNodeId<dir::AssignPatternField>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let assign_pattern_field = tree.get(assign_pattern_field_id);

    match assign_pattern_field {
        dir::AssignPatternField::Named { pattern, .. } => pattern.is_some_and(|pattern_id| {
            assign_pattern_contains_expression(tree, pattern_id, expression_id)
        }),
        dir::AssignPatternField::Computed { key, pattern } => {
            *key == expression_id
                || assign_pattern_contains_expression(tree, *pattern, expression_id)
        }
        dir::AssignPatternField::Positional { pattern } => {
            assign_pattern_contains_expression(tree, *pattern, expression_id)
        }
        dir::AssignPatternField::Spread { pattern } => pattern.is_some_and(|pattern_id| {
            assign_pattern_contains_expression(tree, pattern_id, expression_id)
        }),
        dir::AssignPatternField::Elision => false,
    }
}

/// Resolve the structural context for the innermost expression slot at the cursor.
pub(crate) fn expression_slot_position(
    ctx: DirQueryContext<'_>,
    source: &str,
    offset: u32,
) -> Option<ExpressionSlotPosition> {
    let owner = expression_slot_owner(ctx, source, offset)?;

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
    ctx: DirQueryContext<'_>,
    source: &str,
    offset: u32,
) -> Vec<StringId> {
    let parsed_tree = ctx.tree();

    // missing initializer slots should resolve through the recovered missing node first
    if let Some(declarator_id) = current_initializer_declarator(ctx, source, offset) {
        let declarator = parsed_tree.get(declarator_id);
        let mut names = Vec::new();
        collect_pattern_binding_names(parsed_tree, declarator.pattern, &mut names);
        return names;
    }

    Vec::new()
}

/// Check whether the cursor sits in a missing declarator initializer slot.
pub(crate) fn missing_declarator_value_at_cursor(
    ctx: DirQueryContext<'_>,
    source: &str,
    offset: u32,
) -> bool {
    let parsed_tree = ctx.tree();
    let Some(declarator_id) = current_initializer_declarator(ctx, source, offset) else {
        return false;
    };

    let declarator = parsed_tree.get(declarator_id);
    let Some(value_id) = declarator.value else {
        return false;
    };

    matches!(parsed_tree.get(value_id), dir::Expression::Missing)
}

/// Resolve the structural owner for the innermost expression slot at the cursor.
fn expression_slot_owner(
    ctx: DirQueryContext<'_>,
    source: &str,
    offset: u32,
) -> Option<ExpressionSlotOwner> {
    if let Some(expr_id) = enclosing_missing_expression(ctx, offset) {
        return expression_slot_owner_for_missing_node(ctx.tree(), ctx.parents(), expr_id);
    }

    open_expression_slot_owner(ctx, source, offset)
}

/// Resolve the structural context for one missing expression node.
fn expression_slot_owner_for_missing_node(
    parsed_tree: &dir::Tree,
    parents: &dir::NodeParentIndex,
    expr_id: dir::LocalNodeId<dir::Expression>,
) -> Option<ExpressionSlotOwner> {
    let parent_id = parents.get(expr_id)?;

    match parsed_tree.get_node_type(parent_id) {
        dir::NodeType::Declarator => {
            let declarator_id = dir::LocalNodeId::<dir::Declarator>::new(parent_id);
            let declarator = parsed_tree.get(declarator_id);

            if declarator.value == Some(expr_id) {
                return Some(ExpressionSlotOwner::DeclaratorValue(declarator_id));
            }

            None
        }
        dir::NodeType::Parameter => {
            let parameter = parsed_tree.get(dir::LocalNodeId::<dir::Parameter>::new(parent_id));
            expression_slot_position_in_parameter(parameter, expr_id)
                .map(expression_slot_owner_from_position)
        }
        dir::NodeType::Argument => {
            let argument = parsed_tree.get(dir::LocalNodeId::<dir::Argument>::new(parent_id));
            expression_slot_position_in_argument(argument, expr_id)
                .map(expression_slot_owner_from_position)
        }
        dir::NodeType::Property => {
            let property = parsed_tree.get(dir::LocalNodeId::<dir::Property>::new(parent_id));
            expression_slot_position_in_property(property, expr_id)
                .map(expression_slot_owner_from_position)
        }
        dir::NodeType::Member => {
            let member = parsed_tree.get(dir::LocalNodeId::<dir::Member>::new(parent_id));
            expression_slot_position_in_member(member, expr_id)
                .map(expression_slot_owner_from_position)
        }
        dir::NodeType::Expression => {
            let parent = parsed_tree.get(dir::LocalNodeId::<dir::Expression>::new(parent_id));
            expression_slot_position_in_expression(parsed_tree, parent, expr_id)
                .map(expression_slot_owner_from_position)
        }
        _ => None,
    }
}

/// Resolve the structural context for an explicit value slot without a missing node.
fn open_expression_slot_owner(
    ctx: DirQueryContext<'_>,
    source: &str,
    offset: u32,
) -> Option<ExpressionSlotOwner> {
    let parsed_tree = ctx.tree();

    // walk enclosing expressions from inner to outer
    for enclosing in enclosing_spans_at_cursor(ctx, offset) {
        if parsed_tree.get_node_type(enclosing.idx) != dir::NodeType::Expression {
            continue;
        }

        // unwrap statement wrappers to the actual expression owner
        let expr_id = dir::LocalNodeId::<dir::Expression>::new(enclosing.idx);
        let (expr_id, expr) = unwrap_statement_expression(parsed_tree, expr_id);
        let expr_span = ctx.source_map().get(expr_id.id);

        // return without a value still owns a value slot after the keyword
        if let dir::Expression::Return { value: None } = expr
            && cursor_is_after_expression_keyword(ctx, source, offset, expr_span, "return")
        {
            return Some(ExpressionSlotOwner::Value);
        }

        // yield without a value still owns a value slot after the keyword
        if let dir::Expression::Yield { value: None, .. } = expr
            && cursor_is_after_expression_keyword(ctx, source, offset, expr_span, "yield")
        {
            return Some(ExpressionSlotOwner::Value);
        }

        // bare `new` still owns one constructor slot after the keyword
        if let dir::Expression::New { left, .. } = expr
            && matches!(parsed_tree.get(*left), dir::Expression::Missing)
            && cursor_is_after_expression_keyword(ctx, source, offset, expr_span, "new")
        {
            return Some(ExpressionSlotOwner::Constructor);
        }
    }

    None
}

/// Unwrap statement expressions to the inner structural owner.
fn unwrap_statement_expression(
    parsed_tree: &dir::Tree,
    expr_id: dir::LocalNodeId<dir::Expression>,
) -> (dir::LocalNodeId<dir::Expression>, &dir::Expression) {
    let expr = parsed_tree.get(expr_id);
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
    ctx: DirQueryContext<'_>,
    source: &str,
    offset: u32,
    expr_span: Span,
    keyword: &str,
) -> bool {
    let Some(token) = previous_significant_token(ctx, offset) else {
        return false;
    };

    // require the keyword token inside the owning expression span
    if token.token.ty != dir::TokenType::Identifier {
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
    !tokens_between_offsets_include_statement_boundary(ctx, token.span.end, offset)
}

/// Resolve the structural context for a parameter child.
fn expression_slot_position_in_parameter(
    parameter: &dir::Parameter,
    expr_id: dir::LocalNodeId<dir::Expression>,
) -> Option<ExpressionSlotPosition> {
    match parameter {
        dir::Parameter::Named { default, .. } | dir::Parameter::Pattern { default, .. } => {
            if *default == Some(expr_id) {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        dir::Parameter::VariadicNamed { .. } | dir::Parameter::VariadicPattern { .. } => None,
        dir::Parameter::Error => None,
    }
}

/// Resolve the structural context for an argument child.
fn expression_slot_position_in_argument(
    argument: &dir::Argument,
    expr_id: dir::LocalNodeId<dir::Expression>,
) -> Option<ExpressionSlotPosition> {
    match argument {
        dir::Argument::Named { value, .. }
        | dir::Argument::Labeled { value, .. }
        | dir::Argument::Positional { value, .. }
        | dir::Argument::Spread { value, .. } => {
            if *value == expr_id {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        dir::Argument::Error => None,
    }
}

/// Resolve the structural context for a property child.
fn expression_slot_position_in_property(
    property: &dir::Property,
    expr_id: dir::LocalNodeId<dir::Expression>,
) -> Option<ExpressionSlotPosition> {
    match property {
        dir::Property::Field { value, .. } => {
            if *value == expr_id {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        dir::Property::Method { body, .. } => {
            if *body == Some(expr_id) {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        dir::Property::Spread { value, .. } => {
            if *value == expr_id {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        dir::Property::Error => None,
    }
}

/// Resolve the structural context for a member child.
fn expression_slot_position_in_member(
    member: &dir::Member,
    expr_id: dir::LocalNodeId<dir::Expression>,
) -> Option<ExpressionSlotPosition> {
    match member {
        dir::Member::AssociatedType { .. } => None,
        dir::Member::AssociatedConst { value, .. } => {
            if *value == Some(expr_id) {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        dir::Member::Field { default, .. } => {
            if *default == Some(expr_id) {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        dir::Member::Method { body, .. } => {
            if *body == Some(expr_id) {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        dir::Member::StaticBlock { body, .. } | dir::Member::ComptimeBlock { body, .. } => {
            if *body == expr_id {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        dir::Member::Error => None,
    }
}

/// Resolve the structural context for an expression child.
fn expression_slot_position_in_expression(
    parsed_tree: &dir::Tree,
    expression: &dir::Expression,
    expr_id: dir::LocalNodeId<dir::Expression>,
) -> Option<ExpressionSlotPosition> {
    match expression {
        dir::Expression::ObjectExpression { .. } => None,
        dir::Expression::Parenthesized { expression }
        | dir::Expression::Comptime { body: expression }
        | dir::Expression::Await { expression }
        | dir::Expression::Unary {
            right: expression, ..
        }
        | dir::Expression::MoveOf {
            right: expression, ..
        }
        | dir::Expression::BorrowOf {
            right: expression, ..
        } => {
            if *expression == expr_id {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        dir::Expression::SequenceExpression { expressions } => {
            if expressions.contains(&expr_id) {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        dir::Expression::Binary { left, right, .. } => {
            if *left == expr_id || *right == expr_id {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        dir::Expression::Assign { left, right, .. } => {
            if assign_pattern_contains_expression(parsed_tree, *left, expr_id) || *right == expr_id
            {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        dir::Expression::Return { value } | dir::Expression::Yield { value, .. } => {
            if *value == Some(expr_id) {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        dir::Expression::Throw { value } => {
            if *value == expr_id {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        dir::Expression::Index { left, index, .. } => {
            if *left == expr_id || *index == Some(expr_id) {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        dir::Expression::Member { left, .. }
        | dir::Expression::PrivateMember { left, .. }
        | dir::Expression::Instantiation { left, .. }
        | dir::Expression::Call { left, .. } => {
            if *left == expr_id {
                return Some(ExpressionSlotPosition::Value);
            }

            None
        }
        dir::Expression::New { left, .. } => {
            if *left == expr_id {
                return Some(ExpressionSlotPosition::Constructor);
            }

            None
        }
        dir::Expression::As { expression, .. } | dir::Expression::Satisfies { expression, .. } => {
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
    ctx: DirQueryContext<'_>,
    source: &str,
    offset: u32,
) -> Option<dir::LocalNodeId<dir::Declarator>> {
    // prefer direct slot ownership when the cursor is in one declarator value slot
    if let Some(ExpressionSlotOwner::DeclaratorValue(declarator_id)) =
        expression_slot_owner(ctx, source, offset)
    {
        return Some(declarator_id);
    }

    // otherwise use initializer spans that still own the cursor
    initializer_declarator_at_cursor(ctx, offset)
}

/// Resolve the declarator whose initializer span still owns the cursor.
fn initializer_declarator_at_cursor(
    ctx: DirQueryContext<'_>,
    offset: u32,
) -> Option<dir::LocalNodeId<dir::Declarator>> {
    let parsed_tree = ctx.tree();
    let parents = ctx.parents();

    // walk enclosing nodes and their declarator parents
    for enclosing in enclosing_spans_at_cursor(ctx, offset) {
        for parent_id in
            std::iter::once(enclosing.idx).chain(parents.walk_parents_by_id(enclosing.idx))
        {
            if parsed_tree.get_node_type(parent_id) != dir::NodeType::Declarator {
                continue;
            }

            let declarator_id = dir::LocalNodeId::<dir::Declarator>::new(parent_id);
            let declarator_span = ctx.source_map().get(declarator_id.id);
            let declarator = parsed_tree.get(declarator_id);
            let Some(value_id) = declarator.value else {
                continue;
            };

            // missing initializer gaps still belong to the declarator after `=`
            if matches!(parsed_tree.get(value_id), dir::Expression::Missing)
                && cursor_is_after_initializer_assign(ctx, declarator_span, offset)
            {
                return Some(declarator_id);
            }

            let value_span = ctx.source_map().get(value_id.id);
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
    ctx: DirQueryContext<'_>,
    declarator_span: Span,
    offset: u32,
) -> bool {
    let Some(token) = previous_significant_token(ctx, offset) else {
        return false;
    };

    // the initializer gap starts after the owning `=`
    if token.token.ty != dir::TokenType::Assign {
        return false;
    }

    if token.span.start < declarator_span.start || token.span.end > declarator_span.end {
        return false;
    }

    token.span.end <= offset
}

/// Collect simple binding names from one pattern.
fn collect_pattern_binding_names(
    parsed_tree: &dir::Tree,
    pattern_id: dir::LocalNodeId<dir::Pattern>,
    names: &mut Vec<StringId>,
) {
    let pattern = parsed_tree.get(pattern_id);

    match pattern {
        dir::Pattern::Assign { pattern, .. } => {
            collect_pattern_binding_names(parsed_tree, *pattern, names);
        }
        dir::Pattern::Binding {
            name,
            pattern: nested,
            ..
        } => {
            names.push(*name);

            if let Some(nested) = nested {
                collect_pattern_binding_names(parsed_tree, *nested, names);
            }
        }
        dir::Pattern::Must(right)
        | dir::Pattern::BorrowOf { right, .. }
        | dir::Pattern::MoveOf { right, .. }
        | dir::Pattern::DereferenceOf { right } => {
            collect_pattern_binding_names(parsed_tree, *right, names);
        }
        dir::Pattern::Tuple { fields }
        | dir::Pattern::Sequence { fields }
        | dir::Pattern::Object { fields }
        | dir::Pattern::TaggedTuple { fields, .. }
        | dir::Pattern::TaggedObject { fields, .. } => {
            for field_id in fields {
                let field = parsed_tree.get(*field_id);

                match field {
                    dir::PatternField::Named { name, pattern, .. } => {
                        if let Some(pattern) = pattern {
                            collect_pattern_binding_names(parsed_tree, *pattern, names);
                            continue;
                        }

                        names.push(name.string());
                    }
                    dir::PatternField::Computed { pattern, .. } => {
                        collect_pattern_binding_names(parsed_tree, *pattern, names);
                    }
                    dir::PatternField::Positional { pattern, .. } => {
                        collect_pattern_binding_names(parsed_tree, *pattern, names);
                    }
                    dir::PatternField::Spread { pattern, .. } => {
                        let Some(pattern) = pattern else {
                            continue;
                        };

                        collect_pattern_binding_names(parsed_tree, *pattern, names);
                    }
                    dir::PatternField::Elision => {}
                }
            }
        }
        dir::Pattern::Union { patterns } => {
            for pattern_id in patterns {
                collect_pattern_binding_names(parsed_tree, *pattern_id, names);
            }
        }
        dir::Pattern::Wildcard
        | dir::Pattern::Expression { .. }
        | dir::Pattern::Range { .. }
        | dir::Pattern::TypeExpression { .. } => {}
    }
}
