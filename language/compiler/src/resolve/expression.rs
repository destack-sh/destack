use destack_dir::{DependencySource, Expression, LocalNodeId, NodeTree, Path, SymbolTable};

use crate::{Compiler, ResolveError, ResolveResult};

use destack_workspace::Module;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve a string to a type literal expression (e.g., `int`, `string`).
    pub(super) fn resolve_string_to_type_literal_maybe(&self, string: &str) -> Option<Expression> {
        let ty = self.resolve_string_to_type(string)?;
        Some(Expression::TypeLiteral { value: ty })
    }

    /// Try to resolve `Self` type to an expression referencing the enclosing type.
    pub(super) fn resolve_self_expression(
        &self,
        module: &Module,
        scope: (
            destack_dir::LocalScopeId,
            &destack_dir::Scope,
            destack_dir::LocalScopeMark,
        ),
        path: &Path,
        static_arguments: Option<Vec<LocalNodeId<destack_dir::Argument>>>,
        symbols: &SymbolTable,
    ) -> Option<Expression> {
        let owner_symbol_id = self.resolve_self_type(scope, symbols)?;
        // Self always refers to a type in the same module
        Some(Expression::ModuleReference {
            path: path.clone(),
            static_arguments,
            target_symbol: owner_symbol_id.into_global(module.id),
        })
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
                )?;
                Expression::Import {
                    kind: *kind,
                    target: *target,
                    target_module: remote_module_id,
                    items: items.clone(),
                    arguments: arguments.clone(),
                }
            }

            Expression::UnresolvedReExport {
                target,
                kind,
                items,
            } => {
                let remote_module_id = self.resolve_import(
                    module,
                    expression_id.into_global_any(module.id),
                    DependencySource::ExportStatement,
                    *target,
                )?;
                Expression::ReExport {
                    target: *target,
                    target_module: remote_module_id,
                    kind: *kind,
                    items: items.clone(),
                }
            }

            Expression::UnresolvedPath {
                path,
                static_arguments,
            } => {
                let path = path.clone(); // (clone to release borrow on tree)
                let static_arguments = static_arguments.clone();
                match self.resolve_absolute_path(
                    module,
                    expression_id,
                    expression_id.into_global_any(module.id),
                    scope,
                    &path,
                    static_arguments.clone(),
                    symbols,
                    tree,
                ) {
                    Ok(expression) => expression,
                    Err(error)
                        if matches!(error, ResolveError::MissingSymbol { .. })
                            && path.segments.len() == 1 =>
                    {
                        // try resolving simple terms as builtin expression or Self type
                        let string = self.program.strings.get(path.segments[0]);
                        if string == "Self" {
                            // try to resolve Self type from enclosing type scope
                            self.resolve_self_expression(
                                module,
                                scope,
                                &path,
                                static_arguments.clone(),
                                symbols,
                            )
                            .ok_or(ResolveError::MissingSelf {
                                node: expression_id.into_global_any(module.id),
                            })?
                        } else {
                            // resolve to type literal if possible
                            self.resolve_string_to_type_literal_maybe(string.as_str())
                                .ok_or(error)?
                        }
                    }
                    Err(error) => return Err(error),
                }
            }

            Expression::UnresolvedBreak { target, value } => {
                let symbol_id = self.resolve_label_symbol(
                    module,
                    expression_id.into_global_any(module.id),
                    scope,
                    *target,
                    symbols,
                )?;
                Expression::Break {
                    target: Some(*target),
                    target_symbol: Some(symbol_id.into_global(module.id)),
                    value: *value,
                }
            }

            Expression::UnresolvedContinue { target } => {
                let symbol_id = self.resolve_label_symbol(
                    module,
                    expression_id.into_global_any(module.id),
                    scope,
                    *target,
                    symbols,
                )?;
                Expression::Continue {
                    target: Some(*target),
                    target_symbol: Some(symbol_id.into_global(module.id)),
                }
            }

            _ => return Ok(()),
        };
        *tree.get_mut(expression_id) = expression;

        Ok(())
    }
}
