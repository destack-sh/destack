use crate::{DystFormatter, FormatNode, NodeId, Statement};
use dyst_language_fir::format::FormatResult;
use dyst_language_fir::prelude::*;

impl<'ast> FormatNode<'ast, Statement> for Statement {
    fn format_node(
        &self,
        _node_id: NodeId<Statement>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Statement::Expression(node) => node.format(f),

            Statement::Module(node) => node.format(f),
            Statement::Struct(node) => node.format(f),
            Statement::Enum(node) => node.format(f),
            Statement::Union(node) => node.format(f),
            Statement::Trait(node) => node.format(f),
            Statement::Implement(node) => node.format(f),
            Statement::Function(node) => node.format(f),

            Statement::With(node) => node.format(f),
            Statement::Use(node) => node.format(f),
        }
    }
}
