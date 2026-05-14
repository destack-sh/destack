use destack_source::{BatchEdit, Edit, FileEdit, FileId, Span, Uri};
use destack_workspace::{Repository, Revision};
use serde::{Deserialize, Serialize};

use super::extract::{
    clean_expression_text, line_start_and_indent, resolve_extract_expression,
    statement_span_for_expression,
};
use crate::core::query_context;
use crate::source::{get_module_by_file_id, is_simple_identifier};

/// Request payload for extract variable queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractVariableRequest {
    /// The document URI.
    pub uri: Uri,
    /// The start byte offset of the selection.
    pub start: u32,
    /// The end byte offset of the selection.
    pub end: u32,
    /// The name for the extracted variable.
    pub new_name: String,
}

/// Response payload for extract variable queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractVariableResponse {
    /// Extract variable result, if available.
    pub result: Option<ExtractVariableResult>,
}

/// Result of an extract variable query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractVariableResult {
    /// All edits to apply.
    pub edits: BatchEdit,
}

impl ExtractVariableResult {
    /// Create a result from a batch edit.
    pub fn from_edits(edits: BatchEdit) -> Self {
        Self { edits }
    }

    /// Whether the result has no edits.
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }
}

/// Extract a selected expression into a const variable in the nearest statement scope.
pub fn extract_variable(
    repository: &Repository,
    revision: Revision,
    file: FileId,
    selection: Span,
    new_name: &str,
) -> Option<ExtractVariableResult> {
    // validate the variable name
    if !is_simple_identifier(new_name) {
        return None;
    }

    // resolve the module and query context
    let module = get_module_by_file_id(repository, revision, file)?;
    let ctx = query_context(repository, revision, module.id)?;

    // resolve source text for edits
    let source_file = repository.file(revision, file).ok().flatten()?;
    let source = source_file.text();

    // resolve the selected expression and source text
    let (expression_id, expression_span) = resolve_extract_expression(&ctx, selection)?;
    let expression_text = source_file.span_str(expression_span);
    let expression_text = clean_expression_text(expression_text);
    if expression_text.is_empty() {
        return None;
    }

    // avoid no-op extracts when the selection is already the target identifier
    if expression_text == new_name {
        return None;
    }

    // resolve insertion location and indentation
    let statement_span = statement_span_for_expression(&ctx, expression_id, expression_span);
    let (line_start, indent) = line_start_and_indent(source, statement_span.start);

    // build replacement edits
    let declaration = format!("{indent}const {new_name} = {expression_text};\n");
    let mut file_edit = FileEdit::new(file);
    file_edit.push(Edit::insert(file, line_start, declaration));
    file_edit.push(Edit::replace(expression_span, new_name.to_string()));
    file_edit.sort();

    let mut edits = BatchEdit::new();
    edits.push(file_edit);

    Some(ExtractVariableResult::from_edits(edits))
}
