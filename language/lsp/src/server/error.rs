use std::fmt::Display;

use serde_json::Value;
use tspp_lsp_server::jsonrpc;
use tspp_workspace::Error;

/// Build an internal LSP error without discarding its cause.
pub(crate) fn internal_error(detail: impl Display) -> jsonrpc::Error {
    let mut error = jsonrpc::Error::internal_error();
    error.data = Some(Value::String(detail.to_string()));

    error
}

/// Build an LSP response error from one workspace error.
pub(crate) fn workspace_error(error: Error) -> jsonrpc::Error {
    let mut response = if matches!(&error, Error::StaleRevision { .. }) {
        jsonrpc::Error::content_modified()
    } else {
        jsonrpc::Error::internal_error()
    };
    response.data = Some(Value::String(error.to_string()));

    response
}
