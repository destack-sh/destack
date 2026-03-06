use crate::lsp::runner::{cancellation, diagnostics, edits};
use crate::lsp::{LspFixture, LspTestState};

/// Run the lifecycle scenarios declared by one fixture.
pub(crate) fn run_lifecycle_cases(
    fixture: &LspFixture,
    test_state: &mut LspTestState,
) -> Result<(), String> {
    diagnostics::run_diagnostic_cases(fixture, test_state)?;
    edits::run_edit_cases(fixture, test_state)?;
    cancellation::run_cancellation_cases(fixture, test_state)?;

    Ok(())
}
