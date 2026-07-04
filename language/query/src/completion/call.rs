use destack_dir as dir;
use destack_source::{EnclosingSpan, Span};

use crate::{ExpectedParameterHint, ModuleQueryContext, ScopeAtOffset};

use super::CompletionContext;

/// One generated call completion insertion.
pub(super) struct CallSnippet {
    /// The insertion text.
    pub(super) text: String,
    /// Whether the insertion text uses snippet syntax.
    pub(super) is_snippet: bool,
}

/// The shared call shape for one expression.
struct CallExpression<'a> {
    /// The callee expression on the left side.
    left: dir::LocalNodeId<dir::Expression>,
    /// The argument nodes in source order.
    arguments: &'a [dir::LocalNodeId<dir::Argument>],
}

/// Generate a function-call snippet from one completion label and parameter list.
pub(super) fn call_snippet(function_name: &str, parameter_names: &[String]) -> CallSnippet {
    if parameter_names.is_empty() {
        CallSnippet {
            text: format!("{function_name}()"),
            is_snippet: false,
        }
    } else {
        let parameters = parameter_names
            .iter()
            .enumerate()
            .map(|(index, parameter_name)| format!("${{{}:{}}}", index + 1, parameter_name))
            .collect::<Vec<_>>()
            .join(", ");

        CallSnippet {
            text: format!("{function_name}({parameters})$0"),
            is_snippet: true,
        }
    }
}

impl<'a> CallExpression<'a> {
    /// Resolve one call expression from one enclosing span.
    fn enclosing(
        view: dir::View<'a>,
        span: &EnclosingSpan,
    ) -> Option<(dir::LocalNodeId<dir::Expression>, Self)> {
        let node_id = view.get_node_id_by_source_id(span.source_id)?;
        if node_id.ty != dir::NodeType::Expression {
            return None;
        }

        let expression_id = node_id
            .try_into()
            .unwrap_or_else(|_| panic!("call enclosing node is not an expression: {node_id:?}"));
        let expression = view.get::<dir::Expression>(expression_id);
        let call = Self::try_from(expression).ok()?;

        Some((expression_id, call))
    }
}

impl<'a> TryFrom<&'a dir::Expression> for CallExpression<'a> {
    type Error = ();

    fn try_from(expression: &'a dir::Expression) -> Result<Self, Self::Error> {
        match expression {
            dir::Expression::Call {
                left, arguments, ..
            } => Ok(Self {
                left: *left,
                arguments: arguments.as_slice(),
            }),
            _ => Err(()),
        }
    }
}

impl ModuleQueryContext<'_> {
    /// Return the new expression context.
    pub(super) fn new_expression_cursor_context(&self, offset: u32) -> Option<CompletionContext> {
        // resolve enclosing spans around the cursor boundary
        let enclosing = self.enclosing_spans_at_cursor(offset);

        // scan spans for a new expression containing the cursor
        for enc in &enclosing {
            if let Some(context) = self.new_expression_context(enc, offset) {
                return Some(context);
            }
        }

        None
    }

    /// Return the call argument context.
    pub(super) fn call_argument_cursor_context(&self, offset: u32) -> Option<CompletionContext> {
        // resolve enclosing spans at the cursor
        let enclosing = self.sorted_enclosing_spans(offset, offset);

        // bail out when there are no spans
        if enclosing.is_empty() {
            return None;
        }

        let view = self.view();

        // scan spans for a call or new expression argument list
        for enc in &enclosing {
            if let Some(context) = self.call_argument_context(view, enc, offset) {
                return Some(context);
            }
        }

        // use separator ownership inside a call
        if let Some(separator) = self.previous_significant_token(offset) {
            let is_separator = matches!(
                separator.token.ty(),
                dir::TokenType::OpenParenthesis | dir::TokenType::Comma
            );
            if is_separator {
                if let Some(context) =
                    self.call_argument_context_after_separator(view, offset, separator.span.start)
                {
                    return Some(context);
                }
            }
        }

        None
    }

    /// Build a new expression context from one enclosing span.
    fn new_expression_context(
        &self,
        enc: &EnclosingSpan,
        offset: u32,
    ) -> Option<CompletionContext> {
        // only expression spans can own one `new` constructor region
        if self.tree().get_node_type(enc.source_id) != dir::NodeType::Expression {
            return None;
        }

        let expr_id = dir::LocalNodeId::<dir::Expression>::new(enc.source_id);
        let parsed_tree = self.tree();
        let expr = parsed_tree.get(expr_id);
        let dir::Expression::New { ty, .. } = expr else {
            return None;
        };

        // only explicit constructor text belongs to this path
        if matches!(parsed_tree.get(*ty), dir::TypeExpression::Missing) {
            return None;
        }

        // only the constructor side should classify as one `new` completion position
        let type_span = parsed_tree.source_index.get(ty.id);
        if !type_span.owns_cursor(offset) {
            return None;
        }

        let scope = self.scope_at_offset(offset)?;

        Some(CompletionContext::NewExpression { scope })
    }
}

