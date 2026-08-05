use std::slice;

use destack_dir as dir;
use destack_source::{EnclosingSpan, FileId, Span};

use crate::{
    CompletionCandidate, CompletionItemKind, CompletionOrigin, ModuleQueryContext, QueryError,
    QueryResult, SORT_LOCAL_SYMBOL,
};

use super::CompletionContext;
use super::builder::CompletionBuilder;

/// One generated call completion insertion.
pub(super) struct CallSnippet {
    /// The insertion text.
    pub(super) text: String,
    /// Whether the insertion text contains snippet placeholders.
    pub(super) is_snippet: bool,
}

/// One call expression and its argument nodes.
struct CallExpression<'a> {
    /// The call expression.
    id: dir::LocalNodeId<dir::Expression>,
    /// The callee expression on the left side.
    left: dir::LocalNodeId<dir::Expression>,
    /// The argument nodes in source order.
    arguments: &'a [dir::LocalNodeId<dir::Argument>],
}

impl CallSnippet {
    /// Build a call insertion from named parameters.
    pub(super) fn named(function_name: &str, parameter_names: &[String]) -> Self {
        if parameter_names.is_empty() {
            Self {
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

            Self {
                text: format!("{function_name}({parameters})$0"),
                is_snippet: true,
            }
        }
    }

    /// Build a call insertion from positional parameters.
    pub(super) fn positional(function_name: &str, parameter_count: usize) -> Self {
        if parameter_count == 0 {
            Self {
                text: format!("{function_name}()"),
                is_snippet: false,
            }
        } else {
            let parameters = (1..=parameter_count)
                .map(|index| format!("${{{index}}}"))
                .collect::<Vec<_>>()
                .join(", ");

            Self {
                text: format!("{function_name}({parameters})$0"),
                is_snippet: true,
            }
        }
    }

    /// Build a call insertion from one object argument's fields.
    pub(super) fn object(function_name: &str, fields: &[String]) -> Self {
        if fields.is_empty() {
            Self {
                text: format!("{function_name}()"),
                is_snippet: false,
            }
        } else {
            let fields = fields
                .iter()
                .enumerate()
                .map(|(index, field)| format!("{field}: ${{{}}}", index + 1))
                .collect::<Vec<_>>()
                .join(", ");

            Self {
                text: format!("{function_name}({{ {fields} }})$0"),
                is_snippet: true,
            }
        }
    }
}

impl<'a> CallExpression<'a> {
    /// Resolve one call expression from an enclosing span.
    fn from_span(view: dir::View<'a>, span: &EnclosingSpan) -> Option<Self> {
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
            id: expression_id,
            left: *left,
            arguments,
        };

        Some(call)
    }
}

