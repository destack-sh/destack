use destack_js as js;
use serde::{Deserialize, Serialize};

use crate::SourceMapArtifact;

/// One generated script output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptOutput {
    /// The emitted script language.
    pub language: ScriptLanguage,
    /// The lowered script module.
    pub module: js::Module,
    /// The generated declaration payload when one exists.
    pub declaration: Option<ScriptDeclaration>,
    /// The generated source map payload when one exists.
    pub source_map: Option<SourceMapArtifact>,
    /// Whether the module has top level side effects.
    pub has_top_level_side_effects: bool,
}

/// One generated script declaration payload.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScriptDeclaration {
    /// The emitted declaration text.
    pub text: String,
}

/// One emitted script language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScriptLanguage {
    /// JavaScript output.
    JavaScript,
    /// TypeScript output.
    TypeScript,
}
