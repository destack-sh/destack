use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use parking_lot::RwLock;

use destack_builtin::BuiltinLibKind;
use destack_source::{FileId, FileVersion, LanguageType, ModuleId, ModuleVersion, PackageId, Uri};

use crate::{
    Loader, ModuleAst, ModuleComptime, ModuleDir, ModuleMir, ProfileId, SourceType, TargetId,
    TsConfigId,
};

/// The source/origin of a module.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModuleSource {
    /// User/project code.
    User,
    /// Builtin library code (core, std, or lib).
    Builtin(BuiltinLibKind),
}

/// The type of module content.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ModuleType {
    /// Code module (Destack, TypeScript, JavaScript).
    #[default]
    Code,
    /// Data module (JSON, TOML, YAML).
    Data,
    /// Text module (plain text, markdown, etc.).
    Text,
    /// Binary module (images, fonts, wasm, etc.).
    Binary,
}

/// Code-specific module data (AST, DIR, MIR).
#[derive(Debug, Default)]
pub struct ModuleCode {
    /// The AST-level module data (syntactic). None until Import phase completes.
    pub ast: Option<ModuleAst>,
    /// The base DIR-level module data (bind-only, profile-independent).
    pub dir_base: Option<ModuleDir>,
    /// The DIR-level module data per profile (semantic, profile-dependent).
    pub dirs: Vec<ModuleDir>,
    /// Comptime results per profile.
    pub comptimes: Vec<ModuleComptime>,
    /// The MIR-level module data (target-specific). One per target, populated by Lower phase.
    pub mirs: Vec<ModuleMir>,
}

/// The content of a module.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum ModuleContent {
    /// Code module with AST, DIR, MIR.
    Code(ModuleCode),
    /// Data module (JSON, TOML, YAML) with parsed value.
    Data {
        source: String,
        value: serde_json::Value,
        /// Base DIR for the data module (profile-independent).
        dir_base: Option<ModuleDir>,
        /// Profile-specific DIRs for the data module.
        dirs: Vec<ModuleDir>,
    },
    /// Text module (plain string content).
    Text {
        content: String,
        /// Base DIR for the text module (profile-independent).
        dir_base: Option<ModuleDir>,
        /// Profile-specific DIRs for the text module.
        dirs: Vec<ModuleDir>,
    },
    /// Binary module (raw bytes).
    Binary {
        bytes: Vec<u8>,
        /// Base DIR for the binary module (profile-independent).
        dir_base: Option<ModuleDir>,
        /// Profile-specific DIRs for the binary module.
        dirs: Vec<ModuleDir>,
    },
    /// Content not yet loaded.
    Unloaded,
}

/// A Module is a single source unit.
/// Destack treats all modules as "strict mode".
#[derive(Debug)]
pub struct Module {
    /// The id of the Module itself.
    pub id: ModuleId,
    /// The version of the Module (increments on each recompilation).
    pub version: ModuleVersion,
    /// The version of the source File this module was compiled from.
    pub source_version: FileVersion,
    /// The underlying source File (might be empty if placeholder or synthetic module).
    pub file_id: FileId,
    /// The URI of the Module.
    pub uri: Uri,
    /// The path to the Module.
    pub path: Option<PathBuf>,
    /// The package of the Module (every module belongs to a package).
    pub package_id: PackageId,
    /// The tsconfig of the Module (if any).
    pub tsconfig_id: Option<TsConfigId>,
    /// The source type of the Module (Script vs Module).
    pub source_type: SourceType,
    /// The language type of the Module (Destack, TypeScript, JavaScript, etc.).
    pub language_type: LanguageType,
    /// How the module content is loaded/interpreted.
    pub loader: Loader,
    /// The source/origin of the module (user code or builtin).
    pub source: ModuleSource,
    /// The type of module content (Code, Data, Text, Binary).
    pub module_type: ModuleType,
    /// The module content (code-specific data, or data/text/binary content).
    pub content: ModuleContent,
}

