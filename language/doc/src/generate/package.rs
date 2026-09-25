use std::collections::HashSet;
use std::path::Path;

use tspp_artifact::{IndexKind, ModuleIndex};
use tspp_dir as dir;
use tspp_repository::{ArtifactReader, Package};
use tspp_source::ModuleId;

use crate::{
    DocError, DocResult, ExportReference, ModuleReference, NamespaceReference,
    PACKAGE_REFERENCE_SCHEMA_VERSION, PackageIdentity, PackageReference,
};

use super::Generator;

impl Generator<'_> {
    /// Generate one package reference from checked public modules.
    pub fn generate(
        &self,
        package: &Package,
        modules: impl IntoIterator<Item = (String, ModuleId)>,
    ) -> DocResult<PackageReference> {
        let name = package
            .name
            .clone()
            .ok_or_else(|| DocError::invalid("missing documented package name"))?;
        let configuration = package.configuration.as_deref();
        let mut references = Vec::new();
        let mut pending = Vec::new();
        let mut visited = HashSet::new();

        // project public modules in manifest order
        for (specifier, module_id) in modules {
            visited.insert(module_id);
            references.push(self.module_reference(
                specifier,
                module_id,
                package.path.as_deref(),
                &mut pending,
            )?);
        }

        // visit each exported namespace once, including cyclic and shared exports
        let mut namespaces = Vec::new();
        let mut cursor = 0;
        while cursor < pending.len() {
            let module_id = pending[cursor];
            cursor += 1;
            if visited.insert(module_id) {
                namespaces.push(self.namespace_reference(
                    module_id,
                    package.path.as_deref(),
                    &mut pending,
                )?);
            }
        }

        Ok(PackageReference {
            schema_version: PACKAGE_REFERENCE_SCHEMA_VERSION,
            toolchain_version: env!("CARGO_PKG_VERSION").to_string(),
            package: PackageIdentity {
                name,
                version: package.version.clone(),
                description: configuration.and_then(|config| config.description.clone()),
                license: configuration.and_then(|config| config.license.clone()),
            },
            modules: references,
            namespaces,
        })
    }

    /// Generate one checked public module reference.
    fn module_reference(
        &self,
        specifier: String,
        module_id: ModuleId,
        package_path: Option<&Path>,
        pending: &mut Vec<ModuleId>,
    ) -> DocResult<ModuleReference> {
        let namespace = self.namespace_reference(module_id, package_path, pending)?;

        Ok(ModuleReference {
            module: namespace.module,
            specifier,
            path: namespace.path,
            exports: namespace.exports,
        })
    }

    /// Read a module's exports and enqueue the namespaces they expose.
    fn namespace_reference(
        &self,
        module_id: ModuleId,
        package_path: Option<&Path>,
        pending: &mut Vec<ModuleId>,
    ) -> DocResult<NamespaceReference> {
        let module = self
            .repository()
            .module(self.revision(), module_id)?
            .ok_or_else(|| {
                DocError::invalid(format!("missing documented module: {module_id:?}"))
            })?;
        let path =
            super::source::relative_path(module.path.as_deref(), package_path, self.repository())?;
        let artifacts = ArtifactReader::new(self.repository(), self.revision());
        let index =
            artifacts.read::<ModuleIndex>((module_id, self.profile(), IndexKind::Exports))?;
        let ModuleIndex::Exports(index) = index.as_ref() else {
            return Err(DocError::invalid(format!(
                "expected export index, found {:?}",
                index.kind()
            )));
        };
        pending.extend(
            index
                .entries()
                .iter()
                .filter_map(|entry| entry.target.namespace()),
        );
        let exports = index
            .entries()
            .iter()
            .map(|entry| self.export_reference(entry, package_path))
            .collect::<DocResult<Vec<_>>>()?;

        let identity = self
            .repository()
            .module_display(self.revision(), module_id)?
            .ok_or_else(|| DocError::invalid("missing namespace module identity"))?;

        Ok(NamespaceReference {
            module: identity,
            path,
            exports,
        })
    }

    /// Generate one checked export reference.
    fn export_reference(
        &self,
        entry: &dir::ExportEntry,
        package_path: Option<&Path>,
    ) -> DocResult<ExportReference> {
        let declarations = entry
            .target
            .symbol_ids()
            .into_iter()
            .flatten()
            .map(|symbol| self.declaration_reference(*symbol, package_path))
            .collect::<DocResult<Vec<_>>>()?;
        let namespace = entry
            .target
            .namespace()
            .map(|module_id| self.repository().module_display(self.revision(), module_id))
            .transpose()?
            .flatten();

        Ok(ExportReference {
            name: entry.name.clone(),
            declarations,
            namespace,
        })
    }
}
