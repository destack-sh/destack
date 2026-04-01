use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::PackageId;

use crate::repository::{Repository, RepositoryError};
use crate::revision::Revision;
use crate::{DestackDeclaration, Package, PackageDeclaration, PackageOptions};

impl Repository {
    /// Return the parsed root workspace declaration for one revision.
    pub fn workspace_destack_declaration(
        &self,
        revision: Revision,
    ) -> Result<Option<DestackDeclaration>, RepositoryError> {
        let declaration_path = self.root.join("destack.json");
        let Some(declaration_file) = self.file_at_path(revision, &declaration_path)? else {
            return Ok(None);
        };
        let Ok(declaration) = DestackDeclaration::parse(&declaration_file) else {
            return Ok(None);
        };

        Ok(Some(declaration))
    }

    /// Return the effective workspace options for one revision.
    pub fn workspace_options(
        &self,
        revision: Revision,
    ) -> Result<Option<crate::WorkspaceOptions>, RepositoryError> {
        Ok(self
            .workspace_destack_declaration(revision)?
            .map(|declaration| declaration.workspace_options()))
    }

    /// Return the discovered package paths for one revision.
    pub fn workspace_package_paths(
        &self,
        revision: Revision,
    ) -> Result<Vec<PathBuf>, RepositoryError> {
        let mut package_paths = self
            .workspace_package_ids(revision)?
            .into_iter()
            .filter_map(|package_id| {
                self.package(revision, package_id)
                    .ok()
                    .flatten()
                    .and_then(|package| package.path.clone())
            })
            .collect::<Vec<_>>();

        package_paths.sort();
        package_paths.dedup();

        if package_paths.is_empty() {
            package_paths.push(self.root.clone());
        }

        Ok(package_paths)
    }

    /// Return one package snapshot for one revision and package id.
    pub fn package(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<Option<Arc<Package>>, RepositoryError> {
        let revision_data = self.revision(revision)?;
        let packages = self.derive_packages(revision_data.source.as_ref());
        let mut package = if let Some(package) = packages.get(&package_id) {
            package.clone()
        } else {
            return Ok(None);
        };

        let package_declaration = self.package_declaration(revision, &package)?;
        let destack_declaration = self.package_destack_declaration(revision, &package)?;
        let package_options = destack_declaration
            .as_ref()
            .map(DestackDeclaration::package_options);

        package.package_file_id = package_declaration
            .as_ref()
            .map(|declaration| declaration.file_id);
        package.destack_file_id = destack_declaration
            .as_ref()
            .map(|declaration| declaration.file_id);
        package.name = package_options
            .as_ref()
            .and_then(|options| options.name.clone())
            .or_else(|| {
                package_declaration
                    .as_ref()
                    .and_then(|declaration| declaration.name().map(ToOwned::to_owned))
            });
        package.version = package_options
            .as_ref()
            .and_then(|options| options.version.clone())
            .or_else(|| {
                package_declaration
                    .as_ref()
                    .and_then(|declaration| declaration.version().map(ToOwned::to_owned))
            });

        if let Some(package_options) = package_options.as_ref() {
            package.targets = package_options
                .targets
                .iter()
                .map(|(name, options)| {
                    let target_id = self.intern_target_id(package.id, name);
                    let target = options.to_target(name);

                    (target_id, target)
                })
                .collect();
        }

        Ok(Some(Arc::new(package)))
    }

    /// Return the package ids visible in one revision.
    pub fn workspace_package_ids(
        &self,
        revision: Revision,
    ) -> Result<Vec<PackageId>, RepositoryError> {
        let revision = self.revision(revision)?;
        let packages = self.derive_packages(revision.source.as_ref());
        let mut package_ids = packages.keys().copied().collect::<Vec<_>>();
        package_ids.sort_unstable();
        package_ids.dedup();

        Ok(package_ids)
    }

    /// Return the parsed package declaration for one package.
    pub fn package_declaration(
        &self,
        revision: Revision,
        package: &Package,
    ) -> Result<Option<PackageDeclaration>, RepositoryError> {
        let Some(package_path) = package.path.as_ref() else {
            return Ok(None);
        };

        let declaration_path = package_path.join("package.json");
        let Some(declaration_file) = self.file_at_path(revision, &declaration_path)? else {
            return Ok(None);
        };

        let realpath = package_path.clone();
        let Ok(declaration) = PackageDeclaration::parse(&declaration_file, realpath) else {
            return Ok(None);
        };

        Ok(Some(declaration))
    }

    /// Return the parsed `destack.json` declaration for one package.
    pub fn package_destack_declaration(
        &self,
        revision: Revision,
        package: &Package,
    ) -> Result<Option<DestackDeclaration>, RepositoryError> {
        let Some(package_path) = package.path.as_ref() else {
            return Ok(None);
        };

        let declaration_path = package_path.join("destack.json");
        let Some(declaration_file) = self.file_at_path(revision, &declaration_path)? else {
            return Ok(None);
        };
        let Ok(declaration) = DestackDeclaration::parse(&declaration_file) else {
            return Ok(None);
        };

        Ok(Some(declaration))
    }

    /// Return the effective package options for one package when present.
    pub fn package_options(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<Option<PackageOptions>, RepositoryError> {
        let Some(package) = self.package(revision, package_id)? else {
            return Ok(None);
        };
        let Some(package_path) = package.path.as_ref() else {
            return Ok(None);
        };

        let declaration_path = package_path.join("destack.json");
        let Some(declaration_file) = self.file_at_path(revision, &declaration_path)? else {
            return Ok(None);
        };
        let Ok(declaration) = DestackDeclaration::parse(&declaration_file) else {
            return Ok(None);
        };

        Ok(Some(declaration.package_options()))
    }

    /// Return the package module type for one package.
    pub(crate) fn package_module_type_at(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<Option<String>, RepositoryError> {
        let Some(package) = self.package(revision, package_id)? else {
            return Ok(None);
        };

        let package_options = self.package_options(revision, package_id)?;
        if let Some(module_type) = package_options.and_then(|options| options.module_type) {
            return Ok(Some(module_type));
        }

        let package_declaration = self.package_declaration(revision, &package)?;
        Ok(package_declaration
            .and_then(|declaration| declaration.module_type().map(str::to_string)))
    }

    /// Return one file by workspace path.
    fn file_at_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<Arc<destack_source::File>>, RepositoryError> {
        let file_id = self.file_id_for_workspace_path(path);
        self.file(revision, file_id)
    }
}
