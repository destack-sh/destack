use std::sync::Arc;

use tspp_artifact::PackageNode;
use tspp_source::PackageId;

use crate::import::ImportState;
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Return the import-resolution node of one package, observing its configs.
    pub(in crate::import) fn package_node(
        &self,
        state: &mut ImportState<'_>,
        package_id: PackageId,
    ) -> CompilerResult<Option<Arc<PackageNode>>> {
        // reuse each touched package node
        if let Some(node) = state.packages.get(&package_id) {
            return Ok(node.clone());
        }

        // build and observe one package node
        let node = self
            .repository
            .package_node(state.revision, package_id, state.conditions)
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to build package node {package_id:?}: {error}"),
            })?;
        let source = self
            .repository
            .package_dependency(state.revision, package_id)
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to observe package {package_id:?}: {error}"),
            })?;
        state.observe(source);
        let node = node.map(Arc::new);
        state.packages.insert(package_id, node.clone());

        Ok(node)
    }
}
