use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::ScalarLiteral;
use dyst_source::SourceId;

impl<'a> Compiler<'a> {
    /// Lower a path to a DIR path.
    pub fn lower_scalar_literal(
        &mut self,
        source_id: SourceId,
        _ast: &ast::NodeTree,
        scalar_literal: &ast::ScalarLiteral,
    ) -> ScalarLiteral {
        todo!("Compiler::lower_scalar_literal")
    }

    /// Lower a type literal to a DIR type literal.
    pub fn lower_type_literal(
        &mut self,
        source_id: SourceId,
        _ast: &ast::NodeTree,
        type_literal: &ast::TypeLiteral,
    ) -> TypeLiteral {
        todo!("Compiler::lower_type_literal")
    }
}
