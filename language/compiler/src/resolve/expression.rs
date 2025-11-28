use dyst_dir::{DependencySource, Expression, LocalNodeId, Module, NodeTree, SymbolTable};

use crate::{Compiler, ResolveError, ResolveResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve a string to a builtin expression.
    pub(super) fn resolve_string_to_builtin_expression_maybe(
        &self,
        string: &str,
    ) -> Option<Expression> {
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
        symbols: &mut SymbolTable,
    ) -> ResolveResult<()> {
        let scope = symbols.get_scope(expression_id, tree);
        let expression = tree.get(expression_id);

        let expression: Expression = match expression {
            Expression::UnresolvedImport {
                kind,
                target,
                items,
                arguments,
            } => {
                let remote_module_id = self.resolve_import(
                    module,
                    expression_id.into_global_any(module.id),
                    DependencySource::ImportStatement,
                    *target,
                    symbols,
                )?;
                Expression::Import {
                    kind: *kind,
                    target: *target,
                    target_module: remote_module_id,
                    items: items.clone(),
                    arguments: arguments.clone(),
                }
            }

            Expression::UnresolvedAbsolutePath {
                path,
                static_arguments,
            } => {
                match self.resolve_absolute_path(
                    module,
                    expression_id.into_global_any(module.id),
                    scope,
                    path,
                    static_arguments.clone(),
                    symbols,
                ) {
                    Ok(expression) => expression,
                    Err(error)
                        if matches!(error, ResolveError::MissingSymbol { .. })
                            && path.segments.len() == 1 =>
                    {
                        // try resolving simple terms as builtin expression
                        let string = self.program.strings.get(path.segments[0]);
                        self.resolve_string_to_builtin_expression_maybe(string.as_str())
                            .ok_or(error)?
                    }
                    Err(error) => return Err(error),
                }
            }
            Expression::UnresolvedRelativePath {
                path,
                target_symbol,
                remaining_path,
                static_arguments,
            } => {
                let target_symbol = symbols.get_symbol(*target_symbol);
                self.resolve_relative_path(
                    module,
                    expression_id.into_global_any(module.id),
                    target_symbol,
                    path,
                    remaining_path,
                    static_arguments.clone(),
                    symbols,
                )?
            }
            _ => return Ok(()),
        };
        *tree.get_mut(expression_id) = expression;

        Ok(())
    }
}
