use destack_dir as dir;
use destack_source::Span;

use crate::LintModuleDirContext;

use super::expression_unwrap_statement;

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
        match parent_node_id.ty {
            dir::NodeType::Expression => {
                let parent_expression_id = parent_node_id.into_typed::<dir::Expression>();
                let parent_expression = tree.get(parent_expression_id);
                if expression_is_type_slot_in_parent_expression(parent_expression, current_node_id)
                {
                    return true;
                }
            }
            dir::NodeType::Declarator => {
                let parent_declarator_id = parent_node_id.into_typed::<dir::Declarator>();
                let parent_declarator = tree.get(parent_declarator_id);
                if parent_declarator
                    .ty
                    .is_some_and(|type_expression_id| type_expression_id.id == current_node_id)
                {
                    return true;
                }
            }
            dir::NodeType::Declaration => {
                let parent_declaration_id = parent_node_id.into_typed::<dir::Declaration>();
                let parent_declaration = tree.get(parent_declaration_id);
                if declaration_type_slot_contains_expression(parent_declaration, current_node_id) {
                    return true;
                }
            }
            dir::NodeType::Member => {
                let parent_member_id = parent_node_id.into_typed::<dir::Member>();
                let parent_member = tree.get(parent_member_id);
                if member_type_slot_contains_expression(parent_member, current_node_id) {
                    return true;
                }
            }
            dir::NodeType::WhereClause => {
                let parent_where_clause_id = parent_node_id.into_typed::<dir::WhereClause>();
                let parent_where_clause = tree.get(parent_where_clause_id);
                if parent_where_clause.right.id == current_node_id {
                    return true;
                }
            }
            _ => {}
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
            let dir::Declaration::Function {
                descriptor: _,
                signature,
                scope: _,
                body: _,
            } = declaration
            else {
                return None;
            };
            Some(signature.asynchrony)
        }
        dir::NodeType::Member => {
            let member = tree.get(node_id.into_typed::<dir::Member>());
            let dir::Member::Method {
                modifiers: _,
                key: _,
                signature,
                body: _,
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
                modifiers: _,
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
            descriptor: _,
            declarators: _,
        }
    )
}

/// Return true when this child expression is in one parent expression type slot.
fn expression_is_type_slot_in_parent_expression(
    parent_expression: &dir::Expression,
    child_id: u32,
) -> bool {
    match parent_expression {
        dir::Expression::TypeUnary { operator: _, right } => right.id == child_id,
        dir::Expression::TypeBinary {
            left,
            operator: _,
            right,
        } => left.id == child_id || right.id == child_id,
        dir::Expression::TypeConditional {
            left,
            right,
            then_type,
            else_type,
        } => {
            left.id == child_id
                || right.id == child_id
                || then_type.id == child_id
                || else_type.id == child_id
        }
        dir::Expression::TypeMapped {
            parameter,
            modifiers: _,
            value,
        } => {
            parameter.constraint.id == child_id
                || parameter
                    .key_remap
                    .is_some_and(|key_remap_id| key_remap_id.id == child_id)
                || value.id == child_id
        }
        dir::Expression::TypeIndex { left, index } => left.id == child_id || index.id == child_id,
        dir::Expression::TypeTemplateLiteral { strings: _, spans } => {
            spans.iter().any(|span_id| span_id.id == child_id)
        }
        dir::Expression::TypeImport {
            target,
            arguments: _,
            qualifier: _,
            static_arguments: _,
        } => target.id == child_id,
        dir::Expression::TypeInfer {
            name: _,
            constraint: Some(constraint),
        } => constraint.id == child_id,
        dir::Expression::TypePredicate {
            asserts: _,
            subject: _,
            target: Some(target),
        } => target.id == child_id,
        dir::Expression::Cast {
            operator: _,
            source: _,
            value: _,
            target_type,
        } => target_type.id == child_id,
        _ => false,
    }
}

/// Return true when one declaration type slot points at this expression.
fn declaration_type_slot_contains_expression(
    declaration: &dir::Declaration,
    expression_id: u32,
) -> bool {
    match declaration {
        dir::Declaration::Type { value, .. } => value.id == expression_id,
        dir::Declaration::Function { signature, .. } => signature
            .return_type
            .is_some_and(|return_type_id| return_type_id.id == expression_id),
        dir::Declaration::Extension { target_type, .. } => target_type.id == expression_id,
        dir::Declaration::Struct { heritage, .. }
        | dir::Declaration::Class { heritage, .. }
        | dir::Declaration::Interface { heritage, .. }
        | dir::Declaration::Enum { heritage, .. } => {
            heritage
                .extends_types
                .as_ref()
                .is_some_and(|types| types.iter().any(|type_id| type_id.id == expression_id))
                || heritage
                    .implements_types
                    .as_ref()
                    .is_some_and(|types| types.iter().any(|type_id| type_id.id == expression_id))
                || heritage
                    .embedded_types
                    .as_ref()
                    .is_some_and(|types| types.iter().any(|type_id| type_id.id == expression_id))
        }
        _ => false,
    }
}

/// Return true when one member type slot points at this expression.
fn member_type_slot_contains_expression(member: &dir::Member, expression_id: u32) -> bool {
    match member {
        dir::Member::Type { ty, value, .. } => {
            ty.is_some_and(|type_expression_id| type_expression_id.id == expression_id)
                || value.is_some_and(|value_expression_id| value_expression_id.id == expression_id)
        }
        dir::Member::ComptimeConst { ty, .. } => {
            ty.is_some_and(|type_expression_id| type_expression_id.id == expression_id)
        }
        dir::Member::Field { value, .. } => {
            value.is_some_and(|type_expression_id| type_expression_id.id == expression_id)
        }
        dir::Member::Embed { value, .. } => value.id == expression_id,
        _ => false,
    }
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
