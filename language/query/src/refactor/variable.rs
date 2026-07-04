use destack_serde::Reflect;
use destack_source::{FilePatch, Patch, PatchSet, Span};
use serde::{Deserialize, Serialize};

use super::extract::{ExtractSourceText, LineIndent};
use crate::source::is_simple_identifier;
use crate::{ModuleQueryContext, Range};

/// Request payload for extract variable queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ExtractVariableRequest {
    /// The selected source range.
    pub range: Range,
    /// The name for the extracted variable.
    pub new_name: String,
}

/// Response payload for extract variable queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ExtractVariableResponse {
    /// Extract variable edit, if available.
    pub edit: Option<PatchSet>,
}

impl ModuleQueryContext<'_> {
    /// Extract a selected expression into a const variable in the nearest statement scope.
    pub fn extract_variable(&self, selection: Span, new_name: &str) -> Option<PatchSet> {
        // validate the variable name
        if !is_simple_identifier(new_name) {
            return None;
        }

        // resolve source text for edits
        let source_file = self.source_file();
        let source = source_file.text();

        // resolve the selected expression and source text
        let (expression_id, expression_span) = self.resolve_extract_expression(selection)?;
        let expression_text = source_file.span_str(expression_span);
        let expression_text = expression_text.insertion_expression_text();
        if expression_text.is_empty() {
            return None;
        }

        // avoid no-op extracts when the selection is already the target identifier
        if expression_text == new_name {
            return None;
        }

        // resolve insertion location and indentation
        let statement_span = self.expression_statement_span(expression_id, expression_span);
        let line = LineIndent::at(source, statement_span.start);

        // build replacement edits
        let declaration = format!("{}const {new_name} = {expression_text};\n", line.indent);
        let mut file_edit = FilePatch::new(self.file_id());
        file_edit.push(Patch::insert(self.file_id(), line.start, declaration));
        file_edit.push(Patch::replace(expression_span, new_name.to_string()));
        file_edit.sort();

        let mut edits = PatchSet::new();
        edits.push(file_edit);

        Some(edits)
    }
}
