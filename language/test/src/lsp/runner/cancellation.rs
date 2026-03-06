use destack_lsp_server::jsonrpc::ErrorCode;
use destack_lsp_types as lsp;

use crate::lsp::{LspFixture, LspScenario, LspTestState};

/// Run the cancellation scenarios declared by one fixture.
pub(crate) fn run_cancellation_cases(
    fixture: &LspFixture,
    test_state: &mut LspTestState,
) -> Result<(), String> {
    // work-done cancellation on long references requests
    if fixture.has_scenario(LspScenario::ReferencesProgressCancel) {
        let work_done_token = lsp::ProgressToken::String("refs-cancel".to_string());

        test_state.go_to_marker("cancel")?;
        let request_id =
            test_state.start_references_with_progress(true, work_done_token.clone())?;
        test_state.wait_for_work_done_progress_kind(&work_done_token, "begin")?;
        test_state.cancellation().cancel_progress(work_done_token);

        let response = test_state.await_request(request_id)?;
        let error = response
            .error()
            .ok_or_else(|| "expected references cancellation error response".to_string())?;
        if error.code != ErrorCode::RequestCancelled {
            return Err(format!(
                "references cancellation returned unexpected error code {}\nerror: {error:#?}",
                error.code
            ));
        }
    }

    // request-id cancellation should cancel in-flight references requests
    if fixture.has_scenario(LspScenario::ReferencesRequestCancel) {
        test_state.go_to_marker("cancel")?;
        let request_id = test_state.start_references_request(true)?;

        test_state.cancellation().cancel_request(request_id);

        let response = test_state.await_request(request_id)?;
        let error = response
            .error()
            .ok_or_else(|| "expected references request cancellation error response".to_string())?;
        if error.code != ErrorCode::RequestCancelled {
            return Err(format!(
                "references request cancellation returned unexpected error code {}\nerror: {error:#?}",
                error.code
            ));
        }
    }

    // cancellation facade should apply the native auto-cancel policy on request start
    if fixture.has_scenario(LspScenario::ReferencesRequestCancelledByPolicy) {
        test_state.go_to().marker("cancel")?;
        test_state.cancellation().set_cancelled(0);
        let request_id = test_state.start_references_request(true)?;
        let response = test_state.await_request(request_id)?;
        let error = response
            .error()
            .ok_or_else(|| "expected references request cancellation error response".to_string())?;
        if error.code != ErrorCode::RequestCancelled {
            return Err(format!(
                "policy cancellation returned unexpected error code {}\nerror: {error:#?}",
                error.code
            ));
        }
        test_state.cancellation().reset_cancelled();
    }

    // cancellation facade should also drive work-done progress cancellation paths
    if fixture.has_scenario(LspScenario::ReferencesProgressCancelledByPolicy) {
        let work_done_token = lsp::ProgressToken::String("refs-policy".to_string());

        test_state.go_to().marker("cancel")?;
        test_state.cancellation().set_cancelled(0);
        let request_id = test_state.start_references_with_progress(true, work_done_token)?;
        let response = test_state.await_request(request_id)?;
        let error = response.error().ok_or_else(|| {
            "expected references progress cancellation error response".to_string()
        })?;
        if error.code != ErrorCode::RequestCancelled {
            return Err(format!(
                "policy progress cancellation returned unexpected error code {}\nerror: {error:#?}",
                error.code
            ));
        }
        test_state.cancellation().reset_cancelled();
    }

    Ok(())
}
