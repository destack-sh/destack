use destack_ast::{self as ast, NodeVisitor};
use destack_source::Span;

use crate::rules::common::{
    span_has_comment, stable_hash_debug, stable_hash_token, stable_hash_token_hashed_value,
};
use crate::{ConstValue, LintAstContext};

/// Return the AST expression id with parenthesized source form unwrapped.
pub fn expression_unwrap_parenthesized_source_form(
    tree: &ast::NodeTree,
    mut expression_id: ast::LocalNodeId<ast::Expression>,
) -> ast::LocalNodeId<ast::Expression> {
    // follow parenthesized wrappers until a non parenthesized expression is found
    loop {
        let expression = tree.get(expression_id);
        let ast::Expression::Parenthesized { expression } = expression else {
            return expression_id;
        };

        expression_id = *expression;
    }
}

/// Return the AST expression id with statement source form unwrapped.
pub fn expression_unwrap_statement_source_form(
    tree: &ast::NodeTree,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> ast::LocalNodeId<ast::Expression> {
    let _ = tree;
    expression_id
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
    tree: &ast::NodeTree,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> ConditionAssignmentStyle {
    // inspect the outer condition expression
    let expression = tree.get(expression_id);
    match expression {
        // let and using expressions are valid binding conditions
        ast::Expression::Let { .. } | ast::Expression::Using { .. } => {
            ConditionAssignmentStyle::None
        }
        ast::Expression::Assign { .. } => ConditionAssignmentStyle::Bare,
        ast::Expression::Parenthesized { expression } => {
            // single parens around assignment are still ambiguous
            let inner_expression = tree.get(*expression);
            if matches!(inner_expression, ast::Expression::Assign { .. }) {
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
    expression: &ast::Expression,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    match expression {
        ast::Expression::If { condition, .. } => match condition {
            ast::IfCondition::Expression { condition } => Some(*condition),
            ast::IfCondition::Let { .. } => None,
        },
        ast::Expression::While { condition, .. } => Some(*condition),
        ast::Expression::For {
            condition: Some(condition),
            ..
        } => Some(*condition),
        _ => None,
    }
}

/// Return the outer expression id including parenthesized source form.
pub fn expression_outer_parenthesized_source_form(
    tree: &ast::NodeTree,
    parents: &ast::NodeParentIndex,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> ast::LocalNodeId<ast::Expression> {
    // start from one normalized inner expression
    let mut current_id = expression_unwrap_parenthesized_source_form(tree, expression_id);

    // climb through direct parenthesized wrappers
    loop {
        let Some(parent_id) = parents.get(current_id) else {
            return current_id;
        };
        if tree.get_node_type(parent_id) != ast::NodeType::Expression {
            return current_id;
        }

        let parent_expression_id = ast::LocalNodeId::<ast::Expression>::new(parent_id);
        let parent_expression = tree.get(parent_expression_id);
        if let ast::Expression::Parenthesized { expression } = parent_expression
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
    tree: &ast::NodeTree,
    parents: &ast::NodeParentIndex,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    // start from the outer source form
    let mut current_id = expression_outer_parenthesized_source_form(tree, parents, expression_id);

    // root expressions are statement-position by default
    if parents.get(current_id).is_none() {
        return Some(current_id);
    }

    // walk out through statement-like wrappers until one block boundary is found
    loop {
        let parent_id = parents.get(current_id)?;

        if tree.get_node_type(parent_id) == ast::NodeType::Expression {
            let parent_expression_id = ast::LocalNodeId::<ast::Expression>::new(parent_id);
            let parent_expression = tree.get(parent_expression_id);

            if let ast::Expression::Labelled { body, .. } = parent_expression
                && *body == current_id
            {
                current_id = parent_expression_id;
                continue;
            }

            return None;
        }

        if tree.get_node_type(parent_id) == ast::NodeType::Block {
            let block_id = ast::LocalNodeId::<ast::Block>::new(parent_id);
            let block = tree.get(block_id);

            if block.context == ast::BlockContext::Statement
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
    tree: &ast::NodeTree,
    parents: &ast::NodeParentIndex,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<Span> {
    // resolve the enclosing statement wrapper
    let statement_expression_id = expression_statement_ancestor(tree, parents, expression_id)?;

    // return the statement span
    Some(tree.get_span(statement_expression_id))
}

/// Return true when one expression is the direct child of a statement wrapper.
pub fn expression_is_direct_statement(
    tree: &ast::NodeTree,
    parents: &ast::NodeParentIndex,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let outer_expression_id =
        expression_outer_parenthesized_source_form(tree, parents, expression_id);
    expression_statement_ancestor(tree, parents, expression_id) == Some(outer_expression_id)
}

/// Return true when one expression is a direct leading expression in a block.
pub fn expression_is_direct_block_leading_expression(
    tree: &ast::NodeTree,
    parents: &ast::NodeParentIndex,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    // normalize outer source form first
    let outer_expression_id =
        expression_outer_parenthesized_source_form(tree, parents, expression_id);
    let Some(parent_id) = parents.get(outer_expression_id) else {
        return false;
    };

    // require one enclosing block
    if tree.get_node_type(parent_id) != ast::NodeType::Block {
        return false;
    }

    // keep only leading block expressions, not value tails
    let block_id = ast::LocalNodeId::<ast::Block>::new(parent_id);
    let block = tree.get(block_id);

    block
        .leading_expressions
        .iter()
        .copied()
        .any(|child_id| child_id == outer_expression_id)
}

/// Return true when one expression can safely start an expression statement.
pub fn expression_can_start_expression_statement(expression: &ast::Expression) -> bool {
    matches!(
        expression,
        ast::Expression::Identifier { .. }
            | ast::Expression::QualifiedReference { .. }
            | ast::Expression::Member { .. }
            | ast::Expression::Index { .. }
            | ast::Expression::Call { .. }
            | ast::Expression::New { .. }
            | ast::Expression::Await { .. }
            | ast::Expression::Unary { .. }
            | ast::Expression::Maybe { .. }
            | ast::Expression::Must { .. }
            | ast::Expression::ScalarLiteral(_)
            | ast::Expression::Type { .. }
            | ast::Expression::TemplateExpression { .. }
            | ast::Expression::TaggedTemplateExpression { .. }
    )
}

/// Return the trailing non null assertion span when one expression text ends with `!`.
pub fn expression_trailing_bang_span(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<Span> {
    // resolve one text slice for the expression span
    let expression_span = ctx.tree.get_span(expression_id);
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
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> String {
    // keep the original source text for replacement fidelity
    let expression_span = ctx.tree.get_span(expression_id);
    let expression_text = ctx.get_span_text(expression_span);

    // normalize the expression shape for precedence checks
    let normalized_expression_id =
        expression_unwrap_parenthesized_source_form(ctx.tree, expression_id);
    let normalized_expression = ctx.tree.get(normalized_expression_id);

    // preserve precedence for non-atomic expressions
    if expression_needs_parentheses_for_prefix_not(normalized_expression) {
        return format!("!({expression_text})");
    }

    format!("!{expression_text}")
}

/// Return true when one expression needs parentheses under prefix `!`.
fn expression_needs_parentheses_for_prefix_not(expression: &ast::Expression) -> bool {
    !expression_can_start_expression_statement(expression)
        && !matches!(expression, ast::Expression::Parenthesized { .. })
}

/// Return true when one expression starts a nested declaration scope.
pub fn expression_starts_nested_declaration_scope(expression: &ast::Expression) -> bool {
    matches!(expression, ast::Expression::Declaration(_))
}

/// Return true when one declaration expression is immediately invoked.
pub fn expression_is_immediately_invoked(
    tree: &ast::NodeTree,
    parents: &ast::NodeParentIndex,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let mut current_expression_id = expression_id;

    loop {
        // resolve one expression parent
        let Some(parent_id) = parents.get(current_expression_id) else {
            return false;
        };
        if tree.get_node_type(parent_id) != ast::NodeType::Expression {
            return false;
        }

        // keep climbing through parenthesized wrappers
        let parent_expression_id = ast::LocalNodeId::<ast::Expression>::new(parent_id);
        let parent_expression = tree.get(parent_expression_id);
        match parent_expression {
            ast::Expression::Parenthesized { expression }
                if *expression == current_expression_id =>
            {
                current_expression_id = parent_expression_id;
            }
            ast::Expression::Call { left, .. } | ast::Expression::New { left, .. } => {
                return *left == current_expression_id;
            }
            _ => return false,
        }
    }
}

/// Return true when one expression subtree contains an assignment expression.
pub fn expression_contains_assignment(
    tree: &ast::NodeTree,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    // visit one expression subtree and stop once assignment is found
    let expression = tree.get(expression_id);
    let mut visitor = AssignmentSearchVisitor {
        options: ast::NodeVisitorOptions::default(),
        found_assignment: false,
    };
    visitor.visit_expression(tree, expression_id, expression);

    visitor.found_assignment
}

/// Return true when one expression subtree mentions this identifier name.
pub fn expression_subtree_mentions_identifier_name(
    tree: &ast::NodeTree,
    expression_id: ast::LocalNodeId<ast::Expression>,
    name: ast::StringId,
) -> bool {
    subtree_mentions_identifier_name(tree, ast::NodeType::Expression, expression_id.id, name)
}

/// Return true when one AST subtree mentions this identifier name.
pub fn subtree_mentions_identifier_name(
    tree: &ast::NodeTree,
    node_type: ast::NodeType,
    node_id: u32,
    name: ast::StringId,
) -> bool {
    // walk only the relevant subtree
    let mut visitor = IdentifierNameSearchVisitor {
        options: ast::NodeVisitorOptions::default(),
        name,
        found_name: false,
    };
    ast::walk_any(&mut visitor, tree, node_type, node_id);

    visitor.found_name
}

/// Return true when one expression is the target of optional chaining.
pub fn expression_is_optional_chain_target(
    tree: &ast::NodeTree,
    parents: &ast::NodeParentIndex,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    // require one expression parent
    let Some(parent_id) = parents.get(expression_id) else {
        return false;
    };
    if tree.get_node_type(parent_id) != ast::NodeType::Expression {
        return false;
    }

    // keep optional chain wrappers that reference this expression directly
    let parent_expression_id = ast::LocalNodeId::<ast::Expression>::new(parent_id);
    let parent_expression = tree.get(parent_expression_id);
    matches!(
        parent_expression,
        ast::Expression::Maybe {
            position: ast::PostfixPosition::Indirect,
            left,
        } if *left == expression_id
    )
}

/// One normalized `if` branch chain.
#[derive(Debug)]
pub struct IfBranchChain {
    /// Branch body expressions in source order.
    pub branch_expressions: Vec<ast::LocalNodeId<ast::Expression>>,
    /// Whether the chain ends with an explicit `else` branch.
    pub ends_with_else: bool,
}

/// Return true when an `if` expression is an `else if` child branch.
pub fn expression_is_else_if_branch(
    tree: &ast::NodeTree,
    parents: &ast::NodeParentIndex,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    // require an expression parent
    let Some(parent_id) = parents.get(expression_id) else {
        return false;
    };
    if tree.get_node_type(parent_id) != ast::NodeType::Expression {
        return false;
    }

    // keep parent `if` expressions that reference this node as their `else`
    let parent_expression_id = ast::LocalNodeId::<ast::Expression>::new(parent_id);
    let parent_expression = tree.get(parent_expression_id);
    matches!(
        parent_expression,
        ast::Expression::If {
            else_expression: Some(else_expression_id),
            ..
        } if *else_expression_id == expression_id
    )
}

/// Return one normalized branch chain for an `if` expression.
pub fn if_expression_branch_chain(
    tree: &ast::NodeTree,
    if_expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<IfBranchChain> {
    // require one `if` expression entry point
    let mut current_if_id = if_expression_id;
    let mut branch_expressions = Vec::new();

    loop {
        let current_expression = tree.get(current_if_id);
        let ast::Expression::If {
            kind: _,
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
            ast::Expression::If {
                kind: ast::IfKind::If,
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
    tree: &ast::NodeTree,
    expression_id: ast::LocalNodeId<ast::Expression>,
    name: ast::StringId,
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
    tree: &ast::NodeTree,
    parents: &ast::NodeParentIndex,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    // start from one normalized expression id
    let mut current_id = expression_unwrap_parenthesized_source_form(tree, expression_id).id;

    // climb ancestors until one type annotation slot is found
    while let Some(parent_id) = parents.get_by_id(current_id) {
        let parent_type = tree.get_node_type(parent_id);

        // check declarator annotation slots
        if parent_type == ast::NodeType::Declarator {
            let declarator_id = ast::LocalNodeId::<ast::Declarator>::new(parent_id);
            let declarator = tree.get(declarator_id);
            if declarator.ty.is_some_and(|ty_id| ty_id.id == current_id) {
                return true;
            }
        }

        // check parameter annotation slots
        if parent_type == ast::NodeType::Parameter {
            let parameter_id = ast::LocalNodeId::<ast::Parameter>::new(parent_id);
            let parameter = tree.get(parameter_id);
            let parameter_type = match parameter {
                ast::Parameter::Named { declared_type, .. }
                | ast::Parameter::Pattern { declared_type, .. }
                | ast::Parameter::VariadicNamed { declared_type, .. }
                | ast::Parameter::VariadicPattern { declared_type, .. } => *declared_type,
                ast::Parameter::Error => None,
            };
            if parameter_type.is_some_and(|ty_id| ty_id.id == current_id) {
                return true;
            }
        }

        // check declaration type expression slots
        if parent_type == ast::NodeType::Declaration {
            let declaration_id = ast::LocalNodeId::<ast::Declaration>::new(parent_id);
            let declaration = tree.get(declaration_id);
            match declaration {
                ast::Declaration::Type(declaration) => {
                    if declaration.value.id == current_id {
                        return true;
                    }
                }
                ast::Declaration::Function(declaration) => {
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
        if parent_type == ast::NodeType::Member {
            let member_id = ast::LocalNodeId::<ast::Member>::new(parent_id);
            let member = tree.get(member_id);
            match member {
                ast::Member::AssociatedType {
                    constraint, value, ..
                } => {
                    if constraint.is_some_and(|ty_id| ty_id.id == current_id)
                        || value.is_some_and(|value_id| value_id.id == current_id)
                    {
                        return true;
                    }
                }
                ast::Member::AssociatedConst { declared_type, .. } => {
                    if declared_type.is_some_and(|ty_id| ty_id.id == current_id) {
                        return true;
                    }
                }
                ast::Member::Field { declared_type, .. } => {
                    if declared_type.is_some_and(|value_id| value_id.id == current_id) {
                        return true;
                    }
                }
                ast::Member::Method { signature, .. } => {
                    if signature
                        .return_type
                        .is_some_and(|return_type_id| return_type_id.id == current_id)
                    {
                        return true;
                    }
                }
                ast::Member::Embed { value, .. } => {
                    if value.id == current_id {
                        return true;
                    }
                }
                _ => {}
            }
        }

        // check property method return types
        if parent_type == ast::NodeType::Property {
            let property_id = ast::LocalNodeId::<ast::Property>::new(parent_id);
            let property = tree.get(property_id);
            if let ast::Property::Method { signature, .. } = property
                && signature
                    .return_type
                    .is_some_and(|return_type_id| return_type_id.id == current_id)
            {
                return true;
            }
        }

        // check where clause type slots
        if parent_type == ast::NodeType::WhereClause {
            let where_clause_id = ast::LocalNodeId::<ast::WhereClause>::new(parent_id);
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
    options: ast::NodeVisitorOptions,
    /// Whether an assignment node has been encountered.
    found_assignment: bool,
}

impl ast::NodeVisitor for AssignmentSearchVisitor {
    fn options(&self) -> &ast::NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &ast::NodeTree,
        expression_id: ast::LocalNodeId<ast::Expression>,
        expression: &ast::Expression,
    ) {
        // stop traversal after first assignment match
        if self.found_assignment {
            return;
        }

        // record assignment match and stop descending
        if matches!(expression, ast::Expression::Assign { .. }) {
            self.found_assignment = true;
            return;
        }

        ast::walk_expression(self, tree, expression_id, expression);
    }
}

/// Visitor that tracks whether one expression subtree mentions a name.
struct IdentifierNameSearchVisitor {
    /// Visitor options.
    options: ast::NodeVisitorOptions,
    /// The target identifier name.
    name: ast::StringId,
    /// Whether a matching mention has been found.
    found_name: bool,
}

impl ast::NodeVisitor for IdentifierNameSearchVisitor {
    fn options(&self) -> &ast::NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &ast::NodeTree,
        expression_id: ast::LocalNodeId<ast::Expression>,
        expression: &ast::Expression,
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

        ast::walk_expression(self, tree, expression_id, expression);
    }

    fn visit_declaration(
        &mut self,
        tree: &ast::NodeTree,
        declaration_id: ast::LocalNodeId<ast::Declaration>,
        declaration: &ast::Declaration,
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

        ast::walk_declaration(self, tree, declaration_id, declaration);
    }

    fn visit_parameter(
        &mut self,
        tree: &ast::NodeTree,
        parameter_id: ast::LocalNodeId<ast::Parameter>,
        parameter: &ast::Parameter,
    ) {
        // stop once the target name has been found
        if self.found_name {
            return;
        }

        // match named parameters before descending
        let parameter_name = match parameter {
            ast::Parameter::Named { name, .. } | ast::Parameter::VariadicNamed { name, .. } => {
                Some(*name)
            }
            _ => None,
        };
        if parameter_name == Some(self.name) {
            self.found_name = true;
            return;
        }

        ast::walk_parameter(self, tree, parameter_id, parameter);
    }

    fn visit_pattern(
        &mut self,
        tree: &ast::NodeTree,
        pattern_id: ast::LocalNodeId<ast::Pattern>,
        pattern: &ast::Pattern,
    ) {
        // stop once the target name has been found
        if self.found_name {
            return;
        }

        // match binding patterns before descending
        if let ast::Pattern::Binding { name, .. } = pattern
            && *name == self.name
        {
            self.found_name = true;
            return;
        }

        ast::walk_pattern(self, tree, pattern_id, pattern);
    }
}

/// Return true when one block has no expressions and no comment trivia.
pub fn block_is_empty_without_comment(
    tree: &ast::NodeTree,
    block_id: ast::LocalNodeId<ast::Block>,
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
    tree: &ast::NodeTree,
    parents: &ast::NodeParentIndex,
    block_id: ast::LocalNodeId<ast::Block>,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    let parent_id = parents.get(block_id)?;
    if tree.get_node_type(parent_id) != ast::NodeType::Expression {
        return None;
    }

    let expression_id = ast::LocalNodeId::<ast::Expression>::new(parent_id);
    let expression = tree.get(expression_id);
    if !matches!(expression, ast::Expression::Block(current_id) if *current_id == block_id) {
        return None;
    }

    Some(expression_id)
}

/// Return true when one block expression is the body of a function or method.
pub fn block_is_function_body(
    tree: &ast::NodeTree,
    parents: &ast::NodeParentIndex,
    block_id: ast::LocalNodeId<ast::Block>,
) -> bool {
    // resolve the expression that owns this block
    let Some(block_expression_id) = block_expression_ancestor(tree, parents, block_id) else {
        return false;
    };

    // resolve the parent node that owns the expression
    let Some(owner_id) = parents.get(block_expression_id) else {
        return false;
    };

    // allow direct function declaration bodies
    if tree.get_node_type(owner_id) == ast::NodeType::Declaration {
        let declaration = tree.get(ast::LocalNodeId::<ast::Declaration>::new(owner_id));
        if matches!(
            declaration,
            ast::Declaration::Function(ast::FunctionDeclaration {
                body: Some(body_id),
                ..
            }) if *body_id == block_expression_id
        ) {
            return true;
        }
    }

    // allow method bodies
    if tree.get_node_type(owner_id) == ast::NodeType::Member {
        let member = tree.get(ast::LocalNodeId::<ast::Member>::new(owner_id));
        if matches!(
            member,
            ast::Member::Method {
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
    tree: &ast::NodeTree,
    parents: &ast::NodeParentIndex,
    block_id: ast::LocalNodeId<ast::Block>,
) -> bool {
    // resolve the expression that wraps this block
    let Some(block_expression_id) = block_expression_ancestor(tree, parents, block_id) else {
        return false;
    };

    // require a member owner for static blocks
    let Some(owner_id) = parents.get(block_expression_id) else {
        return false;
    };
    if tree.get_node_type(owner_id) != ast::NodeType::Member {
        return false;
    }

    // accept only static block members with the same body expression
    let member = tree.get(ast::LocalNodeId::<ast::Member>::new(owner_id));
    matches!(
        member,
        ast::Member::StaticBlock { body, .. } if *body == block_expression_id
    )
}

/// Return one declaration wrapper expression id for a declaration node.
pub fn declaration_expression(
    tree: &ast::NodeTree,
    parents: &ast::NodeParentIndex,
    declaration_id: ast::LocalNodeId<ast::Declaration>,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    // resolve the declaration parent node
    let parent_id = parents.get(declaration_id)?;
    if tree.get_node_type(parent_id) != ast::NodeType::Expression {
        return None;
    }

    // require an expression::declaration wrapper with the same declaration id
    let expression_id = ast::LocalNodeId::<ast::Expression>::new(parent_id);
    let expression = tree.get(expression_id);
    if !matches!(expression, ast::Expression::Declaration(current) if *current == declaration_id) {
        return None;
    }

    Some(expression_id)
}

/// Return true when one declaration expression is at an allowed root location.
pub fn declaration_at_allowed_root(
    tree: &ast::NodeTree,
    parents: &ast::NodeParentIndex,
    declaration_expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    // default to allowed when parent structure is missing
    let Some(parent_id) = parents.get(declaration_expression_id) else {
        return true;
    };

    // declarations are allowed only when directly under a block
    if tree.get_node_type(parent_id) != ast::NodeType::Block {
        return false;
    }
    let block_id = ast::LocalNodeId::<ast::Block>::new(parent_id);
    let block = tree.get(block_id);
    if block.format != ast::BlockFormat::Explicit {
        return true;
    }

    // resolve the expression that owns this block
    let Some(block_expression_id) = block_expression_ancestor(tree, parents, block_id) else {
        return false;
    };

    // default to allowed when the block expression has no owner
    let Some(owner_id) = parents.get(block_expression_id) else {
        return true;
    };

    // allow function declaration roots
    if tree.get_node_type(owner_id) == ast::NodeType::Declaration {
        let declaration = tree.get(ast::LocalNodeId::<ast::Declaration>::new(owner_id));
        if matches!(
            declaration,
            ast::Declaration::Function(ast::FunctionDeclaration {
                body: Some(body_id),
                ..
            }) if *body_id == block_expression_id
        ) {
            return true;
        }
    }

    // allow static block roots
    if tree.get_node_type(owner_id) == ast::NodeType::Member {
        let member = tree.get(ast::LocalNodeId::<ast::Member>::new(owner_id));
        if matches!(
            member,
            ast::Member::StaticBlock { body, .. } if *body == block_expression_id
        ) {
            return true;
        }
    }

    false
}

/// Return path segments when the expression is a non-generic reference chain.
pub fn expression_path_segments(
    tree: &ast::NodeTree,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<Vec<ast::StringId>> {
    let mut segments = Vec::new();

    collect_expression_path_segments(tree, expression_id, &mut segments)?;

    Some(segments)
}

/// Collect path segments for one non-generic reference chain.
fn collect_expression_path_segments(
    tree: &ast::NodeTree,
    expression_id: ast::LocalNodeId<ast::Expression>,
    segments: &mut Vec<ast::StringId>,
) -> Option<()> {
    // normalize wrappers first
    let expression_id = expression_unwrap_parenthesized_source_form(tree, expression_id);
    let expression = tree.get(expression_id);

    match expression {
        ast::Expression::Identifier { name } => {
            segments.push(*name);
            Some(())
        }
        ast::Expression::QualifiedReference {
            path,
            generic_arguments,
        } => {
            if !generic_arguments.is_empty() {
                return None;
            }

            segments.extend_from_slice(&path.segments);
            Some(())
        }
        ast::Expression::Member {
            left,
            name: Some(name),
            generic_arguments,
        } => {
            if !generic_arguments.is_empty() {
                return None;
            }

            collect_expression_path_segments(tree, *left, segments)?;
            segments.push(*name);
            Some(())
        }
        _ => None,
    }
}

/// Return one static string literal value from an expression source form.
pub fn expression_static_string_literal_source_form(
    tree: &ast::NodeTree,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<ast::StringId> {
    // normalize expression shape
    let expression_id = expression_unwrap_parenthesized_source_form(tree, expression_id);
    let expression = tree.get(expression_id);

    // match direct string literals
    if let ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(value)) = expression {
        return Some(*value);
    }

    // match template literals without interpolations
    let ast::Expression::TemplateExpression { value } = expression else {
        return None;
    };
    let ast::TemplateLiteral::String { string } = value else {
        return None;
    };

    Some(*string)
}

/// Return one static property access pair as `(left, property_name)` from source form.
pub fn expression_static_property_access_source_form(
    tree: &ast::NodeTree,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<(ast::LocalNodeId<ast::Expression>, ast::StringId)> {
    // normalize expression shape
    let expression_id = expression_unwrap_parenthesized_source_form(tree, expression_id);
    let expression = tree.get(expression_id);

    // match dot member access
    if let ast::Expression::Member { left, name, .. } = expression {
        let name = (*name)?;

        return Some((*left, name));
    }

    // match bracket member access with static string keys
    let ast::Expression::Index { left, index, .. } = expression else {
        return None;
    };
    let index_id = index.as_ref().copied()?;
    let property_name = expression_static_string_literal_source_form(tree, index_id)?;

    Some((*left, property_name))
}

/// Return the value expression id for one argument node.
pub fn argument_value_expression_id(
    tree: &ast::NodeTree,
    argument_id: ast::LocalNodeId<ast::Argument>,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    let argument = tree.get(argument_id);
    match argument {
        ast::Argument::Positional { value, .. }
        | ast::Argument::Spread { value, .. }
        | ast::Argument::Named { value, .. }
        | ast::Argument::Labeled { value, .. } => Some(*value),
        ast::Argument::Error => None,
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
    ctx: &mut LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<f64> {
    // resolve direct constant values first
    if let Some(value) = ctx.const_value(expression_id) {
        return const_value_f64(value);
    }

    // normalize expression shape and require a binary expression
    let expression_id = expression_unwrap_parenthesized_source_form(ctx.tree, expression_id);
    let expression = ctx.tree.get(expression_id);
    let ast::Expression::Binary {
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
        ast::BinaryOperator::Add
        | ast::BinaryOperator::WrappingAdd
        | ast::BinaryOperator::SaturatingAdd => Some(left_value + right_value),
        ast::BinaryOperator::Subtract
        | ast::BinaryOperator::WrappingSubtract
        | ast::BinaryOperator::SaturatingSubtract => Some(left_value - right_value),
        ast::BinaryOperator::Multiply
        | ast::BinaryOperator::WrappingMultiply
        | ast::BinaryOperator::SaturatingMultiply => Some(left_value * right_value),
        ast::BinaryOperator::Divide => Some(left_value / right_value),
        ast::BinaryOperator::Remainder => Some(left_value % right_value),
        ast::BinaryOperator::Exponent
        | ast::BinaryOperator::WrappingExponent
        | ast::BinaryOperator::SaturatingExponent => Some(left_value.powf(right_value)),
        _ => None,
    }
}

/// Evaluate an expression as a signed numeric constant when possible.
pub fn expression_numeric_sign(
    ctx: &mut LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
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
    ctx: &LintAstContext<'_>,
    left_id: ast::LocalNodeId<ast::Expression>,
    right_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let left_id = expression_unwrap_parenthesized_source_form(ctx.tree, left_id);
    let right_id = expression_unwrap_parenthesized_source_form(ctx.tree, right_id);
    let left = ctx.tree.get(left_id);
    let right = ctx.tree.get(right_id);
    match (left, right) {
        // identifiers: compare names
        (
            ast::Expression::Identifier { name: left_name },
            ast::Expression::Identifier { name: right_name },
        ) => string_ids_equal(ctx, *left_name, *right_name),

        // qualified references: compare segments
        (
            ast::Expression::QualifiedReference {
                path: left_path, ..
            },
            ast::Expression::QualifiedReference {
                path: right_path, ..
            },
        ) => paths_equal(ctx, left_path, right_path),

        // scalar literals: direct comparison
        (
            ast::Expression::ScalarLiteral(left_literal),
            ast::Expression::ScalarLiteral(right_literal),
        ) => left_literal == right_literal,

        // type expressions: compare recursively
        (ast::Expression::Type { value: left }, ast::Expression::Type { value: right }) => {
            type_expression_is_equal(ctx, *left, *right)
        }

        // binary expressions: compare operator and operands recursively
        (
            ast::Expression::Binary {
                operator: left_operator,
                left: left_left,
                right: left_right,
            },
            ast::Expression::Binary {
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
            ast::Expression::Unary {
                operator: left_operator,
                right: left_right,
            },
            ast::Expression::Unary {
                operator: right_operator,
                right: right_right,
            },
        ) => left_operator == right_operator && expression_is_equal(ctx, *left_right, *right_right),

        // member access: compare object and member name
        (
            ast::Expression::Member {
                left: left_object,
                name: left_name,
                ..
            },
            ast::Expression::Member {
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
            ast::Expression::Index {
                left: left_object,
                index: left_index,
                position: left_position,
            },
            ast::Expression::Index {
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
            ast::Expression::ValueOf {
                mutability: left_mutability,
                variance: left_variance,
                right: left_right,
            },
            ast::Expression::ValueOf {
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
            ast::Expression::ReferenceOf {
                mutability: left_mutability,
                variance: left_variance,
                right: left_right,
            },
            ast::Expression::ReferenceOf {
                mutability: right_mutability,
                variance: right_variance,
                right: right_right,
            },
        ) => {
            left_mutability == right_mutability
                && left_variance == right_variance
                && expression_is_equal(ctx, *left_right, *right_right)
        }

        // pointer of: compare mutability and operand
        (
            ast::Expression::PointerOf {
                mutability: left_mutability,
                right: left_right,
            },
            ast::Expression::PointerOf {
                mutability: right_mutability,
                right: right_right,
            },
        ) => {
            left_mutability == right_mutability
                && expression_is_equal(ctx, *left_right, *right_right)
        }

        // await expressions: compare inner expression
        (
            ast::Expression::Await {
                expression: left_expression,
            },
            ast::Expression::Await {
                expression: right_expression,
            },
        ) => expression_is_equal(ctx, *left_expression, *right_expression),

        // await? expressions: compare inner expression
        (
            ast::Expression::AwaitMaybe {
                expression: left_expression,
            },
            ast::Expression::AwaitMaybe {
                expression: right_expression,
            },
        ) => expression_is_equal(ctx, *left_expression, *right_expression),

        // throw expressions: compare value
        (
            ast::Expression::Throw { value: left_value },
            ast::Expression::Throw { value: right_value },
        ) => expression_is_equal(ctx, *left_value, *right_value),

        // delete expressions: compare value
        (
            ast::Expression::Delete { value: left_value },
            ast::Expression::Delete { value: right_value },
        ) => expression_is_equal(ctx, *left_value, *right_value),

        // maybe expressions: compare position and operand
        (
            ast::Expression::Maybe {
                position: left_position,
                left: left_left,
            },
            ast::Expression::Maybe {
                position: right_position,
                left: right_left,
            },
        ) => left_position == right_position && expression_is_equal(ctx, *left_left, *right_left),

        // must expressions: compare position and operand
        (
            ast::Expression::Must {
                position: left_position,
                left: left_left,
            },
            ast::Expression::Must {
                position: right_position,
                left: right_left,
            },
        ) => left_position == right_position && expression_is_equal(ctx, *left_left, *right_left),

        // assignment: compare operator and operands
        (
            ast::Expression::Assign {
                left: left_left,
                operator: left_operator,
                right: left_right,
            },
            ast::Expression::Assign {
                left: right_left,
                operator: right_operator,
                right: right_right,
            },
        ) => {
            left_operator == right_operator
                && expression_is_equal(ctx, *left_left, *right_left)
                && expression_is_equal(ctx, *left_right, *right_right)
        }

        // call expressions: compare callee and arguments
        (
            ast::Expression::Call {
                left: left_callee,
                arguments: left_args,
                ..
            },
            ast::Expression::Call {
                left: right_callee,
                arguments: right_args,
                ..
            },
        ) => {
            expression_is_equal(ctx, *left_callee, *right_callee)
                && arguments_are_equal(ctx, left_args, right_args)
        }

        // blocks: compare contents
        (ast::Expression::Block(left_block), ast::Expression::Block(right_block)) => {
            blocks_equal(ctx, *left_block, *right_block)
        }

        // shared structural comparison for remaining expression kinds
        _ => expression_signature_equal(ctx, left_id, right_id),
    }
}

/// Return whether two type expressions are structurally equal.
pub fn type_expression_is_equal(
    ctx: &LintAstContext<'_>,
    left_id: ast::LocalNodeId<ast::TypeExpression>,
    right_id: ast::LocalNodeId<ast::TypeExpression>,
) -> bool {
    let left = ctx.tree.get(left_id);
    let right = ctx.tree.get(right_id);

    match (left, right) {
        (
            ast::TypeExpression::Parenthesized { expression: left },
            ast::TypeExpression::Parenthesized { expression: right },
        ) => type_expression_is_equal(ctx, *left, *right),
        (
            ast::TypeExpression::ScalarLiteral { value: left },
            ast::TypeExpression::ScalarLiteral { value: right },
        ) => left == right,
        (
            ast::TypeExpression::Literal { value: left },
            ast::TypeExpression::Literal { value: right },
        ) => left == right,
        (
            ast::TypeExpression::Reference {
                path: left_path,
                generic_arguments: left_arguments,
            },
            ast::TypeExpression::Reference {
                path: right_path,
                generic_arguments: right_arguments,
            },
        ) => {
            paths_equal(ctx, left_path, right_path)
                && generic_arguments_are_equal(ctx, left_arguments, right_arguments)
        }
        (
            ast::TypeExpression::Member {
                left: left_target,
                name: left_name,
                generic_arguments: left_arguments,
            },
            ast::TypeExpression::Member {
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
            ast::TypeExpression::Readonly {
                target_type: left_target,
            },
            ast::TypeExpression::Readonly {
                target_type: right_target,
            },
        )
        | (
            ast::TypeExpression::KeyOf {
                target_type: left_target,
            },
            ast::TypeExpression::KeyOf {
                target_type: right_target,
            },
        )
        | (
            ast::TypeExpression::Must {
                target_type: left_target,
            },
            ast::TypeExpression::Must {
                target_type: right_target,
            },
        )
        | (
            ast::TypeExpression::AsComptime {
                target_type: left_target,
            },
            ast::TypeExpression::AsComptime {
                target_type: right_target,
            },
        )
        | (
            ast::TypeExpression::Not {
                target_type: left_target,
            },
            ast::TypeExpression::Not {
                target_type: right_target,
            },
        ) => type_expression_is_equal(ctx, *left_target, *right_target),
        (
            ast::TypeExpression::ValueOf {
                mutability: left_mutability,
                variance: left_variance,
                target_type: left_target,
            },
            ast::TypeExpression::ValueOf {
                mutability: right_mutability,
                variance: right_variance,
                target_type: right_target,
            },
        )
        | (
            ast::TypeExpression::ReferenceOf {
                mutability: left_mutability,
                variance: left_variance,
                target_type: left_target,
            },
            ast::TypeExpression::ReferenceOf {
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
            ast::TypeExpression::PointerOf {
                mutability: left_mutability,
                target_type: left_target,
            },
            ast::TypeExpression::PointerOf {
                mutability: right_mutability,
                target_type: right_target,
            },
        ) => {
            left_mutability == right_mutability
                && type_expression_is_equal(ctx, *left_target, *right_target)
        }
        (
            ast::TypeExpression::TypeOfValue { value: left_value },
            ast::TypeExpression::TypeOfValue { value: right_value },
        ) => expression_is_equal(ctx, *left_value, *right_value),
        (
            ast::TypeExpression::Index {
                left: left_target,
                index: left_index,
            },
            ast::TypeExpression::Index {
                left: right_target,
                index: right_index,
            },
        ) => {
            type_expression_is_equal(ctx, *left_target, *right_target)
                && type_expression_is_equal(ctx, *left_index, *right_index)
        }
        (
            ast::TypeExpression::Union { elements: left },
            ast::TypeExpression::Union { elements: right },
        )
        | (
            ast::TypeExpression::Intersection { elements: left },
            ast::TypeExpression::Intersection { elements: right },
        ) => type_expression_list_equal(ctx, left, right),
        _ => {
            let left = ctx.tree.get(left_id);
            let right = ctx.tree.get(right_id);

            let mut left_collector = ExpressionSignatureCollector::new(ctx.strings);
            ast::NodeVisitor::visit_type_expression(&mut left_collector, ctx.tree, left_id, left);

            let mut right_collector = ExpressionSignatureCollector::new(ctx.strings);
            ast::NodeVisitor::visit_type_expression(
                &mut right_collector,
                ctx.tree,
                right_id,
                right,
            );

            left_collector.finish() == right_collector.finish()
        }
    }
}

/// Check if two blocks have identical expressions.
pub fn blocks_equal(
    ctx: &LintAstContext<'_>,
    left_id: ast::LocalNodeId<ast::Block>,
    right_id: ast::LocalNodeId<ast::Block>,
) -> bool {
    let left = ctx.tree.get(left_id);
    let right = ctx.tree.get(right_id);
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
pub fn paths_equal(ctx: &LintAstContext<'_>, left: &ast::Path, right: &ast::Path) -> bool {
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
    ctx: &LintAstContext<'_>,
    left: ast::StringId,
    right: ast::StringId,
) -> bool {
    let left_string = ctx.strings.get(left);
    let right_string = ctx.strings.get(right);
    left_string.as_ref() == right_string.as_ref()
}

/// Return whether two argument lists are equal.
pub fn arguments_are_equal(
    ctx: &LintAstContext<'_>,
    left: &[ast::LocalNodeId<ast::Argument>],
    right: &[ast::LocalNodeId<ast::Argument>],
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
    ctx: &LintAstContext<'_>,
    left: &[ast::LocalNodeId<ast::GenericArgument>],
    right: &[ast::LocalNodeId<ast::GenericArgument>],
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
    ctx: &LintAstContext<'_>,
    left_id: ast::LocalNodeId<ast::GenericArgument>,
    right_id: ast::LocalNodeId<ast::GenericArgument>,
) -> bool {
    let left = ctx.tree.get(left_id);
    let right = ctx.tree.get(right_id);

    match (left, right) {
        (
            ast::GenericArgument::Type { value: left_type },
            ast::GenericArgument::Type { value: right_type },
        ) => type_expression_is_equal(ctx, *left_type, *right_type),
        (
            ast::GenericArgument::Value { value: left_value },
            ast::GenericArgument::Value { value: right_value },
        ) => expression_is_equal(ctx, *left_value, *right_value),
        (ast::GenericArgument::Error, ast::GenericArgument::Error) => true,
        _ => false,
    }
}

/// Return whether two type expression lists are structurally equal.
fn type_expression_list_equal(
    ctx: &LintAstContext<'_>,
    left: &[ast::LocalNodeId<ast::TypeExpression>],
    right: &[ast::LocalNodeId<ast::TypeExpression>],
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
    ctx: &LintAstContext<'_>,
    left_id: ast::LocalNodeId<ast::Argument>,
    right_id: ast::LocalNodeId<ast::Argument>,
) -> bool {
    let left = ctx.tree.get(left_id);
    let right = ctx.tree.get(right_id);

    match (left, right) {
        // positional arguments
        (
            ast::Argument::Positional {
                value: left_value, ..
            },
            ast::Argument::Positional {
                value: right_value, ..
            },
        ) => expression_is_equal(ctx, *left_value, *right_value),

        // spread arguments
        (
            ast::Argument::Spread {
                value: left_value, ..
            },
            ast::Argument::Spread {
                value: right_value, ..
            },
        ) => expression_is_equal(ctx, *left_value, *right_value),

        // named arguments
        (
            ast::Argument::Named {
                name: left_name,
                value: left_value,
                ..
            },
            ast::Argument::Named {
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
            ast::Argument::Labeled {
                label: left_label,
                value: left_value,
                ..
            },
            ast::Argument::Labeled {
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
    ctx: &LintAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expr = ctx.tree.get(expr_id);
    match expr {
        // pure: literals
        ast::Expression::ScalarLiteral(_) | ast::Expression::PrivateIdentifier { .. } => false,
        ast::Expression::Type { value } => type_expression_has_side_effects(ctx, *value),

        // pure: references
        ast::Expression::Identifier { .. }
        | ast::Expression::QualifiedReference { .. }
        | ast::Expression::ImportMeta
        | ast::Expression::NewTarget
        | ast::Expression::This
        | ast::Expression::Super => false,

        // pure: containers (if elements are pure)
        ast::Expression::ArrayExpression { elements }
        | ast::Expression::TupleExpression { elements } => elements.iter().any(|arg_id| {
            let arg = ctx.tree.get(*arg_id);
            match arg {
                ast::Argument::Positional { value, .. }
                | ast::Argument::Spread { value, .. }
                | ast::Argument::Named { value, .. }
                | ast::Argument::Labeled { value, .. } => expression_has_side_effects(ctx, *value),
                ast::Argument::Error => true,
            }
        }),

        // pure: member access (if object is pure)
        ast::Expression::Member { left, .. } | ast::Expression::PrivateMember { left, .. } => {
            expression_has_side_effects(ctx, *left)
        }

        // pure: instantiation (if target is pure)
        ast::Expression::Instantiation { left, .. } => expression_has_side_effects(ctx, *left),

        // pure: index access (if object and index are pure)
        ast::Expression::Index { left, index, .. } => {
            expression_has_side_effects(ctx, *left)
                || index.is_some_and(|idx| expression_has_side_effects(ctx, idx))
        }

        // pure: unary/binary ops on pure expressions
        ast::Expression::Unary { right, .. } => expression_has_side_effects(ctx, *right),
        ast::Expression::Is { value, target_type } => {
            expression_has_side_effects(ctx, *value)
                || type_expression_has_side_effects(ctx, *target_type)
        }
        ast::Expression::InstanceOf { value, target } => {
            expression_has_side_effects(ctx, *value) || expression_has_side_effects(ctx, *target)
        }
        ast::Expression::Binary { left, right, .. } => {
            expression_has_side_effects(ctx, *left) || expression_has_side_effects(ctx, *right)
        }

        // pure: type operations
        ast::Expression::As {
            expression,
            target_type,
        }
        | ast::Expression::Satisfies {
            expression,
            target_type,
        } => {
            expression_has_side_effects(ctx, *expression)
                || type_expression_has_side_effects(ctx, *target_type)
        }

        // pure: reference/value of (if operand is pure)
        ast::Expression::ReferenceOf { right, .. }
        | ast::Expression::ValueOf { right, .. }
        | ast::Expression::PointerOf { right, .. } => expression_has_side_effects(ctx, *right),

        // side effects: calls, assignments, new, await, yield, etc
        ast::Expression::Call { .. }
        | ast::Expression::Assign { .. }
        | ast::Expression::New { .. }
        | ast::Expression::Await { .. }
        | ast::Expression::AwaitMaybe { .. }
        | ast::Expression::Yield { .. }
        | ast::Expression::Delete { .. }
        | ast::Expression::Throw { .. } => true,

        // side effects: control flow
        ast::Expression::Return { .. }
        | ast::Expression::Break { .. }
        | ast::Expression::Continue { .. }
        | ast::Expression::For { .. }
        | ast::Expression::ForEach { .. }
        | ast::Expression::While { .. }
        | ast::Expression::Loop { .. }
        | ast::Expression::If { .. }
        | ast::Expression::Match { .. }
        | ast::Expression::Try { .. } => true,

        // side effects: declarations, imports, exports
        ast::Expression::Declaration(_)
        | ast::Expression::Block(_)
        | ast::Expression::Let { .. }
        | ast::Expression::Using { .. }
        | ast::Expression::Import { .. }
        | ast::Expression::Export { .. }
        | ast::Expression::ExportNamespace { .. }
        | ast::Expression::Labelled { .. } => true,

        // side effects: debugger, error, stub
        ast::Expression::Debugger
        | ast::Expression::Error
        | ast::Expression::Missing
        | ast::Expression::Stub => true,

        // wrapped expressions: check inner
        ast::Expression::Parenthesized { expression } => {
            expression_has_side_effects(ctx, *expression)
        }

        // maybe/must propagation: check inner for side effect
        ast::Expression::Maybe { left, .. } | ast::Expression::Must { left, .. } => {
            expression_has_side_effects(ctx, *left)
        }

        // templates: conservatively assume side effects (could have interpolations with calls)
        ast::Expression::TemplateExpression { .. }
        | ast::Expression::TaggedTemplateExpression { .. } => true,

        // object expressions: check properties for side effects
        ast::Expression::ObjectExpression { .. }
        | ast::Expression::TreeExpression { .. }
        | ast::Expression::SequenceExpression { .. } => true,

        // comptime: check if body has side effects
        ast::Expression::Comptime { body } => expression_has_side_effects(ctx, *body),
    }
}

/// Return whether one type expression subtree contains side effects.
pub fn type_expression_has_side_effects(
    ctx: &LintAstContext<'_>,
    type_expression_id: ast::LocalNodeId<ast::TypeExpression>,
) -> bool {
    let type_expression = ctx.tree.get(type_expression_id);

    match type_expression {
        // pure leaves
        ast::TypeExpression::ScalarLiteral { .. }
        | ast::TypeExpression::Literal { .. }
        | ast::TypeExpression::Intrinsic
        | ast::TypeExpression::Const
        | ast::TypeExpression::This
        | ast::TypeExpression::Missing
        | ast::TypeExpression::Error => false,

        // wrapped type expressions
        ast::TypeExpression::Parenthesized { expression }
        | ast::TypeExpression::Readonly {
            target_type: expression,
        }
        | ast::TypeExpression::KeyOf {
            target_type: expression,
        }
        | ast::TypeExpression::Must {
            target_type: expression,
        }
        | ast::TypeExpression::AsComptime {
            target_type: expression,
        }
        | ast::TypeExpression::Not {
            target_type: expression,
        }
        | ast::TypeExpression::ValueOf {
            target_type: expression,
            ..
        }
        | ast::TypeExpression::ReferenceOf {
            target_type: expression,
            ..
        }
        | ast::TypeExpression::PointerOf {
            target_type: expression,
            ..
        } => type_expression_has_side_effects(ctx, *expression),

        // mostly pure type forms
        ast::TypeExpression::Tuple { .. }
        | ast::TypeExpression::Array { .. }
        | ast::TypeExpression::Object { .. }
        | ast::TypeExpression::Declaration { .. }
        | ast::TypeExpression::Reference { .. }
        | ast::TypeExpression::Infer { .. }
        | ast::TypeExpression::Predicate { .. } => false,

        // type expressions that contain runtime expressions
        ast::TypeExpression::TypeOfValue { value } => expression_has_side_effects(ctx, *value),
        ast::TypeExpression::Import {
            target,
            arguments,
            generic_arguments,
            ..
        } => {
            expression_has_side_effects(ctx, *target)
                || arguments.iter().any(|argument_id| {
                    let argument = ctx.tree.get(*argument_id);
                    match argument {
                        ast::Argument::Named { value, .. }
                        | ast::Argument::Labeled { value, .. }
                        | ast::Argument::Positional { value }
                        | ast::Argument::Spread { value, .. } => {
                            expression_has_side_effects(ctx, *value)
                        }
                        ast::Argument::Error => true,
                    }
                })
                || generic_arguments.iter().any(|argument_id| {
                    let argument = ctx.tree.get(*argument_id);
                    match argument {
                        ast::GenericArgument::Type { value } => {
                            type_expression_has_side_effects(ctx, *value)
                        }
                        ast::GenericArgument::Value { value } => {
                            expression_has_side_effects(ctx, *value)
                        }
                        ast::GenericArgument::Error => true,
                    }
                })
        }

        // composite type expressions
        ast::TypeExpression::Member {
            left,
            generic_arguments,
            ..
        } => {
            type_expression_has_side_effects(ctx, *left)
                || generic_arguments.iter().any(|argument_id| {
                    let argument = ctx.tree.get(*argument_id);
                    match argument {
                        ast::GenericArgument::Type { value } => {
                            type_expression_has_side_effects(ctx, *value)
                        }
                        ast::GenericArgument::Value { value } => {
                            expression_has_side_effects(ctx, *value)
                        }
                        ast::GenericArgument::Error => true,
                    }
                })
        }
        ast::TypeExpression::Union { elements }
        | ast::TypeExpression::Intersection { elements } => elements
            .iter()
            .any(|element_id| type_expression_has_side_effects(ctx, *element_id)),
        ast::TypeExpression::Conditional {
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
        ast::TypeExpression::Mapped {
            parameter, value, ..
        } => {
            type_expression_has_side_effects(ctx, parameter.source_type)
                || parameter
                    .key_remap
                    .is_some_and(|key_remap| type_expression_has_side_effects(ctx, key_remap))
                || type_expression_has_side_effects(ctx, *value)
        }
        ast::TypeExpression::Index { left, index } => {
            type_expression_has_side_effects(ctx, *left)
                || type_expression_has_side_effects(ctx, *index)
        }
        ast::TypeExpression::TemplateLiteral { spans, .. } => spans
            .iter()
            .any(|span_id| type_expression_has_side_effects(ctx, *span_id)),
    }
}

/// Check if an operator is a comparison operator.
pub fn is_comparison_operator(operator: &ast::BinaryOperator) -> bool {
    matches!(
        operator,
        ast::BinaryOperator::Equal
            | ast::BinaryOperator::NotEqual
            | ast::BinaryOperator::EqualStrict
            | ast::BinaryOperator::NotEqualStrict
            | ast::BinaryOperator::LessThan
            | ast::BinaryOperator::LessThanOrEqual
            | ast::BinaryOperator::GreaterThan
            | ast::BinaryOperator::GreaterThanOrEqual
    )
}

/// Check if an expression is a literal value (scalar or type literal).
pub fn expression_is_literal(expression: &ast::Expression) -> bool {
    match expression {
        ast::Expression::ScalarLiteral(_) => true,
        ast::Expression::Type { .. } => false,
        _ => false,
    }
}

/// Check if an expression is a constant expression (evaluates to a fixed value at compile time).
pub fn expression_is_constant_expression(ctx: &LintAstContext<'_>, expr: &ast::Expression) -> bool {
    match expr {
        ast::Expression::ScalarLiteral(_) => true,
        ast::Expression::Type { value } => type_expression_is_constant(ctx, *value),
        ast::Expression::Parenthesized { expression } => {
            expression_is_constant_expression(ctx, ctx.tree.get(*expression))
        }
        ast::Expression::Unary { right, .. } => {
            expression_is_constant_expression(ctx, ctx.tree.get(*right))
        }
        _ => false,
    }
}

/// Evaluate a constant expression to a boolean value if possible.
///
/// Returns Some(true) for truthy constants, Some(false) for falsy constants, None otherwise.
/// Handles booleans, integers, floats, null/undefined, parenthesized expressions, and unary not.
pub fn expression_constant_to_bool(
    ctx: &LintAstContext<'_>,
    expr: &ast::Expression,
) -> Option<bool> {
    match expr {
        ast::Expression::ScalarLiteral(lit) => match lit {
            ast::ScalarLiteral::Boolean(b) => Some(*b),
            ast::ScalarLiteral::Integer(value) => Some(*value != 0),
            ast::ScalarLiteral::Float(value) => Some(*value != 0.0),
            ast::ScalarLiteral::Bigint(value) => Some(*value != 0),
            _ => None,
        },
        ast::Expression::Type { value } => type_expression_constant_to_bool(ctx, *value),
        ast::Expression::Parenthesized { expression } => {
            expression_constant_to_bool(ctx, ctx.tree.get(*expression))
        }
        ast::Expression::Unary { operator, right } => {
            if *operator == ast::UnaryOperator::Not {
                expression_constant_to_bool(ctx, ctx.tree.get(*right)).map(|b| !b)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Return true when one type expression is constant.
fn type_expression_is_constant(
    ctx: &LintAstContext<'_>,
    type_expression_id: ast::LocalNodeId<ast::TypeExpression>,
) -> bool {
    let type_expression = ctx.tree.get(type_expression_id);

    match type_expression {
        ast::TypeExpression::Parenthesized { expression } => {
            type_expression_is_constant(ctx, *expression)
        }
        ast::TypeExpression::ScalarLiteral { .. }
        | ast::TypeExpression::Literal { .. }
        | ast::TypeExpression::Intrinsic
        | ast::TypeExpression::Const
        | ast::TypeExpression::This => true,
        _ => false,
    }
}

/// Return the boolean value of one constant type expression when known.
fn type_expression_constant_to_bool(
    ctx: &LintAstContext<'_>,
    type_expression_id: ast::LocalNodeId<ast::TypeExpression>,
) -> Option<bool> {
    let type_expression = ctx.tree.get(type_expression_id);

    match type_expression {
        ast::TypeExpression::Parenthesized { expression } => {
            type_expression_constant_to_bool(ctx, *expression)
        }
        ast::TypeExpression::ScalarLiteral { value } => match value {
            ast::ScalarLiteral::Boolean(value) => Some(*value),
            ast::ScalarLiteral::Integer(value) => Some(*value != 0),
            ast::ScalarLiteral::Float(value) => Some(*value != 0.0),
            ast::ScalarLiteral::Bigint(value) => Some(*value != 0),
            _ => None,
        },
        ast::TypeExpression::Literal { value } => match value {
            ast::TypeLiteral::Null | ast::TypeLiteral::Undefined => Some(false),
            _ => None,
        },
        _ => None,
    }
}

/// Compare two expressions using one structural signature.
fn expression_signature_equal(
    ctx: &LintAstContext<'_>,
    left_id: ast::LocalNodeId<ast::Expression>,
    right_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let left_signature = expression_signature(ctx, left_id);
    let right_signature = expression_signature(ctx, right_id);
    left_signature == right_signature
}

/// Build a structural signature for one expression subtree.
fn expression_signature(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Vec<u64> {
    expression_signature_for_tree(ctx.tree, ctx.strings, expression_id)
}

/// Build a structural signature for one expression subtree.
pub fn expression_signature_for_tree(
    tree: &ast::NodeTree,
    strings: &ast::StringPool,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Vec<u64> {
    let expression = tree.get(expression_id);
    let mut collector = ExpressionSignatureCollector::new(strings);
    ast::NodeVisitor::visit_expression(&mut collector, tree, expression_id, expression);
    collector.finish()
}

/// Collect a structural signature for an expression subtree.
struct ExpressionSignatureCollector<'a> {
    /// String pool for identifier and literal names.
    strings: &'a ast::StringPool,
    /// Visitor options.
    visitor_options: ast::NodeVisitorOptions,
    /// Signature token stream.
    tokens: Vec<u64>,
}

impl<'a> ExpressionSignatureCollector<'a> {
    /// Build an empty signature collector.
    fn new(strings: &'a ast::StringPool) -> Self {
        Self {
            strings,
            visitor_options: ast::NodeVisitorOptions::default(),
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
    fn push_string_id(&mut self, key: &str, string_id: ast::StringId) {
        self.push_token(key, &self.strings.get(string_id));
    }
}

impl ast::NodeVisitor for ExpressionSignatureCollector<'_> {
    /// Return visitor options.
    fn options(&self) -> &ast::NodeVisitorOptions {
        &self.visitor_options
    }

    /// Visit any AST node and record its node type.
    fn visit_any(&mut self, _tree: &ast::NodeTree, ty: ast::NodeType, _id: u32) {
        self.push_debug("node", ty);
    }

    /// Visit one expression node and record expression specific signature tokens.
    fn visit_expression(
        &mut self,
        tree: &ast::NodeTree,
        id: ast::LocalNodeId<ast::Expression>,
        expression: &ast::Expression,
    ) {
        self.push_debug("expression", std::mem::discriminant(expression));
        match expression {
            ast::Expression::Labelled { label, .. } => {
                self.push_string_id("expression_label", *label);
            }
            ast::Expression::Import {
                source,
                kind,
                target,
                ..
            } => {
                self.push_debug("expression_import_source", *source);
                self.push_debug("expression_import_kind", *kind);
                match target {
                    ast::ImportTarget::String(target) => {
                        self.push_string_id("expression_import_target", *target);
                    }
                    ast::ImportTarget::Expression { .. } => {
                        self.push_debug("expression_import_target_expression", true);
                    }
                }
            }
            ast::Expression::Export { kind, target, .. } => {
                self.push_debug("expression_export_kind", *kind);
                if let Some(target) = target {
                    self.push_string_id("expression_export_target", *target);
                }
            }
            ast::Expression::ExportNamespace { name } => {
                self.push_string_id("expression_export_namespace", *name);
            }
            ast::Expression::If { kind, .. } => {
                self.push_debug("expression_if_kind", *kind);
            }
            ast::Expression::While { kind, .. } => {
                self.push_debug("expression_while_kind", *kind);
            }
            ast::Expression::ForEach {
                asynchrony, kind, ..
            } => {
                self.push_debug("expression_foreach_asynchrony", *asynchrony);
                self.push_debug("expression_foreach_kind", *kind);
            }
            ast::Expression::Match { kind, .. } => {
                self.push_debug("expression_match_kind", *kind);
            }
            ast::Expression::Break { label, value } => {
                self.push_debug("expression_break_has_label", label.is_some());
                self.push_debug("expression_break_has_value", value.is_some());
                if let Some(label) = label {
                    self.push_string_id("expression_break_label", *label);
                }
            }
            ast::Expression::Continue { label } => {
                self.push_debug("expression_continue_has_label", label.is_some());
                if let Some(label) = label {
                    self.push_string_id("expression_continue_label", *label);
                }
            }
            ast::Expression::Yield { cardinality, .. } => {
                self.push_debug("expression_yield_cardinality", *cardinality);
            }
            ast::Expression::Identifier { name } => {
                self.push_string_id("expression_identifier", *name);
            }
            ast::Expression::QualifiedReference { path, .. } => {
                self.push_debug("expression_path_len", path.segments.len());
                for segment in &path.segments {
                    self.push_string_id("expression_path_segment", *segment);
                }
            }
            ast::Expression::PrivateIdentifier { name } => {
                self.push_string_id("expression_private_identifier", *name);
            }
            ast::Expression::ScalarLiteral(literal) => {
                self.push_debug("expression_scalar_literal", literal);
            }
            ast::Expression::Type { .. } => {
                self.push_debug("expression_type", true);
            }
            ast::Expression::TemplateExpression { value }
            | ast::Expression::TaggedTemplateExpression { value, .. } => {
                self.push_debug("expression_template", value);
            }
            ast::Expression::Unary { operator, .. } => {
                self.push_debug("expression_unary", *operator);
            }
            ast::Expression::ValueOf {
                mutability,
                variance,
                ..
            } => {
                self.push_debug("expression_valueof_mutability", *mutability);
                self.push_debug("expression_valueof_variance", *variance);
            }
            ast::Expression::ReferenceOf {
                mutability,
                variance,
                ..
            } => {
                self.push_debug("expression_referenceof_mutability", *mutability);
                self.push_debug("expression_referenceof_variance", *variance);
            }
            ast::Expression::PointerOf { mutability, .. } => {
                self.push_debug("expression_pointerof_mutability", *mutability);
            }
            ast::Expression::Member { name, .. } | ast::Expression::PrivateMember { name, .. } => {
                if let Some(name) = name {
                    self.push_string_id("expression_member", *name);
                }
            }
            ast::Expression::Index { position, .. }
            | ast::Expression::Call { position, .. }
            | ast::Expression::Maybe { position, .. }
            | ast::Expression::Must { position, .. } => {
                self.push_debug("expression_postfix_position", *position);
            }
            ast::Expression::Binary { operator, .. } => {
                self.push_debug("expression_binary", *operator);
            }
            ast::Expression::Assign { operator, .. } => {
                self.push_debug("expression_assign", *operator);
            }
            _ => {}
        }

        ast::walk_expression(self, tree, id, expression);
    }

    /// Visit one type expression node and record type specific signature tokens.
    fn visit_type_expression(
        &mut self,
        tree: &ast::NodeTree,
        id: ast::LocalNodeId<ast::TypeExpression>,
        type_expression: &ast::TypeExpression,
    ) {
        self.push_debug("type_expression", std::mem::discriminant(type_expression));
        match type_expression {
            ast::TypeExpression::ScalarLiteral { value } => {
                self.push_debug("type_expression_scalar_literal", value);
            }
            ast::TypeExpression::Literal { value } => {
                self.push_debug("type_expression_literal", value);
            }
            ast::TypeExpression::Reference {
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
            ast::TypeExpression::Member {
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
            ast::TypeExpression::Mapped {
                parameter,
                readonly,
                optional,
                ..
            } => {
                self.push_string_id("type_expression_mapped_name", parameter.name);
                self.push_debug("type_expression_mapped_readonly", *readonly);
                self.push_debug("type_expression_mapped_optional", *optional);
            }
            ast::TypeExpression::TemplateLiteral { strings, .. } => {
                self.push_debug("type_expression_template_len", strings.len());
                for string in strings {
                    self.push_string_id("type_expression_template_string", *string);
                }
            }
            ast::TypeExpression::Infer { name, .. } => {
                self.push_string_id("type_expression_infer", *name);
            }
            ast::TypeExpression::Predicate {
                asserts, subject, ..
            } => {
                self.push_debug("type_expression_predicate_asserts", *asserts);
                self.push_debug("type_expression_predicate_subject", subject);
            }
            _ => {}
        }

        ast::walk_type_expression(self, tree, id, type_expression);
    }

    /// Visit one declaration node and record declaration signature tokens.
    fn visit_declaration(
        &mut self,
        tree: &ast::NodeTree,
        id: ast::LocalNodeId<ast::Declaration>,
        declaration: &ast::Declaration,
    ) {
        self.push_debug("declaration", std::mem::discriminant(declaration));
        ast::walk_declaration(self, tree, id, declaration);
    }

    /// Visit one property node and record property signature tokens.
    fn visit_property(
        &mut self,
        tree: &ast::NodeTree,
        id: ast::LocalNodeId<ast::Property>,
        property: &ast::Property,
    ) {
        self.push_debug("property", std::mem::discriminant(property));
        ast::walk_property(self, tree, id, property);
    }

    /// Visit one member node and record member signature tokens.
    fn visit_member(
        &mut self,
        tree: &ast::NodeTree,
        id: ast::LocalNodeId<ast::Member>,
        member: &ast::Member,
    ) {
        self.push_debug("member", std::mem::discriminant(member));
        ast::walk_member(self, tree, id, member);
    }

    /// Visit one parameter node and record parameter signature tokens.
    fn visit_parameter(
        &mut self,
        tree: &ast::NodeTree,
        id: ast::LocalNodeId<ast::Parameter>,
        parameter: &ast::Parameter,
    ) {
        self.push_debug("parameter", std::mem::discriminant(parameter));
        ast::walk_parameter(self, tree, id, parameter);
    }

    /// Visit one argument node and record argument signature tokens.
    fn visit_argument(
        &mut self,
        tree: &ast::NodeTree,
        id: ast::LocalNodeId<ast::Argument>,
        argument: &ast::Argument,
    ) {
        self.push_debug("argument", std::mem::discriminant(argument));
        ast::walk_argument(self, tree, id, argument);
    }

    /// Visit one pattern node and record pattern signature tokens.
    fn visit_pattern(
        &mut self,
        tree: &ast::NodeTree,
        id: ast::LocalNodeId<ast::Pattern>,
        pattern: &ast::Pattern,
    ) {
        self.push_debug("pattern", std::mem::discriminant(pattern));
        ast::walk_pattern(self, tree, id, pattern);
    }

    /// Visit one pattern field node and record field signature tokens.
    fn visit_pattern_field(
        &mut self,
        tree: &ast::NodeTree,
        id: ast::LocalNodeId<ast::PatternField>,
        pattern_field: &ast::PatternField,
    ) {
        self.push_debug("pattern_field", std::mem::discriminant(pattern_field));
        ast::walk_pattern_field(self, tree, id, pattern_field);
    }

    /// Visit one match case node and record case signature tokens.
    fn visit_match_case(
        &mut self,
        tree: &ast::NodeTree,
        id: ast::LocalNodeId<ast::MatchCase>,
        match_case: &ast::MatchCase,
    ) {
        self.push_debug("match_case", std::mem::discriminant(match_case));
        ast::walk_match_case(self, tree, id, match_case);
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
