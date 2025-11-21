use dyst_dir::{Expression, LocalNodeId, ModuleId, NodeTree};

use crate::{Compiler, ResolveError, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve an Expression.
    pub(super) fn resolve_expression(
        &self,
        module_id: ModuleId,
        expression_id: LocalNodeId<Expression>,
        tree: &mut NodeTree,
    ) -> ResolveResult<()> {
        let expression = tree.get(expression_id);
        let scope = tree.get_scope(expression_id);

        let expression = match expression {
            Expression::UnresolvedPath {
                path,
                static_arguments,
            } => match self.resolve_path(module_id, scope, path, tree) {
                Ok(symbol_id) => {
                    todo!("resolve_expression({expression_id:?})")
                }
                Err(error) => {
                    if path.segments.len() == 1 {
                        let string = self.session.strings.get(path.segments[0]);
                        let ty = self.resolve_string_to_type(string.as_str()).ok_or(error)?;
                        Expression::TypeLiteral { value: ty }
                    } else {
                        return Err(error);
                    }
                }
            },
            _ => {
                return Err(ResolveError::UnsupportedNode {
                    node: expression_id.into(),
                });
            }
        };
        *tree.get_mut(expression_id) = expression;

        Ok(())
    }
}
