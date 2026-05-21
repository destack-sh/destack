use std::path::Path;
use std::sync::Arc;

use destack_source::{
    File, FileContent, FileContentEntry, FileContentId, FileId, FileMetadata, FileType,
    LanguageType, Loader, ModuleId, PackageId, Uri,
};
use indexmap::IndexMap;

use crate::{Module, Package, PackageKind, Repository};

const BUILTIN_PACKAGE_NAME: &str = "destack";
const BUILTIN_PACKAGE_URI: &str = "destack://";

include!(concat!(env!("OUT_DIR"), "/builtin.rs"));

/// One builtin source file shipped with the toolchain.
#[derive(Debug, Clone, Copy)]
pub struct BuiltinFile {
    /// The canonical module URI.
    pub uri: &'static str,
    /// The path relative to `language/library`.
    pub path: &'static str,
    /// The source content.
    pub content: &'static str,
}

/// Immutable builtin package shipped with the toolchain.
#[derive(Debug, Clone)]
pub struct BuiltinPackage {
    /// The package identity.
    package: Arc<Package>,
    /// The files shipped in this package.
    files: &'static [BuiltinFile],
    /// The modules shipped in this package.
    modules: Vec<(ModuleId, Arc<Module>)>,
}

impl BuiltinPackage {
    /// Create the builtin package.
    pub fn new() -> Self {
        let package = Package {
            id: PackageId::from_uri(&Uri::from_string(BUILTIN_PACKAGE_NAME)),
            kind: PackageKind::Builtin,
            uri: Uri::from_string(BUILTIN_PACKAGE_URI),
            path: None,
            name: Some(BUILTIN_PACKAGE_NAME.to_string()),
            version: None,
            dependencies: IndexMap::new(),
            mode_dependencies: IndexMap::new(),
            vendoring: Default::default(),
            destack_file_id: None,
            targets: IndexMap::new(),
        };

        let package = Arc::new(package);
        let modules = BUILTINS
            .iter()
            .map(|builtin| builtin.module_entry(package.id))
            .collect();

        Self {
            package,
            files: BUILTINS,
            modules,
        }
    }

    /// Return the builtin package id.
    pub fn package_id(&self) -> PackageId {
        self.package.id
    }

    /// Return the builtin package.
    pub fn package(&self) -> Arc<Package> {
        Arc::clone(&self.package)
    }

    /// Return whether one URI belongs to the builtin package.
    pub fn contains_uri(&self, uri: &str) -> bool {
        uri.starts_with(BUILTIN_PACKAGE_URI)
    }

    /// Return all builtin files.
    pub fn files(&self) -> &'static [BuiltinFile] {
        self.files
    }

    /// Return one builtin module URI from an absolute builtin specifier.
    pub fn module_uri_for_specifier(&self, specifier: &str) -> Option<Uri> {
        let path = specifier
            .strip_prefix(BUILTIN_PACKAGE_URI)
            .or_else(|| specifier.strip_prefix("destack:"))?;
        let path = canonical_builtin_path(path);
        let uri = format!("{BUILTIN_PACKAGE_URI}{path}");

        Some(Uri::from_string(uri))
    }

    /// Return one builtin module URI from a relative builtin specifier.
    pub fn module_uri_for_relative_specifier(
        &self,
        base_uri: &str,
        specifier: &str,
    ) -> Option<Uri> {
        let base = base_uri.strip_prefix(BUILTIN_PACKAGE_URI)?;
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
        let path = canonical_builtin_path(&path);
        let uri = format!("{BUILTIN_PACKAGE_URI}{path}");

        Some(Uri::from_string(uri))
    }

    /// Return one builtin file by file id.
    pub fn file(&self, file_id: FileId) -> Option<BuiltinFile> {
        self.files
            .iter()
            .copied()
            .find(|builtin| builtin.file_id() == file_id)
    }

    /// Return one builtin file by path or URI.
    pub fn file_for_path(&self, path: &Path) -> Option<BuiltinFile> {
        let path = path.to_str()?;

        self.files
            .iter()
            .copied()
            .find(|builtin| builtin.matches_path(path))
    }

    /// Return one builtin file id by path or URI.
    pub fn file_id_for_path(&self, path: &Path) -> Option<FileId> {
        self.file_for_path(path).map(BuiltinFile::file_id)
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
}

impl Repository {
    /// Return the builtin package shipped with the toolchain.
    pub fn builtin_package(&self) -> &BuiltinPackage {
        &self.builtin
    }
}

impl Default for BuiltinPackage {
    /// Create the default builtin package.
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
    pub fn content_id(self) -> FileContentId {
        FileContentId::for_text(self.content)
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
            Uri::from_string(self.uri),
            None,
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
        let content = FileContent::Text {
            content: self.content.to_string(),
        };
        let content = Arc::new(FileContentEntry::new(content));

        // build virtual source file
        File::from_content(
            self.file_id(),
            self.name(),
            Uri::from_string(self.uri),
            None,
            FileType::Destack,
            content,
        )
    }

    /// Return metadata for this builtin file.
    pub fn metadata(self) -> FileMetadata {
        FileMetadata::new(true, false, false, self.content.len() as u64, None)
    }

    /// Return whether this builtin matches one path or URI.
    fn matches_path(self, path: &str) -> bool {
        path == self.uri || path == self.path
    }
}

/// Return the canonical path part for one builtin module path.
fn canonical_builtin_path(path: &str) -> &str {
    let path = path.strip_suffix(".ds").unwrap_or(path);

    path.strip_suffix("/index").unwrap_or(path)
}
