use destack_ast::{self as ast};

use crate::LintModuleAstContext;
use crate::rules::common::{stable_hash_debug, stable_hash_token, stable_hash_token_hashed_value};

/// Return whether two expressions are structurally equal.
///
/// Compares expressions by structure, ignoring parentheses.
/// Handles paths, literals, and recursively compares binary/unary operations.
pub fn expression_is_equal(
    ctx: &LintModuleAstContext<'_>,
    left_id: ast::LocalNodeId<ast::Expression>,
    right_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let left = ctx.tree.get(left_id);
    let right = ctx.tree.get(right_id);

    // unwrap parentheses
    let left = expression_unwrap_parentheses(ctx, left);
    let right = expression_unwrap_parentheses(ctx, right);
    match (left, right) {
        // paths: compare segments
        (
            ast::Expression::Path {
                path: left_path, ..
            },
            ast::Expression::Path {
                path: right_path, ..
            },
        ) => paths_equal(ctx, left_path, right_path),

        // scalar literals: direct comparison
        (
            ast::Expression::ScalarLiteral(left_literal),
            ast::Expression::ScalarLiteral(right_literal),
        ) => left_literal == right_literal,

        // type literals: direct comparison
        (
            ast::Expression::TypeLiteral(left_literal),
            ast::Expression::TypeLiteral(right_literal),
        ) => left_literal == right_literal,

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

        // type unary expressions: compare operator and operand
        (
            ast::Expression::TypeUnary {
                operator: left_operator,
                right: left_right,
            },
            ast::Expression::TypeUnary {
                operator: right_operator,
                right: right_right,
            },
        ) => left_operator == right_operator && expression_is_equal(ctx, *left_right, *right_right),

        // type binary expressions: compare operator and operands
        (
            ast::Expression::TypeBinary {
                left: left_left,
                operator: left_operator,
                right: left_right,
            },
            ast::Expression::TypeBinary {
                left: right_left,
                operator: right_operator,
                right: right_right,
            },
        ) => {
            left_operator == right_operator
                && expression_is_equal(ctx, *left_left, *right_left)
                && expression_is_equal(ctx, *left_right, *right_right)
        }

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
                && string_ids_equal(ctx, *left_name, *right_name)
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

        // range expressions: compare start, end, and inclusivity
        (
            ast::Expression::RangeExpression {
                start: left_start,
                end: left_end,
                is_inclusive: left_inclusive,
            },
            ast::Expression::RangeExpression {
                start: right_start,
                end: right_end,
                is_inclusive: right_inclusive,
            },
        ) => {
            left_inclusive == right_inclusive
                && expression_is_equal(ctx, *left_start, *right_start)
                && expression_is_equal(ctx, *left_end, *right_end)
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
                dynamic_arguments: left_args,
                ..
            },
            ast::Expression::Call {
                left: right_callee,
                dynamic_arguments: right_args,
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

        // statements: unwrap and compare
        (ast::Expression::Statement(left_inner), ast::Expression::Statement(right_inner)) => {
            expression_is_equal(ctx, *left_inner, *right_inner)
        }
        (ast::Expression::Statement(left_inner), _) => {
            expression_is_equal(ctx, *left_inner, right_id)
        }
        (_, ast::Expression::Statement(right_inner)) => {
            expression_is_equal(ctx, left_id, *right_inner)
        }

        // fallback: compare structural signatures for remaining expression kinds
        _ => expression_signature_equal(ctx, left_id, right_id),
    }
}

/// Check if two blocks have identical expressions.
pub fn blocks_equal(
    ctx: &LintModuleAstContext<'_>,
    left_id: ast::LocalNodeId<ast::Block>,
    right_id: ast::LocalNodeId<ast::Block>,
) -> bool {
    let left = ctx.tree.get(left_id);
    let right = ctx.tree.get(right_id);
    if left.expressions.len() != right.expressions.len() {
        return false;
    }
    for (left_expr, right_expr) in left.expressions.iter().zip(right.expressions.iter()) {
        if !expression_is_equal(ctx, *left_expr, *right_expr) {
            return false;
        }
    }
    true
}

/// Unwrap parenthesized expressions to get the inner expression.
fn expression_unwrap_parentheses<'a>(
    ctx: &'a LintModuleAstContext<'_>,
    expression: &'a ast::Expression,
) -> &'a ast::Expression {
    match expression {
        ast::Expression::Parenthesized {
            expression: inner_id,
        } => {
            let inner = ctx.tree.get(*inner_id);
            expression_unwrap_parentheses(ctx, inner)
        }
        _ => expression,
    }
}

