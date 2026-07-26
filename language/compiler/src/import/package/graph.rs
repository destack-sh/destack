use destack_artifact::{
    ConditionSet, ExportPattern, ExportTarget, PackageDependency, PackageExports, PackageGraph,
    PackageNode,
};
use destack_repository::{ExportKind, Package, Revision};
use destack_source::ProfileId;

use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Build the active package graph for one profile.
    pub(in crate::import) fn build_package_graph(
        &self,
        revision: Revision,
        profile: ProfileId,
        conditions: &ConditionSet,
    ) -> CompilerResult<PackageGraph> {
        let package_ids =
            self.repository
                .package_ids(revision)
                .map_err(|error| CompilerError::Internal {
                    message: format!("failed to load package ids: {error}"),
                })?;
        let mut packages = indexmap::IndexMap::new();

        // build every active package node
        for package_id in package_ids {
            let package = self.package(revision, package_id)?;
            let node = self.build_package_node(revision, package.as_ref(), conditions)?;

            packages.insert(package_id, node);
        }

        // build exact import specifiers over the program module set
        let modules = self.repository.module_ids(revision)?;
        let module_paths = self.index_module_paths(revision, &modules)?;
        let package_specifiers = self.index_package_specifiers(revision, &packages, &modules)?;

        Ok(PackageGraph::new(
            profile,
            packages,
            module_paths,
            package_specifiers,
        ))
    }

    /// Build one active package graph node.
    fn build_package_node(
        &self,
        revision: Revision,
        package: &Package,
        conditions: &ConditionSet,
    ) -> CompilerResult<PackageNode> {
        let dependencies = self.index_dependencies(revision, package, conditions)?;
        let exports = self.index_package_exports(package, conditions);

        Ok(PackageNode {
            root: package.path.clone(),
            dependencies,
            exports,
        })
    }

    /// Build active direct dependencies for one package.
    fn index_dependencies(
        &self,
        revision: Revision,
        package: &Package,
        conditions: &ConditionSet,
    ) -> CompilerResult<indexmap::IndexMap<String, PackageDependency>> {
        let mut indexed = indexmap::IndexMap::new();
        let dependencies = package.dependencies_for_conditions(conditions);

        // resolve active package dependency declarations once
        for (name, dependency) in dependencies {
            let target = self
                .repository
                .dependency_package(revision, package, &name, &dependency)
                .map_err(|error| CompilerError::Internal {
                    message: format!("failed to resolve dependency package '{name}': {error}"),
                })?;

            let dependency = match target {
                Some(package) => PackageDependency::Resolved(package.id),
                None => PackageDependency::Unavailable,
            };
            indexed.insert(name, dependency);
        }

        Ok(indexed)
    }

    /// Build active package exports for one package.
    fn index_package_exports(
        &self,
        package: &Package,
        conditions: &ConditionSet,
    ) -> PackageExports {
        let mut exact = indexmap::IndexMap::new();
        let mut patterns = Vec::new();

        // index active exact and pattern exports separately
        for (key, export) in &package.exports {
            // skip inactive export branches
            if !export.matches(conditions) {
                continue;
            }

            let target = ExportTarget {
                path: export.path.clone(),
                is_module: export.kind == ExportKind::Module,
            };

            // index wildcard exports separately
            if let Some((prefix, suffix)) = key.split_once('*') {
                patterns.push(ExportPattern {
                    prefix: prefix.to_string(),
                    suffix: suffix.to_string(),
                    target,
                });
            }
            // index exact exports directly
            else {
                exact.insert(key.clone(), target);
            }
        }

        // prefer the most specific pattern before generic catchalls
        patterns.sort_by(|left, right| {
            right
                .prefix
                .len()
                .cmp(&left.prefix.len())
                .then_with(|| right.suffix.len().cmp(&left.suffix.len()))
        });

        PackageExports { exact, patterns }
    }
}
