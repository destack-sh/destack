use serde::{Deserialize, Serialize};

use crate::{JsOutput, NativeOutput};

/// One emitted module output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModuleOutput {
    /// One emitted JS output.
    Js(Box<JsOutput>),
    /// One emitted native output.
    Native(Box<NativeOutput>),
}
