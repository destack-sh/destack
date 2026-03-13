use std::sync::Arc;

use destack_source::{LanguageType, ModuleId, ModuleVersion};
use destack_workspace::{Module, ModuleDirData, ModuleFormat, ModuleSource, ProfileId, SourceType};

use crate::{Compiler, ResolveError, ResolveResult};

/// One immutable snapshot of current-module resolve metadata.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ResolveModuleContext {
    /// The module id being resolved.
    pub id: ModuleId,
    /// The current source-derived module version.
    pub version: ModuleVersion,
    /// The module source origin.
    pub source: ModuleSource,
    /// The source script/module classification.
    pub source_type: SourceType,
    /// The runtime module format.
    pub module_format: ModuleFormat,
    /// The language kind.
    pub language_type: LanguageType,
}

impl ResolveModuleContext {
    /// Build one resolve context from one module.
    pub(crate) fn from_module(module: &Module) -> Self {
        Self {
            id: module.id,
            version: module.version,
            source: module.source,
            source_type: module.source_type,
            module_format: module.module_format,
            language_type: module.language_type,
        }
    }

    /// Return true when this is one builtin module.
    pub(crate) fn is_builtin(self) -> bool {
        matches!(self.source, ModuleSource::Builtin(_))
    }
}

impl Compiler {
    /// Load one resolved remote module artifact for resolve-time reads.
    pub(crate) fn resolved_module_artifact(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> ResolveResult<(ResolveModuleContext, Arc<ModuleDirData>)> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let context = ResolveModuleContext::from_module(&module);
        let dir = self
            .require_artifact_dir_resolved(module_id, profile)
            .map_err(ResolveError::from)?;

        Ok((context, dir))
    }

    /// Load one prepared remote module artifact for resolve-time reads.
    pub(crate) fn prepared_module_artifact(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> ResolveResult<(ResolveModuleContext, Arc<ModuleDirData>)> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let context = ResolveModuleContext::from_module(&module);
        let dir = self
            .require_artifact_dir_prepared(module_id, profile)
            .map_err(ResolveError::from)?;

        Ok((context, dir))
    }
}