#[allow(clippy::too_many_arguments)]
impl Module {
    /// Create a new blank Module with the given loader (content not yet loaded).
    pub fn blank(
        id: ModuleId,
        file_id: FileId,
        source_version: FileVersion,
        uri: Uri,
        path: Option<PathBuf>,
        package_id: PackageId,
        tsconfig_id: Option<TsConfigId>,
        source_type: SourceType,
        language_type: LanguageType,
        loader: Loader,
        source: ModuleSource,
    ) -> Self {
        let module_type = loader.module_type();
        let content = match module_type {
            ModuleType::Code => ModuleContent::Code(ModuleCode::default()),
            _ => ModuleContent::Unloaded,
        };
        Self {
            id,
            version: ModuleVersion::INITIAL,
            source_version,
            file_id,
            uri,
            path,
            package_id,
            tsconfig_id,
            source_type,
            language_type,
            loader,
            source,
            module_type,
            content,
        }
    }

    /// Create a new code Module from an AST.
    pub fn from_ast(
        id: ModuleId,
        file_id: FileId,
        source_version: FileVersion,
        uri: Uri,
        path: Option<PathBuf>,
        package_id: PackageId,
        tsconfig_id: Option<TsConfigId>,
        source_type: SourceType,
        language_type: LanguageType,
        loader: Loader,
        source: ModuleSource,
        ast: ModuleAst,
    ) -> Self {
        Self {
            id,
            version: ModuleVersion::INITIAL,
            source_version,
            file_id,
            uri,
            path,
            package_id,
            tsconfig_id,
            source_type,
            language_type,
            loader,
            source,
            module_type: ModuleType::Code,
            content: ModuleContent::Code(ModuleCode {
                ast: Some(ast),
                dir_base: None,
                dirs: Vec::new(),
                comptimes: Vec::new(),
                mirs: Vec::new(),
            }),
        }
    }

    /// Whether this module is user/project code.
    #[inline]
    pub fn is_user(&self) -> bool {
        matches!(self.source, ModuleSource::User)
    }

    /// Whether this module is a builtin library.
    #[inline]
    pub fn is_builtin(&self) -> bool {
        matches!(self.source, ModuleSource::Builtin(_))
    }

    /// Whether this module is a code module.
    #[inline]
    pub fn is_code(&self) -> bool {
        matches!(self.content, ModuleContent::Code(_))
    }

    /// Get the code-specific data.
    ///
    /// # Panics
    /// Panics if this is not a code module.
    #[inline]
    pub fn code(&self) -> &ModuleCode {
        match &self.content {
            ModuleContent::Code(code) => code,
            _ => panic!("not a code module: {self:?}"),
        }
    }

    /// Get the code-specific data mutably.
    ///
    /// # Panics
    /// Panics if this is not a code module.
    #[inline]
    pub fn code_mut(&mut self) -> &mut ModuleCode {
        match &mut self.content {
            ModuleContent::Code(code) => code,
            _ => panic!("not a code module"),
        }
    }

    /// Get the AST.
    ///
    /// # Panics
    /// Panics if called before Import phase completes or if not a code module.
    #[inline]
    pub fn ast(&self) -> &ModuleAst {
        self.code().ast.as_ref().expect("no AST on module")
    }

    /// Get the AST mutably.
    ///
    /// # Panics
    /// Panics if called before Import phase completes or if not a code module.
    #[inline]
    pub fn ast_mut(&mut self) -> &mut ModuleAst {
        self.code_mut().ast.as_mut().expect("no AST on module")
    }

    /// Get the AST if it exists.
    #[inline]
    pub fn ast_maybe(&self) -> Option<&ModuleAst> {
        match &self.content {
            ModuleContent::Code(code) => code.ast.as_ref(),
            _ => None,
        }
    }

    /// Get the base DIR.
    ///
    /// # Panics
    /// Panics if called before Bind/Parse phase completes.
    #[inline]
    pub fn dir_base(&self) -> &ModuleDir {
        self.dir_base_maybe().expect("no base DIR on module")
    }

    /// Get the base DIR mutably.
    ///
    /// # Panics
    /// Panics if called before Bind/Parse phase completes.
    #[inline]
    pub fn dir_base_mut(&mut self) -> &mut ModuleDir {
        self.dir_base_maybe_mut().expect("no base DIR on module")
    }

    /// Get the base DIR mutably if it exists.
    #[inline]
    pub fn dir_base_maybe_mut(&mut self) -> Option<&mut ModuleDir> {
        match &mut self.content {
            ModuleContent::Code(code) => code.dir_base.as_mut(),
            ModuleContent::Data { dir_base, .. } => dir_base.as_mut(),
            ModuleContent::Text { dir_base, .. } => dir_base.as_mut(),
            ModuleContent::Binary { dir_base, .. } => dir_base.as_mut(),
            ModuleContent::Unloaded => None,
        }
    }

