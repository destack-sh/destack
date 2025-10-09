use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{Annotation, AnnotationPosition, NodeId, SelfParameter};
use dyst_source::SourceId;

impl<'a> Compiler<'a> {
    /// Lower the self parameter of a function.
    pub fn lower_self_parameter(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        self_parameter: &ast::SelfParameter,
    ) -> SelfParameter {
        todo!("Compiler::lower_self_parameter")
    }
}
