use crate::lsp::runner::diagnostics;
use crate::lsp::{LspFixture, LspTestState};

/// Run the lifecycle cases declared by one fixture.
pub(crate) fn run_lifecycle_cases(
    fixture: &LspFixture,
    test_state: &mut LspTestState,
) -> Result<(), String> {
    diagnostics::run_diagnostic_cases(fixture, test_state)?;

    Ok(())
}
