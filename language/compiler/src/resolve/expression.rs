use dyst_dir::{Expression, LocalNodeId, Module, NodeTree, SymbolTable};

use crate::{Compiler, ResolveError, ResolveResult};

#[allow(clippy::too_many_arguments)]
impl<'a> Compiler<'a> {
    /// Resolve a string to a builtin expression.
    pub(super) fn resolve_string_to_builtin_expression(&self, string: &str) -> Option<Expression> {
        // type expression
        let ty = self.resolve_string_to_type(string.as_str())?;
        Some(Expression::TypeLiteral { value: ty })
    }

    /// Resolve an Expression.
    pub(super) fn resolve_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        tree: &mut NodeTree,
        symbols: &SymbolTable,
    ) -> ResolveResult<()> {
        let scope = symbols.get_scope(expression_id, tree);
        let expression = tree.get(expression_id);

        let expression = match expression {
            Expression::UnresolvedAbsolutePath {
                path,
                static_arguments,
            } => {
                match self.resolve_absolute_path(
                    module,
                    expression_id.into_any(),
                    scope,
                    path,
                    static_arguments.clone(),
                    tree,
                    symbols,
                ) {
                    Ok(expression) => expression,
                    Err(error)
                        if matches!(error, ResolveError::MissingSymbol { .. })
                            && path.segments.len() == 1 =>
                    {
                        // try resolving simple terms as builtin expression
                        let string = self.session.strings.get(path.segments[0]);
                        self.resolve_string_to_builtin_expression(string.as_str())
                            .ok_or(error)?
                    }
                    Err(error) => return Err(error),
                }
            }
            Expression::UnresolvedRelativePath {
                path,
                local_symbol: remote_symbol,
                remaining_path,
                static_arguments,
            } => {
                let remote_symbol = symbols.get_symbol(*remote_symbol);
                self.resolve_relative_path(
                    module,
                    expression_id.into_any(),
                    remote_symbol,
                    path,
                    remaining_path,
                    static_arguments.clone(),
                    tree,
                    symbols,
                )?
            }
            _ => return Ok(()),
        };
        *tree.get_mut(expression_id) = expression;

        Ok(())
    }
}
