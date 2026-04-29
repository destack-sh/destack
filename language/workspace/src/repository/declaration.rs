use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{FileId, PackageId};
use im::OrdMap;

use crate::repository::{FileEntry, Repository, RepositoryError, Revision};
use crate::{
    DestackDeclaration, Package, PackageDeclaration, PackageOptions, WorkspaceError,
    WorkspaceOptions,
};

impl Repository {
    /// Return one parsed package declaration by file id.
    pub fn package_declaration_for_file(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<Arc<PackageDeclaration>>, RepositoryError> {
        let Some(content_id) = self.file_content_id(revision, file_id)? else {
            return Ok(None);
        };

        if let Some(declaration) = self.file_cache.package_declarations.get(&content_id) {
            return Ok(declaration.value().as_ref().ok().cloned());
        }

        // parse from source
        let Some(file) = self.file(revision, file_id)? else {
            return Ok(None);
        };
        let declaration = PackageDeclaration::parse(&file)
            .map(Arc::new)
            .map_err(|error| error.to_string());
        let package_declaration = declaration.as_ref().ok().cloned();

        self.file_cache
            .package_declarations
            .insert(content_id, declaration);

        Ok(package_declaration)
    }

    /// Return one parsed destack declaration by file id.
    pub fn destack_declaration_for_file(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<Arc<DestackDeclaration>>, RepositoryError> {
        let Some(content_id) = self.file_content_id(revision, file_id)? else {
            return Ok(None);
        };

        if let Some(declaration) = self.file_cache.destack_declarations.get(&content_id) {
            return Ok(declaration.value().as_ref().ok().cloned());
        }

        // parse from source
        let Some(file) = self.file(revision, file_id)? else {
            return Ok(None);
        };
        let declaration = DestackDeclaration::parse(&file)
            .map(Arc::new)
            .map_err(|error| error.to_string());
        let destack_declaration = declaration.as_ref().ok().cloned();

        self.file_cache
            .destack_declarations
            .insert(content_id, declaration);

        Ok(destack_declaration)
    }

    /// Return workspace construction errors from one file map.
    pub(crate) fn workspace_errors_for_files(
        &self,
        revision: Revision,
        files: &OrdMap<FileId, FileEntry>,
    ) -> Result<Vec<WorkspaceError>, RepositoryError> {
        let mut errors = Vec::new();

        // parse all known workspace declarations
        for (file_id, entry) in files {
            let Some(path) = self.path_for_source(&entry.source) else {
                continue;
            };
            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };

            let path = path.display().to_string();

            match file_name {
                "package.json" => {
                    let _ = self.package_declaration_for_file(revision, *file_id)?;
                    if let Some(parse) = self.file_cache.package_declarations.get(&entry.content_id)
                        && let Err(message) = parse.value().as_ref()
                    {
                        errors.push(WorkspaceError::new(*file_id, path, message.clone()));
                    }
                }
                "destack.json" => {
                    let _ = self.destack_declaration_for_file(revision, *file_id)?;
                    if let Some(parse) = self.file_cache.destack_declarations.get(&entry.content_id)
                        && let Err(message) = parse.value().as_ref()
                    {
                        errors.push(WorkspaceError::new(*file_id, path, message.clone()));
                    }
                }
                "tsconfig.json" => {
                    let _ = self.tsconfig_declaration_for_file(revision, *file_id)?;
                    if let Some(parse) =
                        self.file_cache.tsconfig_declarations.get(&entry.content_id)
                        && let Err(message) = parse.value().as_ref()
                    {
                        errors.push(WorkspaceError::new(*file_id, path, message.clone()));
                    }
                }
                _ => {}
            }
        }

        errors.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then_with(|| left.file_id.cmp(&right.file_id))
        });
        errors.dedup_by(|left, right| {
            left.file_id == right.file_id
                && left.path == right.path
                && left.message == right.message
        });

        Ok(errors)
    }

    /// Return the parsed root workspace declaration for one revision.
    pub(crate) fn destack_declaration_for_workspace(
        &self,
        revision: Revision,
    ) -> Result<Option<Arc<DestackDeclaration>>, RepositoryError> {
        let file_id = self.file_id_for_workspace_path(&self.root.join("destack.json"));
        self.destack_declaration_for_file(revision, file_id)
    }

    /// Return one parsed `destack.json` declaration by workspace path.
    pub fn destack_declaration_for_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<Arc<DestackDeclaration>>, RepositoryError> {
        let file_id = self.file_id_for_workspace_path(path);

        self.destack_declaration_for_file(revision, file_id)
    }

    /// Return the effective workspace options for one revision.
    pub fn workspace_options(
        &self,
        revision: Revision,
    ) -> Result<Option<WorkspaceOptions>, RepositoryError> {
        Ok(self
            .destack_declaration_for_workspace(revision)?
            .map(|declaration| declaration.workspace_options()))
    }

    /// Return the package root paths for one revision.
    pub fn package_roots(&self, revision: Revision) -> Result<Vec<PathBuf>, RepositoryError> {
        let workspace = self.workspace(revision)?;
        let mut package_roots = workspace
            .package_roots()
            .iter()
            .map(|(path, _)| path.clone())
            .collect::<Vec<_>>();

        package_roots.sort();
        package_roots.dedup();

        if package_roots.is_empty() {
            package_roots.push(self.root.clone());
        }

        Ok(package_roots)
    }

    /// Return the parsed package declaration for one package.
    pub fn package_declaration_for_package(
        &self,
        revision: Revision,
        package: &Package,
    ) -> Result<Option<Arc<PackageDeclaration>>, RepositoryError> {
        if let Some(package_file_id) = package.package_file_id {
            return self.package_declaration_for_file(revision, package_file_id);
        }

        let Some(package_path) = package.path.as_ref() else {
            return Ok(None);
        };

        let file_id = self.file_id_for_workspace_path(&package_path.join("package.json"));
        self.package_declaration_for_file(revision, file_id)
    }

    /// Return the parsed `destack.json` declaration for one package.
    pub fn destack_declaration_for_package(
        &self,
        revision: Revision,
        package: &Package,
    ) -> Result<Option<Arc<DestackDeclaration>>, RepositoryError> {
        if let Some(destack_file_id) = package.destack_file_id {
            return self.destack_declaration_for_file(revision, destack_file_id);
        }

        let Some(package_path) = package.path.as_ref() else {
            return Ok(None);
        };

        let file_id = self.file_id_for_workspace_path(&package_path.join("destack.json"));
        self.destack_declaration_for_file(revision, file_id)
    }

    /// Return the effective package options for one package when present.
    pub(crate) fn package_options(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<Option<PackageOptions>, RepositoryError> {
        let workspace = self.workspace(revision)?;
        let Some(package) = workspace.package(package_id) else {
            return Ok(None);
        };

        Ok(self
            .destack_declaration_for_package(revision, package.as_ref())?
            .map(|declaration| declaration.package_options()))
    }

    /// Return the package module type for one package.
    pub(crate) fn package_module_type(
        &self,
        revision: Revision,
        package: &Package,
    ) -> Result<Option<String>, RepositoryError> {
        if let Some(declaration) = self.destack_declaration_for_package(revision, package)?
            && let Some(module_type) = declaration.package_options().module_type
        {
            return Ok(Some(module_type));
        }

        if let Some(declaration) = self.package_declaration_for_package(revision, package)? {
            return Ok(declaration.module_type().map(str::to_string));
        }

        Ok(None)
    }
}
