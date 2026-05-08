use std::path::PathBuf;
use std::sync::Arc;

use destack_source::{FileId, LanguageType, Loader, ModuleId, PackageId, Uri};
use im::OrdMap;

/// One source module.
#[derive(Debug, Clone)]
pub struct Module {
    /// The module id.
    pub id: ModuleId,
    /// The backing file id.
    pub file_id: FileId,
    /// The module uri.
    pub uri: Uri,
    /// The module path when physical.
    pub path: Option<PathBuf>,
    /// The owning package id.
    pub package_id: PackageId,
    /// The source language type for code modules.
    pub language_type: Option<LanguageType>,
    /// The loader used to interpret the module.
    pub loader: Loader,
}

impl Module {
    /// Build one blank module identity.
    pub fn blank(
        id: ModuleId,
        file_id: FileId,
        uri: Uri,
        path: Option<PathBuf>,
        package_id: PackageId,
        language_type: Option<LanguageType>,
        loader: Loader,
    ) -> Self {
        Self {
            id,
            file_id,
            uri,
            path,
            package_id,
            language_type,
            loader,
        }
    }

    /// Return true when this module contains code.
    pub fn is_code(&self) -> bool {
        self.loader.is_code()
    }

    /// Return the code language type for this module.
    pub fn code_language_type(&self) -> LanguageType {
        self.language_type.unwrap_or_else(|| {
            panic!("module has no code language type: {:?}", self.id);
        })
    }

    /// Return true when this module is parsed as Destack.
    pub fn is_destack(&self) -> bool {
        self.language_type
            .is_some_and(|language_type| language_type.is_destack())
    }

    /// Return true when this module is parsed as JavaScript.
    pub fn is_javascript(&self) -> bool {
        self.language_type
            .is_some_and(|language_type| language_type.is_javascript())
    }

    /// Return true when this module is parsed as TypeScript.
    pub fn is_typescript(&self) -> bool {
        self.language_type
            .is_some_and(|language_type| language_type.is_typescript())
    }

    /// Return true when this module is parsed as JavaScript or TypeScript.
    pub fn is_ecmascript(&self) -> bool {
        self.is_javascript() || self.is_typescript()
    }

    /// Return true when this module is parsed as a declaration file.
    pub fn is_declaration(&self) -> bool {
        self.language_type
            .is_some_and(|language_type| language_type.is_declaration())
    }

    /// Return true when this module language supports declaration merging.
    pub fn supports_declaration_merging(&self) -> bool {
        self.language_type
            .is_some_and(|language_type| language_type.supports_declaration_merging())
    }
}

/// Revision-local module lookup data.
#[derive(Debug, Clone)]
pub(crate) struct ModuleIndex {
    /// Modules keyed by module id.
    modules: OrdMap<ModuleId, Arc<Module>>,
}

impl ModuleIndex {
    /// Create one module index.
    pub(crate) fn new(modules: OrdMap<ModuleId, Arc<Module>>) -> Self {
        Self { modules }
    }

    /// Return one module by id.
    pub(crate) fn module(&self, module_id: ModuleId) -> Option<Arc<Module>> {
        self.modules.get(&module_id).cloned()
    }

    /// Return all module ids.
    pub(crate) fn module_ids(&self) -> impl Iterator<Item = ModuleId> + '_ {
        self.modules.keys().copied()
    }

    /// Iterate all modules.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&ModuleId, &Arc<Module>)> {
        self.modules.iter()
    }
}
