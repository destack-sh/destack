use destack_lsp_types as lsp;

use crate::lsp::Cancellation;

impl<'a> Cancellation<'a> {
    /// Reset the native automatic cancellation policy.
    pub fn reset_cancelled(&mut self) {
        self.state.reset_cancelled();
    }

    /// Set the native automatic cancellation policy in request-start checkpoints.
    pub fn set_cancelled(&mut self, number_of_calls: usize) {
        self.state.set_cancelled(number_of_calls);
    }

    /// Cancel one in-flight request by id.
    pub fn cancel_request(&mut self, request_id: i64) {
        self.state.cancel_request(request_id);
    }

    /// Cancel one in-flight work-done progress token.
    pub fn cancel_progress(&mut self, token: lsp::ProgressToken) {
        self.state.cancel_work_done_progress(token);
    }
}