/// Return whether two paths are equal.
pub fn paths_equal(ctx: &LintModuleAstContext<'_>, left: &ast::Path, right: &ast::Path) -> bool {
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
    ctx: &LintModuleAstContext<'_>,
    left: ast::StringId,
    right: ast::StringId,
) -> bool {
    let left_string = ctx.strings.get(left);
    let right_string = ctx.strings.get(right);
    left_string.as_ref() == right_string.as_ref()
}

/// Return whether two argument lists are equal.
pub fn arguments_are_equal(
    ctx: &LintModuleAstContext<'_>,
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

/// Return whether two arguments are structurally equal.
pub fn argument_is_equal(
    ctx: &LintModuleAstContext<'_>,
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
/// This is useful for lints that want to detect expressions that can be safely removed
/// or that need to distinguish between pure and impure expressions.
/// #Cleanup: can expression_has_side_effects use NodeVisitor..?
pub fn expression_has_side_effects(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expr = ctx.tree.get(expr_id);
    match expr {
        // pure: literals
        ast::Expression::ScalarLiteral(_)
        | ast::Expression::TypeLiteral(_)
        | ast::Expression::PrivateIdentifier { .. } => false,

        // pure: paths (variable references)
        ast::Expression::Path { .. } | ast::Expression::This | ast::Expression::Super => false,

        // pure: containers (if elements are pure)
        ast::Expression::ArrayExpression { elements }
        | ast::Expression::TupleExpression { elements } => elements.iter().any(|arg_id| {
            let arg = ctx.tree.get(*arg_id);
            match arg {
                ast::Argument::Positional { value, .. }
                | ast::Argument::Spread { value, .. }
                | ast::Argument::Named { value, .. }
                | ast::Argument::Labeled { value, .. } => expression_has_side_effects(ctx, *value),
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
        ast::Expression::Binary { left, right, .. } => {
            expression_has_side_effects(ctx, *left) || expression_has_side_effects(ctx, *right)
        }

        // pure: type operations
        ast::Expression::TypeUnary { right, .. } => expression_has_side_effects(ctx, *right),
        ast::Expression::TypeBinary { left, right, .. } => {
            expression_has_side_effects(ctx, *left) || expression_has_side_effects(ctx, *right)
        }
        ast::Expression::TypeConditional {
            left,
            right,
            then_type,
            else_type,
        } => {
            expression_has_side_effects(ctx, *left)
                || expression_has_side_effects(ctx, *right)
                || expression_has_side_effects(ctx, *then_type)
                || expression_has_side_effects(ctx, *else_type)
        }
        ast::Expression::TypeMapped {
            parameter, value, ..
        } => {
            expression_has_side_effects(ctx, parameter.constraint)
                || parameter
                    .key_remap
                    .is_some_and(|key_remap| expression_has_side_effects(ctx, key_remap))
                || expression_has_side_effects(ctx, *value)
        }
        ast::Expression::TypeIndex { left, index } => {
            expression_has_side_effects(ctx, *left) || expression_has_side_effects(ctx, *index)
        }
        ast::Expression::TypeTemplateLiteral { spans, .. } => spans
            .iter()
            .any(|span_id| expression_has_side_effects(ctx, *span_id)),
        ast::Expression::TypeImport { .. } => false,
        ast::Expression::TypeInfer { constraint, .. } => {
            constraint.is_some_and(|constraint| expression_has_side_effects(ctx, constraint))
        }
        ast::Expression::TypePredicate { target, .. } => {
            target.is_some_and(|target| expression_has_side_effects(ctx, target))
        }

        // pure: reference/value of (if operand is pure)
        ast::Expression::ReferenceOf { right, .. }
        | ast::Expression::ValueOf { right, .. }
        | ast::Expression::PointerOf { right, .. } => expression_has_side_effects(ctx, *right),

        // pure: range (if bounds are pure)
        ast::Expression::RangeExpression { start, end, .. } => {
            expression_has_side_effects(ctx, *start) || expression_has_side_effects(ctx, *end)
        }

        // side effects: calls, assignments, new, await, yield, etc.
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
        ast::Expression::Debugger | ast::Expression::Error | ast::Expression::Stub => true,

        // wrapped expressions: check inner
        ast::Expression::Parenthesized { expression } => {
            expression_has_side_effects(ctx, *expression)
        }
        ast::Expression::Statement(inner) => expression_has_side_effects(ctx, *inner),

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
    matches!(
        expression,
        ast::Expression::ScalarLiteral(_) | ast::Expression::TypeLiteral(_)
    )
}

/// Check if an expression is a constant expression (evaluates to a fixed value at compile time).
pub fn expression_is_constant_expression(
    ctx: &LintModuleAstContext<'_>,
    expr: &ast::Expression,
) -> bool {
    match expr {
        ast::Expression::ScalarLiteral(_) => true,
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
    ctx: &LintModuleAstContext<'_>,
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
        ast::Expression::TypeLiteral(ast::TypeLiteral::Null | ast::TypeLiteral::Undefined) => {
            Some(false)
        }
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

/// Compare two expressions using a structural fallback signature.
fn expression_signature_equal(
    ctx: &LintModuleAstContext<'_>,
    left_id: ast::LocalNodeId<ast::Expression>,
    right_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let left_signature = expression_signature(ctx, left_id);
    let right_signature = expression_signature(ctx, right_id);
    left_signature == right_signature
}

/// Build a structural signature for one expression subtree.
fn expression_signature(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Vec<u64> {
    expression_structural_signature(ctx.tree, ctx.strings, expression_id)
}

/// Build a structural signature for one expression subtree.
pub fn expression_structural_signature(
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
    fn options(&self) -> &ast::NodeVisitorOptions {
        &self.visitor_options
    }

    fn visit_any(&mut self, _tree: &ast::NodeTree, ty: ast::NodeType, _id: u32) {
        self.push_debug("node", ty);
    }

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
                self.push_string_id("expression_import_target", *target);
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
            ast::Expression::Path { path, .. } => {
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
            ast::Expression::TypeLiteral(literal) => {
                self.push_debug("expression_type_literal", literal);
            }
            ast::Expression::TemplateExpression { value }
            | ast::Expression::TaggedTemplateExpression { value, .. } => {
                self.push_debug("expression_template", value);
            }
            ast::Expression::RangeExpression { is_inclusive, .. } => {
                self.push_debug("expression_range_inclusive", *is_inclusive);
            }
            ast::Expression::TypeUnary { operator, .. } => {
                self.push_debug("expression_type_unary", *operator);
            }
            ast::Expression::TypeBinary { operator, .. } => {
                self.push_debug("expression_type_binary", *operator);
            }
            ast::Expression::TypeMapped {
                parameter,
                modifiers,
                ..
            } => {
                self.push_string_id("expression_type_mapped_name", parameter.name);
                self.push_debug("expression_type_mapped_modifiers", modifiers);
            }
            ast::Expression::TypeTemplateLiteral { strings, .. } => {
                self.push_debug("expression_type_template_len", strings.len());
                for string in strings {
                    self.push_string_id("expression_type_template_string", *string);
                }
            }
            ast::Expression::TypeInfer { name, .. } => {
                self.push_string_id("expression_type_infer", *name);
            }
            ast::Expression::TypePredicate {
                asserts, subject, ..
            } => {
                self.push_debug("expression_type_predicate_asserts", *asserts);
                self.push_debug("expression_type_predicate_subject", subject);
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
                self.push_string_id("expression_member", *name);
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

    fn visit_declaration(
        &mut self,
        tree: &ast::NodeTree,
        id: ast::LocalNodeId<ast::Declaration>,
        declaration: &ast::Declaration,
    ) {
        self.push_debug("declaration", std::mem::discriminant(declaration));
        ast::walk_declaration(self, tree, id, declaration);
    }

    fn visit_property(
        &mut self,
        tree: &ast::NodeTree,
        id: ast::LocalNodeId<ast::Property>,
        property: &ast::Property,
    ) {
        self.push_debug("property", std::mem::discriminant(property));
        ast::walk_property(self, tree, id, property);
    }

    fn visit_member(
        &mut self,
        tree: &ast::NodeTree,
        id: ast::LocalNodeId<ast::Member>,
        member: &ast::Member,
    ) {
        self.push_debug("member", std::mem::discriminant(member));
        ast::walk_member(self, tree, id, member);
    }

    fn visit_parameter(
        &mut self,
        tree: &ast::NodeTree,
        id: ast::LocalNodeId<ast::Parameter>,
        parameter: &ast::Parameter,
    ) {
        self.push_debug("parameter", std::mem::discriminant(parameter));
        ast::walk_parameter(self, tree, id, parameter);
    }

    fn visit_argument(
        &mut self,
        tree: &ast::NodeTree,
        id: ast::LocalNodeId<ast::Argument>,
        argument: &ast::Argument,
    ) {
        self.push_debug("argument", std::mem::discriminant(argument));
        ast::walk_argument(self, tree, id, argument);
    }

    fn visit_pattern(
        &mut self,
        tree: &ast::NodeTree,
        id: ast::LocalNodeId<ast::Pattern>,
        pattern: &ast::Pattern,
    ) {
        self.push_debug("pattern", std::mem::discriminant(pattern));
        ast::walk_pattern(self, tree, id, pattern);
    }

    fn visit_pattern_field(
        &mut self,
        tree: &ast::NodeTree,
        id: ast::LocalNodeId<ast::PatternField>,
        pattern_field: &ast::PatternField,
    ) {
        self.push_debug("pattern_field", std::mem::discriminant(pattern_field));
        ast::walk_pattern_field(self, tree, id, pattern_field);
    }

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
