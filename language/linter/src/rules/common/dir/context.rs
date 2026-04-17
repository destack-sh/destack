use destack_dir as dir;
use destack_source::Span;

use crate::LintModuleDirContext;

use super::expression_unwrap_statement;

/// Return true when one declaration creates a nested executable scope.
pub fn declaration_has_nested_executable_scope(declaration: &dir::Declaration) -> bool {
    matches!(
        declaration,
        dir::Declaration::Global(_)
            | dir::Declaration::Namespace(_)
            | dir::Declaration::Struct(_)
            | dir::Declaration::Class(_)
            | dir::Declaration::Enum(_)
            | dir::Declaration::Interface(_)
            | dir::Declaration::Function(_)
            | dir::Declaration::Extension(_)
    )
}

/// Return true when one expression enters a nested declaration scope.
pub fn expression_enters_nested_declaration_scope(
    tree: &dir::NodeTree,
    expression: &dir::Expression,
) -> bool {
    let dir::Expression::Declaration(declaration) = expression else {
        return false;
    };

    let declaration = tree.get(*declaration);
    declaration_has_nested_executable_scope(declaration)
}

/// Return true when one expression belongs to a known type position.
pub fn expression_is_in_type_position(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let mut current_node_id = expression_id.id;

    loop {
        let Some(parent_node_id) = tree.get_parent(current_node_id) else {
            return false;
        };

        // any ancestor type node means the value expression lives in type space
        if parent_node_id.ty == dir::NodeType::TypeExpression {
            return true;
        }

        current_node_id = parent_node_id.id;
    }
}

/// Return the single returned value expression from one block body.
pub fn block_single_return_value(
    tree: &dir::NodeTree,
    block_id: dir::LocalNodeId<dir::Block>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    let block = tree.get(block_id);
    if block.len() != 1 {
        return None;
    }

    let expression_id = expression_unwrap_statement(tree, block.first_expression()?);
    let expression = tree.get(expression_id);
    let dir::Expression::Return {
        value: Some(value_id),
    } = expression
    else {
        return None;
    };

    Some(*value_id)
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
            let expression_ids = block.iter_expressions().collect::<Vec<_>>();
            let child_index = expression_ids
                .iter()
                .position(|expression_id| *expression_id == child_expression_id);

            // report when a prior using declaration exists in this block scope
            if let Some(child_index) = child_index {
                let has_prior_using_declaration = expression_ids[..child_index]
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
            let dir::Declaration::Function(declaration) = declaration else {
                return None;
            };

            Some(declaration.signature.asynchrony)
        }
        dir::NodeType::Member => {
            let member = tree.get(node_id.into_typed::<dir::Member>());
            let dir::Member::Method {
                key: _,
                signature,
                body: _,
                visibility: _,
                ambient: _,
                is_abstract: _,
                is_override: _,
                is_static: _,
                is_accessor: _,
                is_comptime: _,
                symbol: _,
            } = member
            else {
                return None;
            };
            Some(signature.asynchrony)
        }
        dir::NodeType::Property => {
            let property = tree.get(node_id.into_typed::<dir::Property>());
            let dir::Property::Method {
                key: _,
                signature,
                body: _,
                symbol: _,
            } = property
            else {
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
    // normalize statement and parenthesized wrappers first
    let expression_id = expression_unwrap_statement(tree, expression_id);
    let expression = tree.get(expression_id);

    matches!(
        expression,
        dir::Expression::Using {
            asynchrony: _,
            export: _,
            ambient: _,
            declarators: _,
        }
    )
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
        dir::Expression::As { expression, .. } if *expression == child_expression_id
    ) || matches!(
        parent_expression,
        dir::Expression::Satisfies { expression, .. } if *expression == child_expression_id
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
    let mut outer_expression_id = expression_outer_transparent_ancestor(tree, expression_id);

    // root expressions are statement-position by default
    if tree.get_parent(outer_expression_id.id).is_none() {
        return Some(outer_expression_id);
    }

    loop {
        let parent_node_id = tree.get_parent(outer_expression_id.id)?;

        if parent_node_id.ty == dir::NodeType::Expression {
            let parent_expression_id = parent_node_id.into_typed::<dir::Expression>();
            let parent_expression = tree.get(parent_expression_id);

            if let dir::Expression::Labelled { body, .. } = parent_expression
                && *body == outer_expression_id
            {
                outer_expression_id = parent_expression_id;
                continue;
            }

            return None;
        }

        if parent_node_id.ty == dir::NodeType::Block {
            let block_id = parent_node_id.into_typed::<dir::Block>();
            let block = tree.get(block_id);

            if block
                .leading_expressions
                .iter()
                .copied()
                .any(|child_id| child_id == outer_expression_id)
            {
                return Some(outer_expression_id);
            }
        }

        return None;
    }
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
