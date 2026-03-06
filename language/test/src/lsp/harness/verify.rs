use std::fmt::Debug;

use crate::lsp::{
    NormalizedCallHierarchyCall, NormalizedCodeAction, NormalizedCodeLens,
    NormalizedCompletionItem, NormalizedDiagnostic, NormalizedDocumentLink,
    NormalizedDocumentSymbol, NormalizedFoldingRange, NormalizedHierarchyItem, NormalizedHover,
    NormalizedInlayHint, NormalizedLocation, NormalizedQuickInfo, NormalizedResolvedCompletionItem,
    NormalizedSelectionRange, NormalizedSemanticToken, NormalizedSignatureHelp,
    NormalizedWorkspaceSymbol, sorted_diagnostics, sorted_locations,
};

/// Compare two exact values and return a labeled mismatch on failure.
pub fn verify_exact_eq<T>(label: &str, actual: &T, expected: &T) -> Result<(), String>
where
    T: Debug + PartialEq,
{
    if actual == expected {
        return Ok(());
    }

    Err(format!(
        "{label} mismatch\nactual: {actual:#?}\nexpected: {expected:#?}"
    ))
}

/// Compare exact normalized definition locations.
pub fn verify_definition_locations(
    actual: &[NormalizedLocation],
    expected: &[NormalizedLocation],
) -> Result<(), String> {
    let actual = sorted_locations(actual.to_vec());
    let expected = sorted_locations(expected.to_vec());

    verify_exact_eq("definition locations", &actual, &expected)
}

/// Compare exact normalized references locations.
pub fn verify_reference_locations(
    actual: &[NormalizedLocation],
    expected: &[NormalizedLocation],
) -> Result<(), String> {
    let actual = sorted_locations(actual.to_vec());
    let expected = sorted_locations(expected.to_vec());

    verify_exact_eq("reference locations", &actual, &expected)
}

/// Compare the exact count of edits in one workspace edit payload.
pub fn verify_workspace_edit_count(actual: usize, expected: usize) -> Result<(), String> {
    verify_exact_eq("workspace edit count", &actual, &expected)
}

/// Compare exact normalized diagnostics.
pub fn verify_diagnostics(
    actual: &[NormalizedDiagnostic],
    expected: &[NormalizedDiagnostic],
) -> Result<(), String> {
    let actual = sorted_diagnostics(actual.to_vec());
    let expected = sorted_diagnostics(expected.to_vec());

    verify_exact_eq("diagnostics", &actual, &expected)
}

/// Compare one normalized hover payload.
pub fn verify_hover(actual: &NormalizedHover, expected: &NormalizedHover) -> Result<(), String> {
    verify_exact_eq("hover", actual, expected)
}

/// Compare one normalized quick-info payload.
pub fn verify_quick_info(
    actual: &NormalizedQuickInfo,
    expected: &NormalizedQuickInfo,
) -> Result<(), String> {
    verify_exact_eq("quick info", actual, expected)
}

/// Compare one normalized signature help payload.
pub fn verify_signature_help(
    actual: &NormalizedSignatureHelp,
    expected: &NormalizedSignatureHelp,
) -> Result<(), String> {
    verify_exact_eq("signature help", actual, expected)
}

/// Compare exact normalized document symbols.
pub fn verify_document_symbols(
    actual: &[NormalizedDocumentSymbol],
    expected: &[NormalizedDocumentSymbol],
) -> Result<(), String> {
    verify_exact_eq("document symbols", &actual, &expected)
}

/// Compare exact normalized workspace symbols.
pub fn verify_workspace_symbols(
    actual: &[NormalizedWorkspaceSymbol],
    expected: &[NormalizedWorkspaceSymbol],
) -> Result<(), String> {
    verify_exact_eq("workspace symbols", &actual, &expected)
}

/// Compare exact normalized completion items.
pub fn verify_completion_items(
    actual: &[NormalizedCompletionItem],
    expected: &[NormalizedCompletionItem],
) -> Result<(), String> {
    verify_exact_eq("completion items", &actual, &expected)
}

