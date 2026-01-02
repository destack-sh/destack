//! Module resolution: specifier/path/uri → ModuleId.
//!
//! These are synchronous operations that can be called at any time to get a ModuleId
//! for a given specifier, path, or URI. They register a blank module if one doesn't
//! exist yet, but don't load or parse the file.

use std::path::{Path, PathBuf};

use destack_base::StringId;
use destack_builtin::builtin_lib;
use destack_resolver::Resolver;
use destack_source::{File, FileType, FileVersion, LanguageType, ModuleId, PackageId, Uri};
use destack_workspace::{Module, ModuleType, Package, PackageKind};

use crate::{Compiler, ImportError, ImportResult};

/// Extensions to try for builtin modules.
const BUILTIN_EXTENSIONS: &[&str] = &[
    ".d.ts", ".d.ds", ".ds", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".json", ".node",
];

impl Compiler {
    /// Resolve a specifier to a ModuleId, registering a blank module if needed.
    pub fn resolve_specifier_to_module(
        &self,
        specifier: StringId,
        source_module: Option<ModuleId>,
    ) -> ImportResult<ModuleId> {
        let specifier_str = self.program.strings.get(specifier).to_string();

        // resolve builtin module imports (builtin:// URIs)
        if let Some(source_id) = source_module
            && let Some(module_id) = self.resolve_builtin_specifier(&specifier_str, source_id)
        {
            return Ok(module_id);
        }

        // resolve non-builtin imports
        let resolver = self.create_resolver();
        let directory = self.get_resolve_directory(source_module);

        // resolve specifier to path
        let resolution = resolver
            .resolve(&directory, &specifier_str)
            .map_err(|error| ImportError::ModuleNotFound {
                target: specifier,
                error: Some(error),
            })?;

        // check if module already exists for this path
        if let Some(module_id) = self.program.modules.get_id_by_path(&resolution.path) {
            return Ok(module_id);
        }

        // register blank module
        self.register_blank_module(&resolution.path, None, &resolver)
    }

    /// Resolve a specifier from a builtin module to a builtin module (see LanguageBuiltins).
    fn resolve_builtin_specifier(
        &self,
        specifier: &str,
        source_module: ModuleId,
    ) -> Option<ModuleId> {
        // get source module URI
        let source = self.program.modules.get(source_module);
        let source_uri = source.read().uri.clone();
        let source_str: &str = source_uri.as_ref();

        // check if source is a builtin module
        if !source_str.starts_with("builtin://") {
            return None;
        }

        // resolve builtin libs by name for builtin modules
        if !specifier.starts_with("./") && !specifier.starts_with("../") {
            let builtins = self.program.builtins.as_ref()?;

            // map specifier to builtin lib name
            let lib_name = if let Some(lib) = builtin_lib(specifier) {
                lib.name
            } else if let Some(source_lib_name) = builtins.lib_name_for_module(source_module) {
                let source_lib = builtin_lib(source_lib_name)?;
                source_lib
                    .specifier_aliases
                    .iter()
                    .find(|(alias, _)| *alias == specifier)
                    .map(|(_, target)| *target)?
            } else {
                return None;
            };

            // resolve entry module and ensure lib is loaded
            let lib = builtin_lib(lib_name)?;
            let entry_source = lib
                .sources
                .iter()
                .find(|source| source.name == "index.d.ts")
                .unwrap_or_else(|| &lib.sources[0]);
            let module_path = entry_source.module_path();
            let module_id =
                ModuleId::from_relative_path(builtins.package_id, Path::new(&module_path));
            builtins.load_lib(
                lib_name,
                self.program.files.clone(),
                self.program.modules.clone(),
            );
            return Some(module_id);
        }

        // resolve relative path against source URI
        //  - source: builtin://core/prelude.ds
        //  - specifier: ./reflection/type.ds
        //  - target: builtin://core/reflection/type.ds
        let source_dir = source_str.rsplit_once('/').map(|(dir, _)| dir)?;
        let target_uri_str = resolve_relative_uri(source_dir, specifier);
        let target_uri = Uri::from_string(&target_uri_str);

        // look up target module by exact URI
        if let Some(module_id) = self.program.modules.get_id_by_uri(&target_uri) {
            return Some(module_id);
        }

        // try extensions
        for extension in BUILTIN_EXTENSIONS {
            let candidate_uri = Uri::from_string(format!("{target_uri_str}{extension}"));
            if let Some(module_id) = self.program.modules.get_id_by_uri(&candidate_uri) {
                return Some(module_id);
            }
        }

        // try index (with extensions)
        for extension in BUILTIN_EXTENSIONS {
            let candidate_uri = Uri::from_string(format!("{target_uri_str}/index{extension}"));
            if let Some(module_id) = self.program.modules.get_id_by_uri(&candidate_uri) {
                return Some(module_id);
            }
        }

        None
    }

    /// Resolve a path to a ModuleId, registering a blank module if needed.
    pub fn resolve_path_to_module(&self, path: &PathBuf) -> ImportResult<ModuleId> {
        let resolver = self.create_resolver();

        // check if module already exists for this path
        if let Some(module_id) = self.program.modules.get_id_by_path(path) {
            return Ok(module_id);
        }

        // register blank module
        self.register_blank_module(path, None, &resolver)
    }

