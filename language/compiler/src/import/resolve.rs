//! Module resolution: specifier/path/uri → ModuleId.
//!
//! These are synchronous operations that can be called at any time to get a ModuleId
//! for a given specifier, path, or URI. They register a blank module if one doesn't
//! exist yet, but don't load or parse the file.

use std::path::{Path, PathBuf};

use destack_resolver::Resolver;
use destack_source::{File, FileType, ModuleId, PackageId, StringId, Uri};
use destack_workspace::{Module, ModuleType, Package, PackageKind};

use crate::{Compiler, ImportError, ImportResult};

impl Compiler {
    /// Resolve a specifier to a ModuleId, registering a blank module if needed.
    pub fn resolve_specifier_to_module(
        &self,
        specifier: StringId,
        source_module: Option<ModuleId>,
    ) -> ImportResult<ModuleId> {
        let resolver = self.create_resolver();
        let directory = self.get_resolve_directory(source_module);

        // resolve specifier to path
        let specifier_str = self.program.strings.get(specifier).to_string();
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
            target: self.program.strings.intern(uri),
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
        let extension = path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let ty = ty.unwrap_or_else(|| FileType::from_extension_or_unknown(extension.as_ref()));
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

        // create and register blank module
        let module_id = ModuleId::from_path(package_id, path, package_root.as_deref());
        let module_type = ModuleType::from_extension(path).unwrap_or(ModuleType::Script);
        let module = Module::blank(
            module_id,
            file_id,
            uri,
            Some(path.clone()),
            package_id,
            module_type,
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
            .import
            .resolve
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
        Resolver::new(self.program.clone(), resolver_options)
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
            package_config: None,
            dsconfig: None,
            main_tsconfig_id: None,
            targets: Default::default(),
        };
        self.program.packages.insert(package);

        (package_id, Some(directory.to_path_buf()))
    }
}
