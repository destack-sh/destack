use destack_artifact::{
    ConditionSet, DependencyIndex, ExportIndex, ExportPattern, ExportTarget, PackageImportIndex,
};
use destack_repository::{ExportKind, Package, Revision};
use destack_source::{PackageId, ProfileId};

use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Build the active dependency index for one profile.
    pub(in crate::import) fn build_dependency_index(
        &self,
        revision: Revision,
        profile: ProfileId,
        conditions: &ConditionSet,
    ) -> CompilerResult<DependencyIndex> {
        let package_ids =
            self.repository
                .package_ids(revision)
                .map_err(|error| CompilerError::Internal {
                    message: format!("failed to load package ids: {error}"),
                })?;
        let mut index = DependencyIndex {
            profile,
            packages: indexmap::IndexMap::new(),
        };

        // index every package visible in this revision
        for package_id in package_ids {
            let package = self.package(revision, package_id)?;
            let indexed = self.index_package_imports(revision, package.as_ref(), conditions)?;

            index.packages.insert(package_id, indexed);
        }

        Ok(index)
    }

    /// Build the active import index for one package.
    fn index_package_imports(
        &self,
        revision: Revision,
        package: &Package,
        conditions: &ConditionSet,
    ) -> CompilerResult<PackageImportIndex> {
        let dependencies = self.index_dependencies(revision, package, conditions)?;
        let exports = self.index_package_exports(package, conditions);

        Ok(PackageImportIndex {
            package: package.id,
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
    ) -> CompilerResult<indexmap::IndexMap<String, Option<PackageId>>> {
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

            indexed.insert(name, target.map(|package| package.id));
        }

        Ok(indexed)
    }

    /// Build active package exports for one package.
    fn index_package_exports(&self, package: &Package, conditions: &ConditionSet) -> ExportIndex {
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
                .then_with(|| {
                    let right_len = right.prefix.len() + right.suffix.len();
                    let left_len = left.prefix.len() + left.suffix.len();

                    right_len.cmp(&left_len)
                })
        });

        ExportIndex {
            package: package.id,
            exact,
            patterns,
        }
    }
}
