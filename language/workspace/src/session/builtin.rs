use std::sync::Arc;

use dashmap::DashMap;
use destack_builtin::{CORE_SOURCES, LanguageItem, PRELUDE_SOURCE};
use destack_dir::GlobalSymbolId;
use destack_source::{File, FileRegistry, FileType, LanguageType, ModuleId, PackageId, Uri};

use crate::{Module, ModuleRegistry, ModuleType, Package, PackageKind, PackageRegistry};

/// Well-known package ID for builtins.
pub const BUILTIN_PACKAGE_ID: PackageId = PackageId(1);

/// Well-known package name for builtins.
pub const BUILTIN_PACKAGE_NAME: &str = "@destack/builtin";

/// Language builtins.
#[derive(Debug)]
pub struct LanguageBuiltins {
    /// The builtin package ID.
    pub package_id: PackageId,

    /// Core modules (operators, reflection, intrinsics).
    pub core_modules: Vec<ModuleId>,

    /// The prelude module ID (re-exports items available without imports).
    pub prelude_module_id: ModuleId,

    /// Lib modules cache ("dom" -> modules, "es2024" -> modules).
    pub lib_modules: DashMap<String, Vec<ModuleId>>,

    /// Resolved language items cache (LanguageItem -> GlobalSymbolId).
    /// (Populated lazily when items are first resolved after module compilation.)
    pub items: DashMap<LanguageItem, GlobalSymbolId>,
}

impl LanguageBuiltins {
    /// Create builtins by registering core modules from embedded sources.
    pub fn embedded(
        files: Arc<FileRegistry>,
        modules: Arc<ModuleRegistry>,
        packages: Arc<PackageRegistry>,
    ) -> Self {
        // create builtin package
        let package = Package {
            id: BUILTIN_PACKAGE_ID,
            kind: PackageKind::Builtin,
            uri: Uri::from_string("builtin://"),
            path: None,
            name: Some(BUILTIN_PACKAGE_NAME.to_string()),
            version: None,
            manifest: None,
            dsconfig: None,
            tsconfig: None,
            targets: Default::default(),
        };
        packages.insert(package);

        // register core modules from embedded sources
        let mut core_modules = Vec::with_capacity(CORE_SOURCES.len());
        for source in CORE_SOURCES {
            let uri = Uri::from_string(source.virtual_path());

            // create file
            let file_id = files.next_id();
            let file = File::from_text(
                file_id,
                source.name.to_string(),
                uri.clone(),
                None,
                FileType::Destack,
                source.content.to_string(),
            );
            files.insert(file);

            // create module id from path
            let module_path = if source.path.is_empty() {
                source.name.to_string()
            } else {
                format!("{}/{}", source.path, source.name)
            };
            let module_id = ModuleId::from_relative_path(
                BUILTIN_PACKAGE_ID,
                std::path::Path::new(&module_path),
            );

            // create blank module (will be parsed/bound later)
            let module = Module::blank(
                module_id,
                file_id,
                uri,
                None,
                BUILTIN_PACKAGE_ID,
                None,
                ModuleType::Module,
                LanguageType::Destack,
            );
            modules.insert(module);
            core_modules.push(module_id);
        }

        // compute prelude module ID
        let prelude_path = if PRELUDE_SOURCE.path.is_empty() {
            PRELUDE_SOURCE.name.to_string()
        } else {
            format!("{}/{}", PRELUDE_SOURCE.path, PRELUDE_SOURCE.name)
        };
        let prelude_module_id =
            ModuleId::from_relative_path(BUILTIN_PACKAGE_ID, std::path::Path::new(&prelude_path));

        Self {
            package_id: BUILTIN_PACKAGE_ID,
            core_modules,
            prelude_module_id,
            lib_modules: DashMap::new(),
            items: DashMap::new(),
        }
    }

    /// Get the ModuleId for a language item's defining module.
    pub fn module_for_item(&self, item: LanguageItem) -> ModuleId {
        let module_path = format!("{}.ds", item.module());
        ModuleId::from_relative_path(BUILTIN_PACKAGE_ID, std::path::Path::new(&module_path))
    }

    /// Load a lib module set (e.g., "dom", "es2024").
    pub fn load_lib(
        &self,
        name: &str,
        _files: Arc<FileRegistry>,
        _modules: Arc<ModuleRegistry>,
        _packages: Arc<PackageRegistry>,
    ) -> Vec<ModuleId> {
        // check cache first
        if let Some(cached) = self.lib_modules.get(name) {
            return cached.clone();
        }

        todo!("#Incomplete: load lib from embedded lib/ (into 'builtin://lib/'?)")
    }
}
