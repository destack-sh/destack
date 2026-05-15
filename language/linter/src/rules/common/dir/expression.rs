use destack_core::{StringId, StringPool};
use destack_dir as dir;
use destack_source::{ModuleId, Span};
use destack_workspace::{ArtifactCache, ProfileId};

use crate::ConstValue;

use super::{
    function_return_type, is_any_type, is_async_function_type, is_promise_type,
    symbol_value_type_map_for,
};

/// Return true when one expression is a numeric scalar literal.
pub fn expression_is_numeric_literal(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // normalize parenthesized wrappers first
    let expression_id = expression_unwrap_parenthesized(tree, expression_id);

    matches!(
        tree.get(expression_id),
        dir::Expression::ScalarLiteral(
            dir::ScalarLiteral::Integer(_)
                | dir::ScalarLiteral::Bigint(_)
                | dir::ScalarLiteral::Float(_)
        )
    )
}

/// Return one simple expression target from an assignment pattern.
pub fn assign_pattern_target_expression(
    tree: &dir::Tree,
    mut assign_pattern_id: dir::LocalNodeId<dir::AssignPattern>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    loop {
        let assign_pattern = tree.get(assign_pattern_id);

        // keep direct expression targets
        if let dir::AssignPattern::Expression { value } = assign_pattern {
            return Some(*value);
        }

        // unwrap defaulted targets before checking the base
        if let dir::AssignPattern::Assign { pattern, .. } = assign_pattern {
            assign_pattern_id = *pattern;
            continue;
        }

        // destructuring patterns are not one simple expression target
        return None;
    }
}

/// Return true when one assignment pattern contains one expression node.
pub fn assign_pattern_contains_expression(
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
            fields.iter().copied().any(|field_id| {
                assign_pattern_field_contains_expression(tree, field_id, expression_id)
            })
        }
    }
}

