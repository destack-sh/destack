use destack_base::{StringId, StringPool};
use destack_dir as dir;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, Program};

use crate::ConstValue;

use super::{
    function_return_type, is_any_type, is_async_function_type, is_promise_type,
    symbol_value_type_map_for,
};

/// The base of a reference path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceBase {
    /// A symbol-backed reference.
    Symbol(dir::GlobalSymbolId),
    /// A `this` reference.
    This,
    /// A `super` reference.
    Super,
}

/// A reference path from a base symbol to member names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferencePath {
    /// The base of the path.
    pub base: ReferenceBase,
    /// The member names from the base expression.
    pub members: Vec<StringId>,
}

/// Resolve the target symbol for a reference expression.
pub fn expression_target_symbol(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::GlobalSymbolId> {
    // unwrap parenthesized expressions first
    let expression_id = expression_unwrap_parenthesized(tree, expression_id);

    // return the reference target symbol when present
    let expression = tree.get(expression_id);
    expression.target_symbol()
}

/// Resolve a reference path for member expressions.
pub fn expression_reference_path(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<ReferencePath> {
    // collect member names walking left
    let mut members = Vec::new();
    let base = expression_reference_path_base(tree, expression_id, &mut members)?;

    // normalize member order
    members.reverse();

    Some(ReferencePath { base, members })
}

/// Return true when the expression is a global qualified member access.
pub fn expression_is_global_qualified_member(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
    qualifiers: &[dir::GlobalSymbolId],
    member_name: StringId,
) -> bool {
    // resolve the member path
    let Some(path) = expression_reference_path(tree, expression_id) else {
        return false;
    };

    // ensure the requested member is present
    if path.members.as_slice() != [member_name] {
        return false;
    }

    // ensure the base is a known global qualifier
    match path.base {
        ReferenceBase::Symbol(symbol) => qualifiers.contains(&symbol),
        ReferenceBase::This => false,
        ReferenceBase::Super => false,
    }
}

/// Return the expression id with parenthesized nodes unwrapped.
pub fn expression_unwrap_parenthesized(
    tree: &dir::NodeTree,
    mut expression_id: dir::LocalNodeId<dir::Expression>,
) -> dir::LocalNodeId<dir::Expression> {
    loop {
        let dir::Expression::Parenthesized { expression } = tree.get(expression_id) else {
            return expression_id;
        };
        expression_id = *expression;
    }
}

/// Return true when an expression is a standalone statement value.
///
/// This accepts parenthesized wrappers around the expression before the
/// surrounding statement node.
pub fn expression_is_standalone_statement(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // start from the target expression
    let mut current_id = expression_id;

    // walk parent chain through parenthesized wrappers until a statement boundary
    loop {
        // require an expression parent
        let Some(parent) = tree.get_parent(current_id.id) else {
            return false;
        };
        if parent.ty != dir::NodeType::Expression {
            return false;
        }

        // inspect the parent expression shape
        let parent_id = parent.into_typed::<dir::Expression>();
        let parent_expression = tree.get(parent_id);

        match parent_expression {
            // keep walking through nested parentheses
            dir::Expression::Parenthesized { expression } if *expression == current_id => {
                current_id = parent_id;
            }

            // accept when the current expression is the statement payload
            dir::Expression::Statement { statement } => {
                return *statement == current_id;
            }

            // reject other parent expression contexts
            _ => return false,
        }
    }
}

/// Return one discarded call like value and its replacement expression span owner.
pub fn expression_discarded_call_like_value(
    tree: &dir::NodeTree,
    statement_expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<(
    dir::LocalNodeId<dir::Expression>,
    dir::LocalNodeId<dir::Expression>,
)> {
    let mut expression_id = statement_expression_id;
    let mut replacement_expression_id = statement_expression_id;

    loop {
        let expression = tree.get(expression_id);

        if let dir::Expression::Statement { statement } = expression {
            expression_id = *statement;
            continue;
        }

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

        if matches!(
            expression,
            dir::Expression::Call { .. } | dir::Expression::New { .. }
        ) {
            return Some((expression_id, replacement_expression_id));
        }

        let dir::Expression::Block { block } = expression else {
            return None;
        };
        let block = tree.get(*block);
        let tail_expression_id = *block.expressions.last()?;

        if matches!(
            tree.get(tail_expression_id),
            dir::Expression::Statement { .. }
        ) {
            return None;
        }

        expression_id = tail_expression_id;
        replacement_expression_id = tail_expression_id;
    }
}

/// Return true when an expression is typed as `any` or references a declaration typed as `any`.
pub fn expression_is_any_typed(
    module_id: ModuleId,
    tree: &dir::NodeTree,
    symbols: &dir::SymbolTable,
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

    // check declaration type for symbol-backed references
    let expression = tree.get(expression_id);
    let Some(target_symbol) = expression.target_symbol() else {
        return false;
    };
    if target_symbol.module_id != module_id {
        return false;
    }

    let symbol_entry = symbols.get_symbol(target_symbol.local_id);
    let Some(primary_declaration) = symbol_entry.primary_declaration else {
        return false;
    };
    if declaration_marks_symbol_as_any(tree, primary_declaration, target_symbol.local_id) {
        return true;
    }

    if let Some(secondary_declarations) = &symbol_entry.secondary_declarations {
        for declaration in secondary_declarations.iter().copied() {
            if declaration_marks_symbol_as_any(tree, declaration, target_symbol.local_id) {
                return true;
            }
        }
    }

    false
}

/// Resolve the declared or inferred type for an expression.
///
/// This prefers declared types so lints can inspect the original semantic type
/// in places where context can coerce the inferred type.
pub fn expression_declared_or_inferred_type_id(
    module_id: ModuleId,
    tree: &dir::NodeTree,
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
    program: &Program,
    profile_id: ProfileId,
    module_id: ModuleId,
    tree: &dir::NodeTree,
    symbols: &dir::SymbolTable,
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

    let expression = tree.get(expression_id);
    let symbol_id = expression.target_symbol()?;
    symbol_value_type_map_for(
        program, profile_id, module_id, symbols, types, symbol_id, map,
    )
}

/// Map one expression type from local inference, symbol value types, or call returns.
pub fn expression_type_or_call_return_type_map<T>(
    program: &Program,
    profile_id: ProfileId,
    module_id: ModuleId,
    tree: &dir::NodeTree,
    symbols: &dir::SymbolTable,
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
    if let Some(symbol_id) = expression.target_symbol()
        && let Some(mapped_value) = symbol_value_type_map_for(
            program,
            profile_id,
            module_id,
            symbols,
            types,
            symbol_id,
            |types, type_id| map(types, type_id),
        )
    {
        return Some(mapped_value);
    }

    // resolve return types for call like expressions
    let callee_id = match expression {
        dir::Expression::Call { left, .. } | dir::Expression::New { left, .. } => *left,
        _ => return None,
    };
    let return_type_id = expression_type_map(
        program,
        profile_id,
        module_id,
        tree,
        symbols,
        types,
        callee_id,
        function_return_type,
    )??;

    Some(map(types, return_type_id))
}

/// Return true when an expression evaluates to a Promise like value.
pub fn expression_is_promise_like(
    module_id: ModuleId,
    tree: &dir::NodeTree,
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

    // fall back to async and Promise return checks for call like expressions
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
    tree: &dir::NodeTree,
    declaration_id: dir::GlobalNodeIdAny,
    symbol_id: dir::LocalSymbolId,
) -> bool {
    match declaration_id.local_id.ty {
        dir::NodeType::Declarator => {
            let declarator = tree.get(declaration_id.into_local_typed::<dir::Declarator>());
            declarator
                .ty
                .is_some_and(|type_id| expression_is_explicit_any(tree, type_id))
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
                        .is_some_and(|type_id| expression_is_explicit_any(tree, type_id));
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
                    let pattern = tree.get(declarator.pattern);
                    pattern.symbol() == Some(symbol_id)
                        && declarator
                            .ty
                            .is_some_and(|type_id| expression_is_explicit_any(tree, type_id))
                }),
                _ => false,
            }
        }
        _ => false,
    }
}