    /// Get the base DIR if it exists.
    #[inline]
    pub fn dir_base_maybe(&self) -> Option<&ModuleDir> {
        match &self.content {
            ModuleContent::Code(code) => code.dir_base.as_ref(),
            ModuleContent::Data { dir_base, .. } => dir_base.as_ref(),
            ModuleContent::Text { dir_base, .. } => dir_base.as_ref(),
            ModuleContent::Binary { dir_base, .. } => dir_base.as_ref(),
            ModuleContent::Unloaded => None,
        }
    }

    /// Get the DIR for a profile.
    ///
    /// # Panics
    /// Panics if called before Resolve phase completes for the profile.
    #[inline]
    pub fn dir(&self, profile: ProfileId) -> &ModuleDir {
        self.dir_maybe(profile)
            .unwrap_or_else(|| panic!("no DIR for profile {profile:?}"))
    }

    /// Get the DIR for a profile mutably.
    ///
    /// # Panics
    /// Panics if called before Resolve phase completes for the profile.
    #[inline]
    pub fn dir_mut(&mut self, profile: ProfileId) -> &mut ModuleDir {
        self.dir_maybe_mut(profile)
            .unwrap_or_else(|| panic!("no DIR for profile {profile:?}"))
    }

    /// Get the DIR for a profile if it exists.
    #[inline]
    pub fn dir_maybe(&self, profile: ProfileId) -> Option<&ModuleDir> {
        let dirs = match &self.content {
            ModuleContent::Code(code) => &code.dirs,
            ModuleContent::Data { dirs, .. } => dirs,
            ModuleContent::Text { dirs, .. } => dirs,
            ModuleContent::Binary { dirs, .. } => dirs,
            ModuleContent::Unloaded => return None,
        };
        dirs.iter().find(|dir| dir.profile_id == Some(profile))
    }

    /// Get the DIR for a profile mutably if it exists.
    #[inline]
    pub fn dir_maybe_mut(&mut self, profile: ProfileId) -> Option<&mut ModuleDir> {
        let dirs = match &mut self.content {
            ModuleContent::Code(code) => &mut code.dirs,
            ModuleContent::Data { dirs, .. } => dirs,
            ModuleContent::Text { dirs, .. } => dirs,
            ModuleContent::Binary { dirs, .. } => dirs,
            ModuleContent::Unloaded => return None,
        };
        dirs.iter_mut().find(|dir| dir.profile_id == Some(profile))
    }

    /// Get the comptime results for a profile.
    ///
    /// # Panics
    /// Panics if called before Execute phase completes for the profile or if not a code module.
    #[inline]
    pub fn comptime(&self, profile: ProfileId) -> &ModuleComptime {
        self.code()
            .comptimes
            .iter()
            .find(|comptime| comptime.profile_id == profile)
            .unwrap_or_else(|| panic!("no comptime results for profile {profile:?}"))
    }

    /// Get the comptime results for a profile mutably.
    ///
    /// # Panics
    /// Panics if called before Execute phase completes for the profile or if not a code module.
    #[inline]
    pub fn comptime_mut(&mut self, profile: ProfileId) -> &mut ModuleComptime {
        self.code_mut()
            .comptimes
            .iter_mut()
            .find(|comptime| comptime.profile_id == profile)
            .unwrap_or_else(|| panic!("no comptime results for profile {profile:?}"))
    }

    /// Get the comptime results for a profile if they exist.
    #[inline]
    pub fn comptime_maybe(&self, profile: ProfileId) -> Option<&ModuleComptime> {
        match &self.content {
            ModuleContent::Code(code) => code
                .comptimes
                .iter()
                .find(|comptime| comptime.profile_id == profile),
            _ => None,
        }
    }

    /// Get the MIR for a target if it exists.
    #[inline]
    pub fn mir_maybe(&self, target: &TargetId) -> Option<&ModuleMir> {
        match &self.content {
            ModuleContent::Code(code) => code.mirs.iter().find(|mir| &mir.target == target),
            _ => None,
        }
    }

    /// Get the MIR for a target.
    ///
    /// # Panics
    /// Panics if called before Lower phase completes or if not a code module.
    #[inline]
    pub fn mir(&self, target: &TargetId) -> &ModuleMir {
        self.code()
            .mirs
            .iter()
            .find(|mir| &mir.target == target)
            .unwrap_or_else(|| panic!("no MIR for target {target:?}"))
    }

