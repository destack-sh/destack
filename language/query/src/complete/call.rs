use destack_dir as dir;
use destack_source::{EnclosingSpan, FileId, Span};

use crate::{ModuleQueryContext, QueryError, QueryResult};

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

        let expression_id = dir::LocalNodeId::<dir::Expression>::new(node_id.id);
        let expression = view.get::<dir::Expression>(expression_id);
        let dir::Expression::Call {
            left, arguments, ..
        } = expression
        else {
            return None;
        };
        let call = Self {
            left: *left,
            arguments,
        };

        Some((expression_id, call))
    }
}

impl ModuleQueryContext<'_> {
    /// Return the new expression context.
    pub(super) fn new_expression_cursor_context(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<CompletionContext>> {
        // resolve enclosing spans around the cursor boundary
        let enclosing = self.enclosing_spans_at_cursor(file_id, offset)?;

        // scan spans for a new expression containing the cursor
        for enclosing_span in &enclosing {
            if let Some(context) = self.new_expression_context(file_id, enclosing_span, offset)? {
                return Ok(Some(context));
            }
        }

        Ok(None)
    }

    /// Return the call argument context.
    pub(super) fn call_argument_cursor_context(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<CompletionContext>> {
        // resolve enclosing spans at the cursor
        let enclosing = self.sorted_enclosing_spans(file_id, offset, offset);

        // bail out when there are no spans
        if enclosing.is_empty() {
            return Ok(None);
        }

        let view = self.view();

        // scan spans for a call or new expression argument list
        for enclosing_span in &enclosing {
            if let Some(context) = self.call_argument_context(view, enclosing_span, offset)? {
                return Ok(Some(context));
            }
        }

        // use separator ownership inside a call
        if let Some(separator) = self.previous_significant_token(file_id, offset)? {
            let is_separator = matches!(
                separator.token.ty(),
                dir::TokenType::OpenParenthesis | dir::TokenType::Comma
            );
            if is_separator
                && let Some(context) =
                    self.call_argument_context_after_separator(file_id, view, separator.span.start)?
            {
                return Ok(Some(context));
            }
        }

        Ok(None)
    }

    /// Build a new expression context from one enclosing span.
    fn new_expression_context(
        &self,
        file_id: FileId,
        enclosing_span: &EnclosingSpan,
        offset: u32,
    ) -> QueryResult<Option<CompletionContext>> {
        // only expression spans can own one `new` constructor region
        let view = self.view();
        let Some(node_id) = view.get_node_id_by_source_id(enclosing_span.source_id) else {
            return Ok(None);
        };
        if node_id.ty != dir::NodeType::Expression {
            return Ok(None);
        }

        let expression_id = dir::LocalNodeId::<dir::Expression>::new(node_id.id);
        let expression = view.get(expression_id);
        let dir::Expression::New {
            ty: type_expression,
            ..
        } = expression
        else {
            return Ok(None);
        };

        // only explicit constructor text belongs to this path
        if matches!(view.get(*type_expression), dir::TypeExpression::Missing) {
            return Ok(None);
        }

        // only the constructor side should classify as one `new` completion position
        let type_span = view.get_span(*type_expression);
        if !type_span.owns_cursor(offset) {
            return Ok(None);
        }

        let Some(scope) = self.scope_at_offset(file_id, offset)? else {
            return Ok(None);
        };

        Ok(Some(CompletionContext::NewExpression { scope }))
    }
}

impl ModuleQueryContext<'_> {
    /// Build a call argument context from one enclosing span.
    fn call_argument_context(
        &self,
        view: dir::View<'_>,
        enclosing_span: &EnclosingSpan,
        offset: u32,
    ) -> QueryResult<Option<CompletionContext>> {
        let Some((expression_id, call)) = CallExpression::enclosing(view, enclosing_span) else {
            return Ok(None);
        };
        let left_span = self.left_expression_span(view, call.left)?;
        let call_span = self.source_index().get(enclosing_span.source_id);

        // only the argument list belongs to this path
        if !self.cursor_in_argument_list(view, call.arguments, left_span, call_span, offset)? {
            return Ok(None);
        }

        let Some(scope) = self.expression_scope_at_offset(expression_id) else {
            return Ok(None);
        };
        let expected_type = self.call_argument_type(view, expression_id, call.arguments, offset)?;

        Ok(Some(CompletionContext::CallArgument {
            scope,
            expected_type,
        }))
    }

    /// Build a call argument context from a separator position inside a call.
    fn call_argument_context_after_separator(
        &self,
        file_id: FileId,
        view: dir::View<'_>,
        separator_position: u32,
    ) -> QueryResult<Option<CompletionContext>> {
        let Some(lookup_position) = separator_position.checked_sub(1) else {
            return Ok(None);
        };
        let enclosing = self.sorted_enclosing_spans(file_id, lookup_position, lookup_position);

        // read only checked call shapes
        for enclosing_span in &enclosing {
            let Some((expression_id, call)) = CallExpression::enclosing(view, enclosing_span)
            else {
                continue;
            };
            let left_span = self.left_expression_span(view, call.left)?;

            if separator_position <= left_span.end {
                continue;
            }

            let Some(scope) = self.expression_scope_at_offset(expression_id) else {
                continue;
            };

            return Ok(Some(CompletionContext::CallArgument {
                scope,
                expected_type: None,
            }));
        }

        // FUGU #Incomplete: bind edited call separators to their exact expression scope
        Ok(None)
    }

    /// Return the selected parameter type for the argument under one cursor.
    fn call_argument_type(
        &self,
        view: dir::View<'_>,
        expression_id: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        offset: u32,
    ) -> QueryResult<Option<dir::GlobalTypeId>> {
        let mut selected_argument = None;
        for argument_id in arguments.iter().copied() {
            let span = self.node_span(view, argument_id.into())?;
            if span.owns_cursor(offset) {
                selected_argument = Some(argument_id);
                break;
            }
        }
        let Some(argument_id) = selected_argument else {
            return Ok(None);
        };
        let call_id = expression_id.into_global_any(self.module_id());
        let Some(resolution) = self.resolutions().call_resolution(call_id) else {
            return Ok(None);
        };
        let argument = argument_id.into_global_any(self.module_id());
        let binding = resolution
            .arguments
            .iter()
            .find(|binding| binding_contains_argument(binding, argument))
            .ok_or(QueryError::missing(format!(
                "completion argument binding: {call_id:?}, {argument:?}"
            )))?;

        Ok(Some(binding.ty))
    }
}

