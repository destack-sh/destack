use serde::{Deserialize, Serialize};

use destack_source::ContentId;

use crate::{JsOutput, NativeOutput};

/// One emitted module output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModuleOutput {
    /// One emitted JS output.
    Js(Box<JsOutput>),
    /// One emitted native output.
    Native(Box<NativeOutput>),
}

impl ModuleOutput {
    /// Return all content ids referenced by this module output.
    pub fn content_ids(&self) -> Vec<ContentId> {
        match self {
            Self::Js(_) => Vec::new(),
            Self::Native(output) => output.content_ids(),
        }
    }
}
