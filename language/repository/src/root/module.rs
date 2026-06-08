use std::path::PathBuf;
use std::sync::Arc;

use destack_source::{FileId, LanguageType, Loader, ModuleId, PackageId, Uri};
use im::OrdMap;
use rustc_hash::FxHashMap;

use crate::{ConditionGate, ConditionSet};

/// One source module.
#[derive(Debug, Clone)]
pub struct Module {
    /// The module id.
    pub id: ModuleId,
    /// The (main) backing file id.
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
    /// The full set of files that compose this module.
    pub files: Vec<ModuleFile>,
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
        let base = ModuleFile::new(
            file_id,
            uri.clone(),
            path.clone(),
            language_type,
            loader,
            Vec::new(),
            Vec::new(),
        );

        Self {
            id,
            file_id,
            uri,
            path,
            package_id,
            language_type,
            loader,
            files: vec![base],
        }
    }

    /// Add one conditional file to this module.
    pub fn push_condition_file(&mut self, file: ModuleFile) {
        self.files.push(file);
    }

    /// Return files active for one condition set.
    pub fn files_for_conditions<'a>(&'a self, conditions: &ConditionSet) -> Vec<&'a ModuleFile> {
        self.files
            .iter()
            .filter(|file| file.matches(conditions))
            .collect()
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

/// One source file that contributes to a module.
#[derive(Debug, Clone)]
pub struct ModuleFile {
    /// The source file id.
    pub file_id: FileId,
    /// The source file uri.
    pub uri: Uri,
    /// The source file path when physical.
    pub path: Option<PathBuf>,
    /// The source language type for code files.
    pub language_type: Option<LanguageType>,
    /// The loader used to interpret the source file.
    pub loader: Loader,
    /// The condition aliases that activate this file.
    pub aliases: Vec<String>,
    /// The condition gates that activate this file.
    pub gates: Vec<ConditionGate>,
}

impl ModuleFile {
    /// Build one module file.
    pub fn new(
        file_id: FileId,
        uri: Uri,
        path: Option<PathBuf>,
        language_type: Option<LanguageType>,
        loader: Loader,
        aliases: Vec<String>,
        gates: Vec<ConditionGate>,
    ) -> Self {
        Self {
            file_id,
            uri,
            path,
            language_type,
            loader,
            aliases,
            gates,
        }
    }

    /// Return whether this file is active for one condition set.
    pub fn matches(&self, conditions: &ConditionSet) -> bool {
        self.gates.iter().all(|gate| gate.matches(conditions))
    }
}

/// Revision-local module lookup data.
#[derive(Debug, Clone)]
pub(crate) struct ModuleIndex {
    /// Modules keyed by module id.
    modules: OrdMap<ModuleId, Arc<Module>>,
    /// Module ids keyed by contributing file id.
    module_by_file: FxHashMap<FileId, ModuleId>,
    /// Module ids keyed by contributing file uri.
    module_by_uri: FxHashMap<Uri, ModuleId>,
    /// Module ids keyed by package id.
    module_by_package: FxHashMap<PackageId, Vec<ModuleId>>,
}

impl ModuleIndex {
    /// Create one module index.
    pub(crate) fn new(modules: OrdMap<ModuleId, Arc<Module>>) -> Self {
        let mut module_by_file = FxHashMap::default();
        let mut module_by_uri = FxHashMap::default();
        let mut module_by_package = FxHashMap::default();

        // derive lookup indexes from canonical modules
        for (module_id, module) in &modules {
            module_by_package
                .entry(module.package_id)
                .or_insert_with(Vec::new)
                .push(*module_id);

            for file in &module.files {
                module_by_file.insert(file.file_id, *module_id);
                module_by_uri.insert(file.uri.clone(), *module_id);
            }
        }

        Self {
            modules,
            module_by_file,
            module_by_uri,
            module_by_package,
        }
    }

    /// Return one module by id.
    pub(crate) fn module(&self, module_id: ModuleId) -> Option<Arc<Module>> {
        self.modules.get(&module_id).cloned()
    }