impl ModuleQueryContext<'_> {
    /// Build a call argument context from one enclosing span.
    fn call_argument_context(
        &self,
        view: dir::View<'_>,
        enc: &EnclosingSpan,
        offset: u32,
    ) -> Option<CompletionContext> {
        let (expr_id, call) = CallExpression::enclosing(view, enc)?;
        let left_span = self.left_expression_span(view, call.left);
        let call_span = self.tree().source_index.get(enc.source_id);

        // only the argument list belongs to this path
        if !self.cursor_in_argument_list(view, call.arguments, left_span, call_span, offset) {
            return None;
        }

        let scope = self.expression_scope_at_offset(expr_id, offset)?;
        let active_parameter = self.active_argument_index(view, call.arguments, offset);
        let expected_parameter = self.expected_parameter_hint(call.left, active_parameter);

        Some(CompletionContext::CallArgument {
            scope,
            expected_parameter,
        })
    }

    /// Build a call argument context from a separator position inside a call.
    fn call_argument_context_after_separator(
        &self,
        view: dir::View<'_>,
        offset: u32,
        separator_position: u32,
    ) -> Option<CompletionContext> {
        let lookup_position = separator_position.saturating_sub(1);
        let enclosing = self.sorted_enclosing_spans(lookup_position, lookup_position);

        // prefer checked call shapes first
        for enc in &enclosing {
            let Some((expr_id, call)) = CallExpression::enclosing(view, enc) else {
                continue;
            };
            let left_span = self.left_expression_span(view, call.left);

            if separator_position <= left_span.end {
                continue;
            }

            let scope = self.expression_scope_at_offset(expr_id, offset)?;
            let active_parameter = call.arguments.len();
            let expected_parameter = self.expected_parameter_hint(call.left, active_parameter);

            return Some(CompletionContext::CallArgument {
                scope,
                expected_parameter,
            });
        }

        // use source call shapes that are still being edited
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

            let scope = self.call_argument_scope_near(&[offset, lookup_position])?;
            return Some(CompletionContext::CallArgument {
                scope,
                expected_parameter: None,
            });
        }

        None
    }
}

impl ModuleQueryContext<'_> {
    /// Resolve the active argument index inside one call.
    fn active_argument_index(
        &self,
        view: dir::View<'_>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        offset: u32,
    ) -> usize {
        if arguments.is_empty() {
            return 0;
        }

        let mut active_index = 0usize;
        for (index, argument_id) in arguments.iter().enumerate() {
            let span = self.dir_node_span(view, (*argument_id).into());

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
        &self,
        left_expression_id: dir::LocalNodeId<dir::Expression>,
        parameter_index: usize,
    ) -> Option<ExpectedParameterHint> {
        let target = self.signature_target(left_expression_id);
        let symbol_id = target.symbol_id?;

        self.symbol_expected_parameter(symbol_id, parameter_index)
    }

    /// Resolve the source span for one dir node.
    fn dir_node_span(&self, view: dir::View<'_>, node_id: dir::LocalNodeIdAny) -> Span {
        let source_id = view.get_source_any(node_id);
        self.tree().source_index.get(source_id)
    }

    /// Resolve the source span for one call target expression.
    fn left_expression_span(
        &self,
        view: dir::View<'_>,
        left: dir::LocalNodeId<dir::Expression>,
    ) -> Span {
        let left_node_id: dir::LocalNodeIdAny = left.into();
        self.dir_node_span(view, left_node_id)
    }

    /// Resolve a call argument scope from a small set of nearby offsets.
    fn call_argument_scope_near(&self, offsets: &[u32]) -> Option<ScopeAtOffset> {
        for &offset in offsets {
            if let Some(scope) = self.block_scope_at_offset(offset) {
                return Some(scope);
            }
        }

        for &offset in offsets {
            if let Some(scope) = self.scope_at_offset(offset) {
                return Some(scope);
            }
        }

        None
    }
}

impl ModuleQueryContext<'_> {
    /// Check whether the cursor is inside a call argument list.
    fn cursor_in_argument_list(
        &self,
        view: dir::View<'_>,
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
                let span = self.get_span(view, arg_node_id);
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