    /// Get the MIR for a target mutably.
    ///
    /// # Panics
    /// Panics if called before Lower phase completes or if not a code module.
    #[inline]
    pub fn mir_mut(&mut self, target: &TargetId) -> &mut ModuleMir {
        self.code_mut()
            .mirs
            .iter_mut()
            .find(|mir| &mir.target == target)
            .unwrap_or_else(|| panic!("no MIR for target {target:?}"))
    }
}

/// Registry of Modules. THREAD-SAFE.
#[derive(Debug)]
pub struct ModuleRegistry {
    /// The modules by id.
    modules_by_id: DashMap<ModuleId, Arc<RwLock<Module>>>,
    /// URI-based index for looking up modules by their URI.
    modules_by_uri: DashMap<Uri, ModuleId>,
    /// Path-based index for looking up modules by their path (only for modules with valid paths).
    modules_by_path: DashMap<PathBuf, ModuleId>,
    /// FileId-based index for looking up modules by their source file id.
    modules_by_file_id: DashMap<FileId, ModuleId>,
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleRegistry {
    /// Create a new ModuleRegistry.
    pub fn new() -> Self {
        Self {
            modules_by_id: DashMap::new(),
            modules_by_uri: DashMap::new(),
            modules_by_path: DashMap::new(),
            modules_by_file_id: DashMap::new(),
        }
    }

    /// Insert a module into the registry.
    pub fn insert(&self, module: Module) {
        let uri = module.uri.clone();
        let path = module.path.clone();
        let file_id = module.file_id;
        let id = module.id;
        self.modules_by_id.insert(id, Arc::new(RwLock::new(module)));
        self.modules_by_uri.insert(uri, id);
        self.modules_by_file_id.insert(file_id, id);
        if let Some(path) = path {
            self.modules_by_path.insert(path, id);
        }
    }

    /// Check if a module exists by id.
    pub fn contains(&self, id: ModuleId) -> bool {
        self.modules_by_id.contains_key(&id)
    }

    /// Get a module by module id.
    ///
    /// # Panics
    /// Panics if the module is not found.
    #[inline]
    pub fn get(&self, id: ModuleId) -> Arc<RwLock<Module>> {
        self.modules_by_id
            .get(&id)
            .unwrap_or_else(|| panic!("module not found for id: {id:?}"))
            .clone()
    }

    /// Get a module id by its URI.
    pub fn get_id_by_uri(&self, uri: &Uri) -> Option<ModuleId> {
        self.modules_by_uri.get(uri).map(|r| *r.value())
    }

    /// Get a module by its URI.
    pub fn get_by_uri(&self, uri: &Uri) -> Option<Arc<RwLock<Module>>> {
        let id = self.get_id_by_uri(uri)?;
        Some(self.get(id))
    }

    /// Check if a module exists at the given URI.
    pub fn contains_uri(&self, uri: &Uri) -> bool {
        self.modules_by_uri.contains_key(uri)
    }

    /// Get a module id by its path.
    pub fn get_id_by_path(&self, path: &Path) -> Option<ModuleId> {
        self.modules_by_path.get(path).map(|r| *r.value())
    }

    /// Get a module by its path.
    pub fn get_by_path(&self, path: &Path) -> Option<Arc<RwLock<Module>>> {
        let id = self.get_id_by_path(path)?;
        Some(self.get(id))
    }

    /// Check if a module exists at the given path.
    pub fn contains_path(&self, path: &Path) -> bool {
        self.modules_by_path.contains_key(path)
    }

    /// Get a module id by its source file id.
    pub fn get_id_by_file_id(&self, file_id: FileId) -> Option<ModuleId> {
        self.modules_by_file_id.get(&file_id).map(|r| *r.value())
    }

    /// Get a module by its source file id.
    pub fn get_by_file_id(&self, file_id: FileId) -> Option<Arc<RwLock<Module>>> {
        let id = self.get_id_by_file_id(file_id)?;
        Some(self.get(id))
    }

    /// Iterate over the modules in the registry.
    pub fn iter(&self) -> impl Iterator<Item = Arc<RwLock<Module>>> {
        let snapshot: Vec<_> = self
            .modules_by_id
            .iter()
            .map(|r| r.value().clone())
            .collect();
        snapshot.into_iter()
    }

    /// Get the number of modules in the registry.
    pub fn len(&self) -> usize {
        self.modules_by_id.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.modules_by_id.is_empty()
    }
}
