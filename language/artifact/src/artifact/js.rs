use destack_js as js;
use serde::{Deserialize, Serialize};

use crate::SourceMapArtifact;

/// One generated JS output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsOutput {
    /// The emitted JS language.
    pub language: JsLanguage,
    /// The lowered JS module.
    pub module: js::Module,
    /// The generated declaration payload when one exists.
    pub declaration: Option<JsDeclaration>,
    /// The generated source map payload when one exists.
    pub source_map: Option<SourceMapArtifact>,
    /// Whether the module has top level side effects.
    pub has_top_level_side_effects: bool,
}

/// One generated JS declaration payload.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JsDeclaration {
    /// The emitted declaration text.
    pub text: String,
}

/// One emitted JS language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JsLanguage {
    /// JavaScript output.
    JavaScript,
    /// TypeScript output.
    TypeScript,
}
