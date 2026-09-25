use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use indexmap::IndexMap;
use tspp_artifact::SourceDependency;
use tspp_core::{Blob, BlobId, BlobMemory, BlobStore};
use tspp_source::{
    File, FileId, FileMetadata, FileType, LanguageType, Loader, ModuleId, PackageId, TargetId, Uri,
};

use crate::config::{DestackFile, parse_jsonc_file};
use crate::{
    ExportKind, Module, Package, PackageExport, PackageKind, Repository, RepositoryError, Revision,
};

const BUILTIN_PACKAGE_URI: &str = "tspp://";
const BUILTIN_SOURCE_DIRECTORY: &str = "src";

// NOTE: the builtin library package is generated at build time and auto included as raw strings here.
include!(concat!(env!("OUT_DIR"), "/builtin.rs"));

/// One export from the embedded Builtin Package.
#[derive(Debug, Clone, Copy)]
pub struct BuiltinExport {
    /// The public export specifier.
    pub specifier: &'static str,
    /// The package relative export path.
    pub path: &'static str,
    /// The exported material kind.
    pub kind: ExportKind,
}

/// One source file from the embedded Builtin Package.
#[derive(Debug, Clone, Copy)]
pub struct BuiltinFile {
    /// The stable source file id.
    pub file_id: FileId,
    /// The stable module id.
    pub module_id: ModuleId,
    /// The exact source Blob.
    pub blob: Blob,
    /// The canonical module URI.
    pub uri: &'static str,
    /// The path relative to `language/library/src`.
    pub path: &'static str,
    /// The source content.
    pub content: &'static str,
    /// The source line start byte offsets.
    pub line_starts: &'static [u32],
}

/// Embedded Builtin Package sources and canonical identity.
#[derive(Debug, Clone)]
pub struct EmbeddedBuiltinPackage {
    /// The package identity.
    package: Arc<Package>,
    /// Loaded source files in generated source order.
    files: Vec<Arc<File>>,
    /// The loaded package manifest.
    manifest_file: Arc<File>,
    /// The modules shipped in this package.
    modules: Vec<(ModuleId, Arc<Module>)>,
}

impl EmbeddedBuiltinPackage {
    /// Create the embedded Builtin Package.
    pub fn new(blobs: &BlobStore) -> Self {
        // retain and load the embedded manifest
        let manifest_memory = BUILTIN_MANIFEST_FILE.memory();
        blobs
            .retain_memory(manifest_memory.clone())
            .expect("embedded builtin Blob should be unique");
        let manifest_file = Arc::new(BUILTIN_MANIFEST_FILE.file(manifest_memory));

        // read declared package exports and build targets
        let exports = BUILTIN_EXPORTS
            .iter()
            .map(|export| (export.specifier.to_string(), export.package_export()))
            .collect();
        let id = BUILTIN_PACKAGE_ID;
        let source =
            parse_jsonc_file(&manifest_file).expect("embedded builtin manifest should parse");
        let configuration = DestackFile::from_file(
            BUILTIN_MANIFEST_FILE.file_id,
            vec![BUILTIN_MANIFEST_FILE.file_id],
            PathBuf::from(BUILTIN_MANIFEST_FILE.path),
            source,
        )
        .expect("embedded builtin manifest should build");
        let configuration = Arc::new(configuration);
        let targets = configuration
            .destack
            .targets
            .iter()
            .map(|(name, target)| (TargetId::new(id, name), target.clone()))
            .collect();
        let package = Package {
            id,
            kind: PackageKind::Embedded,
            is_builtin: true,
            uri: Uri::from_string(BUILTIN_PACKAGE_URI),
            path: None,
            name: Some(BUILTIN_PACKAGE_NAME.to_string()),
            version: None,
            dependencies: IndexMap::new(),
            conditional_dependencies: Vec::new(),
            vendor: Default::default(),
            exports,
            topology: Default::default(),
            targets,
            configuration: Some(configuration),
        };

        let package = Arc::new(package);

        // build module records from generated identities
        let modules = BUILTINS
            .iter()
            .map(|builtin| builtin.module_entry())
            .collect();

        // attach generated source descriptors to static memory
        let files = BUILTINS
            .iter()
            .map(|builtin| {
                let memory = builtin.memory();
                let file = Arc::new(builtin.file(memory.clone()));

                // publish the same immutable memory through the host Blob store
                blobs
                    .retain_memory(memory)
                    .expect("embedded builtin Blob should be unique");

                file
            })
            .collect();

        Self {
            package,
            files,
            manifest_file,
            modules,
        }
    }