impl ModuleQueryContext<'_> {
    /// Resolve the source span for one call target expression.
    fn left_expression_span(
        &self,
        view: dir::View<'_>,
        left: dir::LocalNodeId<dir::Expression>,
    ) -> QueryResult<Span> {
        let left_node_id: dir::LocalNodeIdAny = left.into();

        self.node_span(view, left_node_id)
    }
}

/// Return whether one checked binding owns a source argument.
fn binding_contains_argument(
    binding: &dir::ArgumentBinding,
    argument: dir::GlobalNodeIdAny,
) -> bool {
    match &binding.argument {
        dir::ArgumentSource::Provided(source) => *source == argument,
        dir::ArgumentSource::Rest(sources) => sources.contains(&argument),
        dir::ArgumentSource::Static(_) | dir::ArgumentSource::Omitted => false,
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
    ) -> QueryResult<bool> {
        // check the exact authored argument bounds when arguments are present
        if let Some((first, rest)) = arguments.split_first() {
            let first_node_id: dir::LocalNodeIdAny = (*first).into();
            let first_span = self.node_span(view, first_node_id)?;
            let mut min_start = first_span.start;
            let mut max_end = first_span.end;

            for argument_id in rest {
                let argument_node_id: dir::LocalNodeIdAny = (*argument_id).into();
                let span = self.node_span(view, argument_node_id)?;
                min_start = min_start.min(span.start);
                max_end = max_end.max(span.end);
            }

            if offset >= min_start && offset <= max_end {
                return Ok(true);
            }
        }

        Ok(offset > left_span.end && offset <= call_span.end)
    }
}
