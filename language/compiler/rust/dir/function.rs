use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::SelfParameter;
use dyst_source::FileId;

impl<'a> Compiler<'a> {
    /// Lower the self parameter of a function.
    pub fn lower_self_parameter(
        &mut self,
        source_id: FileId,
        ast: &ast::NodeTree,
        self_parameter: &ast::SelfParameter,
    ) -> SelfParameter {
        let mutability = self.lower_scoped_mutability(source_id, ast, &self_parameter.mutability);
        let ty = self_parameter
            .ty
            .as_ref()
            .map(|ty| self.lower_expression_to_type(source_id, ast, *ty));
        let reference_type = self_parameter
            .reference_type
            .map(|reference_type| self.lower_reference_type(reference_type));
        SelfParameter {
            mutability,
            reference_type,
            ty,
        }
    }
}
