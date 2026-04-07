use std::sync::Arc;

use destack_artifact::{
    Ast, DirPatched, EmitFormat, ScriptArtifact, ScriptDeclaration, ScriptLanguage,
};
use destack_core::StringPool;
use destack_workspace::{Module, Target};

use super::{JsBackend, lower_module};
use crate::{CodegenJsError, CodegenJsResult, CodegenJsWarning, ScriptModule};

/// One generator for script module artifacts.
#[derive(Debug)]
pub struct ScriptArtifactGenerator<'a> {
    /// The current module snapshot.
    module: Arc<Module>,
    /// The current module AST.
    ast: Arc<Ast>,
    /// The current patched DIR artifact.
    dir: Arc<DirPatched>,
    /// The shared string pool.
    strings: Arc<StringPool>,
    /// The target configuration.
    target: &'a Target,
}

impl<'a> ScriptArtifactGenerator<'a> {
    /// Create one script artifact generator.
    pub fn new(
        module: Arc<Module>,
        ast: Arc<Ast>,
        dir: Arc<DirPatched>,
        strings: Arc<StringPool>,
        target: &'a Target,
    ) -> Self {
        Self {
            module,
            ast,
            dir,
            strings,
            target,
        }
    }

    /// Generate one script artifact.
    pub fn generate(
        self,
    ) -> CodegenJsResult<(ScriptArtifact, Vec<CodegenJsWarning>, Vec<CodegenJsError>)> {
        // validate target
        if !self.target.uses_js_generate_pipeline() {
            return Err(CodegenJsError::UnsupportedTarget {
                format: format!("{:?}", self.target.emit),
                message: Some("expected JS, TS, or HTML".to_string()),
            });
        }

        // current module inputs
        let module = self.module.as_ref();
        let ast = self.ast.as_ref();
        let dir = self.dir.as_ref();

        // resource modules are linked directly in the script linker
        if !module.is_code() {
            return Err(CodegenJsError::Internal {
                message: format!(
                    "resource script artifacts are linked directly for module '{}'",
                    module.uri
                ),
            });
        }

        // emit one lowered JavaScript module tree
        let lower = lower_module(module, ast, self.strings.as_ref(), dir, self.target)?;
        let warnings = lower.warnings;
        let errors = lower.errors;
        let artifact = self.build_script_artifact(lower.module, true)?;

        Ok((artifact, warnings, errors))
    }

    /// Build one script artifact from one lowered module tree.
    fn build_script_artifact(
        &self,
        module: ScriptModule,
        has_top_level_side_effects: bool,
    ) -> CodegenJsResult<ScriptArtifact> {
        // declaration output
        let declaration = if self.target.declaration && matches!(self.target.emit, EmitFormat::Js) {
            Some(ScriptDeclaration::default())
        } else {
            None
        };

        // language selection
        let language = match self.target.emit {
            EmitFormat::Js | EmitFormat::Html => ScriptLanguage::JavaScript,
            EmitFormat::Ts => ScriptLanguage::TypeScript,
            _ => {
                return Err(CodegenJsError::UnsupportedTarget {
                    format: format!("{:?}", self.target.emit),
                    message: Some("expected JS, TS, or HTML".to_string()),
                });
            }
        };

        // linkage metadata
        let linkage = JsBackend::collect_script_linkage(&module);

        Ok(ScriptArtifact {
            language,
            module,
            linkage,
            declaration,
            source_map: None,
            has_top_level_side_effects,
        })
    }
}
