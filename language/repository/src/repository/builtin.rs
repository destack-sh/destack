use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use destack_source::{
    Content, ContentEntry, ContentId, File, FileId, FileMetadata, FileType, LanguageType, Loader,
    ModuleId, PackageId, Uri,
};
use indexmap::IndexMap;

use crate::{
    ExportKind, Module, Package, PackageExport, PackageKind, Repository, RepositoryError, Revision,
};

const BUILTIN_PACKAGE_URI: &str = "destack://";
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
    /// The canonical module URI.
    pub uri: &'static str,
    /// The path relative to `language/library/src`.
    pub path: &'static str,
    /// The source content.
    pub content: &'static str,
}

/// Embedded Builtin Package sources and canonical identity.
#[derive(Debug, Clone)]
pub struct EmbeddedBuiltinPackage {
    /// The package identity.
    package: Arc<Package>,
    /// The files shipped in this package.
    builtin_files: &'static [BuiltinFile],
    /// Builtin files keyed by file id.
    builtin_file_by_id: IndexMap<FileId, BuiltinFile>,
    /// Builtin files keyed by path or URI.
    builtin_file_by_path: IndexMap<&'static str, BuiltinFile>,
    /// Loaded source files keyed by file id, built once on first read.
    file_by_id: IndexMap<FileId, Arc<File>>,
    /// The modules shipped in this package.
    modules: Vec<(ModuleId, Arc<Module>)>,
}

