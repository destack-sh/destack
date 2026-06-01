use destack_dir as dir;
use destack_source::{EnclosingSpan, Span};

use crate::core::DirQueryContext;
use crate::dir::{ExpectedParameterHint, ScopeAtOffset};
use crate::source::span_owns_cursor;

use super::CompletionContext;

/// The shared call shape for one DIR expression.
struct DirCallExpression<'a> {
    /// The callee expression on the left side.
    left: dir::LocalNodeId<dir::Expression>,
    /// The argument nodes in source order.
    arguments: &'a [dir::LocalNodeId<dir::Argument>],
}

impl DirQueryContext<'_> {
    /// Detect whether the cursor is in a new expression context.
    pub(super) fn detect_new_expression_context(self, offset: u32) -> Option<CompletionContext> {
        // resolve enclosing spans around the cursor boundary
        let enclosing = self.enclosing_spans_at_cursor(offset);

        // scan spans for a new expression containing the cursor
        for enc in &enclosing {
            if let Some(context) = self.new_expression_context_for_span(enc, offset) {
                return Some(context);
            }
        }

        None
    }

    /// Detect whether the cursor is in a call argument context.
    pub(super) fn detect_call_argument_context(self, offset: u32) -> Option<CompletionContext> {
        // resolve enclosing spans at the cursor
        let enclosing = self.sorted_enclosing_spans(offset, offset);

        // bail out when there are no spans
        if enclosing.is_empty() {
            return None;
        }

        let dir_tree = self.view();

        // scan spans for a call or new expression argument list
        for enc in &enclosing {
            if let Some(context) = self.call_argument_context_for_span(dir_tree, enc, offset) {
                return Some(context);
            }
        }

        // recover separator ownership inside a call
        if let Some(separator) = self.previous_significant_token(offset)
            && matches!(
                separator.token.ty(),
                dir::TokenType::OpenParenthesis | dir::TokenType::Comma
            )
            && let Some(context) =
                self.call_argument_context_after_separator(dir_tree, offset, separator.span.start)
        {
            return Some(context);
        }

        None
    }

    /// Build a new expression context from a single enclosing span.
    fn new_expression_context_for_span(
        self,
        enc: &EnclosingSpan,
        offset: u32,
    ) -> Option<CompletionContext> {
        // only expression spans can own one `new` constructor region
        if self.tree().get_node_type(enc.source_id) != dir::NodeType::Expression {
            return None;
        }

        let expr_id = dir::LocalNodeId::<dir::Expression>::new(enc.source_id);
        let parsed_tree = self.tree();
        let (_expr_id, expr) = unwrap_statement_expression(parsed_tree, expr_id);
        let dir::Expression::New { ty, .. } = expr else {
            return None;
        };

        // only explicit constructor text belongs to this path
        if matches!(parsed_tree.get(*ty), dir::TypeExpression::Missing) {
            return None;
        }

        // only the constructor side should classify as one `new` completion position
        let type_span = parsed_tree.source_index.get(ty.id);
        if !span_owns_cursor(type_span, offset) {
            return None;
        }

        let scope = self.scope_at_offset(offset);

        Some(CompletionContext::NewExpression {
            scope_id: scope.map(|scope| scope.scope_id),
            scope_mark: scope.map(|scope| scope.scope_mark),
        })
    }
}

/// Unwrap statement expressions to the underlying inner expression.
fn unwrap_statement_expression(
    parsed_tree: &dir::Tree,
    expr_id: dir::LocalNodeId<dir::Expression>,
) -> (dir::LocalNodeId<dir::Expression>, &dir::Expression) {
    let expr = parsed_tree.get(expr_id);
    (expr_id, expr)
}

impl DirQueryContext<'_> {
    /// Build a call argument context from a single enclosing span.
    fn call_argument_context_for_span(
        self,
        dir_tree: dir::View<'_>,
        enc: &EnclosingSpan,
        offset: u32,
    ) -> Option<CompletionContext> {
        let (expr_id, call) = dir_call_expression_for_enclosing_span(dir_tree, enc)?;
        let left_span = self.left_expression_span(dir_tree, call.left);
        let call_span = self.tree().source_index.get(enc.source_id);

        // only the argument list belongs to this path
        if !self.cursor_in_argument_list(dir_tree, call.arguments, left_span, call_span, offset) {
            return None;
        }

        let scope = self.expression_scope_at_offset(expr_id, offset);
        let active_parameter = self.active_argument_index(dir_tree, call.arguments, offset);
        let expected_parameter = self.expected_parameter_hint(call.left, active_parameter);

        Some(CompletionContext::CallArgument {
            scope_id: Some(scope.scope_id),
            scope_mark: Some(scope.scope_mark),
            expected_parameter,
        })
    }

    /// Build a call argument context from a separator position inside a call.
    fn call_argument_context_after_separator(
        self,
        dir_tree: dir::View<'_>,
        offset: u32,
        separator_position: u32,
    ) -> Option<CompletionContext> {
        let lookup_position = separator_position.saturating_sub(1);
        let enclosing = self.sorted_enclosing_spans(lookup_position, lookup_position);

        // prefer dir backed call shapes first
        for enc in &enclosing {
            let Some((expr_id, call)) = dir_call_expression_for_enclosing_span(dir_tree, enc)
            else {
                continue;
            };
            let left_span = self.left_expression_span(dir_tree, call.left);

            if separator_position <= left_span.end {
                continue;
            }

            let scope = self.expression_scope_at_offset(expr_id, offset);
            let active_parameter = call.arguments.len();
            let expected_parameter = self.expected_parameter_hint(call.left, active_parameter);

            return Some(CompletionContext::CallArgument {
                scope_id: Some(scope.scope_id),
                scope_mark: Some(scope.scope_mark),
                expected_parameter,
            });
        }

        // recover source call shapes that are still being edited
        for enc in &enclosing {
            if self.tree().get_node_type(enc.source_id) != dir::NodeType::Expression {
                continue;
            }

            let expr_id = dir::LocalNodeId::<dir::Expression>::new(enc.source_id);
            let expr = self.tree().get(expr_id);

            let head_end = match expr {
                dir::Expression::Call { left, .. } => self.tree().source_index.get(left.id).end,
                dir::Expression::New { ty, .. } => self.tree().source_index.get(ty.id).end,
                _ => continue,
            };

            if separator_position <= head_end {
                continue;
            }

            let scope = self.call_argument_scope_from_offsets(&[offset, lookup_position])?;
            return Some(CompletionContext::CallArgument {
                scope_id: Some(scope.scope_id),
                scope_mark: Some(scope.scope_mark),
                expected_parameter: None,
            });
        }

        None
    }
}