    /// Return all module ids.
    pub(crate) fn module_ids(&self) -> impl Iterator<Item = ModuleId> + '_ {
        self.modules.keys().copied()
    }

    /// Return module ids for one package.
    pub(crate) fn package_module_ids(&self, package_id: PackageId) -> &[ModuleId] {
        self.module_by_package
            .get(&package_id)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// Return the module id for one contributing file id.
    pub(crate) fn module_id_for_file(&self, file_id: FileId) -> Option<ModuleId> {
        self.module_by_file.get(&file_id).copied()
    }

    /// Return the module id for one contributing file uri.
    pub(crate) fn module_id_for_uri(&self, uri: &Uri) -> Option<ModuleId> {
        self.module_by_uri.get(uri).copied()
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use destack_source::{FileType, PackageId, Uri};

    use super::*;

    #[test]
    fn test_select_module_files_for_active_conditions() {
        let package_id = PackageId::new(1);
        let module_id = ModuleId::new(package_id, 1);
        let mut module = Module::blank(
            module_id,
            FileId::new(1),
            Uri::from_path(Path::new("main.ds")),
            Some(PathBuf::from("main.ds")),
            package_id,
            Some(LanguageType::Destack),
            Loader::Destack,
        );
        let test_file = ModuleFile::new(
            FileId::new(2),
            Uri::from_path(Path::new("main.test.ds")),
            Some(PathBuf::from("main.test.ds")),
            Some(LanguageType::Destack),
            Loader::Destack,
            vec!["test".to_string()],
            vec![ConditionGate::mode("test")],
        );
        module.push_condition_file(test_file);

        // select the base file without active modes
        let conditions = ConditionSet::default();
        let files = module.files_for_conditions(&conditions);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_id, FileId::new(1));

        // select base and conditional files with matching modes
        let mut conditions = ConditionSet::default();
        conditions.modes.insert("test".to_string());
        let files = module.files_for_conditions(&conditions);
        let file_ids = files.iter().map(|file| file.file_id).collect::<Vec<_>>();
        assert_eq!(file_ids, vec![FileId::new(1), FileId::new(2)]);
    }

    #[test]
    fn test_module_index_tracks_conditional_files() {
        let package_id = PackageId::new(1);
        let module_id = ModuleId::new(package_id, 1);
        let mut module = Module::blank(
            module_id,
            FileId::new(1),
            Uri::from_path(Path::new("main.ds")),
            Some(PathBuf::from("main.ds")),
            package_id,
            Some(LanguageType::Destack),
            Loader::Destack,
        );
        module.push_condition_file(ModuleFile::new(
            FileId::new(2),
            Uri::from_path(Path::new("main.test.ds")),
            Some(PathBuf::from("main.test.ds")),
            Some(LanguageType::try_from(FileType::Destack).unwrap()),
            Loader::Destack,
            vec!["test".to_string()],
            vec![ConditionGate::mode("test")],
        ));

        // index every contributing file against the canonical module
        let mut modules = OrdMap::new();
        modules.insert(module_id, Arc::new(module));
        let index = ModuleIndex::new(modules);

        assert_eq!(index.module_id_for_file(FileId::new(1)), Some(module_id));
        assert_eq!(index.module_id_for_file(FileId::new(2)), Some(module_id));
    }

    #[test]
    fn test_module_index_tracks_builtin_uri() {
        let package_id = PackageId::new(1);
        let module_id = ModuleId::new(package_id, 1);
        let module = Module::blank(
            module_id,
            FileId::new(1),
            Uri::from_string("destack://types/function"),
            None,
            package_id,
            Some(LanguageType::Destack),
            Loader::Destack,
        );

        // index the builtin URI without a physical path
        let mut modules = OrdMap::new();
        modules.insert(module_id, Arc::new(module));
        let index = ModuleIndex::new(modules);

        assert_eq!(
            index.module_id_for_uri(&Uri::from_string("destack://types/function")),
            Some(module_id)
        );
    }
}
