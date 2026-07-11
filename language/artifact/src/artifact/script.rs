use destack_js::Module;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::SourceMap;

/// One structured script linker input for a target.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct Script {
    /// The target language of this script.
    pub language: ScriptLanguage,
    /// The structured script body.
    pub body: ScriptBody,
    /// The source map when one exists.
    pub map: Option<SourceMap>,
    /// Whether this script has top level side effects.
    pub has_top_level_side_effects: bool,
}

impl Script {
    /// Return the ECMAScript module body.
    pub fn ecmascript_module(&self) -> Option<&Module> {
        match &self.body {
            ScriptBody::EcmaScript(module) => Some(module),
        }
    }

    /// Consume this script into its ECMAScript module body.
    pub fn into_ecmascript_module(self) -> Option<Module> {
        match self.body {
            ScriptBody::EcmaScript(module) => Some(module),
        }
    }

    /// Replace the ECMAScript module body.
    pub fn replace_ecmascript_module(&mut self, module: Module) {
        self.body = ScriptBody::EcmaScript(module);
    }
}

/// One structured script body.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub enum ScriptBody {
    /// ECMAScript-family module IR.
    EcmaScript(Module),
}

/// One structured target language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum ScriptLanguage {
    /// JavaScript output.
    JavaScript,
    /// TypeScript output.
    TypeScript,
}
