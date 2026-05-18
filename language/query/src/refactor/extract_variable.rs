use destack_source::{BatchEdit, Edit, FileEdit, Span};
use serde::{Deserialize, Serialize};

use super::extract::{
    expression_text_for_insert, line_start_and_indent, resolve_extract_expression,
    statement_span_for_expression,
};
use crate::core::{ModuleQueryContext, QueryRange};
use crate::source::is_simple_identifier;

/// Request payload for extract variable queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractVariableRequest {
    /// The selected source range.
    pub range: QueryRange,
    /// The name for the extracted variable.
    pub new_name: String,
}

/// Response payload for extract variable queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractVariableResponse {
    /// Extract variable edit, if available.
    pub edit: Option<BatchEdit>,
}

/// Extract a selected expression into a const variable in the nearest statement scope.
pub fn extract_variable(
    ctx: &ModuleQueryContext<'_>,
    selection: Span,
    new_name: &str,
) -> Option<BatchEdit> {
    // validate the variable name
    if !is_simple_identifier(new_name) {
        return None;
    }

    // resolve source text for edits
    let source_file = ctx
        .repository()
        .file(ctx.revision(), ctx.file_id())
        .ok()
        .flatten()?;
    let source = source_file.text();

    // resolve the selected expression and source text
    let (expression_id, expression_span) = resolve_extract_expression(ctx, selection)?;
    let expression_text = source_file.span_str(expression_span);
    let expression_text = expression_text_for_insert(expression_text);
    if expression_text.is_empty() {
        return None;
    }

    // avoid no-op extracts when the selection is already the target identifier
    if expression_text == new_name {
        return None;
    }

    // resolve insertion location and indentation
    let statement_span = statement_span_for_expression(ctx, expression_id, expression_span);
    let (line_start, indent) = line_start_and_indent(source, statement_span.start);

    // build replacement edits
    let declaration = format!("{indent}const {new_name} = {expression_text};\n");
    let mut file_edit = FileEdit::new(ctx.file_id());
    file_edit.push(Edit::insert(ctx.file_id(), line_start, declaration));
    file_edit.push(Edit::replace(expression_span, new_name.to_string()));
    file_edit.sort();

    let mut edits = BatchEdit::new();
    edits.push(file_edit);

    Some(edits)
}
