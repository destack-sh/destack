use std::sync::Arc;

use destack_artifact::{
    DirBound, DirChecked, DirDeclared, DirExpanded, DirImported, DirMaterialized, DirParsed,
    EmitFormat, Script, ScriptBody, ScriptLanguage,
};
use destack_core::StringPool;
use destack_repository::{Module, Target};

use crate::EmitError;

use super::lower::lower_module;

/// One generator for structured scripts.
#[derive(Debug)]
pub(crate) struct ScriptGenerator<'a> {
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
    /// The current declared DIR artifact.
    declared: Arc<DirDeclared>,
    /// The current checked DIR artifact.
    checked: Arc<DirChecked>,
    /// The current materialized DIR artifact.
    materialized: Arc<DirMaterialized>,
    /// The shared string pool.
    strings: Arc<StringPool>,
    /// The target configuration.
    target: &'a Target,
}

impl<'a> ScriptGenerator<'a> {
    /// Create one script generator.
    pub(crate) fn new(
        module: Arc<Module>,
        parsed: Arc<DirParsed>,
        bound: Arc<DirBound>,
        imported: Arc<DirImported>,
        expanded: Arc<DirExpanded>,
        declared: Arc<DirDeclared>,
        checked: Arc<DirChecked>,
        materialized: Arc<DirMaterialized>,
        strings: Arc<StringPool>,
        target: &'a Target,
    ) -> Self {
        Self {
            module,
            parsed,
            bound,
            imported,
            expanded,
            declared,
            checked,
            materialized,
            strings,
            target,
        }
    }

    /// Emit one structured script.
    pub(crate) fn emit(self) -> Result<(Script, Vec<EmitError>), EmitError> {
        // validate target
        if !self.target.emit.is_script() {
            return Err(self.unsupported_target("expected JS, TS, or HTML".to_string()));
        }

        // current module inputs
        let module = self.module.as_ref();
        let parsed = self.parsed.as_ref();
        let bound = self.bound.as_ref();
        let imported = self.imported.as_ref();
        let expanded = self.expanded.as_ref();
        let declared = self.declared.as_ref();
        let checked = self.checked.as_ref();
        let materialized = self.materialized.as_ref();

        // asset modules are linked directly in the JS linker
        if !module.is_code() {
            return Err(self.internal_error(format!(
                "asset modules are linked directly for module '{}'",
                module.uri
            )));
        }

        // emit one lowered JavaScript module tree
        let lower = lower_module(
            module,
            parsed,
            self.strings.as_ref(),
            bound,
            imported,
            expanded,
            declared,
            checked,
            materialized,
        )?;
        let errors = lower.errors;

        // language selection
        let language = match self.target.emit {
            EmitFormat::Js => ScriptLanguage::JavaScript,
            EmitFormat::Ts => ScriptLanguage::TypeScript,
            _ => {
                return Err(self.unsupported_target("expected JS, TS, or HTML".to_string()));
            }
        };

        let script = Script {
            language,
            body: ScriptBody::EcmaScript(lower.module),
            map: None,
            has_top_level_side_effects: true,
        };

        Ok((script, errors))
    }

    /// Build one unsupported target error.
    fn unsupported_target(&self, message: String) -> EmitError {
        EmitError::UnsupportedTarget {
            anchor: self.module.id.into(),
            module: self.module.id,
            target: format!("{:?}: {message}", self.target.emit),
        }
    }

    /// Build one internal JS emit error.
    fn internal_error(&self, message: String) -> EmitError {
        EmitError::Internal {
            anchor: self.module.id.into(),
            module: self.module.id,
            message,
        }
    }
}
