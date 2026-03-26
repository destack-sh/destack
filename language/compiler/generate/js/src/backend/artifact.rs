use std::sync::Arc;

use destack_artifact::{
    ArtifactStore, EmitFormat, ScriptArtifact, ScriptDeclaration, ScriptLanguage,
};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, Program, Target};

use super::{JsBackend, lower_module};
use crate::{CodegenJsError, CodegenJsResult};

/// One generator for script module artifacts.
#[derive(Debug)]
pub struct ScriptArtifactGenerator<'a> {
    /// The shared program state.
    program: Arc<Program>,
    /// The shared artifact store.
    artifacts: Arc<ArtifactStore>,
    /// The module to generate.
    module_id: ModuleId,
    /// The target configuration.
    target: &'a Target,
    /// The active profile.
    profile: ProfileId,
}

impl<'a> ScriptArtifactGenerator<'a> {
    /// Create one script artifact generator.
    pub fn new(
        program: Arc<Program>,
        artifacts: Arc<ArtifactStore>,
        module_id: ModuleId,
        target: &'a Target,
        profile: ProfileId,
    ) -> Self {
        Self {
            program,
            artifacts,
            module_id,
            target,
            profile,
        }
    }

    /// Generate one script artifact.
    pub fn generate(
        self,
    ) -> CodegenJsResult<(
        ScriptArtifact,
        Vec<crate::CodegenJsWarning>,
        Vec<CodegenJsError>,
    )> {
        // validate target
        if !self.target.uses_js_generate_pipeline() {
            return Err(CodegenJsError::UnsupportedTarget {
                format: format!("{:?}", self.target.emit),
                message: Some("expected JS, TS, or HTML".to_string()),
            });
        }

        // get module
        let module_ref = self.program.modules.get(self.module_id);
        let module = module_ref.as_ref();
        let ast = self
            .artifacts
            .ast(self.module_id)
            .unwrap_or_else(|| panic!("missing committed AST artifact for {:?}", self.module_id));
        let dir = self
            .artifacts
            .dir_patched(self.module_id, self.profile)
            .unwrap_or_else(|| {
                panic!(
                    "missing committed patched DIR artifact for {:?}",
                    self.module_id
                )
            });

        // emit one lowered JavaScript module tree
        let lower = lower_module(
            &module,
            &ast,
            &self.program.strings,
            dir.as_ref(),
            self.target,
        )?;
        let warnings = lower.warnings;
        let errors = lower.errors;

        // current JS generation only knows that a declaration output exists
        let declaration = if self.target.declaration && matches!(self.target.emit, EmitFormat::Js) {
            Some(ScriptDeclaration::default())
        } else {
            None
        };

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

        let linkage = JsBackend::collect_script_linkage(&lower.module);

        let artifact = ScriptArtifact {
            language,
            module: lower.module,
            linkage,
            declaration,
            source_map: None,
            has_top_level_side_effects: true,
        };

        Ok((artifact, warnings, errors))
    }
}
