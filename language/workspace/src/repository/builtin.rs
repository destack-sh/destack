use std::path::Path;
use std::sync::Arc;

use destack_builtin::{BuiltinLibraryKind, INTRINSIC_SOURCES, builtin_libraries};
use destack_source::{
    File, FileContent, FileContentId, FileId, FileType, LanguageType, Loader, ModuleId, PackageId,
    Uri,
};
use indexmap::IndexMap;

use crate::repository::{FileSource, Repository};
use crate::{Module, ModuleSource, Package, PackageKind};

/// Well-known package ID for builtins.
pub const BUILTIN_PACKAGE_ID: PackageId = PackageId(1);

/// Well-known package name for builtins.
pub const BUILTIN_PACKAGE_NAME: &str = "@destack/builtin";

/// Return the deterministic module id for one builtin path.
pub(crate) fn builtin_module_id_for_path(module_path: &str) -> ModuleId {
    ModuleId::from_relative_path(BUILTIN_PACKAGE_ID, Path::new(module_path))
}

/// Visit every builtin source file.
fn visit_builtin_sources(mut visit: impl FnMut(String, ModuleSource, &'static str)) {
    // intrinsic sources
    for source in INTRINSIC_SOURCES {
        visit(
            source.module_path(),
            ModuleSource::Builtin(BuiltinLibraryKind::Intrinsic),
            source.content,
        );
    }

    // library sources
    for library in builtin_libraries() {
        for source in library.sources {
            visit(
                source.module_path(),
                ModuleSource::Builtin(library.kind),
                source.content,
            );
        }
    }
}

impl Repository {
    /// Return the builtin package.
    pub(crate) fn builtin_package(&self) -> Package {
        Package {
            id: BUILTIN_PACKAGE_ID,
            kind: PackageKind::Builtin,
            uri: Uri::from_string("builtin://"),
            path: None,
            name: Some(BUILTIN_PACKAGE_NAME.to_string()),
            version: None,
            package_file_id: None,
            destack_file_id: None,
            tsconfig_file_id: None,
            targets: IndexMap::new(),
        }
    }

    /// Return all builtin modules.
    pub(crate) fn builtin_modules(&self) -> Vec<Module> {
        let mut modules = Vec::new();

        // builtin sources
        visit_builtin_sources(|module_path, source, _| {
            let file_id = self.builtin_file_id_for_path(&module_path);
            let file_type = FileType::from_path_or_unknown(Path::new(&module_path));
            let language_type = LanguageType::from(file_type);
            let loader = Loader::from_file_type(file_type);
            let module_id = builtin_module_id_for_path(&module_path);

            modules.push(Module::blank(
                module_id,
                file_id,
                Uri::from_string(format!("builtin://{module_path}")),
                None,
                BUILTIN_PACKAGE_ID,
                language_type,
                loader,
                source,
            ));
        });

        modules
    }

    /// Return one builtin file by file id.
    pub(crate) fn builtin_file(&self, file_id: FileId) -> Option<Arc<File>> {
        let mut file = None;

        // matching builtin source
        visit_builtin_sources(|module_path, _, content| {
            if self.builtin_file_id_for_path(&module_path) != file_id || file.is_some() {
                return;
            }
            let content = FileContent::Text {
                content: content.to_string(),
            };
            let content_id = self.files.intern(content);
            let Some(content) = self.files.get(content_id) else {
                return;
            };
            let file_type = FileType::from_path_or_unknown(Path::new(&module_path));

            file = Some(Arc::new(File::from_content(
                file_id,
                module_path.clone(),
                Uri::from_string(format!("builtin://{module_path}")),
                None,
                file_type,
                content,
            )));
        });

        file
    }

    /// Return one builtin file content id by file id.
    pub(crate) fn builtin_file_content_id(&self, file_id: FileId) -> Option<FileContentId> {
        let mut content_id = None;

        // matching builtin source
        visit_builtin_sources(|module_path, _, content| {
            if self.builtin_file_id_for_path(&module_path) != file_id || content_id.is_some() {
                return;
            }

            let content = FileContent::Text {
                content: content.to_string(),
            };

            content_id = Some(self.files.intern(content));
        });

        content_id
    }

    /// Return the deterministic file id for one builtin path.
    fn builtin_file_id_for_path(&self, module_path: &str) -> FileId {
        let source = FileSource::builtin(module_path);

        self.file_id_for_source(&source)
    }
}