/// Compare one exact resolved completion item.
pub fn verify_resolved_completion_item(
    actual: &NormalizedResolvedCompletionItem,
    expected: &NormalizedResolvedCompletionItem,
) -> Result<(), String> {
    verify_exact_eq("resolved completion item", actual, expected)
}

/// Compare exact normalized document links.
pub fn verify_document_links(
    actual: &[NormalizedDocumentLink],
    expected: &[NormalizedDocumentLink],
) -> Result<(), String> {
    verify_exact_eq("document links", &actual, &expected)
}

/// Compare exact normalized folding ranges.
pub fn verify_folding_ranges(
    actual: &[NormalizedFoldingRange],
    expected: &[NormalizedFoldingRange],
) -> Result<(), String> {
    verify_exact_eq("folding ranges", &actual, &expected)
}

/// Compare exact normalized inlay hints.
pub fn verify_inlay_hints(
    actual: &[NormalizedInlayHint],
    expected: &[NormalizedInlayHint],
) -> Result<(), String> {
    verify_exact_eq("inlay hints", &actual, &expected)
}

/// Compare exact normalized code lenses.
pub fn verify_code_lenses(
    actual: &[NormalizedCodeLens],
    expected: &[NormalizedCodeLens],
) -> Result<(), String> {
    verify_exact_eq("code lenses", &actual, &expected)
}

/// Compare exact file text.
pub fn verify_file_text(actual: &str, expected: &str) -> Result<(), String> {
    verify_exact_eq("file text", &actual, &expected)
}

/// Compare exact current line text.
pub fn verify_current_line_content(actual: &str, expected: &str) -> Result<(), String> {
    verify_exact_eq("current line content", &actual, &expected)
}

/// Compare exact current file text.
pub fn verify_current_file_content(actual: &str, expected: &str) -> Result<(), String> {
    verify_exact_eq("current file content", &actual, &expected)
}

/// Compare exact text at the caret.
pub fn verify_text_at_caret(actual: &str, expected: &str) -> Result<(), String> {
    verify_exact_eq("text at caret", &actual, &expected)
}

/// Compare the active caret location against one marker.
pub fn verify_caret_at_marker(
    actual_file_path: &str,
    actual_offset: usize,
    expected_text_path: &str,
    expected_offset: usize,
) -> Result<(), String> {
    let actual = (actual_file_path, actual_offset);
    let expected = (expected_text_path, expected_offset);

    verify_exact_eq("caret marker", &actual, &expected)
}

/// Verify that a diagnostic list is empty.
pub fn verify_no_errors(actual: &[NormalizedDiagnostic]) -> Result<(), String> {
    let expected: &[NormalizedDiagnostic] = &[];

    verify_exact_eq("diagnostics", &actual, &expected)
}

/// Compare exact normalized code actions.
pub fn verify_code_actions(
    actual: &[NormalizedCodeAction],
    expected: &[NormalizedCodeAction],
) -> Result<(), String> {
    verify_exact_eq("code actions", &actual, &expected)
}

/// Compare exact normalized semantic tokens.
pub fn verify_semantic_tokens(
    actual: &[NormalizedSemanticToken],
    expected: &[NormalizedSemanticToken],
) -> Result<(), String> {
    verify_exact_eq("semantic tokens", &actual, &expected)
}

/// Compare exact normalized call hierarchy edges.
pub fn verify_call_hierarchy_calls(
    actual: &[NormalizedCallHierarchyCall],
    expected: &[NormalizedCallHierarchyCall],
) -> Result<(), String> {
    verify_exact_eq("call hierarchy", &actual, &expected)
}

/// Compare exact normalized type hierarchy items.
pub fn verify_type_hierarchy_items(
    actual: &[NormalizedHierarchyItem],
    expected: &[NormalizedHierarchyItem],
) -> Result<(), String> {
    verify_exact_eq("type hierarchy", &actual, &expected)
}

/// Compare exact normalized selection-range chains.
pub fn verify_selection_ranges(
    actual: &[NormalizedSelectionRange],
    expected: &[NormalizedSelectionRange],
) -> Result<(), String> {
    verify_exact_eq("selection ranges", &actual, &expected)
}