impl EmbeddedBuiltinPackage {
    /// Create the embedded Builtin Package.
    pub fn new() -> Self {
        let exports = BUILTIN_EXPORTS
            .iter()
            .map(|export| (export.specifier.to_string(), export.package_export()))
            .collect();
        let package = Package {
            id: PackageId::from_uri(&Uri::from_string(BUILTIN_PACKAGE_NAME)),
            kind: PackageKind::Builtin,
            uri: Uri::from_string(BUILTIN_PACKAGE_URI),
            path: None,
            name: Some(BUILTIN_PACKAGE_NAME.to_string()),
            version: None,
            dependencies: IndexMap::new(),
            conditional_dependencies: Vec::new(),
            vendor: Default::default(),
            exports,
            topology: Default::default(),
            destack_file_id: None,
            targets: IndexMap::new(),
        };

        let package = Arc::new(package);
        let modules = BUILTINS
            .iter()
            .map(|builtin| builtin.module_entry(package.id))
            .collect();
        let builtin_file_by_id = BUILTINS
            .iter()
            .copied()
            .map(|builtin| (builtin.file_id(), builtin))
            .collect();
        // key by canonical uri only so workspace files never shadow builtins
        let builtin_file_by_path = BUILTINS
            .iter()
            .copied()
            .map(|builtin| (builtin.uri, builtin))
            .collect();

        let file_by_id = BUILTINS
            .iter()
            .copied()
            .map(|builtin| (builtin.file_id(), Arc::new(builtin.file())))
            .collect();

        Self {
            package,
            builtin_files: BUILTINS,
            builtin_file_by_id,
            builtin_file_by_path,
            file_by_id,
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

    /// Return the embedded fallback Package.
    pub fn package(&self) -> Arc<Package> {
        Arc::clone(&self.package)
    }

    /// Return all embedded Builtin files.
    pub fn files(&self) -> &'static [BuiltinFile] {
        self.builtin_files
    }

    /// Return one builtin module URI from an absolute builtin specifier.
    pub fn module_uri_for_specifier(&self, specifier: &str) -> Option<Uri> {
        let path = specifier
            .strip_prefix(BUILTIN_PACKAGE_URI)
            .or_else(|| specifier.strip_prefix("destack:"))?;

        self.module_uri_for_export_key(&builtin_export_key(path))
    }

    /// Return one internal builtin module URI from an absolute builtin specifier.
    pub fn module_uri_for_internal_specifier(&self, specifier: &str) -> Option<Uri> {
        let path = specifier
            .strip_prefix(BUILTIN_PACKAGE_URI)
            .or_else(|| specifier.strip_prefix("destack:"))?;

        Some(self.canonical_module_uri(canonical_builtin_path(path)))
    }

    /// Return the canonical module URI for one extensionless builtin path.
    fn canonical_module_uri(&self, path: &str) -> Uri {
        let key = format!("{BUILTIN_PACKAGE_URI}{path}");
        match self.builtin_file_by_path.get(key.as_str()) {
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
        let base = match self.builtin_file_by_path.get(base_uri) {
            Some(file) => file.path.strip_suffix(".ds").unwrap_or(file.path),
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
        self.builtin_file_by_id.get(&file_id).copied()
    }

    /// Return one loaded builtin source file by file id.
    pub fn file(&self, file_id: FileId) -> Option<&Arc<File>> {
        self.file_by_id.get(&file_id)
    }

    /// Return one builtin file by path or URI.
    pub fn builtin_file_for_path(&self, path: &Path) -> Option<BuiltinFile> {
        let path = path.to_str()?;

        self.builtin_file_by_path.get(path).copied()
    }

    /// Return one builtin file id by path or URI.
    pub fn file_id_for_path(&self, path: &Path) -> Option<FileId> {
        self.builtin_file_for_path(path).map(BuiltinFile::file_id)
    }

    /// Return builtin modules keyed by module id.
    pub(crate) fn modules(&self) -> impl Iterator<Item = (ModuleId, Arc<Module>)> + '_ {
        self.modules
            .iter()
            .map(|(module_id, module)| (*module_id, Arc::clone(module)))
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
        &self.embedded_builtin
    }

    /// Return the Builtin Package selected for one revision.
    pub fn builtin_package(&self, revision: Revision) -> Result<Arc<Package>, RepositoryError> {
        self.package(revision, self.embedded_builtin.package_id())?
            .ok_or(RepositoryError::MissingPackage {
                package: self.embedded_builtin.package_id(),
            })
    }

    /// Return whether one package id is the canonical builtin identity.
    pub fn is_builtin_package(&self, package: PackageId) -> bool {
        package == self.embedded_builtin.package_id()
    }

    /// Return module ids from the Builtin Package selected for one revision.
    pub fn builtin_module_ids(&self, revision: Revision) -> Result<Vec<ModuleId>, RepositoryError> {
        self.package_module_ids(revision, self.embedded_builtin.package_id())
    }

    /// Resolve one public specifier through the Builtin Package selected for one revision.
    pub fn builtin_module_uri_for_specifier(
        &self,
        revision: Revision,
        specifier: &str,
    ) -> Result<Option<Uri>, RepositoryError> {
        let Some(path) = specifier
            .strip_prefix(BUILTIN_PACKAGE_URI)
            .or_else(|| specifier.strip_prefix("destack:"))
        else {
            return Ok(None);
        };
        let export_key = builtin_export_key(path);
        let package = self.builtin_package(revision)?;
        let Some(export) = package.export(&export_key) else {
            return Ok(None);
        };
        if export.kind != ExportKind::Module {
            return Ok(None);
        }

        // embedded package exports already use canonical builtin URIs
        let Some(package_root) = package.path.as_deref() else {
            return Ok(self.embedded_builtin.module_uri_for_specifier(specifier));
        };

        // authored package exports resolve from their physical source paths
        let export_path = export.path.strip_prefix("./").unwrap_or(&export.path);
        let export_path = package_root.join(export_path);
        let uri = self
            .embedded_builtin
            .module_uri_for_path(&export_path, package_root)?;

        Ok(Some(uri))
    }

    /// Resolve one internal specifier through the Builtin Package selected for one revision.
    pub fn builtin_module_uri_for_internal_specifier(
        &self,
        revision: Revision,
        specifier: &str,
    ) -> Result<Option<Uri>, RepositoryError> {
        let Some(path) = specifier
            .strip_prefix(BUILTIN_PACKAGE_URI)
            .or_else(|| specifier.strip_prefix("destack:"))
        else {
            return Ok(None);
        };
        let package = self.builtin_package(revision)?;

        // embedded sources resolve through their generated file table
        if package.path.is_none() {
            return Ok(self
                .embedded_builtin
                .module_uri_for_internal_specifier(specifier));
        }

        self.authored_builtin_module_uri(revision, path)
    }

    /// Resolve one relative specifier through the Builtin Package selected for one revision.
    pub fn builtin_module_uri_for_relative_specifier(
        &self,
        revision: Revision,
        base_uri: &str,
        specifier: &str,
    ) -> Result<Option<Uri>, RepositoryError> {
        let package = self.builtin_package(revision)?;
        let Some(uri) = self
            .embedded_builtin
            .module_uri_for_relative_specifier(base_uri, specifier)
        else {
            return Ok(None);
        };

        // embedded sources resolve through their generated file table
        if package.path.is_none() {
            return Ok(Some(uri));
        }

        let Some(path) = uri.as_ref().strip_prefix(BUILTIN_PACKAGE_URI) else {
            return Ok(None);
        };

        self.authored_builtin_module_uri(revision, path)
    }

    /// Resolve one extensionless authored builtin path to its loaded module URI.
    fn authored_builtin_module_uri(
        &self,
        revision: Revision,
        path: &str,
    ) -> Result<Option<Uri>, RepositoryError> {
        let path = canonical_builtin_path(path);
        let candidates = [
            format!("{BUILTIN_PACKAGE_URI}{path}"),
            format!("{BUILTIN_PACKAGE_URI}{path}.ds"),
            format!("{BUILTIN_PACKAGE_URI}{path}/index.ds"),
        ];

        for candidate in candidates {
            let candidate = Uri::from_string(candidate);
            if self.module_id_for_uri(revision, &candidate)?.is_some() {
                return Ok(Some(candidate));
            }
        }

        Ok(None)
    }
}

impl Default for EmbeddedBuiltinPackage {
    /// Create the default embedded Builtin Package.
    fn default() -> Self {
        Self::new()
    }
}

impl BuiltinFile {
    /// Return the stable builtin source file id.
    pub fn file_id(self) -> FileId {
        FileId::from_logical_str(self.uri)
    }

    /// Return the stable builtin source content id.
    pub fn content_id(self) -> ContentId {
        ContentId::for_text(self.content)
    }

    /// Return the stable builtin module id.
    pub fn module_id(self, package: PackageId) -> ModuleId {
        ModuleId::from_path(package, Path::new(self.path), None)
    }

    /// Return this builtin as a module index entry.
    fn module_entry(self, package: PackageId) -> (ModuleId, Arc<Module>) {
        let module = Module::blank(
            self.module_id(package),
            self.file_id(),
            Uri::from_string(format!("{BUILTIN_PACKAGE_URI}{}", self.path)),
            Some(PathBuf::from(self.path)),
            package,
            Some(LanguageType::Destack),
            Loader::Destack,
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
    pub fn file(self) -> File {
        // build shared text content
        let content = Content::Text {
            content: self.content.to_string(),
        };
        let content = Arc::new(ContentEntry::new(content));

        // build virtual source file
        File::from_content(
            self.file_id(),
            self.name(),
            Uri::from_string(format!("{BUILTIN_PACKAGE_URI}{}", self.path)),
            None,
            FileType::Destack,
            content,
        )
    }

    /// Return metadata for this builtin file.
    pub fn metadata(self) -> FileMetadata {
        FileMetadata::new(true, false, false, self.content.len() as u64, None)
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
    let path = path.strip_suffix(".ds").unwrap_or(path);

    path.strip_suffix("/index").unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use destack_source::{Edit, FileId, ModuleId};

    use super::{EmbeddedBuiltinPackage, RepositoryError, Uri};
    use crate::{
        DestackLayoutOverride, Environment, PackageKind, Ref, Repository, Revision, Settings,
        open_repository_from_memory,
    };

    const BUILTIN_MANIFEST: &str =
        r#"{"name":"destack","exports":{"./error":{"path":"./src/error/index.ds"}}}"#;
    const DESTACK_MANIFEST: &str = r#"{"name":"destack"}"#;
    const WORKSPACE_MANIFEST: &str = r#"{"workspace":{"packages":["packages/*"]}}"#;

    /// One Repository revision built from authored test files.
    struct TestRepository {
        /// The Repository under test.
        repository: Repository,
        /// The authored source revision.
        revision: Revision,
    }

    impl TestRepository {
        /// Open one Repository from the provided source files.
        fn open(files: &[(&str, &str)]) -> Self {
            let edits = files
                .iter()
                .map(|(path, text)| Edit::SetText {
                    path: PathBuf::from(path),
                    text: text.to_string(),
                })
                .collect();
            let repository = open_repository_from_memory(
                PathBuf::from("/workspace"),
                edits,
                Environment::default(),
                Settings::default(),
                DestackLayoutOverride::default(),
            )
            .expect("test repository should open");
            let revision = repository
                .current(&Ref::for_root(repository.path()))
                .expect("test repository revision should exist");

            Self {
                repository,
                revision,
            }
        }
    }

    #[test]
    fn test_resolve_relative_from_builtin_index_uri() {
        let package = EmbeddedBuiltinPackage::new();
        let uri = package
            .module_uri_for_relative_specifier("destack://error", "./panic.ds")
            .expect("builtin relative import should resolve");

        assert_eq!(uri, Uri::from_string("destack://error/panic.ds"));
    }

    #[test]
    fn test_resolve_relative_from_builtin_file_uri() {
        let package = EmbeddedBuiltinPackage::new();
        let uri = package
            .module_uri_for_relative_specifier("destack://error/host", "./panic.ds")
            .expect("builtin relative import should resolve");

        assert_eq!(uri, Uri::from_string("destack://error/panic.ds"));
    }

    #[test]
    fn test_use_authored_builtin_package() {
        let test = TestRepository::open(&[
            ("destack.json", BUILTIN_MANIFEST),
            ("src/error/index.ds", "export const panic = true;\n"),
            ("src/debug/debug.ds", "export const debug = true;\n"),
        ]);
        let package = test
            .repository
            .builtin_package(test.revision)
            .expect("Builtin Package lookup should work");
        let error_module = ModuleId::from_path(package.id, Path::new("error/index.ds"), None);
        let debug_module = ModuleId::from_path(package.id, Path::new("debug/debug.ds"), None);
        let debug_uri = Uri::from_string("destack://debug/debug.ds");
        let error_uri = Uri::from_string("destack://error/index.ds");
        let mut expected_modules = vec![debug_module, error_module];
        expected_modules.sort_unstable();
        let mut expected_files = ["destack.json", "src/debug/debug.ds", "src/error/index.ds"]
            .map(FileId::from_logical_str)
            .to_vec();
        expected_files.sort_unstable();

        let public = test
            .repository
            .builtin_module_uri_for_specifier(test.revision, "destack:error");
        let internal = test
            .repository
            .builtin_module_uri_for_internal_specifier(test.revision, "destack:error/index");
        let relative = test.repository.builtin_module_uri_for_relative_specifier(
            test.revision,
            "destack://error/index.ds",
            "./index.ds",
        );

        assert_eq!(package.id, test.repository.embedded_builtin().package_id());
        assert_eq!(package.kind, PackageKind::Builtin);
        assert_eq!(package.uri, Uri::from_string("destack://"));
        assert_eq!(package.path, Some(PathBuf::new()));
        assert_eq!(
            test.repository.builtin_module_ids(test.revision),
            Ok(expected_modules)
        );
        assert_eq!(test.repository.file_ids(test.revision), Ok(expected_files));
        assert_eq!(
            [
                test.repository.module_id_for_uri(test.revision, &debug_uri),
                test.repository.module_id_for_uri(test.revision, &error_uri),
            ],
            [Ok(Some(debug_module)), Ok(Some(error_module))]
        );

        assert_eq!(
            [public, internal, relative],
            [
                Ok(Some(error_uri.clone())),
                Ok(Some(error_uri.clone())),
                Ok(Some(error_uri)),
            ]
        );
    }

    #[test]
    fn test_reject_multiple_authored_builtin_packages() {
        let test = TestRepository::open(&[
            ("destack.json", WORKSPACE_MANIFEST),
            ("packages/first/destack.json", DESTACK_MANIFEST),
            ("packages/second/destack.json", DESTACK_MANIFEST),
        ]);
        let error = test
            .repository
            .package_ids(test.revision)
            .expect_err("duplicate builtin sources should fail");

        assert_eq!(
            error,
            RepositoryError::DuplicatePackageName {
                name: "destack".to_string(),
            }
        );
    }

    #[test]
    fn test_reject_authored_builtin_export_outside_source_directory() {
        let test = TestRepository::open(&[
            (
                "destack.json",
                r#"{"name":"destack","exports":{"./outside":{"path":"./outside.ds"}}}"#,
            ),
            ("outside.ds", "export const outside = true;\n"),
        ]);
        let error = test
            .repository
            .builtin_module_uri_for_specifier(test.revision, "destack:outside")
            .expect_err("builtin export outside source directory should fail");

        assert_eq!(
            error,
            RepositoryError::BuiltinModuleOutsideSourceDirectory {
                path: PathBuf::from("outside.ds"),
                source_directory: PathBuf::from("src"),
            }
        );
    }
}
