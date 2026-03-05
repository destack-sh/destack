use destack_base::{StringId, StringPool};
use destack_dir as dir;
use destack_source::{ModuleId, Span};
use destack_workspace::{ProfileId, Program};

use crate::{ConstValue, LintModuleDirContext};

use super::{
    function_return_type, is_any_type, is_async_function_type, is_promise_type,
    symbol_value_type_map_for,
};

/// The base of a reference path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceBase {
    /// A symbol backed reference.
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
    // unwrap transparent wrappers first
    let expression_id = expression_unwrap_transparent(tree, expression_id);

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

/// Return true when two expressions have equivalent syntax ignoring parentheses and spacing.
pub fn expressions_have_equivalent_syntax(
    ctx: &LintModuleDirContext<'_>,
    left_id: dir::LocalNodeId<dir::Expression>,
    right_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // normalize transparent wrappers before comparisons
    let left_id = expression_unwrap_transparent(ctx.tree, left_id);
    let right_id = expression_unwrap_transparent(ctx.tree, right_id);

    // compare canonicalized reference paths first
    let left_path = expression_reference_path(ctx.tree, left_id);
    let right_path = expression_reference_path(ctx.tree, right_id);
    if left_path.is_some() || right_path.is_some() {
        return left_path == right_path;
    }

    // compare syntax text with spacing removed
    let left_text = ctx.get_span_text(ctx.get_span(left_id));
    let right_text = ctx.get_span_text(ctx.get_span(right_id));
    normalize_expression_syntax(left_text.as_ref())
        == normalize_expression_syntax(right_text.as_ref())
}

