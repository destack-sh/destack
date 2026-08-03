use std::path::{Path, PathBuf};

use destack_lsp_server::jsonrpc;
use destack_repository::Revision;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, from_value, to_value};

use crate::server::internal_error;

/// State carried from one query request into a related request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct QueryContinuation<T> {
    /// The workspace path that anchored the original request.
    pub(crate) path: PathBuf,
    /// The exact revision that produced the value.
    pub(crate) revision: Revision,
    /// The query value consumed by the related request.
    pub(crate) value: T,
}

impl<T> QueryContinuation<T> {
    /// Create query continuation state.
    pub(crate) fn new(path: &Path, revision: Revision, value: T) -> Self {
        Self {
            path: path.to_path_buf(),
            revision,
            value,
        }
    }
}

impl<T: Serialize> QueryContinuation<T> {
    /// Serialize this continuation into an LSP data field.
    pub(crate) fn into_value(self) -> jsonrpc::Result<Value> {
        to_value(self).map_err(internal_error)
    }
}

impl<T: DeserializeOwned> QueryContinuation<T> {
    /// Deserialize query continuation state from an LSP data field.
    pub(crate) fn from_value(data: Option<&Value>) -> jsonrpc::Result<Self> {
        let data = data
            .ok_or_else(|| jsonrpc::Error::invalid_params("request has no query continuation"))?;

        from_value(data.clone()).map_err(|error| {
            jsonrpc::Error::invalid_params(format!(
                "request has an invalid query continuation: {error}"
            ))
        })
    }
}
