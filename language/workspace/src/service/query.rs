use std::path::PathBuf;
use std::sync::Arc;

use destack_query::Module;
use destack_repository::Revision;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{Error, FileImage, QueryFile, RunQueryInput};

/// Request to resolve one source file for semantic queries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ResolveQueryFileRequest {
    /// Owning workspace root.
    pub root: PathBuf,
    /// Source path to resolve.
    pub path: PathBuf,
}

/// Serialized source file prepared for semantic queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct QueryFileResponse {
    /// Requested source path.
    pub path: PathBuf,
    /// Exact semantic revision.
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
            path: response.path,
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
            path: file.path.clone(),
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