    /// Return the canonical Builtin PackageId.
    pub fn package_id(&self) -> PackageId {
        self.package.id
    }

    /// Return the reserved Builtin Package name.
    pub fn package_name(&self) -> &str {
        BUILTIN_PACKAGE_NAME
    }

    /// Return the canonical Builtin Package URI.
    pub fn package_uri(&self) -> &Uri {
        &self.package.uri
    }

    /// Return the embedded Builtin Package.
    pub fn package(&self) -> Arc<Package> {
        self.package.clone()
    }

    /// Return all embedded Builtin files.
    pub fn files(&self) -> &'static [BuiltinFile] {
        BUILTINS
    }

    /// Return one builtin module URI from an absolute builtin specifier.
    pub fn module_uri_for_specifier(&self, specifier: &str) -> Option<Uri> {
        let path = specifier
            .strip_prefix(BUILTIN_PACKAGE_URI)
            .or_else(|| specifier.strip_prefix("tspp:"))?;

        self.module_uri_for_export_key(&builtin_export_key(path))
    }

    /// Return one internal builtin module URI from an absolute builtin specifier.
    pub fn module_uri_for_internal_specifier(&self, specifier: &str) -> Option<Uri> {
        let path = specifier
            .strip_prefix(BUILTIN_PACKAGE_URI)
            .or_else(|| specifier.strip_prefix("tspp:"))?;

        Some(self.canonical_module_uri(canonical_builtin_path(path)))
    }

    /// Return the canonical module URI for one extensionless builtin path.
    fn canonical_module_uri(&self, path: &str) -> Uri {
        let key = format!("{BUILTIN_PACKAGE_URI}{path}");
        match Self::builtin_file_for_uri(&key) {
            Some(file) => Uri::from_string(format!("{BUILTIN_PACKAGE_URI}{}", file.path)),
            None => Uri::from_string(key),
        }
    }

    /// Return one builtin module URI from a relative builtin specifier.
    pub fn module_uri_for_relative_specifier(
        &self,
        base_uri: &str,
        specifier: &str,
    ) -> Option<Uri> {
        let base = match Self::builtin_file_for_uri(base_uri) {
            Some(file) => file.path.strip_suffix(".tspp").unwrap_or(file.path),
            None => base_uri.strip_prefix(BUILTIN_PACKAGE_URI)?,
        };
        let mut parts = base.split('/').collect::<Vec<_>>();
        parts.pop();

        // fold relative path segments
        for part in specifier.split('/') {
            if part.is_empty() || part == "." {
                continue;
            }

            if part == ".." {
                parts.pop()?;
            } else {
                parts.push(part);
            }
        }

        let path = parts.join("/");

        Some(self.canonical_module_uri(canonical_builtin_path(&path)))
    }

    /// Return one builtin file by file id.
    pub fn builtin_file(&self, file_id: FileId) -> Option<BuiltinFile> {
        if file_id == BUILTIN_MANIFEST_FILE.file_id {
            return Some(BUILTIN_MANIFEST_FILE);
        }

        let index = Self::source_index(file_id)?;

        Some(BUILTINS[index])
    }

    /// Return one builtin file's content blob by file id.
    pub fn builtin_blob(&self, file_id: FileId) -> Option<Blob> {
        self.file(file_id).map(|file| file.blob)
    }

    /// Return one loaded builtin source file by file id.
    pub fn file(&self, file_id: FileId) -> Option<&Arc<File>> {
        if file_id == BUILTIN_MANIFEST_FILE.file_id {
            return Some(&self.manifest_file);
        }

        let index = Self::source_index(file_id)?;

        self.files.get(index)
    }

    /// Return one builtin file by path or URI.
    pub fn builtin_file_for_path(&self, path: &Path) -> Option<BuiltinFile> {
        let path = path.to_str()?;

        Self::builtin_file_for_uri(path)
    }

    /// Return one builtin file id by path or URI.
    pub fn file_id_for_path(&self, path: &Path) -> Option<FileId> {
        self.builtin_file_for_path(path).map(|file| file.file_id)
    }

    /// Return one loaded builtin source file by its source URI.
    pub fn file_for_uri(&self, uri: &Uri) -> Option<&Arc<File>> {
        if uri == &self.manifest_file.uri {
            return Some(&self.manifest_file);
        }

        let path = uri.as_ref().strip_prefix(BUILTIN_PACKAGE_URI)?;
        let index = BUILTINS
            .binary_search_by(|builtin| builtin.path.cmp(path))
            .ok()?;

        self.files.get(index)
    }

    /// Return builtin modules keyed by module id.
    pub(crate) fn modules(&self) -> impl Iterator<Item = (ModuleId, Arc<Module>)> + '_ {
        self.modules
            .iter()
            .map(|(module_id, module)| (*module_id, module.clone()))
    }

    /// Return builtin module ids.
    pub fn module_ids(&self) -> impl Iterator<Item = ModuleId> + '_ {
        self.modules.iter().map(|(module_id, _)| *module_id)
    }

    /// Return one authored builtin module id.
    pub(crate) fn module_id_for_path(
        &self,
        path: &Path,
        package_root: &Path,
        loader: Option<&str>,
    ) -> Result<ModuleId, RepositoryError> {
        let path = self.relative_source_path(path, package_root)?;

        Ok(ModuleId::from_path_with_loader(
            self.package_id(),
            path,
            None,
            loader,
        ))
    }

    /// Return one authored builtin module URI.
    pub(crate) fn module_uri_for_path(
        &self,
        path: &Path,
        package_root: &Path,
    ) -> Result<Uri, RepositoryError> {
        let path = self.relative_source_path(path, package_root)?;
        let path = path.to_string_lossy().replace('\\', "/");

        Ok(Uri::from_string(format!("{BUILTIN_PACKAGE_URI}{path}")))
    }

    /// Return one path relative to the canonical builtin source directory.
    fn relative_source_path<'a>(
        &self,
        path: &'a Path,
        package_root: &Path,
    ) -> Result<&'a Path, RepositoryError> {
        let source_root = package_root.join(BUILTIN_SOURCE_DIRECTORY);
        let Ok(path) = path.strip_prefix(&source_root) else {
            return Err(RepositoryError::BuiltinModuleOutsideSourceDirectory {
                path: path.to_path_buf(),
                source_directory: source_root,
            });
        };
        let is_relative = path
            .components()
            .all(|component| matches!(component, Component::Normal(_)));
        if !is_relative {
            return Err(RepositoryError::BuiltinModuleOutsideSourceDirectory {
                path: source_root.join(path),
                source_directory: source_root,
            });
        }

        Ok(path)
    }

    /// Return one source-table position by stable file id.
    fn source_index(file_id: FileId) -> Option<usize> {
        let index = BUILTIN_FILE_IDS
            .binary_search_by_key(&file_id, |(candidate, _)| *candidate)
            .ok()?;

        Some(BUILTIN_FILE_IDS[index].1)
    }

    /// Return one generated builtin file by canonical URI.
    fn builtin_file_for_uri(uri: &str) -> Option<BuiltinFile> {
        if uri == BUILTIN_MANIFEST_FILE.uri {
            return Some(BUILTIN_MANIFEST_FILE);
        }

        let path = uri.strip_prefix(BUILTIN_PACKAGE_URI)?;

        // accept an exact source path
        if let Ok(index) = BUILTINS.binary_search_by(|builtin| builtin.path.cmp(path)) {
            return Some(BUILTINS[index]);
        }

        // resolve one extensionless source path
        let file_path = format!("{path}.tspp");
        if let Ok(index) = BUILTINS.binary_search_by(|builtin| builtin.path.cmp(&file_path)) {
            return Some(BUILTINS[index]);
        }

        // resolve one extensionless index module
        let index_path = format!("{path}/index.tspp");
        let index = BUILTINS
            .binary_search_by(|builtin| builtin.path.cmp(&index_path))
            .ok()?;

        Some(BUILTINS[index])
    }
}

