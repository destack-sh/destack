use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{Argument, Expression, NodeId, Parameter, Path};
use dyst_source::SourceId;

impl<'a> Compiler<'a> {
    /// Lower an argument into a DIR argument.
    pub fn lower_argument(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        argument_id: ast::NodeId<ast::Argument>,
    ) -> NodeId<Argument> {
        let argument = ast.get(argument_id);
        match argument {
            ast::Argument::Named { name, value } => {
                let name = self.intern_string(source_id, *name);
                let value = self.lower_expression(source_id, ast, *value);
                self.tree
                    .allocate(Argument::Named { name, value }, source_id, argument_id)
            }
            ast::Argument::NamedShorthand { name } => {
                let name = self.intern_string(source_id, *name);
                let path = Path::String {
                    segments: vec![name],
                };
                let value = self
                    .tree
                    .allocate(Expression::Path { path }, source_id, argument_id);
                self.tree
                    .allocate(Argument::Named { name, value }, source_id, argument_id)
            }
            ast::Argument::Positional { value } => {
                let value = self.lower_expression(source_id, ast, *value);
                self.tree
                    .allocate(Argument::Positional { value }, source_id, argument_id)
            }
        }
    }

    /// Lower a parameter into a DIR parameter.
    pub fn lower_parameter(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        parameter_id: ast::NodeId<ast::Parameter>,
    ) -> NodeId<Parameter> {
        let parameter = ast.get(parameter_id);
        let name = self.intern_string(source_id, parameter.name);
        let ty = parameter
            .ty
            .map(|ty| self.lower_expression_to_type(source_id, ast, ty));
        let default = parameter
            .default
            .map(|default| self.lower_expression(source_id, ast, default));
        self.tree
            .allocate(Parameter { name, ty, default }, source_id, parameter_id)
    }
}
