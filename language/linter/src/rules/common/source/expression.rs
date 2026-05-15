use destack_dir::{self as dir, NodeVisitor};
use destack_source::Span;

use crate::rules::common::{
    span_has_comment, stable_hash_debug, stable_hash_token, stable_hash_token_hashed_value,
};
use crate::{ConstValue, LintModuleContext};

/// Return the source expression id with parenthesized source form unwrapped.
pub fn expression_unwrap_parenthesized_source_form(
    tree: &dir::Tree,
    mut expression_id: dir::LocalNodeId<dir::Expression>,
) -> dir::LocalNodeId<dir::Expression> {
    // follow parenthesized wrappers until a non parenthesized expression is found
    loop {
        let expression = tree.get(expression_id);
        let dir::Expression::Parenthesized { expression } = expression else {
            return expression_id;
        };

        expression_id = *expression;
    }
}

/// Return the source expression id with statement source form unwrapped.
pub fn expression_unwrap_statement_source_form(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> dir::LocalNodeId<dir::Expression> {
    let _ = tree;
    expression_id
}

/// Return one simple expression target from one assignment pattern.
pub fn assign_pattern_expression(
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

/// Return true when one assignment pattern is one unqualified path name.
pub fn assign_pattern_is_unqualified_path_name(
    tree: &dir::Tree,
    assign_pattern_id: dir::LocalNodeId<dir::AssignPattern>,
    target_name: destack_core::StringId,
) -> bool {
    let Some(expression_id) = assign_pattern_expression(tree, assign_pattern_id) else {
        return false;
    };

    expression_is_unqualified_path_name(tree, expression_id, target_name)
}

/// Return true when one assignment pattern matches one expression.
pub fn assign_pattern_is_equal(
    ctx: &LintModuleContext<'_>,
    assign_pattern_id: dir::LocalNodeId<dir::AssignPattern>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let Some(assign_expression_id) = assign_pattern_expression(ctx.dir.tree(), assign_pattern_id)
    else {
        return false;
    };

    expression_is_equal(ctx, assign_expression_id, expression_id)
}

/// Return true when two assignment patterns are structurally equal.
pub fn assign_patterns_are_equal(
    ctx: &LintModuleContext<'_>,
    left_id: dir::LocalNodeId<dir::AssignPattern>,
    right_id: dir::LocalNodeId<dir::AssignPattern>,
) -> bool {
    let left_pattern = ctx.dir.get(left_id);
    let right_pattern = ctx.dir.get(right_id);

    match (left_pattern, right_pattern) {
        (
            dir::AssignPattern::Expression { value: left_value },
            dir::AssignPattern::Expression { value: right_value },
        ) => expression_is_equal(ctx, *left_value, *right_value),
        (
            dir::AssignPattern::Assign {
                pattern: left_pattern,
                value: left_value,
            },
            dir::AssignPattern::Assign {
                pattern: right_pattern,
                value: right_value,
            },
        ) => {
            assign_patterns_are_equal(ctx, *left_pattern, *right_pattern)
                && expression_is_equal(ctx, *left_value, *right_value)
        }
        (
            dir::AssignPattern::Sequence {
                fields: left_fields,
            },
            dir::AssignPattern::Sequence {
                fields: right_fields,
            },
        )
        | (
            dir::AssignPattern::Object {
                fields: left_fields,
            },
            dir::AssignPattern::Object {
                fields: right_fields,
            },
        ) => {
            left_fields.len() == right_fields.len()
                && left_fields
                    .iter()
                    .zip(right_fields.iter())
                    .all(|(left_field, right_field)| {
                        assign_pattern_fields_are_equal(ctx, *left_field, *right_field)
                    })
        }
        _ => false,
    }
}

/// Return true when two assignment pattern fields are structurally equal.
pub fn assign_pattern_fields_are_equal(
    ctx: &LintModuleContext<'_>,
    left_id: dir::LocalNodeId<dir::AssignPatternField>,
    right_id: dir::LocalNodeId<dir::AssignPatternField>,
) -> bool {
    let left_field = ctx.dir.get(left_id);
    let right_field = ctx.dir.get(right_id);

    match (left_field, right_field) {
        (
            dir::AssignPatternField::Named {
                name: left_name,
                is_shorthand: left_is_shorthand,
                pattern: left_pattern,
            },
            dir::AssignPatternField::Named {
                name: right_name,
                is_shorthand: right_is_shorthand,
                pattern: right_pattern,
            },
        ) => {
            left_name == right_name
                && left_is_shorthand == right_is_shorthand
                && match (left_pattern, right_pattern) {
                    (Some(left_pattern), Some(right_pattern)) => {
                        assign_patterns_are_equal(ctx, *left_pattern, *right_pattern)
                    }
                    (None, None) => true,
                    _ => false,
                }
        }
        (
            dir::AssignPatternField::Computed {
                key: left_key,
                pattern: left_pattern,
            },
            dir::AssignPatternField::Computed {
                key: right_key,
                pattern: right_pattern,
            },
        ) => {
            expression_is_equal(ctx, *left_key, *right_key)
                && assign_patterns_are_equal(ctx, *left_pattern, *right_pattern)
        }
        (
            dir::AssignPatternField::Positional {
                pattern: left_pattern,
            },
            dir::AssignPatternField::Positional {
                pattern: right_pattern,
            },
        ) => assign_patterns_are_equal(ctx, *left_pattern, *right_pattern),
        (
            dir::AssignPatternField::Spread {
                pattern: left_pattern,
            },
            dir::AssignPatternField::Spread {
                pattern: right_pattern,
            },
        ) => match (left_pattern, right_pattern) {
            (Some(left_pattern), Some(right_pattern)) => {
                assign_patterns_are_equal(ctx, *left_pattern, *right_pattern)
            }
            (None, None) => true,
            _ => false,
        },
        (dir::AssignPatternField::Elision, dir::AssignPatternField::Elision) => true,
        _ => false,
    }
}

/// The assignment wrapping style for one conditional expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionAssignmentStyle {
    /// No direct assignment style matched.
    None,
    /// Bare assignment expression: `x = y`.
    Bare,
    /// Single parenthesized assignment: `(x = y)`.
    SingleParenthesized,
}

/// Return the assignment wrapping style for one conditional expression.
pub fn condition_assignment_style(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> ConditionAssignmentStyle {
    // inspect the outer condition expression
    let expression = tree.get(expression_id);
    match expression {
        // let and using expressions are valid binding conditions
        dir::Expression::Let { .. } | dir::Expression::Using { .. } => {
            ConditionAssignmentStyle::None
        }
        dir::Expression::Assign { .. } => ConditionAssignmentStyle::Bare,
        dir::Expression::Parenthesized { expression } => {
            // single parens around assignment are still ambiguous
            let inner_expression = tree.get(*expression);
            if matches!(inner_expression, dir::Expression::Assign { .. }) {
                ConditionAssignmentStyle::SingleParenthesized
            } else {
                ConditionAssignmentStyle::None
            }
        }
        _ => ConditionAssignmentStyle::None,
    }
}

/// Return the condition expression id for supported control-flow expressions.
pub fn control_flow_condition_expression(
    expression: &dir::Expression,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    match expression {
        dir::Expression::If { condition, .. } => match condition {
            dir::IfCondition::Expression { condition } => Some(*condition),
            dir::IfCondition::Let { .. } => None,
        },
        dir::Expression::While { condition, .. } => Some(*condition),
        dir::Expression::For {
            condition: Some(condition),
            ..
        } => Some(*condition),
        _ => None,
    }
}

/// Return the outer expression id including parenthesized source form.
pub fn expression_outer_parenthesized_source_form(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> dir::LocalNodeId<dir::Expression> {
    // start from one normalized inner expression
    let mut current_id = expression_unwrap_parenthesized_source_form(tree, expression_id);

    // climb through direct parenthesized wrappers
    loop {
        let Some(parent_id) = tree.get_parent_id(current_id.id) else {
            return current_id;
        };
        if tree.get_node_type(parent_id) != dir::NodeType::Expression {
            return current_id;
        }

        let parent_expression_id = dir::LocalNodeId::<dir::Expression>::new(parent_id);
        let parent_expression = tree.get(parent_expression_id);
        if let dir::Expression::Parenthesized { expression } = parent_expression
            && *expression == current_id
        {
            current_id = parent_expression_id;
            continue;
        }

        return current_id;
    }
}

/// Return the surrounding statement expression for a standalone expression.
pub fn expression_statement_ancestor(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    // start from the outer source form
    let mut current_id = expression_outer_parenthesized_source_form(tree, expression_id);

    // root expressions are statement-position by default
    if tree.get_parent_id(current_id.id).is_none() {
        return Some(current_id);
    }

    // walk out through statement-like wrappers until one block boundary is found
    loop {
        let parent_id = tree.get_parent_id(current_id.id)?;

        if tree.get_node_type(parent_id) == dir::NodeType::Expression {
            let parent_expression_id = dir::LocalNodeId::<dir::Expression>::new(parent_id);
            let parent_expression = tree.get(parent_expression_id);

            if let dir::Expression::Label { body, .. } = parent_expression
                && *body == current_id
            {
                current_id = parent_expression_id;
                continue;
            }

            return None;
        }

        if tree.get_node_type(parent_id) == dir::NodeType::Block {
            let block_id = dir::LocalNodeId::<dir::Block>::new(parent_id);
            let block = tree.get(block_id);

            if block.context == dir::BlockContext::Statement
                && block
                    .iter_expressions()
                    .any(|child_id| child_id == current_id)
            {
                return Some(current_id);
            }
        }

        return None;
    }
}

/// Return the enclosing statement span for a standalone expression.
pub fn expression_statement_span(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<Span> {
    // resolve the enclosing statement wrapper
    let statement_expression_id = expression_statement_ancestor(tree, expression_id)?;

    // return the statement span
    Some(tree.get_span(statement_expression_id))
}

/// Return true when one expression is the direct child of a statement wrapper.
pub fn expression_is_direct_statement(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let outer_expression_id = expression_outer_parenthesized_source_form(tree, expression_id);
    expression_statement_ancestor(tree, expression_id) == Some(outer_expression_id)
}

/// Return true when one expression is a direct leading expression in a block.
pub fn expression_is_direct_block_leading_expression(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // normalize outer source form first
    let outer_expression_id = expression_outer_parenthesized_source_form(tree, expression_id);
    let Some(parent_id) = tree.get_parent_id(outer_expression_id.id) else {
        return false;
    };

    // require one enclosing block
    if tree.get_node_type(parent_id) != dir::NodeType::Block {
        return false;
    }

    // keep only leading block expressions, not value tails
    let block_id = dir::LocalNodeId::<dir::Block>::new(parent_id);
    let block = tree.get(block_id);

    block
        .leading_expressions
        .iter()
        .copied()
        .any(|child_id| child_id == outer_expression_id)
}

/// Return true when one expression can safely start an expression statement.
pub fn expression_can_start_expression_statement(expression: &dir::Expression) -> bool {
    matches!(
        expression,
        dir::Expression::Identifier { .. }
            | dir::Expression::QualifiedReference { .. }
            | dir::Expression::Member { .. }
            | dir::Expression::Index { .. }
            | dir::Expression::Call { .. }
            | dir::Expression::New { .. }
            | dir::Expression::Await { .. }
            | dir::Expression::Unary { .. }
            | dir::Expression::Maybe { .. }
            | dir::Expression::Must { .. }
            | dir::Expression::ScalarLiteral(_)
            | dir::Expression::Type { .. }
            | dir::Expression::TemplateExpression { .. }
            | dir::Expression::TaggedTemplateExpression { .. }
    )
}

/// Return the trailing non null assertion span when one expression text ends with `!`.
pub fn expression_trailing_bang_span(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<Span> {
    // resolve one text slice for the expression span
    let expression_span = ctx.dir.get_span(expression_id);
    let expression_text = ctx.get_span_text(expression_span);

    // require one trailing non whitespace `!` character
    let trimmed_text = expression_text.trim_end();
    if !trimmed_text.ends_with('!') {
        return None;
    }

    // resolve the byte offset for the trailing `!`
    let bang_relative_start = trimmed_text.len().checked_sub(1)? as u32;
    let bang_start = expression_span.start + bang_relative_start;
    let bang_end = bang_start + 1;

    // return one source span for the assertion token
    Some(Span::new(expression_span.file, bang_start, bang_end))
}

/// Build one negated expression string while preserving precedence.
pub fn expression_negated_source_text(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> String {
    // keep the original source text for replacement fidelity
    let expression_span = ctx.dir.get_span(expression_id);
    let expression_text = ctx.get_span_text(expression_span);

    // normalize the expression shape for precedence checks
    let normalized_expression_id =
        expression_unwrap_parenthesized_source_form(ctx.dir.tree(), expression_id);
    let normalized_expression = ctx.dir.get(normalized_expression_id);

    // preserve precedence for non-atomic expressions
    if expression_needs_parentheses_for_prefix_not(normalized_expression) {
        return format!("!({expression_text})");
    }

    format!("!{expression_text}")
}

/// Return true when one expression needs parentheses under prefix `!`.
fn expression_needs_parentheses_for_prefix_not(expression: &dir::Expression) -> bool {
    !expression_can_start_expression_statement(expression)
        && !matches!(expression, dir::Expression::Parenthesized { .. })
}

/// Return true when one expression starts a nested declaration scope.
pub fn expression_starts_nested_declaration_scope(expression: &dir::Expression) -> bool {
    matches!(expression, dir::Expression::Declaration(_))
}

/// Return true when one declaration expression is immediately invoked.
pub fn expression_is_immediately_invoked(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let mut current_expression_id = expression_id;

    loop {
        // resolve one expression parent
        let Some(parent_id) = tree.get_parent_id(current_expression_id.id) else {
            return false;
        };
        if tree.get_node_type(parent_id) != dir::NodeType::Expression {
            return false;
        }

        // keep climbing through parenthesized wrappers
        let parent_expression_id = dir::LocalNodeId::<dir::Expression>::new(parent_id);
        let parent_expression = tree.get(parent_expression_id);
        match parent_expression {
            dir::Expression::Parenthesized { expression }
                if *expression == current_expression_id =>
            {
                current_expression_id = parent_expression_id;
            }
            dir::Expression::Call { left, .. } | dir::Expression::New { left, .. } => {
                return *left == current_expression_id;
            }
            _ => return false,
        }
    }
}

/// Return true when one expression subtree contains an assignment expression.
pub fn expression_contains_assignment(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // visit one expression subtree and stop once assignment is found
    let expression = tree.get(expression_id);
    let mut visitor = AssignmentSearchVisitor {
        options: dir::NodeVisitorOptions::default(),
        found_assignment: false,
    };
    visitor.visit_expression(tree, expression_id, expression);

    visitor.found_assignment
}

/// Return true when one expression subtree mentions this identifier name.
pub fn expression_subtree_mentions_identifier_name(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
    name: dir::StringId,
) -> bool {
    subtree_mentions_identifier_name(tree, dir::NodeType::Expression, expression_id.id, name)
}

/// Return true when one source subtree mentions this identifier name.
pub fn subtree_mentions_identifier_name(
    tree: &dir::Tree,
    node_type: dir::NodeType,
    node_id: u32,
    name: dir::StringId,
) -> bool {
    // walk only the relevant subtree
    let mut visitor = IdentifierNameSearchVisitor {
        options: dir::NodeVisitorOptions::default(),
        name,
        found_name: false,
    };
    dir::walk_any(&mut visitor, tree, node_type, node_id);

    visitor.found_name
}

/// Return true when one expression is the target of optional chaining.
pub fn expression_is_optional_chain_target(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // require one expression parent
    let Some(parent_id) = tree.get_parent_id(expression_id.id) else {
        return false;
    };
    if tree.get_node_type(parent_id) != dir::NodeType::Expression {
        return false;
    }

    // keep optional chain wrappers that reference this expression directly
    let parent_expression_id = dir::LocalNodeId::<dir::Expression>::new(parent_id);
    let parent_expression = tree.get(parent_expression_id);
    matches!(
        parent_expression,
        dir::Expression::Maybe {
            position: dir::PostfixPosition::Indirect,
            left,
        } if *left == expression_id
    )
}

/// One normalized `if` branch chain.
#[derive(Debug)]
pub struct IfBranchChain {
    /// Branch body expressions in source order.
    pub branch_expressions: Vec<dir::LocalNodeId<dir::Expression>>,
    /// Whether the chain ends with an explicit `else` branch.
    pub ends_with_else: bool,
}

/// Return true when an `if` expression is an `else if` child branch.
pub fn expression_is_else_if_branch(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // require an expression parent
    let Some(parent_id) = tree.get_parent_id(expression_id.id) else {
        return false;
    };
    if tree.get_node_type(parent_id) != dir::NodeType::Expression {
        return false;
    }

    // keep parent `if` expressions that reference this node as their `else`
    let parent_expression_id = dir::LocalNodeId::<dir::Expression>::new(parent_id);
    let parent_expression = tree.get(parent_expression_id);
    matches!(
        parent_expression,
        dir::Expression::If {
            else_expression: Some(else_expression_id),
            ..
        } if *else_expression_id == expression_id
    )
}

/// Return one normalized branch chain for an `if` expression.
pub fn if_expression_branch_chain(
    tree: &dir::Tree,
    if_expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<IfBranchChain> {
    // require one `if` expression entry point
    let mut current_if_id = if_expression_id;
    let mut branch_expressions = Vec::new();

    loop {
        let current_expression = tree.get(current_if_id);
        let dir::Expression::If {
            form: _,
            then_expression,
            else_expression,
            ..
        } = current_expression
        else {
            return None;
        };

        // track the current then branch body
        branch_expressions.push(*then_expression);

        // continue into else-if chains or stop on a final else body
        let Some(else_expression_id) = else_expression else {
            return Some(IfBranchChain {
                branch_expressions,
                ends_with_else: false,
            });
        };

        let else_expression = tree.get(*else_expression_id);
        if matches!(
            else_expression,
            dir::Expression::If {
                form: dir::IfForm::If,
                ..
            }
        ) {
            current_if_id = *else_expression_id;
            continue;
        }

        branch_expressions.push(*else_expression_id);
        return Some(IfBranchChain {
            branch_expressions,
            ends_with_else: true,
        });
    }
}

/// Return true when one expression is a direct unqualified path to a name.
pub fn expression_is_unqualified_path_name(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
    name: dir::StringId,
) -> bool {
    // normalize wrappers and resolve path segments
    let Some(path_segments) = expression_path_segments(tree, expression_id) else {
        return false;
    };

    // match one bare identifier segment
    path_segments.len() == 1 && path_segments[0] == name
}

/// Return true when one expression belongs to one type annotation position.
pub fn expression_is_type_annotation(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // start from one normalized expression id
    let mut current_id = expression_unwrap_parenthesized_source_form(tree, expression_id).id;

    // climb ancestors until one type annotation slot is found
    while let Some(parent_id) = tree.get_parent_id(current_id) {
        let parent_type = tree.get_node_type(parent_id);

        // check declarator annotation slots
        if parent_type == dir::NodeType::Declarator {
            let declarator_id = dir::LocalNodeId::<dir::Declarator>::new(parent_id);
            let declarator = tree.get(declarator_id);
            if declarator.ty.is_some_and(|ty_id| ty_id.id == current_id) {
                return true;
            }
        }

        // check parameter annotation slots
        if parent_type == dir::NodeType::Parameter {
            let parameter_id = dir::LocalNodeId::<dir::Parameter>::new(parent_id);
            let parameter = tree.get(parameter_id);
            let parameter_type = match parameter {
                dir::Parameter::Named { declared_type, .. }
                | dir::Parameter::Pattern { declared_type, .. }
                | dir::Parameter::VariadicNamed { declared_type, .. }
                | dir::Parameter::VariadicPattern { declared_type, .. } => *declared_type,
                dir::Parameter::Error => None,
            };
            if parameter_type.is_some_and(|ty_id| ty_id.id == current_id) {
                return true;
            }
        }

        // check declaration type expression slots
        if parent_type == dir::NodeType::Declaration {
            let declaration_id = dir::LocalNodeId::<dir::Declaration>::new(parent_id);
            let declaration = tree.get(declaration_id);
            match declaration {
                dir::Declaration::Type(declaration) => {
                    if declaration.value.id == current_id {
                        return true;
                    }
                }
                dir::Declaration::Function(declaration) => {
                    if declaration
                        .signature
                        .return_type
                        .is_some_and(|return_type_id| return_type_id.id == current_id)
                    {
                        return true;
                    }
                }
                _ => {}
            }
        }

        // check member type expression slots
        if parent_type == dir::NodeType::Member {
            let member_id = dir::LocalNodeId::<dir::Member>::new(parent_id);
            let member = tree.get(member_id);
            match member {
                dir::Member::AssociatedType {
                    constraint, value, ..
                } => {
                    if constraint.is_some_and(|ty_id| ty_id.id == current_id)
                        || value.is_some_and(|value_id| value_id.id == current_id)
                    {
                        return true;
                    }
                }
                dir::Member::AssociatedConst { declared_type, .. } => {
                    if declared_type.is_some_and(|ty_id| ty_id.id == current_id) {
                        return true;
                    }
                }
                dir::Member::Field { declared_type, .. } => {
                    if declared_type.is_some_and(|value_id| value_id.id == current_id) {
                        return true;
                    }
                }
                dir::Member::Method { signature, .. } => {
                    if signature
                        .return_type
                        .is_some_and(|return_type_id| return_type_id.id == current_id)
                    {
                        return true;
                    }
                }
                _ => {}
            }
        }

        // check property method return types
        if parent_type == dir::NodeType::Property {
            let property_id = dir::LocalNodeId::<dir::Property>::new(parent_id);
            let property = tree.get(property_id);
            if let dir::Property::Method { signature, .. } = property
                && signature
                    .return_type
                    .is_some_and(|return_type_id| return_type_id.id == current_id)
            {
                return true;
            }
        }

        // check where clause type slots
        if parent_type == dir::NodeType::WhereClause {
            let where_clause_id = dir::LocalNodeId::<dir::WhereClause>::new(parent_id);
            let where_clause = tree.get(where_clause_id);
            if where_clause.right.id == current_id {
                return true;
            }
        }

        current_id = parent_id;
    }

    false
}

/// Visitor that tracks whether one expression subtree contains assignment.
struct AssignmentSearchVisitor {
    /// Traversal options for expression walk.
    options: dir::NodeVisitorOptions,
    /// Whether an assignment node has been encountered.
    found_assignment: bool,
}

impl dir::NodeVisitor for AssignmentSearchVisitor {
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // stop traversal after first assignment match
        if self.found_assignment {
            return;
        }

        // record assignment match and stop descending
        if matches!(expression, dir::Expression::Assign { .. }) {
            self.found_assignment = true;
            return;
        }

        dir::walk_expression(self, tree, expression_id, expression);
    }
}

/// Visitor that tracks whether one expression subtree mentions a name.
struct IdentifierNameSearchVisitor {
    /// Visitor options.
    options: dir::NodeVisitorOptions,
    /// The target identifier name.
    name: dir::StringId,
    /// Whether a matching mention has been found.
    found_name: bool,
}

impl dir::NodeVisitor for IdentifierNameSearchVisitor {
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // stop once the target name has been found
        if self.found_name {
            return;
        }

        // match direct unqualified paths
        if expression_is_unqualified_path_name(tree, expression_id, self.name) {
            self.found_name = true;
            return;
        }

        dir::walk_expression(self, tree, expression_id, expression);
    }

    fn visit_declaration(
        &mut self,
        tree: &dir::Tree,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) {
        // stop once the target name has been found
        if self.found_name {
            return;
        }

        // match declared names before descending
        if declaration
            .name()
            .is_some_and(|declaration_name| declaration_name.string() == self.name)
        {
            self.found_name = true;
            return;
        }

        dir::walk_declaration(self, tree, declaration_id, declaration);
    }

    fn visit_parameter(
        &mut self,
        tree: &dir::Tree,
        parameter_id: dir::LocalNodeId<dir::Parameter>,
        parameter: &dir::Parameter,
    ) {
        // stop once the target name has been found
        if self.found_name {
            return;
        }

        // match named parameters before descending
        let parameter_name = match parameter {
            dir::Parameter::Named { name, .. } | dir::Parameter::VariadicNamed { name, .. } => {
                Some(*name)
            }
            _ => None,
        };
        if parameter_name == Some(self.name) {
            self.found_name = true;
            return;
        }

        dir::walk_parameter(self, tree, parameter_id, parameter);
    }

    fn visit_pattern(
        &mut self,
        tree: &dir::Tree,
        pattern_id: dir::LocalNodeId<dir::Pattern>,
        pattern: &dir::Pattern,
    ) {
        // stop once the target name has been found
        if self.found_name {
            return;
        }

        // match binding patterns before descending
        if let dir::Pattern::Binding { name, .. } = pattern
            && *name == self.name
        {
            self.found_name = true;
            return;
        }

        dir::walk_pattern(self, tree, pattern_id, pattern);
    }
}

/// Return true when one block has no expressions and no comment trivia.
pub fn block_is_empty_without_comment(
    tree: &dir::Tree,
    block_id: dir::LocalNodeId<dir::Block>,
) -> bool {
    let block = tree.get(block_id);
    if !block.is_empty() {
        return false;
    }

    let block_span = tree.get_span(block_id);
    !span_has_comment(tree, block_span)
}

/// Return the block expression id for one block node.
pub fn block_expression_ancestor(
    tree: &dir::Tree,
    block_id: dir::LocalNodeId<dir::Block>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    let parent_id = tree.get_parent_id(block_id.id)?;
    if tree.get_node_type(parent_id) != dir::NodeType::Expression {
        return None;
    }

    let expression_id = dir::LocalNodeId::<dir::Expression>::new(parent_id);
    let expression = tree.get(expression_id);
    if !matches!(expression, dir::Expression::Block(current_id) if *current_id == block_id) {
        return None;
    }

    Some(expression_id)
}

/// Return true when one block expression is the body of a function or method.
pub fn block_is_function_body(tree: &dir::Tree, block_id: dir::LocalNodeId<dir::Block>) -> bool {
    // resolve the expression that owns this block
    let Some(block_expression_id) = block_expression_ancestor(tree, block_id) else {
        return false;
    };

    // resolve the parent node that owns the expression
    let Some(owner_id) = tree.get_parent_id(block_expression_id.id) else {
        return false;
    };

    // allow direct function declaration bodies
    if tree.get_node_type(owner_id) == dir::NodeType::Declaration {
        let declaration = tree.get(dir::LocalNodeId::<dir::Declaration>::new(owner_id));
        if matches!(
            declaration,
            dir::Declaration::Function(dir::FunctionDeclaration {
                body: Some(body_id),
                ..
            }) if *body_id == block_expression_id
        ) {
            return true;
        }
    }

    // allow method bodies
    if tree.get_node_type(owner_id) == dir::NodeType::Member {
        let member = tree.get(dir::LocalNodeId::<dir::Member>::new(owner_id));
        if matches!(
            member,
            dir::Member::Method {
                body: Some(body_id),
                ..
            } if *body_id == block_expression_id
        ) {
            return true;
        }
    }

    false
}

/// Return true when one block expression belongs to a static block member.
pub fn block_is_static_block_body(
    tree: &dir::Tree,
    block_id: dir::LocalNodeId<dir::Block>,
) -> bool {
    // resolve the expression that wraps this block
    let Some(block_expression_id) = block_expression_ancestor(tree, block_id) else {
        return false;
    };

    // require a member owner for static blocks
    let Some(owner_id) = tree.get_parent_id(block_expression_id.id) else {
        return false;
    };
    if tree.get_node_type(owner_id) != dir::NodeType::Member {
        return false;
    }

    // accept only static block members with the same body expression
    let member = tree.get(dir::LocalNodeId::<dir::Member>::new(owner_id));
    matches!(
        member,
        dir::Member::StaticBlock { body, .. } if *body == block_expression_id
    )
}

/// Return one declaration wrapper expression id for a declaration node.
pub fn declaration_expression(
    tree: &dir::Tree,
    declaration_id: dir::LocalNodeId<dir::Declaration>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    // resolve the declaration parent node
    let parent_id = tree.get_parent_id(declaration_id.id)?;
    if tree.get_node_type(parent_id) != dir::NodeType::Expression {
        return None;
    }

    // require an expression::declaration wrapper with the same declaration id
    let expression_id = dir::LocalNodeId::<dir::Expression>::new(parent_id);
    let expression = tree.get(expression_id);
    if !matches!(expression, dir::Expression::Declaration(current) if *current == declaration_id) {
        return None;
    }

    Some(expression_id)
}

/// Return true when one declaration expression is at an allowed root location.
pub fn declaration_at_allowed_root(
    tree: &dir::Tree,
    declaration_expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // default to allowed when parent structure is missing
    let Some(parent_id) = tree.get_parent_id(declaration_expression_id.id) else {
        return true;
    };

    // declarations are allowed only when directly under a block
    if tree.get_node_type(parent_id) != dir::NodeType::Block {
        return false;
    }
    let block_id = dir::LocalNodeId::<dir::Block>::new(parent_id);
    let block = tree.get(block_id);
    if !block.is_explicit() {
        return true;
    }

    // resolve the expression that owns this block
    let Some(block_expression_id) = block_expression_ancestor(tree, block_id) else {
        return false;
    };

    // default to allowed when the block expression has no owner
    let Some(owner_id) = tree.get_parent_id(block_expression_id.id) else {
        return true;
    };

    // allow function declaration roots
    if tree.get_node_type(owner_id) == dir::NodeType::Declaration {
        let declaration = tree.get(dir::LocalNodeId::<dir::Declaration>::new(owner_id));
        if matches!(
            declaration,
            dir::Declaration::Function(dir::FunctionDeclaration {
                body: Some(body_id),
                ..
            }) if *body_id == block_expression_id
        ) {
            return true;
        }
    }

    // allow static block roots
    if tree.get_node_type(owner_id) == dir::NodeType::Member {
        let member = tree.get(dir::LocalNodeId::<dir::Member>::new(owner_id));
        if matches!(
            member,
            dir::Member::StaticBlock { body, .. } if *body == block_expression_id
        ) {
            return true;
        }
    }

    false
}

/// Return path segments when the expression is a non-generic reference chain.
pub fn expression_path_segments(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<Vec<dir::StringId>> {
    let mut segments = Vec::new();

    collect_expression_path_segments(tree, expression_id, &mut segments)?;

    Some(segments)
}

/// Collect path segments for one non-generic reference chain.
fn collect_expression_path_segments(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
    segments: &mut Vec<dir::StringId>,
) -> Option<()> {
    // normalize wrappers first
    let expression_id = expression_unwrap_parenthesized_source_form(tree, expression_id);
    let expression = tree.get(expression_id);

    match expression {
        dir::Expression::Identifier { name } => {
            segments.push(*name);
            Some(())
        }
        dir::Expression::QualifiedReference {
            path,
            generic_arguments,
        } => {
            if !generic_arguments.is_empty() {
                return None;
            }

            segments.extend_from_slice(&path.segments);
            Some(())
        }
        dir::Expression::Member {
            left,
            name: Some(name),
        } => {
            collect_expression_path_segments(tree, *left, segments)?;
            segments.push(*name);
            Some(())
        }
        _ => None,
    }
}

/// Return one static string literal value from an expression source form.
pub fn expression_static_string_literal_source_form(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::StringId> {
    // normalize expression shape
    let expression_id = expression_unwrap_parenthesized_source_form(tree, expression_id);
    let expression = tree.get(expression_id);

    // match direct string literals
    if let dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(value)) = expression {
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

/// Return one static property access pair as `(left, property_name)` from source form.
pub fn expression_static_property_access_source_form(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<(dir::LocalNodeId<dir::Expression>, dir::StringId)> {
    // normalize expression shape
    let expression_id = expression_unwrap_parenthesized_source_form(tree, expression_id);
    let expression = tree.get(expression_id);

    // match dot member access
    if let dir::Expression::Member { left, name, .. } = expression {
        let name = (*name)?;

        return Some((*left, name));
    }

    // match bracket member access with static string keys
    let dir::Expression::Index { left, index, .. } = expression else {
        return None;
    };
    let index_id = index.as_ref().copied()?;
    let property_name = expression_static_string_literal_source_form(tree, index_id)?;

    Some((*left, property_name))
}

/// Return the value expression id for one argument node.
pub fn argument_value_expression_id(
    tree: &dir::Tree,
    argument_id: dir::LocalNodeId<dir::Argument>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    let argument = tree.get(argument_id);
    match argument {
        dir::Argument::Positional { value, .. }
        | dir::Argument::Spread { value, .. }
        | dir::Argument::Named { value, .. }
        | dir::Argument::Labeled { value, .. } => Some(*value),
        dir::Argument::Error => None,
    }
}

/// Convert a constant value into an f64 when possible.
pub fn const_value_f64(value: ConstValue) -> Option<f64> {
    match value {
        ConstValue::Boolean(value) => Some(if value { 1.0 } else { 0.0 }),
        ConstValue::Integer(value) => Some(value as f64),
        ConstValue::Bigint(value) => Some(value as f64),
        ConstValue::Float(value) => Some(value),
        ConstValue::Null => Some(0.0),
        ConstValue::Undefined => None,
    }
}

/// Evaluate an expression as a numeric constant when possible.
pub fn expression_numeric_value(
    ctx: &mut LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<f64> {
    // resolve direct constant values first
    if let Some(value) = ctx.const_value(expression_id) {
        return const_value_f64(value);
    }

    // normalize expression shape and require a binary expression
    let expression_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), expression_id);
    let expression = ctx.dir.get(expression_id);
    let dir::Expression::Binary {
        operator,
        left,
        right,
    } = expression
    else {
        return None;
    };

    // evaluate both sides recursively
    let left_value = expression_numeric_value(ctx, *left)?;
    let right_value = expression_numeric_value(ctx, *right)?;

    // apply numeric operator semantics
    match operator {
        dir::BinaryOperator::Add => Some(left_value + right_value),
        dir::BinaryOperator::Subtract => Some(left_value - right_value),
        dir::BinaryOperator::Multiply => Some(left_value * right_value),
        dir::BinaryOperator::Divide => Some(left_value / right_value),
        dir::BinaryOperator::Remainder => Some(left_value % right_value),
        dir::BinaryOperator::Exponent => Some(left_value.powf(right_value)),
        _ => None,
    }
}

/// Evaluate an expression as a signed numeric constant when possible.
pub fn expression_numeric_sign(
    ctx: &mut LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<i8> {
    // resolve numeric value
    let value = expression_numeric_value(ctx, expression_id)?;

    // classify positive values
    if value > 0.0 {
        return Some(1);
    }

    // classify negative values
    if value < 0.0 {
        return Some(-1);
    }

    None
}

/// Return whether two expressions are structurally equal.
///
/// Compares expressions by structure, ignoring parentheses.
/// Handles paths, literals, and recursively compares binary/unary operations.
pub fn expression_is_equal(
    ctx: &LintModuleContext<'_>,
    left_id: dir::LocalNodeId<dir::Expression>,
    right_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let left_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), left_id);
    let right_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), right_id);
    let left = ctx.dir.get(left_id);
    let right = ctx.dir.get(right_id);
    match (left, right) {
        // identifiers: compare names
        (
            dir::Expression::Identifier { name: left_name },
            dir::Expression::Identifier { name: right_name },
        ) => string_ids_equal(ctx, *left_name, *right_name),

        // qualified references: compare segments
        (
            dir::Expression::QualifiedReference {
                path: left_path, ..
            },
            dir::Expression::QualifiedReference {
                path: right_path, ..
            },
        ) => paths_equal(ctx, left_path, right_path),

        // scalar literals: direct comparison
        (
            dir::Expression::ScalarLiteral(left_literal),
            dir::Expression::ScalarLiteral(right_literal),
        ) => left_literal == right_literal,

        // type expressions: compare recursively
        (dir::Expression::Type { value: left }, dir::Expression::Type { value: right }) => {
            type_expression_is_equal(ctx, *left, *right)
        }

        // binary expressions: compare operator and operands recursively
        (
            dir::Expression::Binary {
                operator: left_operator,
                left: left_left,
                right: left_right,
            },
            dir::Expression::Binary {
                operator: right_operator,
                left: right_left,
                right: right_right,
            },
        ) => {
            left_operator == right_operator
                && expression_is_equal(ctx, *left_left, *right_left)
                && expression_is_equal(ctx, *left_right, *right_right)
        }

        // unary expressions: compare operator and operand recursively
        (
            dir::Expression::Unary {
                operator: left_operator,
                right: left_right,
            },
            dir::Expression::Unary {
                operator: right_operator,
                right: right_right,
            },
        ) => left_operator == right_operator && expression_is_equal(ctx, *left_right, *right_right),

        // member access: compare object and member name
        (
            dir::Expression::Member {
                left: left_object,
                name: left_name,
                ..
            },
            dir::Expression::Member {
                left: right_object,
                name: right_name,
                ..
            },
        ) => {
            expression_is_equal(ctx, *left_object, *right_object)
                && left_name
                    .zip(*right_name)
                    .is_some_and(|(left_name, right_name)| {
                        string_ids_equal(ctx, left_name, right_name)
                    })
        }

        // index access: compare object and index
        (
            dir::Expression::Index {
                left: left_object,
                index: left_index,
                position: left_position,
            },
            dir::Expression::Index {
                left: right_object,
                index: right_index,
                position: right_position,
            },
        ) => {
            if left_position != right_position {
                return false;
            }
            if !expression_is_equal(ctx, *left_object, *right_object) {
                return false;
            }
            match (left_index, right_index) {
                (Some(left), Some(right)) => expression_is_equal(ctx, *left, *right),
                (None, None) => true,
                _ => false,
            }
        }

        // value of: compare mutability, variance, and operand
        (
            dir::Expression::MoveOf {
                mutability: left_mutability,
                variance: left_variance,
                right: left_right,
            },
            dir::Expression::MoveOf {
                mutability: right_mutability,
                variance: right_variance,
                right: right_right,
            },
        ) => {
            left_mutability == right_mutability
                && left_variance == right_variance
                && expression_is_equal(ctx, *left_right, *right_right)
        }

        // reference of: compare mutability, variance, and operand
        (
            dir::Expression::BorrowOf {
                mutability: left_mutability,
                variance: left_variance,
                right: left_right,
            },
            dir::Expression::BorrowOf {
                mutability: right_mutability,
                variance: right_variance,
                right: right_right,
            },
        ) => {
            left_mutability == right_mutability
                && left_variance == right_variance
                && expression_is_equal(ctx, *left_right, *right_right)
        }

        // await expressions: compare inner expression
        (
            dir::Expression::Await {
                expression: left_expression,
            },
            dir::Expression::Await {
                expression: right_expression,
            },
        ) => expression_is_equal(ctx, *left_expression, *right_expression),

        // await? expressions: compare inner expression
        (
            dir::Expression::AwaitMaybe {
                expression: left_expression,
            },
            dir::Expression::AwaitMaybe {
                expression: right_expression,
            },
        ) => expression_is_equal(ctx, *left_expression, *right_expression),

        // await! expressions: compare inner expression
        (
            dir::Expression::AwaitMust {
                expression: left_expression,
            },
            dir::Expression::AwaitMust {
                expression: right_expression,
            },
        ) => expression_is_equal(ctx, *left_expression, *right_expression),

        // throw expressions: compare value
        (
            dir::Expression::Throw { value: left_value },
            dir::Expression::Throw { value: right_value },
        ) => expression_is_equal(ctx, *left_value, *right_value),

        // maybe expressions: compare position and operand
        (
            dir::Expression::Maybe {
                position: left_position,
                left: left_left,
            },
            dir::Expression::Maybe {
                position: right_position,
                left: right_left,
            },
        ) => left_position == right_position && expression_is_equal(ctx, *left_left, *right_left),

        // must expressions: compare position and operand
        (
            dir::Expression::Must {
                position: left_position,
                left: left_left,
            },
            dir::Expression::Must {
                position: right_position,
                left: right_left,
            },
        ) => left_position == right_position && expression_is_equal(ctx, *left_left, *right_left),

        // assignment: compare operator and operands
        (
            dir::Expression::Assign {
                left: left_left,
                operator: left_operator,
                right: left_right,
            },
            dir::Expression::Assign {
                left: right_left,
                operator: right_operator,
                right: right_right,
            },
        ) => {
            left_operator == right_operator
                && assign_patterns_are_equal(ctx, *left_left, *right_left)
                && expression_is_equal(ctx, *left_right, *right_right)
        }

        // call expressions: compare callee and arguments
        (
            dir::Expression::Call {
                left: left_callee,
                arguments: left_args,
                ..
            },
            dir::Expression::Call {
                left: right_callee,
                arguments: right_args,
                ..
            },
        ) => {
            expression_is_equal(ctx, *left_callee, *right_callee)
                && arguments_are_equal(ctx, left_args, right_args)
        }

        // blocks: compare contents
        (dir::Expression::Block(left_block), dir::Expression::Block(right_block)) => {
            blocks_equal(ctx, *left_block, *right_block)
        }

        // shared structural comparison for remaining expression kinds
        _ => expression_signature_equal(ctx, left_id, right_id),
    }
}

/// Return whether two type expressions are structurally equal.
pub fn type_expression_is_equal(
    ctx: &LintModuleContext<'_>,
    left_id: dir::LocalNodeId<dir::TypeExpression>,
    right_id: dir::LocalNodeId<dir::TypeExpression>,
) -> bool {
    let left = ctx.dir.get(left_id);
    let right = ctx.dir.get(right_id);

    match (left, right) {
        (
            dir::TypeExpression::Parenthesized { expression: left },
            dir::TypeExpression::Parenthesized { expression: right },
        ) => type_expression_is_equal(ctx, *left, *right),
        (
            dir::TypeExpression::ScalarLiteral { value: left },
            dir::TypeExpression::ScalarLiteral { value: right },
        ) => left == right,
        (
            dir::TypeExpression::Literal { value: left },
            dir::TypeExpression::Literal { value: right },
        ) => left == right,
        (
            dir::TypeExpression::Reference {
                path: left_path,
                generic_arguments: left_arguments,
            },
            dir::TypeExpression::Reference {
                path: right_path,
                generic_arguments: right_arguments,
            },
        ) => {
            paths_equal(ctx, left_path, right_path)
                && generic_arguments_are_equal(ctx, left_arguments, right_arguments)
        }
        (
            dir::TypeExpression::Member {
                left: left_target,
                name: left_name,
                generic_arguments: left_arguments,
            },
            dir::TypeExpression::Member {
                left: right_target,
                name: right_name,
                generic_arguments: right_arguments,
            },
        ) => {
            string_ids_equal(ctx, *left_name, *right_name)
                && type_expression_is_equal(ctx, *left_target, *right_target)
                && generic_arguments_are_equal(ctx, left_arguments, right_arguments)
        }
        (
            dir::TypeExpression::Readonly {
                target_type: left_target,
            },
            dir::TypeExpression::Readonly {
                target_type: right_target,
            },
        )
        | (
            dir::TypeExpression::KeyOf {
                target_type: left_target,
            },
            dir::TypeExpression::KeyOf {
                target_type: right_target,
            },
        )
        | (
            dir::TypeExpression::Must {
                target_type: left_target,
            },
            dir::TypeExpression::Must {
                target_type: right_target,
            },
        )
        | (
            dir::TypeExpression::AsComptime {
                target_type: left_target,
            },
            dir::TypeExpression::AsComptime {
                target_type: right_target,
            },
        )
        | (
            dir::TypeExpression::Not {
                target_type: left_target,
            },
            dir::TypeExpression::Not {
                target_type: right_target,
            },
        ) => type_expression_is_equal(ctx, *left_target, *right_target),
        (
            dir::TypeExpression::OwnedOf {
                mutability: left_mutability,
                variance: left_variance,
                target_type: left_target,
            },
            dir::TypeExpression::OwnedOf {
                mutability: right_mutability,
                variance: right_variance,
                target_type: right_target,
            },
        )
        | (
            dir::TypeExpression::BorrowedOf {
                mutability: left_mutability,
                variance: left_variance,
                target_type: left_target,
            },
            dir::TypeExpression::BorrowedOf {
                mutability: right_mutability,
                variance: right_variance,
                target_type: right_target,
            },
        ) => {
            left_mutability == right_mutability
                && left_variance == right_variance
                && type_expression_is_equal(ctx, *left_target, *right_target)
        }
        (
            dir::TypeExpression::PointerOf {
                mutability: left_mutability,
                target_type: left_target,
            },
            dir::TypeExpression::PointerOf {
                mutability: right_mutability,
                target_type: right_target,
            },
        ) => {
            left_mutability == right_mutability
                && type_expression_is_equal(ctx, *left_target, *right_target)
        }
        (
            dir::TypeExpression::TypeOfValue { value: left_value },
            dir::TypeExpression::TypeOfValue { value: right_value },
        ) => expression_is_equal(ctx, *left_value, *right_value),
        (
            dir::TypeExpression::Index {
                left: left_target,
                index: left_index,
            },
            dir::TypeExpression::Index {
                left: right_target,
                index: right_index,
            },
        ) => {
            type_expression_is_equal(ctx, *left_target, *right_target)
                && type_expression_is_equal(ctx, *left_index, *right_index)
        }
        (
            dir::TypeExpression::Union { elements: left },
            dir::TypeExpression::Union { elements: right },
        )
        | (
            dir::TypeExpression::Intersection { elements: left },
            dir::TypeExpression::Intersection { elements: right },
        ) => type_expression_list_equal(ctx, left, right),
        _ => {
            let left = ctx.dir.get(left_id);
            let right = ctx.dir.get(right_id);

            let mut left_collector = ExpressionSignatureCollector::new(ctx.strings);
            dir::NodeVisitor::visit_type_expression(
                &mut left_collector,
                ctx.dir.tree(),
                left_id,
                left,
            );

            let mut right_collector = ExpressionSignatureCollector::new(ctx.strings);
            dir::NodeVisitor::visit_type_expression(
                &mut right_collector,
                ctx.dir.tree(),
                right_id,
                right,
            );

            left_collector.finish() == right_collector.finish()
        }
    }
}

/// Check if two blocks have identical expressions.
pub fn blocks_equal(
    ctx: &LintModuleContext<'_>,
    left_id: dir::LocalNodeId<dir::Block>,
    right_id: dir::LocalNodeId<dir::Block>,
) -> bool {
    let left = ctx.dir.get(left_id);
    let right = ctx.dir.get(right_id);
    if left.len() != right.len() {
        return false;
    }

    for (left_expr, right_expr) in left.iter_expressions().zip(right.iter_expressions()) {
        if !expression_is_equal(ctx, left_expr, right_expr) {
            return false;
        }
    }
    true
}

/// Return whether two paths are equal.
pub fn paths_equal(ctx: &LintModuleContext<'_>, left: &dir::Path, right: &dir::Path) -> bool {
    if left.segments.len() != right.segments.len() {
        return false;
    }

    for (left_segment, right_segment) in left.segments.iter().zip(right.segments.iter()) {
        if !string_ids_equal(ctx, *left_segment, *right_segment) {
            return false;
        }
    }

    true
}

/// Return whether two string IDs refer to equal strings.
pub fn string_ids_equal(
    ctx: &LintModuleContext<'_>,
    left: dir::StringId,
    right: dir::StringId,
) -> bool {
    let left_string = ctx.strings.get(left);
    let right_string = ctx.strings.get(right);
    left_string == right_string
}

/// Return whether two argument lists are equal.
pub fn arguments_are_equal(
    ctx: &LintModuleContext<'_>,
    left: &[dir::LocalNodeId<dir::Argument>],
    right: &[dir::LocalNodeId<dir::Argument>],
) -> bool {
    if left.len() != right.len() {
        return false;
    }

    for (left_arg_id, right_arg_id) in left.iter().zip(right.iter()) {
        if !argument_is_equal(ctx, *left_arg_id, *right_arg_id) {
            return false;
        }
    }

    true
}

/// Return whether two generic argument lists are structurally equal.
pub fn generic_arguments_are_equal(
    ctx: &LintModuleContext<'_>,
    left: &[dir::LocalNodeId<dir::GenericArgument>],
    right: &[dir::LocalNodeId<dir::GenericArgument>],
) -> bool {
    if left.len() != right.len() {
        return false;
    }

    for (left_argument_id, right_argument_id) in left.iter().zip(right.iter()) {
        if !generic_argument_is_equal(ctx, *left_argument_id, *right_argument_id) {
            return false;
        }
    }

    true
}

/// Return whether two generic arguments are structurally equal.
pub fn generic_argument_is_equal(
    ctx: &LintModuleContext<'_>,
    left_id: dir::LocalNodeId<dir::GenericArgument>,
    right_id: dir::LocalNodeId<dir::GenericArgument>,
) -> bool {
    let left = ctx.dir.get(left_id);
    let right = ctx.dir.get(right_id);

    match (left, right) {
        (
            dir::GenericArgument::Type { value: left_type },
            dir::GenericArgument::Type { value: right_type },
        ) => type_expression_is_equal(ctx, *left_type, *right_type),
        (
            dir::GenericArgument::Value { value: left_value },
            dir::GenericArgument::Value { value: right_value },
        ) => expression_is_equal(ctx, *left_value, *right_value),
        (dir::GenericArgument::Error, dir::GenericArgument::Error) => true,
        _ => false,
    }
}

/// Return whether two type expression lists are structurally equal.
fn type_expression_list_equal(
    ctx: &LintModuleContext<'_>,
    left: &[dir::LocalNodeId<dir::TypeExpression>],
    right: &[dir::LocalNodeId<dir::TypeExpression>],
) -> bool {
    if left.len() != right.len() {
        return false;
    }

    for (left_id, right_id) in left.iter().zip(right.iter()) {
        if !type_expression_is_equal(ctx, *left_id, *right_id) {
            return false;
        }
    }

    true
}

/// Return whether two arguments are structurally equal.
pub fn argument_is_equal(
    ctx: &LintModuleContext<'_>,
    left_id: dir::LocalNodeId<dir::Argument>,
    right_id: dir::LocalNodeId<dir::Argument>,
) -> bool {
    let left = ctx.dir.get(left_id);
    let right = ctx.dir.get(right_id);

    match (left, right) {
        // positional arguments
        (
            dir::Argument::Positional {
                value: left_value, ..
            },
            dir::Argument::Positional {
                value: right_value, ..
            },
        ) => expression_is_equal(ctx, *left_value, *right_value),

        // spread arguments
        (
            dir::Argument::Spread {
                value: left_value, ..
            },
            dir::Argument::Spread {
                value: right_value, ..
            },
        ) => expression_is_equal(ctx, *left_value, *right_value),

        // named arguments
        (
            dir::Argument::Named {
                name: left_name,
                value: left_value,
                ..
            },
            dir::Argument::Named {
                name: right_name,
                value: right_value,
                ..
            },
        ) => {
            left_name.string() == right_name.string()
                && expression_is_equal(ctx, *left_value, *right_value)
        }

        // labeled arguments
        (
            dir::Argument::Labeled {
                label: left_label,
                value: left_value,
                ..
            },
            dir::Argument::Labeled {
                label: right_label,
                value: right_value,
                ..
            },
        ) => {
            string_ids_equal(ctx, *left_label, *right_label)
                && expression_is_equal(ctx, *left_value, *right_value)
        }

        // different argument types
        _ => false,
    }
}

/// Check if an expression has side effects (conservatively returns true if unsure).
///
/// This is useful for lints that want to detect expressions that can be safely removed.
/// or that need to distinguish between pure and impure expressions.
/// #Cleanup: can expression_has_side_effects use NodeVisitor..?
pub fn expression_has_side_effects(
    ctx: &LintModuleContext<'_>,
    expr_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expr = ctx.dir.get(expr_id);
    match expr {
        // pure: literals
        dir::Expression::ScalarLiteral(_) | dir::Expression::PrivateIdentifier { .. } => false,
        dir::Expression::Type { value } => type_expression_has_side_effects(ctx, *value),

        // pure: references
        dir::Expression::Identifier { .. }
        | dir::Expression::QualifiedReference { .. }
        | dir::Expression::ImportMeta
        | dir::Expression::NewTarget
        | dir::Expression::This
        | dir::Expression::Super => false,

        // pure: containers (if elements are pure)
        dir::Expression::ArrayExpression { elements }
        | dir::Expression::TupleExpression { elements } => elements.iter().any(|arg_id| {
            let arg = ctx.dir.get(*arg_id);
            match arg {
                dir::Argument::Positional { value, .. }
                | dir::Argument::Spread { value, .. }
                | dir::Argument::Named { value, .. }
                | dir::Argument::Labeled { value, .. } => expression_has_side_effects(ctx, *value),
                dir::Argument::Error => true,
            }
        }),
        dir::Expression::FixedArrayExpression { value, length } => {
            expression_has_side_effects(ctx, *value) || expression_has_side_effects(ctx, *length)
        }

        // pure: member access (if object is pure)
        dir::Expression::Member { left, .. } | dir::Expression::PrivateMember { left, .. } => {
            expression_has_side_effects(ctx, *left)
        }

        // pure: instantiation (if target is pure)
        dir::Expression::Instantiation { left, .. } => expression_has_side_effects(ctx, *left),

        // pure: index access (if object and index are pure)
        dir::Expression::Index { left, index, .. } => {
            expression_has_side_effects(ctx, *left)
                || index.is_some_and(|idx| expression_has_side_effects(ctx, idx))
        }

        // pure: unary/binary ops on pure expressions
        dir::Expression::Unary { right, .. } => expression_has_side_effects(ctx, *right),
        dir::Expression::Is { value, target_type } => {
            expression_has_side_effects(ctx, *value)
                || type_expression_has_side_effects(ctx, *target_type)
        }
        dir::Expression::InstanceOf { value, target } => {
            expression_has_side_effects(ctx, *value) || expression_has_side_effects(ctx, *target)
        }
        dir::Expression::Binary { left, right, .. } => {
            expression_has_side_effects(ctx, *left) || expression_has_side_effects(ctx, *right)
        }
        dir::Expression::RangeExpression { start, end, .. } => {
            start.is_some_and(|start| expression_has_side_effects(ctx, start))
                || end.is_some_and(|end| expression_has_side_effects(ctx, end))
        }

        // pure: type operations
        dir::Expression::As {
            expression,
            target_type,
        }
        | dir::Expression::Satisfies {
            expression,
            target_type,
        } => {
            expression_has_side_effects(ctx, *expression)
                || type_expression_has_side_effects(ctx, *target_type)
        }

        // pure: reference/value of (if operand is pure)
        dir::Expression::BorrowOf { right, .. } | dir::Expression::MoveOf { right, .. } => {
            expression_has_side_effects(ctx, *right)
        }

        // side effects: calls, assignments, new, await, yield, etc
        dir::Expression::Call { .. }
        | dir::Expression::Assign { .. }
        | dir::Expression::New { .. }
        | dir::Expression::Await { .. }
        | dir::Expression::AwaitMaybe { .. }
        | dir::Expression::AwaitMust { .. }
        | dir::Expression::Yield { .. }
        | dir::Expression::Throw { .. } => true,

        // side effects: control flow
        dir::Expression::Return { .. }
        | dir::Expression::Break { .. }
        | dir::Expression::Continue { .. }
        | dir::Expression::For { .. }
        | dir::Expression::ForEach { .. }
        | dir::Expression::While { .. }
        | dir::Expression::Loop { .. }
        | dir::Expression::If { .. }
        | dir::Expression::Match { .. }
        | dir::Expression::Try { .. } => true,

        // side effects: declarations, imports, exports
        dir::Expression::Declaration(_)
        | dir::Expression::Block(_)
        | dir::Expression::Let { .. }
        | dir::Expression::LetElse { .. }
        | dir::Expression::Using { .. }
        | dir::Expression::Import { .. }
        | dir::Expression::Export { .. }
        | dir::Expression::Label { .. } => true,

        // side effects: debugger, error, stub
        dir::Expression::Debugger
        | dir::Expression::Error
        | dir::Expression::Missing
        | dir::Expression::Stub => true,

        // wrapped expressions: check inner
        dir::Expression::Parenthesized { expression } => {
            expression_has_side_effects(ctx, *expression)
        }

        // maybe/must propagation: check inner for side effect
        dir::Expression::Maybe { left, .. } | dir::Expression::Must { left, .. } => {
            expression_has_side_effects(ctx, *left)
        }

        // templates: conservatively assume side effects (could have interpolations with calls)
        dir::Expression::TemplateExpression { .. }
        | dir::Expression::TaggedTemplateExpression { .. } => true,

        // object expressions: check properties for side effects
        dir::Expression::ObjectExpression { .. }
        | dir::Expression::StructExpression { .. }
        | dir::Expression::TreeExpression { .. }
        | dir::Expression::SequenceExpression { .. } => true,

        // comptime: check if body has side effects
        dir::Expression::Comptime { body } => expression_has_side_effects(ctx, *body),
    }
}

/// Return whether one type expression subtree contains side effects.
pub fn type_expression_has_side_effects(
    ctx: &LintModuleContext<'_>,
    type_expression_id: dir::LocalNodeId<dir::TypeExpression>,
) -> bool {
    let type_expression = ctx.dir.get(type_expression_id);

    match type_expression {
        // pure leaves
        dir::TypeExpression::ScalarLiteral { value: _ }
        | dir::TypeExpression::Literal { .. }
        | dir::TypeExpression::Intrinsic
        | dir::TypeExpression::Const
        | dir::TypeExpression::This
        | dir::TypeExpression::Missing
        | dir::TypeExpression::Error => false,

        // wrapped type expressions
        dir::TypeExpression::Parenthesized { expression }
        | dir::TypeExpression::Readonly {
            target_type: expression,
        }
        | dir::TypeExpression::Shared {
            target_type: expression,
        }
        | dir::TypeExpression::KeyOf {
            target_type: expression,
        }
        | dir::TypeExpression::Must {
            target_type: expression,
        }
        | dir::TypeExpression::AsComptime {
            target_type: expression,
        }
        | dir::TypeExpression::Not {
            target_type: expression,
        }
        | dir::TypeExpression::OwnedOf {
            target_type: expression,
            ..
        }
        | dir::TypeExpression::BorrowedOf {
            target_type: expression,
            ..
        }
        | dir::TypeExpression::PointerOf {
            target_type: expression,
            ..
        } => type_expression_has_side_effects(ctx, *expression),

        // mostly pure type forms
        dir::TypeExpression::Tuple { .. }
        | dir::TypeExpression::ArrayTuple { .. }
        | dir::TypeExpression::Array { .. }
        | dir::TypeExpression::Slice { .. }
        | dir::TypeExpression::FixedArray { .. }
        | dir::TypeExpression::Object { .. }
        | dir::TypeExpression::Declaration { .. }
        | dir::TypeExpression::FunctionTypeDeclaration(_)
        | dir::TypeExpression::ConstructorTypeDeclaration(_)
        | dir::TypeExpression::Reference { .. }
        | dir::TypeExpression::Infer { .. }
        | dir::TypeExpression::Predicate { .. } => false,

        // type expressions that contain runtime expressions
        dir::TypeExpression::TypeOfValue { value } => expression_has_side_effects(ctx, *value),

        // composite type expressions
        dir::TypeExpression::Member {
            left,
            generic_arguments,
            ..
        } => {
            type_expression_has_side_effects(ctx, *left)
                || generic_arguments.iter().any(|argument_id| {
                    let argument = ctx.dir.get(*argument_id);
                    match argument {
                        dir::GenericArgument::Type { value } => {
                            type_expression_has_side_effects(ctx, *value)
                        }
                        dir::GenericArgument::Value { value } => {
                            expression_has_side_effects(ctx, *value)
                        }
                        dir::GenericArgument::Error => true,
                    }
                })
        }
        dir::TypeExpression::Union { elements }
        | dir::TypeExpression::Intersection { elements } => elements
            .iter()
            .any(|element_id| type_expression_has_side_effects(ctx, *element_id)),
        dir::TypeExpression::Range { start, end, .. } => {
            start.is_some_and(|start| type_expression_has_side_effects(ctx, start))
                || end.is_some_and(|end| type_expression_has_side_effects(ctx, end))
        }
        dir::TypeExpression::Conditional {
            left,
            extends_type,
            then_type,
            else_type,
        } => {
            type_expression_has_side_effects(ctx, *left)
                || type_expression_has_side_effects(ctx, *extends_type)
                || type_expression_has_side_effects(ctx, *then_type)
                || type_expression_has_side_effects(ctx, *else_type)
        }
        dir::TypeExpression::In { left, right }
        | dir::TypeExpression::Extends { left, right }
        | dir::TypeExpression::Implements { left, right } => {
            type_expression_has_side_effects(ctx, *left)
                || type_expression_has_side_effects(ctx, *right)
        }
        dir::TypeExpression::Mapped {
            parameter, value, ..
        } => {
            let parameter = ctx.dir.get(*parameter);
            type_expression_has_side_effects(ctx, parameter.source_type)
                || parameter
                    .key_remap
                    .is_some_and(|key_remap| type_expression_has_side_effects(ctx, key_remap))
                || value.is_some_and(|value| type_expression_has_side_effects(ctx, value))
        }
        dir::TypeExpression::Index { left, index } => {
            type_expression_has_side_effects(ctx, *left)
                || type_expression_has_side_effects(ctx, *index)
        }
        dir::TypeExpression::TemplateLiteral { spans, .. } => spans
            .iter()
            .any(|span_id| type_expression_has_side_effects(ctx, *span_id)),
    }
}

/// Check if an operator is a comparison operator.
pub fn is_comparison_operator(operator: &dir::BinaryOperator) -> bool {
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

/// Check if an expression is a literal value (scalar or type literal).
pub fn expression_is_literal(expression: &dir::Expression) -> bool {
    match expression {
        dir::Expression::ScalarLiteral(_) => true,
        dir::Expression::Type { .. } => false,
        _ => false,
    }
}

/// Check if an expression is a constant expression (evaluates to a fixed value at compile time).
pub fn expression_is_constant_expression(
    ctx: &LintModuleContext<'_>,
    expr: &dir::Expression,
) -> bool {
    match expr {
        dir::Expression::ScalarLiteral(_) => true,
        dir::Expression::Type { value } => type_expression_is_constant(ctx, *value),
        dir::Expression::Parenthesized { expression } => {
            expression_is_constant_expression(ctx, ctx.dir.get(*expression))
        }
        dir::Expression::Unary { right, .. } => {
            expression_is_constant_expression(ctx, ctx.dir.get(*right))
        }
        _ => false,
    }
}

/// Evaluate a constant expression to a boolean value if possible.
///
/// Returns Some(true) for truthy constants, Some(false) for falsy constants, None otherwise.
/// Handles booleans, integers, floats, null/undefined, parenthesized expressions, and unary not.
pub fn expression_constant_to_bool(
    ctx: &LintModuleContext<'_>,
    expr: &dir::Expression,
) -> Option<bool> {
    match expr {
        dir::Expression::ScalarLiteral(lit) => match lit {
            dir::ScalarLiteral::Boolean(b) => Some(*b),
            dir::ScalarLiteral::Integer(value) => Some(*value != 0),
            dir::ScalarLiteral::Float(value) => Some(*value != 0.0),
            dir::ScalarLiteral::Bigint(value) => Some(*value != 0),
            _ => None,
        },
        dir::Expression::Type { value } => type_expression_constant_to_bool(ctx, *value),
        dir::Expression::Parenthesized { expression } => {
            expression_constant_to_bool(ctx, ctx.dir.get(*expression))
        }
        dir::Expression::Unary { operator, right } => {
            if *operator == dir::UnaryOperator::Not {
                expression_constant_to_bool(ctx, ctx.dir.get(*right)).map(|b| !b)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Return true when one type expression is constant.
fn type_expression_is_constant(
    ctx: &LintModuleContext<'_>,
    type_expression_id: dir::LocalNodeId<dir::TypeExpression>,
) -> bool {
    let type_expression = ctx.dir.get(type_expression_id);

    match type_expression {
        dir::TypeExpression::Parenthesized { expression } => {
            type_expression_is_constant(ctx, *expression)
        }
        dir::TypeExpression::ScalarLiteral { value: _ }
        | dir::TypeExpression::Literal { .. }
        | dir::TypeExpression::Intrinsic
        | dir::TypeExpression::Const
        | dir::TypeExpression::This => true,
        _ => false,
    }
}

/// Return the boolean value of one constant type expression when known.
fn type_expression_constant_to_bool(
    ctx: &LintModuleContext<'_>,
    type_expression_id: dir::LocalNodeId<dir::TypeExpression>,
) -> Option<bool> {
    let type_expression = ctx.dir.get(type_expression_id);

    match type_expression {
        dir::TypeExpression::Parenthesized { expression } => {
            type_expression_constant_to_bool(ctx, *expression)
        }
        dir::TypeExpression::ScalarLiteral { value } => match value {
            dir::ScalarLiteral::Boolean(value) => Some(*value),
            dir::ScalarLiteral::Integer(value) => Some(*value != 0),
            dir::ScalarLiteral::Float(value) => Some(*value != 0.0),
            dir::ScalarLiteral::Bigint(value) => Some(*value != 0),
            _ => None,
        },
        dir::TypeExpression::Literal { value } => match value {
            dir::TypeLiteral::Null | dir::TypeLiteral::Undefined => Some(false),
            _ => None,
        },
        _ => None,
    }
}

/// Compare two expressions using one structural signature.
fn expression_signature_equal(
    ctx: &LintModuleContext<'_>,
    left_id: dir::LocalNodeId<dir::Expression>,
    right_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let left_signature = expression_signature(ctx, left_id);
    let right_signature = expression_signature(ctx, right_id);
    left_signature == right_signature
}

/// Build a structural signature for one expression subtree.
fn expression_signature(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Vec<u64> {
    expression_signature_for_tree(ctx.dir.tree(), ctx.strings, expression_id)
}

/// Build a structural signature for one expression subtree.
pub fn expression_signature_for_tree(
    tree: &dir::Tree,
    strings: &dir::StringPool,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Vec<u64> {
    let expression = tree.get(expression_id);
    let mut collector = ExpressionSignatureCollector::new(strings);
    dir::NodeVisitor::visit_expression(&mut collector, tree, expression_id, expression);
    collector.finish()
}

/// Collect a structural signature for an expression subtree.
struct ExpressionSignatureCollector<'a> {
    /// String pool for identifier and literal names.
    strings: &'a dir::StringPool,
    /// Visitor options.
    visitor_options: dir::NodeVisitorOptions,
    /// Signature token stream.
    tokens: Vec<u64>,
}

impl<'a> ExpressionSignatureCollector<'a> {
    /// Build an empty signature collector.
    fn new(strings: &'a dir::StringPool) -> Self {
        Self {
            strings,
            visitor_options: dir::NodeVisitorOptions::default(),
            tokens: Vec::new(),
        }
    }

    /// Finalize and return the signature tokens.
    fn finish(self) -> Vec<u64> {
        self.tokens
    }

    /// Push one key-value token.
    fn push_token(&mut self, key: &str, value: &str) {
        self.tokens.push(hash_signature_token(key, value));
    }

    /// Push one debug token value.
    fn push_debug<T: std::fmt::Debug>(&mut self, key: &str, value: T) {
        let value_hash = stable_hash_debug(&value);
        self.tokens
            .push(hash_signature_token_hashed_value(key, value_hash));
    }

    /// Push one string id as text.
    fn push_string_id(&mut self, key: &str, string_id: dir::StringId) {
        self.push_token(key, self.strings.get(string_id));
    }
}

impl dir::NodeVisitor for ExpressionSignatureCollector<'_> {
    /// Return visitor options.
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.visitor_options
    }

    /// Visit any source node and record its node type.
    fn visit_any(&mut self, _tree: &dir::Tree, ty: dir::NodeType, _id: u32) {
        self.push_debug("node", ty);
    }

    /// Visit one expression node and record expression specific signature tokens.
    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        self.push_debug("expression", std::mem::discriminant(expression));
        match expression {
            dir::Expression::Label { label, .. } => {
                self.push_string_id("expression_label", *label);
            }
            dir::Expression::Import { form, target, .. } => {
                self.push_debug("expression_import_form", *form);
                self.push_string_id("expression_import_target", *target);
            }
            dir::Expression::Export { form, target, .. } => {
                self.push_debug("expression_export_form", *form);
                if let Some(target) = target {
                    self.push_string_id("expression_export_target", *target);
                }
            }
            dir::Expression::If { form, .. } => {
                self.push_debug("expression_if_form", *form);
            }
            dir::Expression::While { form, .. } => {
                self.push_debug("expression_while_form", *form);
            }
            dir::Expression::ForEach {
                asynchrony,
                operator,
                ..
            } => {
                self.push_debug("expression_foreach_asynchrony", *asynchrony);
                self.push_debug("expression_foreach_operator", *operator);
            }
            dir::Expression::Match { form, .. } => {
                self.push_debug("expression_match_form", *form);
            }
            dir::Expression::Break { label, value } => {
                self.push_debug("expression_break_has_label", label.is_some());
                self.push_debug("expression_break_has_value", value.is_some());
                if let Some(label) = label {
                    self.push_string_id("expression_break_label", *label);
                }
            }
            dir::Expression::Continue { label } => {
                self.push_debug("expression_continue_has_label", label.is_some());
                if let Some(label) = label {
                    self.push_string_id("expression_continue_label", *label);
                }
            }
            dir::Expression::Yield { cardinality, .. } => {
                self.push_debug("expression_yield_cardinality", *cardinality);
            }
            dir::Expression::Identifier { name } => {
                self.push_string_id("expression_identifier", *name);
            }
            dir::Expression::QualifiedReference { path, .. } => {
                self.push_debug("expression_path_len", path.segments.len());
                for segment in &path.segments {
                    self.push_string_id("expression_path_segment", *segment);
                }
            }
            dir::Expression::PrivateIdentifier { name } => {
                self.push_string_id("expression_private_identifier", *name);
            }
            dir::Expression::ScalarLiteral(literal) => {
                self.push_debug("expression_scalar_literal", literal);
            }
            dir::Expression::Type { .. } => {
                self.push_debug("expression_type", true);
            }
            dir::Expression::TemplateExpression { value }
            | dir::Expression::TaggedTemplateExpression { value, .. } => {
                self.push_debug("expression_template", value);
            }
            dir::Expression::Unary { operator, .. } => {
                self.push_debug("expression_unary", *operator);
            }
            dir::Expression::MoveOf {
                mutability,
                variance,
                ..
            } => {
                self.push_debug("expression_valueof_mutability", *mutability);
                self.push_debug("expression_valueof_variance", *variance);
            }
            dir::Expression::BorrowOf {
                mutability,
                variance,
                ..
            } => {
                self.push_debug("expression_referenceof_mutability", *mutability);
                self.push_debug("expression_referenceof_variance", *variance);
            }
            dir::Expression::Member { name, .. } | dir::Expression::PrivateMember { name, .. } => {
                if let Some(name) = name {
                    self.push_string_id("expression_member", *name);
                }
            }
            dir::Expression::Index { position, .. }
            | dir::Expression::Call { position, .. }
            | dir::Expression::Maybe { position, .. }
            | dir::Expression::Must { position, .. } => {
                self.push_debug("expression_postfix_position", *position);
            }
            dir::Expression::Binary { operator, .. } => {
                self.push_debug("expression_binary", *operator);
            }
            dir::Expression::Assign { operator, .. } => {
                self.push_debug("expression_assign", *operator);
            }
            _ => {}
        }

        dir::walk_expression(self, tree, id, expression);
    }

    /// Visit one type expression node and record type specific signature tokens.
    fn visit_type_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeExpression>,
        type_expression: &dir::TypeExpression,
    ) {
        self.push_debug("type_expression", std::mem::discriminant(type_expression));
        match type_expression {
            dir::TypeExpression::ScalarLiteral { value } => {
                self.push_debug("type_expression_scalar_literal", value);
            }
            dir::TypeExpression::Literal { value } => {
                self.push_debug("type_expression_literal", value);
            }
            dir::TypeExpression::Reference {
                path,
                generic_arguments,
            } => {
                self.push_debug("type_expression_path_len", path.segments.len());
                self.push_debug(
                    "type_expression_has_generic_arguments",
                    !generic_arguments.is_empty(),
                );
                for segment in &path.segments {
                    self.push_string_id("type_expression_path_segment", *segment);
                }
            }
            dir::TypeExpression::Member {
                name,
                generic_arguments,
                ..
            } => {
                self.push_string_id("type_expression_member", *name);
                self.push_debug(
                    "type_expression_has_generic_arguments",
                    !generic_arguments.is_empty(),
                );
            }
            dir::TypeExpression::Mapped {
                parameter,
                readonly,
                optional,
                ..
            } => {
                let parameter = tree.get(*parameter);
                self.push_string_id("type_expression_mapped_name", parameter.name);
                self.push_debug("type_expression_mapped_readonly", *readonly);
                self.push_debug("type_expression_mapped_optional", *optional);
            }
            dir::TypeExpression::TemplateLiteral { strings, .. } => {
                self.push_debug("type_expression_template_len", strings.len());
                for string in strings {
                    self.push_string_id("type_expression_template_string", *string);
                }
            }
            dir::TypeExpression::Infer { name, .. } => {
                if let Some(name) = name {
                    self.push_string_id("type_expression_infer", *name);
                }
            }
            dir::TypeExpression::Predicate {
                asserts, subject, ..
            } => {
                self.push_debug("type_expression_predicate_asserts", *asserts);
                self.push_debug("type_expression_predicate_subject", subject);
            }
            _ => {}
        }

        dir::walk_type_expression(self, tree, id, type_expression);
    }

    /// Visit one declaration node and record declaration signature tokens.
    fn visit_declaration(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) {
        self.push_debug("declaration", std::mem::discriminant(declaration));
        dir::walk_declaration(self, tree, id, declaration);
    }

    /// Visit one property node and record property signature tokens.
    fn visit_property(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Property>,
        property: &dir::Property,
    ) {
        self.push_debug("property", std::mem::discriminant(property));
        dir::walk_property(self, tree, id, property);
    }

    /// Visit one member node and record member signature tokens.
    fn visit_member(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Member>,
        member: &dir::Member,
    ) {
        self.push_debug("member", std::mem::discriminant(member));
        dir::walk_member(self, tree, id, member);
    }

    /// Visit one parameter node and record parameter signature tokens.
    fn visit_parameter(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Parameter>,
        parameter: &dir::Parameter,
    ) {
        self.push_debug("parameter", std::mem::discriminant(parameter));
        dir::walk_parameter(self, tree, id, parameter);
    }

    /// Visit one argument node and record argument signature tokens.
    fn visit_argument(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Argument>,
        argument: &dir::Argument,
    ) {
        self.push_debug("argument", std::mem::discriminant(argument));
        dir::walk_argument(self, tree, id, argument);
    }

    /// Visit one pattern node and record pattern signature tokens.
    fn visit_pattern(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Pattern>,
        pattern: &dir::Pattern,
    ) {
        self.push_debug("pattern", std::mem::discriminant(pattern));
        dir::walk_pattern(self, tree, id, pattern);
    }

    /// Visit one pattern field node and record field signature tokens.
    fn visit_pattern_field(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::PatternField>,
        pattern_field: &dir::PatternField,
    ) {
        self.push_debug("pattern_field", std::mem::discriminant(pattern_field));
        dir::walk_pattern_field(self, tree, id, pattern_field);
    }

    /// Visit one match case node and record case signature tokens.
    fn visit_match_case(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::MatchCase>,
        match_case: &dir::MatchCase,
    ) {
        self.push_debug("match_case", std::mem::discriminant(match_case));
        dir::walk_match_case(self, tree, id, match_case);
    }
}

/// Hash one expression signature token.
fn hash_signature_token(key: &str, value: &str) -> u64 {
    stable_hash_token(key, value)
}

/// Hash one expression signature token from prehashed value bytes.
fn hash_signature_token_hashed_value(key: &str, value_hash: u64) -> u64 {
    stable_hash_token_hashed_value(key, value_hash)
}