/// Normalize expression syntax for token style equality checks.
fn normalize_expression_syntax(source: &str) -> String {
    source
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

/// Return true when the expression is a global qualified member access.
pub fn expression_is_global_qualified_member(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
    qualifiers: &[dir::GlobalSymbolId],
    member_name: StringId,
) -> bool {
    // normalize transparent wrappers first
    let expression_id = expression_unwrap_transparent(tree, expression_id);

    // resolve the member path
    if let Some(path) = expression_reference_path(tree, expression_id) {
        // ensure the requested member is present
        if path.members.as_slice() != [member_name] {
            return false;
        }

        // ensure the base is a known global qualifier
        return match path.base {
            ReferenceBase::Symbol(symbol) => qualifiers.contains(&symbol),
            ReferenceBase::This => false,
            ReferenceBase::Super => false,
        };
    }

    // support computed static string access like `window["alert"]`
    let Some((base_id, property_name)) = expression_static_property_access(tree, expression_id)
    else {
        return false;
    };
    if property_name != member_name {
        return false;
    }

    let Some(base_symbol) = expression_target_symbol(tree, base_id) else {
        return false;
    };

    qualifiers.contains(&base_symbol)
}

/// Return true when one expression resolves to a symbol or its global-qualified member form.
pub fn expression_is_symbol_or_global_qualified_member(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
    symbol_id: dir::GlobalSymbolId,
    qualifiers: &[dir::GlobalSymbolId],
    member_name: StringId,
) -> bool {
    // match direct symbol references
    let target_symbol = expression_target_symbol(tree, expression_id);
    if target_symbol == Some(symbol_id) {
        return true;
    }

    // match global qualified references
    expression_is_global_qualified_member(tree, expression_id, qualifiers, member_name)
}

/// Return one static string literal value from an expression.
pub fn expression_static_string_literal(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<StringId> {
    // normalize transparent wrappers first
    let expression_id = expression_unwrap_transparent(tree, expression_id);
    let expression = tree.get(expression_id);

    // match direct string literals
    if let dir::Expression::ScalarLiteral {
        value: dir::ScalarLiteral::String(value),
    } = expression
    {
        return Some(*value);
    }

    // match template literals without interpolations
    let dir::Expression::TemplateExpression { value } = expression else {
        return None;
    };
    let dir::TemplateLiteral::String { string } = value else {
        return None;
    };

    Some(*string)
}

/// Return one static regex literal pair as `(pattern, flags)`.
pub fn expression_regex_literal(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<(StringId, Option<StringId>)> {
    // normalize transparent wrappers first
    let expression_id = expression_unwrap_transparent(tree, expression_id);
    let expression = tree.get(expression_id);

    // match direct regex literals
    let dir::Expression::ScalarLiteral {
        value: dir::ScalarLiteral::RegexString { content, flags },
    } = expression
    else {
        return None;
    };

    Some((*content, *flags))
}

/// Return one static property access pair as `(left, property_name)`.
pub fn expression_static_property_access(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<(dir::LocalNodeId<dir::Expression>, StringId)> {
    // normalize transparent wrappers first
    let expression_id = expression_unwrap_transparent(tree, expression_id);
    let expression = tree.get(expression_id);

    // match dot member access
    if let dir::Expression::Member { left, name, .. } = expression {
        return Some((*left, *name));
    }

    // match bracket member access with static string keys
    let dir::Expression::Index { left, right } = expression else {
        return None;
    };
    let index_id = right.as_ref().copied()?;
    let property_name = expression_static_string_literal(tree, index_id)?;

    Some((*left, property_name))
}

/// Return one assignment target expression for assignment-like expressions.
pub fn expression_assignment_target(
    expression: &dir::Expression,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    match expression {
        dir::Expression::Assign { left, .. } | dir::Expression::AssignBinary { left, .. } => {
            Some(*left)
        }
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
        dir::Expression::Import { target, .. }
        | dir::Expression::ReExport { target, .. }
        | dir::Expression::UnresolvedReExport { target, .. } => Some(*target),
        dir::Expression::UnresolvedImport {
            target: dir::ImportTarget::String(target),
            ..
        } => Some(*target),
        dir::Expression::UnresolvedImport {
            target: dir::ImportTarget::Expression { .. },
            ..
        } => None,
        _ => None,
    }
}

/// Return one static import target specifier for import-like expressions.
pub fn expression_import_target_specifier(
    tree: &dir::NodeTree,
    expression: &dir::Expression,
) -> Option<StringId> {
    // match direct static module targets
    if let Some(target_id) = expression_import_target_static_specifier(expression) {
        return Some(target_id);
    }

    // match unresolved expression targets that evaluate to static strings
    let dir::Expression::UnresolvedImport { target, .. } = expression else {
        return None;
    };
    let dir::ImportTarget::Expression { target } = target else {
        return None;
    };

    expression_static_string_literal(tree, *target)
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

/// Resolve the value expression for one DIR argument node.
pub fn argument_expression_id(
    tree: &dir::NodeTree,
    argument_id: dir::LocalNodeId<dir::Argument>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    let argument = tree.get(argument_id);

    match argument {
        dir::Argument::Named { value, .. }
        | dir::Argument::Labeled { value, .. }
        | dir::Argument::Positional { value, .. }
        | dir::Argument::Spread { value, .. } => Some(*value),
    }
}

/// Return true when one type expression contains a reference ending in the target segment.
pub fn type_expression_contains_reference_segment(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
    target_segment: StringId,
) -> bool {
    let expression = tree.get(expression_id);
    match expression {
        dir::Expression::LocalReference {
            path,
            static_arguments,
            ..
        }
        | dir::Expression::ModuleReference {
            path,
            static_arguments,
            ..
        }
        | dir::Expression::GlobalReference {
            path,
            static_arguments,
            ..
        } => {
            if path.last_segment() == Some(target_segment) {
                return true;
            }

            static_arguments.as_ref().is_some_and(|static_arguments| {
                static_arguments.iter().any(|argument_id| {
                    argument_expression_id(tree, *argument_id).is_some_and(
                        |argument_expression_id| {
                            type_expression_contains_reference_segment(
                                tree,
                                argument_expression_id,
                                target_segment,
                            )
                        },
                    )
                })
            })
        }
        dir::Expression::TypeImport {
            target,
            arguments,
            static_arguments,
            ..
        } => {
            type_expression_contains_reference_segment(tree, *target, target_segment)
                || arguments.iter().any(|argument_id| {
                    argument_expression_id(tree, *argument_id).is_some_and(
                        |argument_expression_id| {
                            type_expression_contains_reference_segment(
                                tree,
                                argument_expression_id,
                                target_segment,
                            )
                        },
                    )
                })
                || static_arguments.as_ref().is_some_and(|static_arguments| {
                    static_arguments.iter().any(|argument_id| {
                        argument_expression_id(tree, *argument_id).is_some_and(
                            |argument_expression_id| {
                                type_expression_contains_reference_segment(
                                    tree,
                                    argument_expression_id,
                                    target_segment,
                                )
                            },
                        )
                    })
                })
        }
        dir::Expression::Parenthesized { expression } => {
            type_expression_contains_reference_segment(tree, *expression, target_segment)
        }
        dir::Expression::TypeUnary { right, .. } => {
            type_expression_contains_reference_segment(tree, *right, target_segment)
        }
        dir::Expression::TypeIndex { left, index } => {
            type_expression_contains_reference_segment(tree, *left, target_segment)
                || type_expression_contains_reference_segment(tree, *index, target_segment)
        }
        dir::Expression::TypeConditional {
            left,
            right,
            then_type,
            else_type,
        } => {
            type_expression_contains_reference_segment(tree, *left, target_segment)
                || type_expression_contains_reference_segment(tree, *right, target_segment)
                || type_expression_contains_reference_segment(tree, *then_type, target_segment)
                || type_expression_contains_reference_segment(tree, *else_type, target_segment)
        }
        dir::Expression::TypeMapped {
            parameter, value, ..
        } => {
            type_expression_contains_reference_segment(tree, parameter.constraint, target_segment)
                || parameter.key_remap.is_some_and(|key_remap| {
                    type_expression_contains_reference_segment(tree, key_remap, target_segment)
                })
                || type_expression_contains_reference_segment(tree, *value, target_segment)
        }
        dir::Expression::TypeTemplateLiteral { spans, .. } => spans.iter().any(|span_id| {
            type_expression_contains_reference_segment(tree, *span_id, target_segment)
        }),
        _ => false,
    }
}

/// Return true when one function signature return type contains the target segment.
pub fn function_signature_return_type_contains_reference_segment(
    tree: &dir::NodeTree,
    signature: &dir::FunctionSignature,
    target_segment: StringId,
) -> bool {
    let Some(return_type_id) = signature.return_type else {
        return false;
    };

    type_expression_contains_reference_segment(tree, return_type_id, target_segment)
}

/// Return true when one declaration creates a nested executable scope.
pub fn declaration_has_nested_executable_scope(declaration: &dir::Declaration) -> bool {
    matches!(
        declaration,
        dir::Declaration::Global { .. }
            | dir::Declaration::Namespace { .. }
            | dir::Declaration::Struct { .. }
            | dir::Declaration::Class { .. }
            | dir::Declaration::Enum { .. }
            | dir::Declaration::Interface { .. }
            | dir::Declaration::Function { .. }
            | dir::Declaration::Extension { .. }
    )
}

/// Return true when one expression enters a nested declaration scope.
pub fn expression_enters_nested_declaration_scope(
    tree: &dir::NodeTree,
    expression: &dir::Expression,
) -> bool {
    let dir::Expression::Declaration { declaration } = expression else {
        return false;
    };

    let declaration = tree.get(*declaration);
    declaration_has_nested_executable_scope(declaration)
}

/// Return the expression id with transparent wrappers unwrapped.
pub fn expression_unwrap_transparent(
    tree: &dir::NodeTree,
    mut expression_id: dir::LocalNodeId<dir::Expression>,
) -> dir::LocalNodeId<dir::Expression> {
    loop {
        let expression = tree.get(expression_id);
        match expression {
            dir::Expression::Parenthesized { expression } => {
                expression_id = *expression;
            }
            dir::Expression::Maybe { left }
            | dir::Expression::Must { left }
            | dir::Expression::Instantiation { left, .. } => {
                expression_id = *left;
            }
            dir::Expression::Cast { value, .. } | dir::Expression::OwnershipCast { value, .. } => {
                expression_id = *value;
            }
            dir::Expression::ValueOf { right, .. }
            | dir::Expression::ReferenceOf { right, .. }
            | dir::Expression::PointerOf { right, .. } => {
                expression_id = *right;
            }
            _ => {
                return expression_id;
            }
        }
    }
}

/// Return one parent expression id when the parent node is an expression.
pub fn expression_parent_id(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    // resolve one parent node
    let parent = tree.get_parent(expression_id.id)?;
    if parent.ty != dir::NodeType::Expression {
        return None;
    }

    // return one typed parent expression id
    Some(parent.into_typed::<dir::Expression>())
}

/// Return true when one expression belongs to an async callable boundary.
pub fn expression_is_inside_async_callable(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let mut current_parent_id = tree.get_parent(expression_id.id);

    // walk ancestors until one callable boundary is reached
    while let Some(parent_id) = current_parent_id {
        let Some(asynchrony) = callable_boundary_asynchrony(tree, parent_id) else {
            current_parent_id = tree.get_parent(parent_id.id);
            continue;
        };

        return asynchrony == dir::Asynchrony::Async;
    }

    false
}

/// Return true when one expression is in an error handling context.
pub fn expression_affects_error_handling_context(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let mut current_child_id = expression_id.into_any();
    let mut current_parent_id = tree.get_parent(current_child_id.id);

    // walk ancestors and keep try context semantics scoped per callable
    while let Some(parent_id) = current_parent_id {
        // stop at callable boundaries and keep context local
        if callable_boundary_asynchrony(tree, parent_id).is_some() {
            return false;
        }

        // inspect direct try ancestry for this child path
        if parent_id.ty == dir::NodeType::Expression {
            let parent_expression_id = parent_id.into_typed::<dir::Expression>();
            let parent_expression = tree.get(parent_expression_id);
            if let dir::Expression::Try {
                try_expression,
                catch_expression,
                finally_expression,
                ..
            } = parent_expression
            {
                let try_context = try_context_from_direct_child(
                    *try_expression,
                    *catch_expression,
                    *finally_expression,
                    current_child_id,
                );
                match try_context {
                    Some(TryContext::Try) => {
                        return true;
                    }
                    Some(TryContext::Catch) => {
                        if finally_expression.is_some() {
                            return true;
                        }

                        current_child_id = parent_expression_id.into_any();
                        current_parent_id = tree.get_parent(current_child_id.id);
                        continue;
                    }
                    Some(TryContext::Finally) => {
                        current_child_id = parent_expression_id.into_any();
                        current_parent_id = tree.get_parent(current_child_id.id);
                        continue;
                    }
                    None => {}
                }
            }
        }

        current_child_id = parent_id;
        current_parent_id = tree.get_parent(current_child_id.id);
    }

    false
}

/// Return true when one expression is in a resource-management-sensitive context.
pub fn expression_affects_resource_management_context(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let mut current_child_id = expression_id.into_any();
    let mut current_parent_id = tree.get_parent(current_child_id.id);

    // walk ancestors and keep resource context semantics scoped per callable
    while let Some(parent_id) = current_parent_id {
        // stop at callable boundaries and keep context local
        if callable_boundary_asynchrony(tree, parent_id).is_some() {
            return false;
        }

        // inspect enclosing blocks for earlier using declarations
        if parent_id.ty == dir::NodeType::Block && current_child_id.ty == dir::NodeType::Expression
        {
            let block_id = parent_id.into_typed::<dir::Block>();
            let block = tree.get(block_id);
            let child_expression_id = current_child_id.into_typed::<dir::Expression>();
            let child_index = block
                .expressions
                .iter()
                .position(|expression_id| *expression_id == child_expression_id);

            // report when a prior using declaration exists in this block scope
            if let Some(child_index) = child_index {
                let has_prior_using_declaration = block.expressions[..child_index]
                    .iter()
                    .copied()
                    .any(|statement_expression_id| {
                        expression_is_using_declaration(tree, statement_expression_id)
                    });
                if has_prior_using_declaration {
                    return true;
                }
            }
        }

        current_child_id = parent_id;
        current_parent_id = tree.get_parent(current_child_id.id);
    }

    false
}

/// One direct child try context classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TryContext {
    /// The child belongs to the try branch.
    Try,
    /// The child belongs to the catch branch.
    Catch,
    /// The child belongs to the finally branch.
    Finally,
}

/// Return the async marker for one callable boundary node.
fn callable_boundary_asynchrony(
    tree: &dir::NodeTree,
    node_id: dir::LocalNodeIdAny,
) -> Option<dir::Asynchrony> {
    match node_id.ty {
        dir::NodeType::Declaration => {
            let declaration = tree.get(node_id.into_typed::<dir::Declaration>());
            let dir::Declaration::Function { signature, .. } = declaration else {
                return None;
            };
            Some(signature.asynchrony)
        }
        dir::NodeType::Member => {
            let member = tree.get(node_id.into_typed::<dir::Member>());
            let dir::Member::Method { signature, .. } = member else {
                return None;
            };
            Some(signature.asynchrony)
        }
        dir::NodeType::Property => {
            let property = tree.get(node_id.into_typed::<dir::Property>());
            let dir::Property::Method { signature, .. } = property else {
                return None;
            };
            Some(signature.asynchrony)
        }
        _ => None,
    }
}

/// Return one direct try branch for a child node.
fn try_context_from_direct_child(
    try_expression_id: dir::LocalNodeId<dir::Expression>,
    catch_expression_id: Option<dir::LocalNodeId<dir::Expression>>,
    finally_expression_id: Option<dir::LocalNodeId<dir::Expression>>,
    child_id: dir::LocalNodeIdAny,
) -> Option<TryContext> {
    // classify the direct try branch from expression ids
    if child_id == try_expression_id.into_any() {
        return Some(TryContext::Try);
    }
    if catch_expression_id.is_some_and(|id| child_id == id.into_any()) {
        return Some(TryContext::Catch);
    }
    if finally_expression_id.is_some_and(|id| child_id == id.into_any()) {
        return Some(TryContext::Finally);
    }

    None
}

/// Return true when one statement expression is a using declaration.
fn expression_is_using_declaration(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expression = tree.get(expression_id);

    // match direct using declarations
    if matches!(expression, dir::Expression::Using { .. }) {
        return true;
    }

    // recurse through transparent statement wrappers
    if let dir::Expression::Statement { statement } = expression {
        return expression_is_using_declaration(tree, *statement);
    }
    if let dir::Expression::Parenthesized { expression } = expression {
        return expression_is_using_declaration(tree, *expression);
    }

    false
}

/// Return the outermost transparent wrapper that still contains this expression.
pub fn expression_outer_transparent_ancestor(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> dir::LocalNodeId<dir::Expression> {
    // walk through transparent parent wrappers
    let mut current_expression_id = expression_id;
    loop {
        let Some(parent_expression_id) = expression_parent_id(tree, current_expression_id) else {
            return current_expression_id;
        };

        let parent_expression = tree.get(parent_expression_id);
        if !expression_is_transparent_parent_of(parent_expression, current_expression_id) {
            return current_expression_id;
        }

        current_expression_id = parent_expression_id;
    }
}

/// Return true when the parent expression transparently wraps the child expression.
fn expression_is_transparent_parent_of(
    parent_expression: &dir::Expression,
    child_expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    matches!(
        parent_expression,
        dir::Expression::Parenthesized { expression } if *expression == child_expression_id
    ) || matches!(
        parent_expression,
        dir::Expression::Maybe { left } if *left == child_expression_id
    ) || matches!(
        parent_expression,
        dir::Expression::Must { left } if *left == child_expression_id
    ) || matches!(
        parent_expression,
        dir::Expression::Instantiation { left, .. } if *left == child_expression_id
    ) || matches!(
        parent_expression,
        dir::Expression::Cast { value, .. } if *value == child_expression_id
    ) || matches!(
        parent_expression,
        dir::Expression::OwnershipCast { value, .. } if *value == child_expression_id
    ) || matches!(
        parent_expression,
        dir::Expression::ValueOf { right, .. } if *right == child_expression_id
    ) || matches!(
        parent_expression,
        dir::Expression::ReferenceOf { right, .. } if *right == child_expression_id
    ) || matches!(
        parent_expression,
        dir::Expression::PointerOf { right, .. } if *right == child_expression_id
    )
}

/// Return the surrounding statement expression for a standalone expression.
pub fn statement_expression_ancestor(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    // normalize transparent wrappers before checking statement ownership
    let outer_expression_id = expression_outer_transparent_ancestor(tree, expression_id);

    // resolve one parent expression
    let parent_expression_id = expression_parent_id(tree, outer_expression_id)?;
    let parent_expression = tree.get(parent_expression_id);

    // return one statement wrapper parent
    if let dir::Expression::Statement { statement } = parent_expression
        && *statement == outer_expression_id
    {
        return Some(parent_expression_id);
    }

    None
}

/// Return the enclosing statement span for a standalone expression.
pub fn statement_expression_span(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<Span> {
    // resolve the enclosing statement wrapper
    let statement_expression_id = statement_expression_ancestor(ctx.tree, expression_id)?;

    // return the statement span
    Some(ctx.get_span(statement_expression_id))
}

/// Return true when an expression is a standalone statement value.
///
/// This accepts transparent wrappers around the expression before the
/// surrounding statement node.
pub fn expression_is_standalone_statement(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    statement_expression_ancestor(tree, expression_id).is_some()
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

/// Return receiver text for one member expression.
pub fn member_receiver_text(
    ctx: &LintModuleDirContext<'_>,
    left_expression_id: dir::LocalNodeId<dir::Expression>,
    member_text: &str,
    member_name: StringId,
    is_private: bool,
) -> Option<String> {
    let member_name = ctx.program.strings.get(member_name);
    let suffix = if is_private {
        format!(".#{}", member_name.as_ref())
    } else {
        format!(".{}", member_name.as_ref())
    };

    // prefer parsing from full member text for best source fidelity
    if let Some(receiver) = member_text.strip_suffix(&suffix) {
        let receiver = receiver.trim();
        if !receiver.is_empty() {
            return Some(receiver.to_string());
        }
    }

    // fall back to left expression span text
    let left_span = ctx.get_span(left_expression_id);
    let left_text = ctx.get_span_text(left_span).trim().to_string();
    if left_text.is_empty() {
        return None;
    }

    Some(left_text)
}

/// Return true when this expression is used as receiver helper target.
pub fn parent_is_receiver_helper(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
    bind_name: StringId,
    call_name: StringId,
    apply_name: StringId,
) -> bool {
    let Some(parent) = tree.get_parent(expression_id.id) else {
        return false;
    };
    if parent.ty != dir::NodeType::Expression {
        return false;
    }

    // inspect the immediate parent expression for helper member access
    let parent_id = parent.into_typed::<dir::Expression>();
    let parent_expression = tree.get(parent_id);
    matches!(
        parent_expression,
        dir::Expression::Member { left, name, .. }
            | dir::Expression::PrivateMember { left, name, .. }
            if *left == expression_id
                && (*name == bind_name || *name == call_name || *name == apply_name)
    )
}

/// Return true when a call-like invocation safely binds method receivers.
pub fn call_like_invocation_is_receiver_bound(
    tree: &dir::NodeTree,
    call_like_id: dir::LocalNodeId<dir::Expression>,
    bind_name: StringId,
    call_name: StringId,
    apply_name: StringId,
) -> bool {
    let expression = tree.get(call_like_id);
    let (callee_id, dynamic_arguments) = match expression {
        dir::Expression::Call {
            left,
            dynamic_arguments,
            ..
        }
        | dir::Expression::New {
            left,
            dynamic_arguments,
            ..
        } => (*left, dynamic_arguments.as_slice()),
        _ => return false,
    };

    // inspect callee helper usage for bind, call, and apply
    let callee = tree.get(callee_id);
    let uses_receiver_helper = matches!(
        callee,
        dir::Expression::Member { name, .. } | dir::Expression::PrivateMember { name, .. }
            if *name == bind_name || *name == call_name || *name == apply_name
    );

    // non helper callees are safe by construction
    if !uses_receiver_helper {
        return true;
    }

    !dynamic_arguments.is_empty()
}

/// Return one discarded call-like value and its replacement expression span owner.
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

        if let dir::Expression::Await { expression } | dir::Expression::AwaitMaybe { expression } =
            expression
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

    // check declaration type for symbol backed references
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

    // resolve return types for call-like expressions
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

    // these expression kinds can carry user controlled data
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

/// Info about one call-like expression (`call(...)` or `new call(...)`).
#[derive(Debug, Clone, Copy)]
pub struct CallLikeExpressionInfo<'a> {
    /// The call target expression.
    pub left: dir::LocalNodeId<dir::Expression>,
    /// Optional static arguments.
    pub static_arguments: Option<&'a [dir::LocalNodeId<dir::Argument>]>,
    /// Dynamic arguments.
    pub dynamic_arguments: &'a [dir::LocalNodeId<dir::Argument>],
    /// Whether this expression is `new`.
    pub is_new: bool,
}

/// Match one call-like expression and extract call target and arguments.
pub fn expression_call_like(expression: &dir::Expression) -> Option<CallLikeExpressionInfo<'_>> {
    match expression {
        dir::Expression::Call {
            left,
            static_arguments,
            dynamic_arguments,
        } => Some(CallLikeExpressionInfo {
            left: *left,
            static_arguments: static_arguments.as_deref(),
            dynamic_arguments,
            is_new: false,
        }),
        dir::Expression::New {
            left,
            static_arguments,
            dynamic_arguments,
        } => Some(CallLikeExpressionInfo {
            left: *left,
            static_arguments: static_arguments.as_deref(),
            dynamic_arguments,
            is_new: true,
        }),
        _ => None,
    }
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
