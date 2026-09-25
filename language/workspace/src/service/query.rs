use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tspp_query::Module;
use tspp_repository::Revision;
use tspp_serde::Reflect;
use tspp_source::Uri;

use crate::{Error, FileImage, QueryFile, RunQueryInput};

/// Request to resolve one source file for queries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ResolveQueryFileRequest {
    /// Owning workspace root.
    pub root: PathBuf,
    /// Exact source revision.
    pub revision: Revision,
    /// Source URI to resolve.
    pub uri: Uri,
}

/// Serialized source file prepared for queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct QueryFileResponse {
    /// Owning workspace root.
    pub root: PathBuf,
    /// Exact source revision.
    pub revision: Revision,
    /// Module containing the file.
    pub module: Module,
    /// Exact source file image.
    pub file: FileImage,
}

impl TryFrom<QueryFileResponse> for QueryFile {
    type Error = Error;

    /// Convert one serialized response into runtime query state.
    fn try_from(response: QueryFileResponse) -> Result<Self, Self::Error> {
        let file = response.file.into_file()?;

        Ok(Self {
            root: response.root,
            revision: response.revision,
            module: response.module,
            file: Arc::new(file),
        })
    }
}

impl From<&QueryFile> for QueryFileResponse {
    /// Build one serialized response from runtime query state.
    fn from(file: &QueryFile) -> Self {
        Self {
            root: file.root.clone(),
            revision: file.revision,
            module: file.module,
            file: FileImage::from(file.file.as_ref()),
        }
    }
}

/// Request to execute one semantic query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RunQueryRequest {
    /// Owning workspace root.
    pub root: PathBuf,
    /// Semantic query input.
    pub input: RunQueryInput,
}