impl BuiltinExport {
    /// Return this builtin export as a resolved package export.
    fn package_export(self) -> PackageExport {
        PackageExport {
            kind: self.kind,
            path: self.path.to_string(),
            when: None,
        }
    }
}

impl Repository {
    /// Return the embedded Builtin Package.
    pub fn embedded_builtin(&self) -> &EmbeddedBuiltinPackage {
        self.host.embedded_builtin()
    }

    /// Return the Builtin Package selected for one revision.
    pub fn builtin_package(&self, revision: Revision) -> Result<Arc<Package>, RepositoryError> {
        self.package(revision, self.embedded_builtin().package_id())?
            .ok_or(RepositoryError::MissingPackage {
                package: self.embedded_builtin().package_id(),
            })
    }

    /// Return whether one package id is the canonical builtin identity.
    pub fn is_builtin_package(&self, package: PackageId) -> bool {
        package == self.embedded_builtin().package_id()
    }

    /// Return module ids from the Builtin Package selected for one revision.
    pub fn builtin_module_ids(&self, revision: Revision) -> Result<Vec<ModuleId>, RepositoryError> {
        self.package_module_ids(revision, self.embedded_builtin().package_id())
    }

    /// Resolve one public specifier through the Builtin Package selected for one revision.
    pub fn builtin_module_uri_for_specifier(
        &self,
        revision: Revision,
        specifier: &str,
        observations: &mut Vec<SourceDependency>,
    ) -> Result<Option<Uri>, RepositoryError> {
        let Some(path) = specifier
            .strip_prefix(BUILTIN_PACKAGE_URI)
            .or_else(|| specifier.strip_prefix("tspp:"))
        else {
            return Ok(None);
        };
        let export_key = builtin_export_key(path);
        let package = self.builtin_package(revision)?;
        observations.push(self.package_dependency(revision, package.id)?);
        let Some(export) = package.export(&export_key) else {
            return Ok(None);
        };
        if export.kind != ExportKind::Module {
            return Ok(None);
        }

        // embedded package exports already use canonical builtin URIs
        let Some(package_root) = package.path.as_deref() else {
            return Ok(self.embedded_builtin().module_uri_for_specifier(specifier));
        };

        // authored package exports resolve from their physical source paths
        let export_path = export.path.strip_prefix("./").unwrap_or(&export.path);
        let export_path = package_root.join(export_path);
        let uri = self
            .embedded_builtin()
            .module_uri_for_path(&export_path, package_root)?;
        let file = self.file_id(&export_path);
        let module = self.module_id_for_file(revision, file)?;
        observations.push(SourceDependency::module_path(file, module));

        Ok(Some(uri))
    }

