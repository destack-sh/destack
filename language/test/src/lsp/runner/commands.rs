use crate::lsp::{LspFixture, LspTestState};

/// Run the workspace-command requests declared by one fixture.
pub(crate) fn run_command_cases(
    fixture: &LspFixture,
    test_state: &mut LspTestState,
) -> Result<(), String> {
    // execute commands before later capability runners assert post-command behavior
    for command in &fixture.expectations.commands.execute {
        test_state.execute_command(command)?;
    }

    Ok(())
}