/// Return true when a type expression is an explicit `any` literal.
fn expression_is_explicit_any(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expression_id = expression_unwrap_parenthesized(tree, expression_id);
    let expression = tree.get(expression_id);
    matches!(
        expression,
        dir::Expression::TypeLiteral {
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
    text.as_ref().encode_utf16().count()
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

/// Return true when the expression is potentially user-controlled.
///
/// Literals are considered safe; references, calls, member access, index access,
/// binary operations, and template expressions are considered potentially tainted.
pub fn expression_is_potentially_tainted(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // unwrap parentheses
    let expression_id = expression_unwrap_parenthesized(tree, expression_id);
    let expression = tree.get(expression_id);

    // literals are safe
    if matches!(expression, dir::Expression::ScalarLiteral { .. }) {
        return false;
    }

    // these expression kinds can carry user-controlled data
    matches!(
        expression,
        dir::Expression::LocalReference { .. }
            | dir::Expression::ModuleReference { .. }
            | dir::Expression::Member { .. }
            | dir::Expression::Call { .. }
            | dir::Expression::Index { .. }
            | dir::Expression::Binary { .. }
            | dir::Expression::TemplateExpression { .. }
    )
}

/// Get the base of a reference path.
fn expression_reference_path_base(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
    members: &mut Vec<StringId>,
) -> Option<ReferenceBase> {
    // inspect the expression node
    let expression = tree.get(expression_id);

    // match the base or member steps
    match expression {
        dir::Expression::Parenthesized { expression } => {
            expression_reference_path_base(tree, *expression, members)
        }
        dir::Expression::Member { left, name, .. } => {
            members.push(*name);
            expression_reference_path_base(tree, *left, members)
        }
        dir::Expression::This => Some(ReferenceBase::This),
        dir::Expression::Super => Some(ReferenceBase::Super),
        _ => expression.target_symbol().map(ReferenceBase::Symbol),
    }
}

/// Info about a method call expression (receiver.method(...)).
#[derive(Debug, Clone, Copy)]
pub struct MethodCallInfo {
    /// The call expression id.
    pub call_id: dir::LocalNodeId<dir::Expression>,
    /// The receiver expression id (the object the method is called on).
    pub receiver_id: dir::LocalNodeId<dir::Expression>,
    /// The method name.
    pub method_name: StringId,
}

/// Match a method call expression and extract its parts.
pub fn expression_method_call(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<MethodCallInfo> {
    // match call expression
    let expression = tree.get(expression_id);
    let dir::Expression::Call { left, .. } = expression else {
        return None;
    };

    // match member access for the callee
    let callee = tree.get(*left);
    let dir::Expression::Member { left, name, .. } = callee else {
        return None;
    };

    Some(MethodCallInfo {
        call_id: expression_id,
        receiver_id: *left,
        method_name: *name,
    })
}
