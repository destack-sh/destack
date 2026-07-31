use std::sync::Arc;

use destack_artifact::PackageNode;
use destack_source::PackageId;

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
        for source in self.package_config_sources(state.revision, package_id)? {
            state.observe(source);
        }
        let node = node.map(Arc::new);
        state.packages.insert(package_id, node.clone());

        Ok(node)
    }
}
