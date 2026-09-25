use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tspp_lsp_server::jsonrpc;
use tspp_repository::Revision;
use tspp_serde::{Codec, Reflect, from_slice, to_vec};

use crate::server::internal_error;

/// State carried from one query request into a related request.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub(crate) struct QueryContinuation<T> {
    /// The workspace root that owns the original request.
    pub(crate) root: PathBuf,
    /// The exact revision that produced the value.
    pub(crate) revision: Revision,
    /// The query value consumed by the related request.
    pub(crate) value: T,
}

impl<T> QueryContinuation<T> {
    /// Create query continuation state.
    pub(crate) fn new(root: &Path, revision: Revision, value: T) -> Self {
        Self {
            root: root.to_path_buf(),
            revision,
            value,
        }
    }
}

impl<T: Codec> QueryContinuation<T> {
    /// Serialize this continuation into an LSP data field.
    pub(crate) fn into_value(self) -> jsonrpc::Result<Value> {
        let bytes = to_vec(&self).map_err(internal_error)?;
        let token = URL_SAFE_NO_PAD.encode(bytes);

        Ok(Value::String(token))
    }

    /// Deserialize query continuation state from an LSP data field.
    pub(crate) fn from_value(data: Option<&Value>) -> jsonrpc::Result<Self> {
        let data = data
            .ok_or_else(|| jsonrpc::Error::invalid_params("request has no query continuation"))?;
        let token = data.as_str().ok_or_else(|| {
            jsonrpc::Error::invalid_params("request has a non-string query continuation")
        })?;

        let bytes = URL_SAFE_NO_PAD.decode(token).map_err(|error| {
            jsonrpc::Error::invalid_params(format!(
                "request has an invalid query continuation: {error}"
            ))
        })?;

        from_slice(&bytes).map_err(|error| {
            jsonrpc::Error::invalid_params(format!(
                "request has an invalid query continuation: {error}"
            ))
        })
    }
}