impl ModuleQueryContext<'_> {
    /// Classify new expression completion at one offset.
    pub(super) fn classify_new_expression(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<CompletionContext>> {
        // search the spans enclosing the cursor
        let enclosing = self.enclosing_spans_at_cursor(file_id, offset)?;

        // scan spans for a new expression containing the cursor
        for enclosing_span in &enclosing {
            if let Some(context) =
                self.classify_new_expression_span(file_id, enclosing_span, offset)?
            {
                return Ok(Some(context));
            }
        }

        Ok(None)
    }

    /// Classify call argument completion at one offset.
    pub(super) fn classify_call_argument(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<CompletionContext>> {
        // resolve enclosing spans at the cursor
        let enclosing = self.sorted_enclosing_spans(file_id, offset, offset)?;

        // bail out when there are no spans
        if enclosing.is_empty() {
            return Ok(None);
        }

        let view = self.view()?;

        // scan spans for a call or new expression argument list
        for enclosing_span in &enclosing {
            if let Some(context) = self.classify_call_argument_span(view, enclosing_span, offset)? {
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
                    self.classify_call_argument_separator(file_id, view, separator.span.start)?
            {
                return Ok(Some(context));
            }
        }

        Ok(None)
    }

    /// Classify new expression completion from one enclosing span.
    fn classify_new_expression_span(
        &self,
        file_id: FileId,
        enclosing_span: &EnclosingSpan,
        offset: u32,
    ) -> QueryResult<Option<CompletionContext>> {
        // only expression spans can own one `new` constructor region
        let view = self.view()?;
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
    /// Classify call argument completion from one enclosing span.
    fn classify_call_argument_span(
        &self,
        view: dir::View<'_>,
        enclosing_span: &EnclosingSpan,
        offset: u32,
    ) -> QueryResult<Option<CompletionContext>> {
        let Some(call) = CallExpression::from_span(view, enclosing_span) else {
            return Ok(None);
        };
        let left_span = self.left_expression_span(view, call.left)?;
        let call_span = self.source_index()?.get(enclosing_span.source_id);

        // only the argument list belongs to this path
        if !self.cursor_in_argument_list(view, call.arguments, left_span, call_span, offset)? {
            return Ok(None);
        }

        let Some(scope) = self.expression_scope_at_offset(call.id)? else {
            return Ok(None);
        };
        let expected_type = self.call_argument_type(view, call.id, call.arguments, offset)?;

        Ok(Some(CompletionContext::CallArgument {
            call: call.id,
            scope,
            expected_type,
        }))
    }

    /// Classify call argument completion after one separator.
    fn classify_call_argument_separator(
        &self,
        file_id: FileId,
        view: dir::View<'_>,
        separator_position: u32,
    ) -> QueryResult<Option<CompletionContext>> {
        let Some(lookup_position) = separator_position.checked_sub(1) else {
            return Ok(None);
        };
        let enclosing = self.sorted_enclosing_spans(file_id, lookup_position, lookup_position)?;

        // read the exact authored call and its bound lexical scope
        for enclosing_span in &enclosing {
            let Some(call) = CallExpression::from_span(view, enclosing_span) else {
                continue;
            };
            let left_span = self.left_expression_span(view, call.left)?;
            if separator_position <= left_span.end {
                continue;
            }
            let Some(scope) = self.expression_scope_at_offset(call.id)? else {
                continue;
            };

            return Ok(Some(CompletionContext::CallArgument {
                call: call.id,
                scope,
                expected_type: None,
            }));
        }

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
        let call = self.resolutions()?.call_resolution(call_id);
        let construct = self.resolutions()?.construct_resolution(call_id);
        if call.is_some() && construct.is_some() {
            return Err(QueryError::conflict(format!(
                "completion resolution columns: {call_id:?}"
            )));
        }
        let argument = argument_id.into_global_any(self.module_id());
        let mut types = call
            .into_iter()
            .flat_map(dir::CallResolution::iter)
            .flat_map(|selection| selection.arguments.iter())
            .chain(
                construct
                    .into_iter()
                    .flat_map(|selection| selection.arguments.iter()),
            )
            .filter(|binding| binding.contains_argument(argument))
            .map(|binding| binding.ty)
            .collect::<Vec<_>>();
        if types.is_empty() && call.is_none() && construct.is_none() {
            return Ok(None);
        }
        types.sort();
        types.dedup();

        match types.as_slice() {
            [] => Err(QueryError::missing(format!(
                "completion argument binding: {call_id:?}, {argument:?}"
            ))),
            [type_id] => Ok(Some(*type_id)),
            _ => Ok(None),
        }
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

impl CompletionBuilder<'_, '_, '_> {
    /// Complete the callee's parameter names not yet bound at one call.
    pub(super) fn complete_unbound_parameters(
        &self,
        call: dir::LocalNodeId<dir::Expression>,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let mut results = Vec::new();

        let node = call.into_global_any(self.module.module_id());
        // FUGU #Incomplete: DIR must retain attempted candidates for rejected overloads
        let resolution =
            self.module
                .resolutions()?
                .call_resolution(node)
                .ok_or(QueryError::missing(format!(
                    "completion call selection: {node:?}"
                )))?;

        // union calls offer only the names every runtime arm still accepts
        let selections = match resolution {
            dir::OperationResolution::One(selected) => slice::from_ref(selected),
            dir::OperationResolution::Union { arms, .. } => arms.as_slice(),
        };

        // intersect the unbound names across every arm
        let mut shared: Option<Vec<String>> = None;
        for selected in selections {
            let Some(names) = self.unbound_parameter_names(selected)? else {
                return Ok(results);
            };
            shared = Some(match shared {
                None => names,
                Some(shared) => shared
                    .into_iter()
                    .filter(|name| names.contains(name))
                    .collect(),
            });
        }

        // offer each shared unbound parameter name
        for name in shared.unwrap_or_default() {
            let completion = CompletionCandidate::new(
                name,
                CompletionItemKind::ValueParameter,
                CompletionOrigin::Local,
                SORT_LOCAL_SYMBOL,
            );
            results.push(completion);
        }

        Ok(results)
    }

    /// Return one selected call's parameter names past the bound arguments.
    fn unbound_parameter_names(&self, selected: &dir::Call) -> QueryResult<Option<Vec<String>>> {
        // read the selected target's parameter names
        let names = match selected.target.symbol() {
            Some(symbol) => self
                .program
                .symbol_parameter_names(symbol)?
                .map(|names| names.into_iter().map(Some).collect::<Vec<_>>()),
            None => match &selected.target {
                dir::CallTarget::Dynamic {
                    function:
                        dir::DynamicFunction::CallSignature(source)
                        | dir::DynamicFunction::IndexRead(source)
                        | dir::DynamicFunction::IndexWrite(source)
                        | dir::DynamicFunction::ConstructSignature(source),
                    ..
                } => self.program.node_parameter_names(*source)?,
                _ => None,
            },
        };
        let Some(names) = names else {
            return Ok(None);
        };

        // exclude parameters already bound by positional arguments
        let bound = selected.arguments.len();

        Ok(Some(names.into_iter().skip(bound).flatten().collect()))
    }
}