/// Resolve one DIR call expression from one enclosing span.
fn dir_call_expression_for_enclosing_span<'a>(
    dir_tree: dir::View<'a>,
    enc: &EnclosingSpan,
) -> Option<(dir::LocalNodeId<dir::Expression>, DirCallExpression<'a>)> {
    let dir_node_id = dir_tree.get_node_id_by_source_id(enc.source_id)?;
    if dir_node_id.ty != dir::NodeType::Expression {
        return None;
    }

    let Ok(expr_id) = dir_node_id.try_into() else {
        return None;
    };
    let expr: &dir::Expression = dir_tree.get(expr_id);
    let call = dir_call_expression(expr)?;

    Some((expr_id, call))
}

/// Resolve the shared call shape for one DIR expression.
fn dir_call_expression(expr: &dir::Expression) -> Option<DirCallExpression<'_>> {
    match expr {
        dir::Expression::Call {
            left, arguments, ..
        } => Some(DirCallExpression {
            left: *left,
            arguments: arguments.as_slice(),
        }),
        _ => None,
    }
}

impl DirQueryContext<'_> {
    /// Resolve the active argument index inside one call.
    fn active_argument_index(
        self,
        dir_tree: dir::View<'_>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        offset: u32,
    ) -> usize {
        if arguments.is_empty() {
            return 0;
        }

        let mut active_index = 0usize;
        for (index, argument_id) in arguments.iter().enumerate() {
            let span = self.dir_node_span(dir_tree, (*argument_id).into());

            if offset < span.start {
                break;
            }

            active_index = index;
            if offset <= span.end {
                break;
            }
        }

        active_index
    }

    /// Resolve one expected-parameter hint for one call target.
    fn expected_parameter_hint(
        self,
        left_expression_id: dir::LocalNodeId<dir::Expression>,
        parameter_index: usize,
    ) -> Option<ExpectedParameterHint> {
        let _ctx = self;
        let target = self.call_target(left_expression_id);
        let symbol_id = target.symbol?;

        let ctx = self.module_context(self.module_id())?;
        ctx.expected_parameter_hint_for_symbol(symbol_id, parameter_index)
    }

    /// Resolve the source span for one dir node.
    fn dir_node_span(self, dir_tree: dir::View<'_>, node_id: dir::LocalNodeIdAny) -> Span {
        let source_id = dir_tree.get_source_any(node_id);
        self.tree().source_index.get(source_id)
    }

    /// Resolve the source span for one call target expression.
    fn left_expression_span(
        self,
        dir_tree: dir::View<'_>,
        left: dir::LocalNodeId<dir::Expression>,
    ) -> Span {
        let left_node_id: dir::LocalNodeIdAny = left.into();
        self.dir_node_span(dir_tree, left_node_id)
    }

    /// Resolve a call argument scope from a small set of nearby offsets.
    fn call_argument_scope_from_offsets(self, offsets: &[u32]) -> Option<ScopeAtOffset> {
        for &offset in offsets {
            if let Some(scope) = self.block_scope_at_offset(offset) {
                return Some(normalize_call_argument_scope(scope));
            }
        }

        for &offset in offsets {
            if let Some(scope) = self.scope_at_offset(offset) {
                return Some(normalize_call_argument_scope(scope));
            }
        }

        None
    }
}

/// Normalize the scope used for one call argument position.
fn normalize_call_argument_scope(scope: ScopeAtOffset) -> ScopeAtOffset {
    let scope_mark = if scope.scope_mark == dir::LocalScopeMark(0) {
        dir::LocalScopeMark::end()
    } else {
        scope.scope_mark
    };

    ScopeAtOffset {
        scope_id: scope.scope_id,
        scope_mark,
    }
}

impl DirQueryContext<'_> {
    /// Check whether the cursor is inside a call argument list.
    fn cursor_in_argument_list(
        self,
        dir_tree: dir::View<'_>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        left_span: Span,
        call_span: Span,
        offset: u32,
    ) -> bool {
        // check argument spans when arguments are present
        if !arguments.is_empty() {
            let mut min_start = u32::MAX;
            let mut max_end = 0u32;

            for argument_id in arguments {
                let arg_node_id: dir::LocalNodeIdAny = (*argument_id).into();
                let span = self.span_for_dir_node(dir_tree, arg_node_id);
                min_start = min_start.min(span.start);
                max_end = max_end.max(span.end);
            }

            if offset >= min_start && offset <= max_end {
                return true;
            }
        }

        offset > left_span.end && offset <= call_span.end
    }
}
