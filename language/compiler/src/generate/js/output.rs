use std::sync::Arc;

use destack_artifact::{
    DirBound, DirCheckedModule, DirExpanded, DirImported, DirParsed, EmitFormat, JsDeclaration,
    JsLanguage, JsOutput,
};
use destack_core::StringPool;
use destack_js as js;
use destack_repository::{Module, Target};

use crate::generate::js::{CodegenJsError, CodegenJsResult, CodegenJsWarning};

use super::lower::lower_module;

/// One generator for JS module outputs.
#[derive(Debug)]
pub struct JsOutputGenerator<'a> {
    /// The current module snapshot.
    module: Arc<Module>,
    /// The current parsed DIR.
    parsed: Arc<DirParsed>,
    /// The current bound DIR artifact.
    bound: Arc<DirBound>,
    /// The current imported DIR artifact.
    imported: Arc<DirImported>,
    /// The current expanded DIR artifact.
    expanded: Arc<DirExpanded>,
    /// The current checked DIR artifact.
    checked: Arc<DirCheckedModule>,
    /// The shared string pool.
    strings: Arc<StringPool>,
    /// The target configuration.
    target: &'a Target,
}

impl<'a> JsOutputGenerator<'a> {
    /// Create one JS output generator.
    pub fn new(
        module: Arc<Module>,
        parsed: Arc<DirParsed>,
        bound: Arc<DirBound>,
        imported: Arc<DirImported>,
        expanded: Arc<DirExpanded>,
        checked: Arc<DirCheckedModule>,
        strings: Arc<StringPool>,
        target: &'a Target,
    ) -> Self {
        Self {
            module,
            parsed,
            bound,
            imported,
            expanded,
            checked,
            strings,
            target,
        }
    }

    /// Generate one JS output.
    pub fn generate(
        self,
    ) -> CodegenJsResult<(JsOutput, Vec<CodegenJsWarning>, Vec<CodegenJsError>)> {
        // validate target
        if !self.target.uses_js_generate_pipeline() {
            return Err(CodegenJsError::UnsupportedTarget {
                format: format!("{:?}", self.target.emit),
                message: Some("expected JS, TS, or HTML".to_string()),
            });
        }

        // current module inputs
        let module = self.module.as_ref();
        let parsed = self.parsed.as_ref();
        let bound = self.bound.as_ref();
        let imported = self.imported.as_ref();
        let expanded = self.expanded.as_ref();
        let checked = self.checked.as_ref();

        // resource modules are linked directly in the JS linker
        if !module.is_code() {
            return Err(CodegenJsError::Internal {
                message: format!(
                    "resource JS outputs are linked directly for module '{}'",
                    module.uri
                ),
            });
        }

        // emit one lowered JavaScript module tree
        let lower = lower_module(
            module,
            parsed,
            self.strings.as_ref(),
            bound,
            imported,
            expanded,
            checked,
            self.target,
        )?;
        let warnings = lower.warnings;
        let errors = lower.errors;
        let artifact = self.build_js_artifact(lower.module, true)?;

        Ok((artifact, warnings, errors))
    }

    /// Build one JS output from one lowered module tree.
    fn build_js_artifact(
        &self,
        module: js::Module,
        has_top_level_side_effects: bool,
    ) -> CodegenJsResult<JsOutput> {
        // declaration output
        let declaration = if self.target.declaration && matches!(self.target.emit, EmitFormat::Js) {
            Some(JsDeclaration::default())
        } else {
            None
        };

        // language selection
        let language = match self.target.emit {
            EmitFormat::Js => JsLanguage::JavaScript,
            EmitFormat::Ts => JsLanguage::TypeScript,
            _ => {
                return Err(CodegenJsError::UnsupportedTarget {
                    format: format!("{:?}", self.target.emit),
                    message: Some("expected JS, TS, or HTML".to_string()),
                });
            }
        };

        Ok(JsOutput {
            language,
            module,
            declaration,
            source_map: None,
            has_top_level_side_effects,
        })
    }
}