/// Return true when one assignment pattern field contains one expression node.
pub fn assign_pattern_field_contains_expression(
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

/// Return one assignment target expression for assignment-like expressions.
pub fn expression_assignment_target(
    tree: &dir::Tree,
    expression: &dir::Expression,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    match expression {
        dir::Expression::Assign {
            left,
            operator: _,
            right: _,
        } => assign_pattern_target_expression(tree, *left),
        dir::Expression::Unary {
            operator:
                dir::UnaryOperator::PreIncrement
                | dir::UnaryOperator::PostIncrement
                | dir::UnaryOperator::PreDecrement
                | dir::UnaryOperator::PostDecrement,
            right,
        } => Some(*right),
        _ => None,
    }
}

/// Return a static import target specifier for import-like expressions.
pub fn expression_import_target_static_specifier(expression: &dir::Expression) -> Option<StringId> {
    match expression {
        dir::Expression::Import { target, .. } => Some(*target),
        dir::Expression::Export {
            target: Some(target),
            ..
        } => Some(*target),
        _ => None,
    }
}

/// Return one static import target specifier for import-like expressions.
pub fn expression_import_target_specifier(
    _tree: &dir::Tree,
    expression: &dir::Expression,
) -> Option<StringId> {
    expression_import_target_static_specifier(expression)
}

/// Return the expression id with parenthesized nodes unwrapped.
pub fn expression_unwrap_parenthesized(
    tree: &dir::Tree,
    mut expression_id: dir::LocalNodeId<dir::Expression>,
) -> dir::LocalNodeId<dir::Expression> {
    loop {
        let dir::Expression::Parenthesized { expression } = tree.get(expression_id) else {
            return expression_id;
        };
        expression_id = *expression;
    }
}

/// Resolve the value expression for one DIR argument node.
pub fn argument_expression_id(
    tree: &dir::Tree,
    argument_id: dir::LocalNodeId<dir::Argument>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    let argument = tree.get(argument_id);

    match argument {
        dir::Argument::Named { name: _, value }
        | dir::Argument::Labeled { label: _, value }
        | dir::Argument::Positional { value }
        | dir::Argument::Spread { label: _, value } => Some(*value),
        dir::Argument::Error => None,
    }
}

/// Return true when one generic argument list contains the target segment.
fn generic_arguments_contain_reference_segment(
    tree: &dir::Tree,
    generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    target_segment: StringId,
) -> bool {
    generic_arguments.iter().any(|generic_argument_id| {
        generic_argument_contains_reference_segment(tree, *generic_argument_id, target_segment)
    })
}

/// Return true when one path or generic argument list contains the target segment.
fn path_or_generic_arguments_contain_reference_segment(
    tree: &dir::Tree,
    path: &dir::Path,
    generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    target_segment: StringId,
) -> bool {
    if path.last_segment() == Some(target_segment) {
        return true;
    }

    generic_arguments_contain_reference_segment(tree, generic_arguments, target_segment)
}

/// Return true when one type expression contains a reference ending in the target segment.
pub fn type_expression_contains_reference_segment(
    tree: &dir::Tree,
    type_expression_id: dir::LocalNodeId<dir::TypeExpression>,
    target_segment: StringId,
) -> bool {
    let type_expression = tree.get(type_expression_id);

    match type_expression {
        dir::TypeExpression::Reference {
            path,
            generic_arguments,
        } => path_or_generic_arguments_contain_reference_segment(
            tree,
            path,
            generic_arguments,
            target_segment,
        ),

        dir::TypeExpression::Member {
            left,
            name: _,
            generic_arguments,
        } => {
            type_expression_contains_reference_segment(tree, *left, target_segment)
                || generic_arguments_contain_reference_segment(
                    tree,
                    generic_arguments,
                    target_segment,
                )
        }

        dir::TypeExpression::Parenthesized { expression } => {
            type_expression_contains_reference_segment(tree, *expression, target_segment)
        }

        dir::TypeExpression::Readonly { target_type }
        | dir::TypeExpression::Shared { target_type }
        | dir::TypeExpression::KeyOf { target_type }
        | dir::TypeExpression::Must { target_type }
        | dir::TypeExpression::AsComptime { target_type }
        | dir::TypeExpression::Not { target_type }
        | dir::TypeExpression::OwnedOf {
            target_type,
            mutability: _,
            variance: _,
        }
        | dir::TypeExpression::BorrowedOf {
            target_type,
            mutability: _,
            variance: _,
        }
        | dir::TypeExpression::PointerOf {
            target_type,
            mutability: _,
        } => type_expression_contains_reference_segment(tree, *target_type, target_segment),

        dir::TypeExpression::TypeOfValue { value } => {
            expression_contains_reference_segment(tree, *value, target_segment)
        }

        dir::TypeExpression::Index { left, index } => {
            type_expression_contains_reference_segment(tree, *left, target_segment)
                || type_expression_contains_reference_segment(tree, *index, target_segment)
        }

        dir::TypeExpression::Conditional {
            left,
            extends_type,
            then_type,
            else_type,
        } => {
            type_expression_contains_reference_segment(tree, *left, target_segment)
                || type_expression_contains_reference_segment(tree, *extends_type, target_segment)
                || type_expression_contains_reference_segment(tree, *then_type, target_segment)
                || type_expression_contains_reference_segment(tree, *else_type, target_segment)
        }

        dir::TypeExpression::Mapped {
            parameter,
            readonly: _,
            optional: _,
            value,
        } => {
            let parameter = tree.get(*parameter);
            type_expression_contains_reference_segment(tree, parameter.source_type, target_segment)
                || parameter.key_remap.is_some_and(|key_remap| {
                    type_expression_contains_reference_segment(tree, key_remap, target_segment)
                })
                || value.is_some_and(|value| {
                    type_expression_contains_reference_segment(tree, value, target_segment)
                })
        }

        dir::TypeExpression::TemplateLiteral { strings: _, spans } => spans.iter().any(|span_id| {
            type_expression_contains_reference_segment(tree, *span_id, target_segment)
        }),

        dir::TypeExpression::Union { elements }
        | dir::TypeExpression::Intersection { elements } => elements.iter().any(|element_id| {
            type_expression_contains_reference_segment(tree, *element_id, target_segment)
        }),

        dir::TypeExpression::Infer {
            name: _,
            constraint,
            ..
        } => constraint.is_some_and(|constraint| {
            type_expression_contains_reference_segment(tree, constraint, target_segment)
        }),

        dir::TypeExpression::Predicate {
            asserts: _,
            subject: _,
            target,
        } => target.is_some_and(|target| {
            type_expression_contains_reference_segment(tree, target, target_segment)
        }),

        _ => false,
    }
}

/// Return true when one function signature return type contains the target segment.
pub fn function_signature_return_type_contains_reference_segment(
    tree: &dir::Tree,
    signature: &dir::FunctionSignature,
    target_segment: StringId,
) -> bool {
    let Some(return_type_id) = signature.return_type else {
        return false;
    };

    type_expression_contains_reference_segment(tree, return_type_id, target_segment)
}

/// Return the expression id with transparent wrappers unwrapped.
pub fn expression_unwrap_transparent(
    tree: &dir::Tree,
    mut expression_id: dir::LocalNodeId<dir::Expression>,
) -> dir::LocalNodeId<dir::Expression> {
    loop {
        let expression = tree.get(expression_id);
        match expression {
            dir::Expression::Parenthesized { expression } => {
                expression_id = *expression;
            }
            dir::Expression::Maybe { left, .. }
            | dir::Expression::Must { left, .. }
            | dir::Expression::Instantiation { left, .. } => {
                expression_id = *left;
            }
            dir::Expression::As { expression, .. }
            | dir::Expression::Satisfies { expression, .. } => {
                expression_id = *expression;
            }
            dir::Expression::MoveOf { right, .. } | dir::Expression::BorrowOf { right, .. } => {
                expression_id = *right;
            }
            _ => {
                return expression_id;
            }
        }
    }
}

/// Return the expression id with statement wrappers unwrapped.
pub fn expression_unwrap_statement(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> dir::LocalNodeId<dir::Expression> {
    expression_unwrap_transparent(tree, expression_id)
}

/// Return true when one binary expression is nested under the same operator.
pub fn binary_expression_is_nested_same_operator(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
    operator: dir::BinaryOperator,
) -> bool {
    let mut current_node_id = expression_id.id;

    loop {
        let Some(parent_node_id) = tree.get_parent(current_node_id) else {
            return false;
        };
        if parent_node_id.ty != dir::NodeType::Expression {
            return false;
        }

        let parent_expression_id = parent_node_id.into_typed::<dir::Expression>();
        let parent_expression = tree.get(parent_expression_id);
        match parent_expression {
            dir::Expression::Parenthesized { expression } if expression.id == current_node_id => {
                current_node_id = parent_node_id.id;
            }
            dir::Expression::Binary {
                operator: parent_operator,
                ..
            } => return *parent_operator == operator,
            _ => return false,
        }
    }
}

/// Collect all members of one flattened binary operator chain.
pub fn binary_expression_chain_members(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
    operator: dir::BinaryOperator,
    members: &mut Vec<dir::LocalNodeId<dir::Expression>>,
) {
    let expression_id = expression_unwrap_parenthesized(tree, expression_id);
    let expression = tree.get(expression_id);
    if let dir::Expression::Binary {
        left,
        operator: child_operator,
        right,
    } = expression
        && *child_operator == operator
    {
        binary_expression_chain_members(tree, *left, operator, members);
        binary_expression_chain_members(tree, *right, operator, members);
        return;
    }

    members.push(expression_id);
}

/// Return one source span that removes trailing arguments from an argument list.
pub fn trailing_argument_removal_span(
    source: &str,
    expression_span: Span,
    argument_spans: &[Span],
    first_redundant_index: usize,
) -> Option<Span> {
    // require a valid argument range
    if argument_spans.is_empty() || first_redundant_index >= argument_spans.len() {
        return None;
    }

    let source = source.as_bytes();
    let last_argument_span = *argument_spans.last()?;

    // remove from the comma before first redundant argument to the last argument end
    if first_redundant_index > 0 {
        let previous_argument_span = argument_spans[first_redundant_index - 1];
        let first_redundant_argument_span = argument_spans[first_redundant_index];
        if previous_argument_span.end >= last_argument_span.end {
            return None;
        }

        // resolve the comma that starts the removable tail
        let comma_start = find_first_byte_between(
            source,
            previous_argument_span.end as usize,
            first_redundant_argument_span.start as usize,
            b',',
        )?;

        return Some(Span::new(
            previous_argument_span.file,
            comma_start as u32,
            last_argument_span.end,
        ));
    }

    // remove the whole static argument list: `<...>`
    let first_argument_span = argument_spans[0];

    // scan left to find the opening `<`
    let mut left = first_argument_span.start as usize;
    while left > expression_span.start as usize {
        left -= 1;
        if source[left] == b'<' {
            break;
        }
    }
    if source[left] != b'<' {
        return None;
    }

    // scan right to find the closing `>`
    let mut right = last_argument_span.end as usize;
    while right < expression_span.end as usize && source[right] != b'>' {
        right += 1;
    }
    if right >= expression_span.end as usize || source[right] != b'>' {
        return None;
    }

    Some(Span::new(
        expression_span.file,
        left as u32,
        right.saturating_add(1) as u32,
    ))
}

/// Find the first matching byte between two byte offsets.
pub fn find_first_byte_between(source: &[u8], start: usize, end: usize, byte: u8) -> Option<usize> {
    // require one valid byte window
    if start >= end || end > source.len() {
        return None;
    }

    // return the first matching byte offset
    for (offset, current_byte) in source[start..end].iter().enumerate() {
        if *current_byte == byte {
            return Some(start + offset);
        }
    }

    None
}
/// Return one discarded call-like value and its replacement expression span owner.
pub fn expression_discarded_call_like_value(
    tree: &dir::Tree,
    statement_expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<(
    dir::LocalNodeId<dir::Expression>,
    dir::LocalNodeId<dir::Expression>,
)> {
    let mut expression_id = statement_expression_id;
    let mut replacement_expression_id = statement_expression_id;

    loop {
        let expression = tree.get(expression_id);

        if let dir::Expression::Parenthesized { expression } = expression {
            let value_id = expression_unwrap_parenthesized(tree, *expression);
            if matches!(
                tree.get(value_id),
                dir::Expression::Call { .. } | dir::Expression::New { .. }
            ) {
                return Some((value_id, expression_id));
            }
            expression_id = *expression;
            replacement_expression_id = expression_id;
            continue;
        }

        if let dir::Expression::Await { expression }
        | dir::Expression::AwaitMaybe { expression }
        | dir::Expression::AwaitMust { expression } = expression
        {
            let value_id = expression_unwrap_parenthesized(tree, *expression);
            if matches!(
                tree.get(value_id),
                dir::Expression::Call { .. } | dir::Expression::New { .. }
            ) {
                return Some((value_id, expression_id));
            }

            replacement_expression_id = expression_id;
            expression_id = *expression;
            continue;
        }

        if matches!(
            expression,
            dir::Expression::Call { .. } | dir::Expression::New { .. }
        ) {
            return Some((expression_id, replacement_expression_id));
        }

        let dir::Expression::Block(block_id) = expression else {
            return None;
        };
        let block = tree.get(*block_id);
        let tail_expression_id = block.tail_expression?;

        expression_id = tail_expression_id;
        replacement_expression_id = tail_expression_id;
    }
}

/// Return true when an expression is typed as `any` or references a declaration typed as `any`.
pub fn expression_is_any_typed(
    module_id: ModuleId,
    tree: &dir::Tree,
    symbols: &dir::BindingTable,
    types: &dir::TypeTable,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // unwrap parenthesized expressions
    let expression_id = expression_unwrap_parenthesized(tree, expression_id);

    // check inferred or declared expression type first
    if let Some(type_id) =
        expression_declared_or_inferred_type_id(module_id, tree, types, expression_id)
        && is_any_type(types, type_id)
    {
        return true;
    }

    // check declaration type for symbol backed references
    let Some(target_symbol) = types.symbol_resolution(expression_id.into_global_any(module_id))
    else {
        return false;
    };
    if target_symbol.module_id != module_id {
        return false;
    }

    let symbol_entry = symbols.get_symbol(target_symbol.local_id);
    let Some(declaration) = symbol_entry.declaration else {
        return false;
    };
    if declaration_marks_symbol_as_any(tree, symbols, declaration, target_symbol.local_id) {
        return true;
    }

    false
}

/// Resolve the declared or inferred type for an expression.
///
/// This prefers declared types so lints can inspect the original semantic type
/// in places where context can coerce the inferred type.
pub fn expression_declared_or_inferred_type_id(
    module_id: ModuleId,
    tree: &dir::Tree,
    types: &dir::TypeTable,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::LocalTypeId> {
    // unwrap parenthesized expressions
    let expression_id = expression_unwrap_parenthesized(tree, expression_id);

    // resolve expression type from type tables
    let global_expression_id = dir::GlobalNodeIdAny::new(module_id, expression_id.into_any());
    types
        .get_declared_type_id(global_expression_id)
        .or_else(|| types.get_inferred_type_id(global_expression_id))
}

/// Map one expression type from local inference or symbol value types.
pub fn expression_type_map<T>(
    artifacts: &ArtifactCache,
    profile_id: ProfileId,
    module_id: ModuleId,
    tree: &dir::Tree,
    types: &dir::TypeTable,
    expression_id: dir::LocalNodeId<dir::Expression>,
    map: impl FnOnce(&dir::TypeTable, dir::LocalTypeId) -> T,
) -> Option<T> {
    let expression_id = expression_unwrap_parenthesized(tree, expression_id);

    if let Some(type_id) =
        expression_declared_or_inferred_type_id(module_id, tree, types, expression_id)
    {
        return Some(map(types, type_id));
    }

    let symbol_id = types.symbol_resolution(expression_id.into_global_any(module_id))?;
    symbol_value_type_map_for(artifacts, profile_id, module_id, types, symbol_id, map)
}

/// Map one expression type from local inference, symbol value types, or call returns.
pub fn expression_type_or_call_return_type_map<T>(
    artifacts: &ArtifactCache,
    profile_id: ProfileId,
    module_id: ModuleId,
    tree: &dir::Tree,
    types: &dir::TypeTable,
    expression_id: dir::LocalNodeId<dir::Expression>,
    mut map: impl FnMut(&dir::TypeTable, dir::LocalTypeId) -> T,
) -> Option<T> {
    let expression_id = expression_unwrap_parenthesized(tree, expression_id);

    // resolve direct expression types first
    if let Some(type_id) =
        expression_declared_or_inferred_type_id(module_id, tree, types, expression_id)
    {
        return Some(map(types, type_id));
    }

    let expression = tree.get(expression_id);

    // resolve symbol backed value types
    if let Some(symbol_id) = types.symbol_resolution(expression_id.into_global_any(module_id))
        && let Some(mapped_value) = symbol_value_type_map_for(
            artifacts,
            profile_id,
            module_id,
            types,
            symbol_id,
            |types, type_id| map(types, type_id),
        )
    {
        return Some(mapped_value);
    }

    // resolve return types for call-like expressions
    let callee_id = match expression {
        dir::Expression::Call { left, .. } | dir::Expression::New { left, .. } => *left,
        _ => return None,
    };
    let return_type_id = expression_type_map(
        artifacts,
        profile_id,
        module_id,
        tree,
        types,
        callee_id,
        function_return_type,
    )??;

    Some(map(types, return_type_id))
}

/// Return true when an expression evaluates to a Promise like value.
pub fn expression_is_promise_like(
    module_id: ModuleId,
    tree: &dir::Tree,
    types: &dir::TypeTable,
    promise_symbol: dir::GlobalSymbolId,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // unwrap parenthesized expressions
    let expression_id = expression_unwrap_parenthesized(tree, expression_id);
    let expression = tree.get(expression_id);

    // prefer declared or inferred expression types
    if let Some(type_id) =
        expression_declared_or_inferred_type_id(module_id, tree, types, expression_id)
        && is_promise_type(types, type_id, Some(promise_symbol))
    {
        return true;
    }

    // fall back to async and Promise return checks for call-like expressions
    let callee_id = match expression {
        dir::Expression::Call { left, .. } => Some(*left),
        dir::Expression::New { left, .. } => Some(*left),
        _ => None,
    };
    let Some(callee_id) = callee_id else {
        return false;
    };

    let Some(callee_type_id) =
        expression_declared_or_inferred_type_id(module_id, tree, types, callee_id)
    else {
        return false;
    };

    if is_async_function_type(types, callee_type_id) {
        return true;
    }

    function_return_type(types, callee_type_id)
        .is_some_and(|return_type| is_promise_type(types, return_type, Some(promise_symbol)))
}

/// Return true when a declaration marks a symbol as `any`.
fn declaration_marks_symbol_as_any(
    tree: &dir::Tree,
    symbols: &dir::BindingTable,
    declaration_id: dir::GlobalNodeIdAny,
    symbol_id: dir::LocalSymbolId,
) -> bool {
    match declaration_id.local_id.ty {
        dir::NodeType::Declarator => {
            let declarator = tree.get(declaration_id.into_local_typed::<dir::Declarator>());
            declarator
                .ty
                .is_some_and(|type_id| type_expression_is_explicit_any(tree, type_id))
        }
        dir::NodeType::Pattern => {
            let mut current = Some(declaration_id.local_id.id);

            while let Some(node_id) = current {
                let Some(parent_id) = tree.get_parent(node_id) else {
                    return false;
                };

                // check the enclosing declarator annotation
                if parent_id.ty == dir::NodeType::Declarator {
                    let declarator = tree.get(parent_id.into_typed::<dir::Declarator>());
                    return declarator
                        .ty
                        .is_some_and(|type_id| type_expression_is_explicit_any(tree, type_id));
                }

                current = Some(parent_id.id);
            }

            false
        }
        dir::NodeType::Expression => {
            let expression = tree.get(declaration_id.into_local_typed::<dir::Expression>());
            match expression {
                dir::Expression::Let { declarators, .. }
                | dir::Expression::Using { declarators, .. } => declarators.iter().any(|id| {
                    let declarator = tree.get(*id);
                    symbols.symbol_for_declaration(
                        declarator.pattern.into_global_any(symbols.module_id),
                    ) == Some(symbol_id)
                        && declarator
                            .ty
                            .is_some_and(|type_id| type_expression_is_explicit_any(tree, type_id))
                }),
                _ => false,
            }
        }
        _ => false,
    }
}

/// Return true when a type expression is an explicit `any` literal.
fn type_expression_is_explicit_any(
    tree: &dir::Tree,
    type_expression_id: dir::LocalNodeId<dir::TypeExpression>,
) -> bool {
    let type_expression = tree.get(type_expression_id);

    // peel off parenthesized wrappers first
    let type_expression = match type_expression {
        dir::TypeExpression::Parenthesized { expression } => {
            return type_expression_is_explicit_any(tree, *expression);
        }
        type_expression => type_expression,
    };

    matches!(
        type_expression,
        dir::TypeExpression::Literal {
            value: dir::TypeLiteral::Any
        }
    )
}

/// Convert a constant value into an i64 when possible.
pub fn const_i64(value: &ConstValue) -> Option<i64> {
    // evaluate constant variants
    match value {
        ConstValue::Integer(value) => Some(*value),
        ConstValue::Bigint(value) => Some(*value),
        ConstValue::Float(value) => {
            if value.is_finite() && value.fract() == 0.0 {
                Some(*value as i64)
            } else {
                None
            }
        }
        ConstValue::Boolean(value) => Some(i64::from(*value)),
        ConstValue::Null | ConstValue::Undefined => None,
    }
}

/// Return the utf16 length of a string literal.
pub fn string_literal_utf16_length(strings: &StringPool, value: StringId) -> usize {
    // count utf16 code units
    let text = strings.get(value);
    text.encode_utf16().count()
}

/// Flip a comparison operator when the operands are swapped.
pub fn flip_binary_operator(operator: dir::BinaryOperator) -> Option<dir::BinaryOperator> {
    match operator {
        dir::BinaryOperator::GreaterThan => Some(dir::BinaryOperator::LessThan),
        dir::BinaryOperator::GreaterThanOrEqual => Some(dir::BinaryOperator::LessThanOrEqual),
        dir::BinaryOperator::LessThan => Some(dir::BinaryOperator::GreaterThan),
        dir::BinaryOperator::LessThanOrEqual => Some(dir::BinaryOperator::GreaterThanOrEqual),
        dir::BinaryOperator::Equal
        | dir::BinaryOperator::EqualStrict
        | dir::BinaryOperator::NotEqual
        | dir::BinaryOperator::NotEqualStrict => Some(operator),
        _ => None,
    }
}

/// Return true when one binary operator compares two operands.
pub fn is_binary_comparison_operator(operator: dir::BinaryOperator) -> bool {
    matches!(
        operator,
        dir::BinaryOperator::Equal
            | dir::BinaryOperator::NotEqual
            | dir::BinaryOperator::EqualStrict
            | dir::BinaryOperator::NotEqualStrict
            | dir::BinaryOperator::LessThan
            | dir::BinaryOperator::LessThanOrEqual
            | dir::BinaryOperator::GreaterThan
            | dir::BinaryOperator::GreaterThanOrEqual
    )
}

/// Return true when the expression is potentially user controlled.
///
/// Literals are considered safe; references, calls, member access, index access,
/// binary operations, and template expressions are considered potentially tainted.
pub fn expression_is_potentially_tainted(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // unwrap parentheses
    let expression_id = expression_unwrap_parenthesized(tree, expression_id);
    let expression = tree.get(expression_id);

    // literals are safe
    if matches!(expression, dir::Expression::ScalarLiteral(_)) {
        return false;
    }

    // these expression kinds can carry user controlled data
    matches!(
        expression,
        dir::Expression::QualifiedReference { .. }
            | dir::Expression::Member { left: _, name: _ }
            | dir::Expression::Call {
                position: _,
                left: _,
                generic_arguments: _,
                arguments: _,
            }
            | dir::Expression::Index {
                position: _,
                left: _,
                index: _,
            }
            | dir::Expression::Binary {
                left: _,
                operator: _,
                right: _,
            }
            | dir::Expression::TemplateExpression { value: _ }
    )
}

/// Return true when one generic argument contains a matching type-space reference segment.
fn generic_argument_contains_reference_segment(
    tree: &dir::Tree,
    generic_argument_id: dir::LocalNodeId<dir::GenericArgument>,
    target_segment: StringId,
) -> bool {
    let generic_argument = tree.get(generic_argument_id);

    match generic_argument {
        dir::GenericArgument::Type { value } => {
            type_expression_contains_reference_segment(tree, *value, target_segment)
        }
        dir::GenericArgument::Value { value } => {
            expression_contains_reference_segment(tree, *value, target_segment)
        }
        dir::GenericArgument::Error => false,
    }
}

/// Return true when one value expression contains a matching reference segment.
fn expression_contains_reference_segment(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
    target_segment: StringId,
) -> bool {
    let expression_id = expression_unwrap_parenthesized(tree, expression_id);
    let expression = tree.get(expression_id);

    match expression {
        dir::Expression::Type { value, .. } => {
            type_expression_contains_reference_segment(tree, *value, target_segment)
        }

        dir::Expression::QualifiedReference {
            path,
            generic_arguments,
        } => path_or_generic_arguments_contain_reference_segment(
            tree,
            path,
            generic_arguments,
            target_segment,
        ),

        dir::Expression::Member { left, name: _ }
        | dir::Expression::PrivateMember { left, name: _ } => {
            expression_contains_reference_segment(tree, *left, target_segment)
        }

        dir::Expression::As { expression, .. }
        | dir::Expression::Satisfies { expression, .. }
        | dir::Expression::Await { expression }
        | dir::Expression::AwaitMaybe { expression }
        | dir::Expression::AwaitMust { expression } => {
            expression_contains_reference_segment(tree, *expression, target_segment)
        }

        _ => false,
    }
}
