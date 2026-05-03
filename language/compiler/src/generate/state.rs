use crate::DiagnosticAnchor;
use destack_dir as dir;
#[cfg(feature = "native-codegen")]
use destack_mir as mir;
use destack_source::ModuleId;

/// State for mapping backend diagnostics into compiler diagnostics.
pub(in crate::generate) struct GenerateState<'a, Tree> {
    /// The module being generated.
    pub(in crate::generate) module_id: ModuleId,
    /// The source-bearing tree used by the backend.
    tree: &'a Tree,
}

impl<'a, Tree> GenerateState<'a, Tree> {
    /// Create generate diagnostic mapping state.
    pub(in crate::generate) fn new(module_id: ModuleId, tree: &'a Tree) -> Self {
        Self { module_id, tree }
    }
}

impl GenerateState<'_, dir::Tree> {
    /// Return the source anchor for one backend DIR node.
    pub(in crate::generate) fn anchor(&self, node: dir::GlobalNodeIdAny) -> DiagnosticAnchor {
        assert_eq!(
            self.module_id, node.module_id,
            "script diagnostic node belongs to a different module"
        );

        let span = self
            .tree
            .get_span_by_id(node.local_id.id)
            .expect("script diagnostic node is missing a source span");

        DiagnosticAnchor::Span(span)
    }
}

#[cfg(feature = "native-codegen")]
impl GenerateState<'_, mir::Tree> {
    /// Return the source anchor for one backend MIR node.
    pub(in crate::generate) fn anchor(&self, node: mir::LocalNodeIdAny) -> DiagnosticAnchor {
        let span = self
            .tree
            .get_span_by_id(node.id)
            .expect("binary diagnostic node is missing a source span");

        DiagnosticAnchor::Span(span)
    }
}
