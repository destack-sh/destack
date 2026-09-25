use tspp_dir as dir;
use tspp_source::Span;

use crate::{DocError, DocResult};

use super::Module;

impl Module<'_> {
    /// Return the required authored span of a node.
    pub(crate) fn node_span(
        &self,
        view: dir::View<'_>,
        node_id: dir::LocalNodeIdAny,
    ) -> DocResult<Span> {
        view.get_span_by_id(node_id.id).ok_or_else(|| {
            DocError::missing(format!(
                "authored node span: {:?}",
                node_id.into_global(self.module_id())
            ))
        })
    }
}
