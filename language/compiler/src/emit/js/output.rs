use std::sync::Arc;

use tspp_artifact::{DirView, Output, Script};
use tspp_core::StringPool;
use tspp_repository::{Module, Target};

use crate::EmitError;

use super::lower::lower_module;

/// One generator for structured scripts.
#[derive(Debug)]
pub(crate) struct ScriptGenerator<'a> {
    /// The current module snapshot.
    module: Arc<Module>,
    /// The module's DIR stages, their tables stacked once.
    view: DirView,
    /// The shared string pool.
    strings: Arc<StringPool>,
    /// The target configuration.
    target: &'a Target,
}

impl<'a> ScriptGenerator<'a> {
    /// Create one script generator.
    pub(crate) fn new(
        module: Arc<Module>,
        view: DirView,
        strings: Arc<StringPool>,
        target: &'a Target,
    ) -> Self {
        Self {
            module,
            view,
            strings,
            target,
        }
    }

    /// Emit one structured script.
    pub(crate) fn emit(self) -> Result<(Script, Vec<EmitError>), EmitError> {
        // validate target
        if self.target.output != Output::Bundle {
            return Err(self.unsupported_target("expected JavaScript".to_string()));
        }

        // current module inputs
        let module = self.module.as_ref();

        // asset modules are linked directly in the JS linker
        if !module.is_code() {
            return Err(self.internal_error(format!(
                "asset modules are linked directly for module '{}'",
                module.uri
            )));
        }

        // emit one lowered JavaScript module tree
        let (module, errors) = lower_module(module, &self.view, self.strings.as_ref());

        let script = Script {
            module,
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
            target: format!("{}: {message}", self.target.output.canonical_tag()),
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
