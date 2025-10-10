use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::SelfParameter;
use dyst_source::SourceId;

impl<'a> Compiler<'a> {
    /// Lower the self parameter of a function.
    pub fn lower_self_parameter(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        self_parameter: &ast::SelfParameter,
    ) -> SelfParameter {
        let mutability = self.lower_scoped_mutability(source_id, ast, &self_parameter.mutability);
        SelfParameter {
            mutability,
            is_reference: self_parameter.is_reference,
        }
    }
}
