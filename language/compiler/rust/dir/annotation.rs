use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{Annotation, NodeId};
use dyst_source::SourceId;

impl<'a> Compiler<'a> {
    /// Lower and attach all annotations for a source.
    pub fn attach_annotations(&mut self, source_id: SourceId, ast: &ast::NodeTree) {
        todo!("Compiler::attach_annotations")
    }

    /// Lower an annotation to a DIR annotation.
    pub fn lower_annotation(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        annotation_id: ast::NodeId<ast::Annotation>,
    ) -> NodeId<Annotation> {
        todo!("Compiler::lower_annotation")
    }
}
