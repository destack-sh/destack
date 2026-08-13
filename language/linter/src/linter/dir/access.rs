use destack_dir as dir;

use super::DirModule;

impl DirModule<'_> {
    /// Return the stable storage selected by one checked expression.
    pub fn access_resolution(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Option<&dir::AccessResolution> {
        let global = node.into_global_any(self.id);

        self.decisions.access_resolution(global)
    }
}
