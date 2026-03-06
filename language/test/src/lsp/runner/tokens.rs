use destack_lsp_types as lsp;

use crate::lsp::runner::primary_file_path;
use crate::lsp::{
    LspFixture, LspTestState, NormalizedSemanticToken, normalize_semantic_tokens,
    parse_expected_semantic_tokens, verify_semantic_tokens,
};

/// Run the semantic-token scenarios declared by one fixture.
pub(crate) fn run_token_cases(
    fixture: &LspFixture,
    test_state: &mut LspTestState,
) -> Result<(), String> {
    // semantic tokens
    if let Some(expected_snapshot) = fixture
        .expectations
        .semantic_tokens
        .full_expected_text
        .as_deref()
    {
        let source_file_path = primary_file_path(fixture)?;
        let expected_tokens = parse_expected_semantic_tokens(&expected_snapshot)?;

        test_state.go_to().file(&source_file_path)?;

        let tokens = test_state
            .request_semantic_tokens_full()?
            .ok_or_else(|| "expected semantic tokens result".to_string())?;
        let source_text = test_state.current_document_text(&source_file_path)?;
        let actual_tokens = normalize_semantic_tokens(source_text, &tokens)?;

        verify_semantic_tokens(&actual_tokens, &expected_tokens)?;
    }

    // ranged semantic tokens
    if let Some(expected_snapshot) = fixture
        .expectations
        .semantic_tokens
        .range_expected_text
        .as_deref()
    {
        let source_file_path = primary_file_path(fixture)?;
        let expected_tokens = parse_expected_semantic_tokens(&expected_snapshot)?;
        let selected_range =
            fixture.ranges.first().cloned().ok_or_else(|| {
                "semantic token range fixture is missing [|...|] range".to_string()
            })?;

        test_state.go_to().file(&source_file_path)?;

        let tokens = test_state
            .request_semantic_tokens_range_for_range(&selected_range)?
            .ok_or_else(|| "expected semantic token range result".to_string())?;
        let source_text = test_state.current_document_text(&source_file_path)?;
        let actual_tokens = normalize_semantic_tokens_range(source_text, &tokens)?;

        verify_semantic_tokens(&actual_tokens, &expected_tokens)?;
    }

    // semantic token delta
    if let (Some(delta_source_path), Some(expected_text_path)) = (
        fixture
            .expectations
            .semantic_tokens
            .delta_source_text
            .as_deref(),
        fixture
            .expectations
            .semantic_tokens
            .delta_expected_text
            .as_deref(),
    ) {
        let source_file_path = primary_file_path(fixture)?;
        let changed_source = delta_source_path;
        let expected_snapshot = expected_text_path;
        let expected_tokens = parse_expected_semantic_tokens(&expected_snapshot)?;

        test_state.go_to().file(&source_file_path)?;

        let baseline_result = test_state
            .request_semantic_tokens_full()?
            .ok_or_else(|| "expected baseline semantic token result".to_string())?;
        let baseline_result_id = semantic_tokens_result_id(&baseline_result)?
            .ok_or_else(|| "expected baseline semantic token result id".to_string())?;

        test_state.replace_document_text(&source_file_path, &changed_source)?;
        let _ = test_state.wait_for_diagnostics_version(&source_file_path, 2)?;
        let delta_result = test_state
            .request_semantic_tokens_full_delta(baseline_result_id)?
            .ok_or_else(|| "expected semantic token delta result".to_string())?;
        let merged_result = semantic_tokens_after_delta(&baseline_result, &delta_result)?;
        let actual_tokens = normalize_semantic_tokens(&changed_source, &merged_result)?;

        verify_semantic_tokens(&actual_tokens, &expected_tokens)?;
    }

    Ok(())
}

/// Normalize one semantic token range response into exact comparable tokens.
fn normalize_semantic_tokens_range(
    text: &str,
    result: &lsp::SemanticTokensRangeResult,
) -> Result<Vec<NormalizedSemanticToken>, String> {
    let result = match result {
        lsp::SemanticTokensRangeResult::Tokens(tokens) => {
            lsp::SemanticTokensResult::Tokens(tokens.clone())
        }
        lsp::SemanticTokensRangeResult::Partial(partial) => {
            lsp::SemanticTokensResult::Partial(partial.clone())
        }
    };

    normalize_semantic_tokens(text, &result)
}

/// Return the current semantic token result id when the response carries one.
fn semantic_tokens_result_id(result: &lsp::SemanticTokensResult) -> Result<Option<String>, String> {
    match result {
        lsp::SemanticTokensResult::Tokens(tokens) => Ok(tokens.result_id.clone()),
        lsp::SemanticTokensResult::Partial(_) => {
            Err("semantic token partial results do not carry result ids".to_string())
        }
    }
}

/// Apply one semantic token delta result to a baseline token response.
fn semantic_tokens_after_delta(
    baseline: &lsp::SemanticTokensResult,
    delta: &lsp::SemanticTokensFullDeltaResult,
) -> Result<lsp::SemanticTokensResult, String> {
    match delta {
        lsp::SemanticTokensFullDeltaResult::Tokens(tokens) => {
            Ok(lsp::SemanticTokensResult::Tokens(tokens.clone()))
        }
        lsp::SemanticTokensFullDeltaResult::TokensDelta(delta) => {
            apply_semantic_token_delta_edits(baseline, &delta.edits)
        }
        lsp::SemanticTokensFullDeltaResult::PartialTokensDelta { edits } => {
            apply_semantic_token_delta_edits(baseline, edits)
        }
    }
}

/// Apply semantic token edits to a baseline token stream.
fn apply_semantic_token_delta_edits(
    baseline: &lsp::SemanticTokensResult,
    edits: &[lsp::SemanticTokensEdit],
) -> Result<lsp::SemanticTokensResult, String> {
    let mut tokens = match baseline {
        lsp::SemanticTokensResult::Tokens(tokens) => tokens.data.clone(),
        lsp::SemanticTokensResult::Partial(partial) => partial.data.clone(),
    };

    // rewrite token slices in protocol order
    for edit in edits {
        let start = edit.start as usize;
        let end = start + edit.delete_count as usize;
        if end > tokens.len() {
            return Err("semantic token delta edit exceeds baseline token stream".to_string());
        }

        let replacement = edit.data.clone().unwrap_or_default();
        tokens.splice(start..end, replacement);
    }

    Ok(lsp::SemanticTokensResult::Tokens(lsp::SemanticTokens {
        result_id: None,
        data: tokens,
    }))
}