    /// Resolve one internal specifier through the Builtin Package selected for one revision.
    pub fn builtin_module_uri_for_internal_specifier(
        &self,
        revision: Revision,
        specifier: &str,
        observations: &mut Vec<SourceDependency>,
    ) -> Result<Option<Uri>, RepositoryError> {
        let Some(path) = specifier
            .strip_prefix(BUILTIN_PACKAGE_URI)
            .or_else(|| specifier.strip_prefix("tspp:"))
        else {
            return Ok(None);
        };
        let package = self.builtin_package(revision)?;
        observations.push(self.package_dependency(revision, package.id)?);

        // embedded sources resolve through their generated file table
        let Some(package_root) = package.path.as_deref() else {
            return Ok(self
                .embedded_builtin()
                .module_uri_for_internal_specifier(specifier));
        };

        self.authored_builtin_module_uri(revision, package_root, path, observations)
    }

    /// Resolve one relative specifier through the Builtin Package selected for one revision.
    pub fn builtin_module_uri_for_relative_specifier(
        &self,
        revision: Revision,
        base_uri: &str,
        specifier: &str,
        observations: &mut Vec<SourceDependency>,
    ) -> Result<Option<Uri>, RepositoryError> {
        let package = self.builtin_package(revision)?;
        observations.push(self.package_dependency(revision, package.id)?);
        let Some(uri) = self
            .embedded_builtin()
            .module_uri_for_relative_specifier(base_uri, specifier)
        else {
            return Ok(None);
        };

        // embedded sources resolve through their generated file table
        let Some(package_root) = package.path.as_deref() else {
            return Ok(Some(uri));
        };

        let Some(path) = uri.as_ref().strip_prefix(BUILTIN_PACKAGE_URI) else {
            return Ok(None);
        };

        self.authored_builtin_module_uri(revision, package_root, path, observations)
    }

