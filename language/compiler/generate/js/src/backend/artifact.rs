use std::sync::Arc;

use destack_artifact::{
    Ast, DirChecked, DirDeclared, EmitFormat, ScriptDeclaration, ScriptLanguage, ScriptOutput,
};
use destack_core::StringPool;
use destack_js as js;
use destack_workspace::{Module, Target};

use super::lower_module;
use crate::{CodegenJsError, CodegenJsResult, CodegenJsWarning};

/// One generator for script module outputs.
#[derive(Debug)]
pub struct ScriptOutputGenerator<'a> {
    /// The current module snapshot.
    module: Arc<Module>,
    /// The current module AST.
    ast: Arc<Ast>,
    /// The current declared DIR artifact.
    declared: Arc<DirDeclared>,
    /// The current checked DIR artifact.
    checked: Arc<DirChecked>,
    /// The shared string pool.
    strings: Arc<StringPool>,
    /// The target configuration.
    target: &'a Target,
}

impl<'a> ScriptOutputGenerator<'a> {
    /// Create one script output generator.
    pub fn new(
        module: Arc<Module>,
        ast: Arc<Ast>,
        declared: Arc<DirDeclared>,
        checked: Arc<DirChecked>,
        strings: Arc<StringPool>,
        target: &'a Target,
    ) -> Self {
        Self {
            module,
            ast,
            declared,
            checked,
            strings,
            target,
        }
    }

    /// Generate one script output.
    pub fn generate(
        self,
    ) -> CodegenJsResult<(ScriptOutput, Vec<CodegenJsWarning>, Vec<CodegenJsError>)> {
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
        let declared = self.declared.as_ref();
        let checked = self.checked.as_ref();

        // resource modules are linked directly in the script linker
        if !module.is_code() {
            return Err(CodegenJsError::Internal {
                message: format!(
                    "resource script outputs are linked directly for module '{}'",
                    module.uri
                ),
            });
        }

        // emit one lowered JavaScript module tree
        let lower = lower_module(
            module,
            ast,
            self.strings.as_ref(),
            declared,
            checked,
            self.target,
        )?;
        let warnings = lower.warnings;
        let errors = lower.errors;
        let artifact = self.build_script_artifact(lower.module, true)?;

        Ok((artifact, warnings, errors))
    }

    /// Build one script output from one lowered module tree.
    fn build_script_artifact(
        &self,
        module: js::Module,
        has_top_level_side_effects: bool,
    ) -> CodegenJsResult<ScriptOutput> {
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

        Ok(ScriptOutput {
            language,
            module,
            declaration,
            source_map: None,
            has_top_level_side_effects,
        })
    }
}
