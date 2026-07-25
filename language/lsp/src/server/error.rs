use std::fmt::Display;

use destack_lsp_server::jsonrpc;
use destack_workspace::Error;
use serde_json::Value;

/// Build an internal LSP error without discarding its cause.
pub(crate) fn internal_error(detail: impl Display) -> jsonrpc::Error {
    let mut error = jsonrpc::Error::internal_error();
    error.data = Some(Value::String(detail.to_string()));

    error
}

/// Convert a workspace error into an LSP response error.
pub(super) fn workspace_error(error: Error) -> jsonrpc::Error {
    let mut response = if matches!(&error, Error::StaleRevision { .. }) {
        jsonrpc::Error::content_modified()
    } else {
        jsonrpc::Error::internal_error()
    };
    response.data = Some(Value::String(error.to_string()));

    response
}