    /// Resolve one extensionless authored builtin path to its loaded module URI.
    fn authored_builtin_module_uri(
        &self,
        revision: Revision,
        package_root: &Path,
        path: &str,
        observations: &mut Vec<SourceDependency>,
    ) -> Result<Option<Uri>, RepositoryError> {
        // enumerate extensionless source candidates in resolution order
        let path = canonical_builtin_path(path);
        let candidates = [
            path.to_string(),
            format!("{path}.tspp"),
            format!("{path}/index.tspp"),
        ];
        let source_root = package_root.join(BUILTIN_SOURCE_DIRECTORY);

        // observe each candidate, including paths that do not exist
        for candidate in candidates {
            let file = self.file_id(&source_root.join(&candidate));
            let module = self.module_id_for_file(revision, file)?;
            observations.push(SourceDependency::module_path(file, module));
            let candidate = Uri::from_string(format!("{BUILTIN_PACKAGE_URI}{candidate}"));
            if self.module_id_for_uri(revision, &candidate)?.is_some() {
                return Ok(Some(candidate));
            }
        }

        Ok(None)
    }
}

impl BuiltinFile {
    /// Return the stable builtin source file id.
    pub fn file_id(self) -> FileId {
        self.file_id
    }

    /// Return the stable builtin module id.
    pub fn module_id(self) -> ModuleId {
        self.module_id
    }

    /// Return this builtin as a module index entry.
    fn module_entry(self) -> (ModuleId, Arc<Module>) {
        let module = Module::blank(
            self.module_id,
            self.file_id(),
            Uri::from_string(format!("{BUILTIN_PACKAGE_URI}{}", self.path)),
            Some(PathBuf::from(self.path)),
            self.module_id.package_id,
            Some(LanguageType::Tspp),
            Loader::Tspp,
        );

        (module.id, Arc::new(module))
    }

    /// Return the source file name.
    pub fn name(self) -> String {
        Path::new(self.path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.path.to_string())
    }

    /// Return this builtin as a source file.
    fn file(self, memory: Arc<BlobMemory>) -> File {
        // build virtual source file
        let file_type = if self.path.ends_with(".json") {
            FileType::Json
        } else {
            FileType::Tspp
        };

        // safety: include_str provides UTF-8 and the build script derives these line starts
        let file = unsafe {
            File::from_indexed_text(
                self.file_id,
                self.name(),
                Uri::from_string(format!("{BUILTIN_PACKAGE_URI}{}", self.path)),
                None,
                file_type,
                memory,
                self.line_starts,
            )
        };

        file.expect("embedded Builtin File should fit source coordinates")
    }

    /// Retain the build-verified static source bytes.
    fn memory(self) -> Arc<BlobMemory> {
        // safety: the build script derives the Blob from these exact included bytes
        let memory = unsafe { BlobMemory::from_static(self.blob, self.content.as_bytes()) };

        Arc::new(memory)
    }

    /// Return metadata for this builtin file.
    pub fn metadata(self) -> FileMetadata {
        FileMetadata::new(true, false, false, self.blob.byte_len, None)
    }
}

impl EmbeddedBuiltinPackage {
    /// Return one builtin module URI from one package export key.
    fn module_uri_for_export_key(&self, export_key: &str) -> Option<Uri> {
        let export = self.package.export(export_key)?;
        if export.kind != ExportKind::Module {
            return None;
        }

        let path = export.path.strip_prefix("./src/")?;

        Some(self.canonical_module_uri(canonical_builtin_path(path)))
    }
}

/// Return the package export key for one builtin specifier path.
fn builtin_export_key(path: &str) -> String {
    let path = canonical_builtin_path(path);

    // package root
    if path.is_empty() {
        ".".to_string()
    }
    // package subpath
    else {
        format!("./{path}")
    }
}

/// Return the canonical path part for one builtin module path.
fn canonical_builtin_path(path: &str) -> &str {
    let path = path.strip_suffix(".tspp").unwrap_or(path);

    path.strip_suffix("/index").unwrap_or(path)
}