    /// Resolve a URI to a ModuleId, registering a blank module if needed.
    pub fn resolve_uri_to_module(&self, uri: &Uri) -> ImportResult<ModuleId> {
        // check if module already exists for this URI
        if let Some(module_id) = self.program.modules.get_id_by_uri(uri) {
            return Ok(module_id);
        }

        let path = uri.to_path().ok_or_else(|| ImportError::ModuleNotFound {
            target: self.program.strings.intern(uri.as_ref()),
            error: None,
        })?;

        self.resolve_path_to_module(&path.to_path_buf())
    }

    /// Register a blank module for a path.
    ///
    /// Creates a blank File, determines the package, computes ModuleId,
    /// and registers a blank Module (without AST).
    fn register_blank_module(
        &self,
        path: &PathBuf,
        ty: Option<FileType>,
        resolver: &Resolver,
    ) -> ImportResult<ModuleId> {
        // lock to prevent race conditions
        let uri = Uri::from_path(path);
        let import_lock = self.get_import_lock(&uri);
        let mut import_guard = import_lock.lock();

        // check if another thread already registered this module
        if let Some(module_id) = *import_guard {
            return Ok(module_id);
        }
        if let Some(module_id) = self.program.modules.get_id_by_uri(&uri) {
            *import_guard = Some(module_id);
            return Ok(module_id);
        }

        // create blank file entry (content loaded later during import)
        let ty = ty.unwrap_or_else(|| FileType::from_path_or_unknown(path));
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let file_id = self.program.files.next_id();
        let file = File::unloaded(file_id, name, uri.clone(), Some(path.clone()), ty);
        self.program.files.insert(file);

        // find or create package
        let (package_id, package_root) = self.resolve_or_create_package_for_path(path, resolver);

        // find tsconfig (if any)
        let tsconfig_id = resolver.find_tsconfig(path);

        // create and register blank module
        let module_id = ModuleId::from_path(package_id, path, package_root.as_deref());
        let module_type = ModuleType::from_extension(path).unwrap_or(ModuleType::Script);
        let language_type = LanguageType::from(ty);
        let module = Module::blank(
            module_id,
            file_id,
            FileVersion::INITIAL,
            uri,
            Some(path.clone()),
            package_id,
            tsconfig_id,
            module_type,
            language_type,
        );
        self.program.modules.insert(module);

        // mark as registered
        *import_guard = Some(module_id);
        drop(import_guard);

        tracing::trace!(?module_id, ?path, "import.resolve.register");
        Ok(module_id)
    }

    /// Create the resolver with standard options.
    pub(super) fn create_resolver(&self) -> Resolver {
        let resolver_options = self
            .options
            .import_resolve
            .clone()
            .with_extensions(vec![
                ".ds".into(),
                ".tsx".into(),
                ".ts".into(),
                ".jsx".into(),
                ".js".into(),
                ".mjs".into(),
                ".cjs".into(),
                ".json".into(),
                ".node".into(),
            ])
            .with_conditions(vec!["types".to_string(), "import".to_string()]);
        Resolver::from_program(&self.program, resolver_options)
    }

    /// Get the directory to resolve from for a source module.
    pub(super) fn get_resolve_directory(&self, source_module: Option<ModuleId>) -> PathBuf {
        if let Some(module_id) = source_module {
            let module = self.program.modules.get(module_id);
            let module_file = self.program.files.get(module.read().file_id);
            module_file
                .uri
                .to_path_buf()
                .and_then(|path| path.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| self.program.cwd.clone())
        } else {
            self.program.cwd.clone()
        }
    }

    /// Resolve or create a package for a file path.
    pub(super) fn resolve_or_create_package_for_path(
        &self,
        path: &Path,
        resolver: &Resolver,
    ) -> (PackageId, Option<PathBuf>) {
        // try to find a physical package (package.json)
        if let Some(package_id) = resolver.find_package(path) {
            let package = self.program.packages.get(package_id);
            let package_root = package.read().path.clone();
            return (package_id, package_root);
        }

        // create synthetic package for file's directory
        let directory = path.parent().unwrap_or(path);
        let package_id = PackageId::from_synthetic_path(directory);

        // check if synthetic package already exists
        if self.program.packages.contains(package_id) {
            return (package_id, Some(directory.to_path_buf()));
        }

        // create and insert synthetic package
        let package = Package {
            id: package_id,
            kind: PackageKind::Synthetic,
            uri: Uri::from_path(directory),
            path: Some(directory.to_path_buf()),
            name: None,
            version: None,
            manifest: None,
            dsconfig: None,
            tsconfig: None,
            targets: Default::default(),
        };
        self.program.packages.insert(package);

        (package_id, Some(directory.to_path_buf()))
    }
}

/// Resolve a relative specifier against a base URI directory.
/// e.g., resolve_relative_uri("builtin://core", "./reflection/type.ds")
///       -> "builtin://core/reflection/type.ds"
fn resolve_relative_uri(base_dir: &str, specifier: &str) -> String {
    let mut parts: Vec<&str> = base_dir.split('/').collect();

    for segment in specifier.split('/') {
        match segment {
            "." | "" => continue,
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }

    parts.join("/")
}
