use crate::Compiler;
use destack_ast as ast;
use destack_dir::{
    Generics, Heritage, LocalNodeIdAny, LocalScopeId, LocalScopeMark, LocalTypeId, Module,
    Mutability, NodeTree, SymbolTable, Type, TypeKind, TypeTable, VarianceBound,
};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a an expression into a type (without evaluating it at all).
    pub(super) fn bind_expression_to_type(
        &self,
        module: &Module,
        scope: (LocalScopeId, LocalScopeMark),
        ast_expression_id: ast::LocalNodeId<ast::Expression>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        let expression_id = self.bind_expression(
            module,
            scope,
            ast_expression_id,
            parent_id,
            tree,
            symbols,
            types,
        );
        types.insert_from(Type::UnresolvedExpression(expression_id), expression_id)
    }

    /// Bind mutability into a DIR mutability.
    #[inline]
    pub(super) fn bind_mutability(&self, mutability: ast::Mutability) -> Mutability {
        match mutability {
            ast::Mutability::Immutable => Mutability::Immutable,
            ast::Mutability::Mutable => Mutability::Mutable,
        }
    }

    /// Bind a TypeKind to a DIR type kind.
    pub(super) fn bind_type_kind(&self, kind: ast::TypeKind) -> TypeKind {
        match kind {
            ast::TypeKind::Structural => TypeKind::Structural,
            ast::TypeKind::Nominal => TypeKind::Nominal,
        }
    }

    /// Bind a VarianceBound to a DIR variance bound.
    pub(super) fn bind_variance_bound(&self, bound: ast::VarianceBound) -> VarianceBound {
        match bound {
            ast::VarianceBound::Implements => VarianceBound::Implements,
            ast::VarianceBound::Extends => VarianceBound::Extends,
            ast::VarianceBound::Super => VarianceBound::Super,
        }
    }

    /// Bind AST declaration generics into DIR declaration generics.
    pub(super) fn bind_generics(
        &self,
        module: &Module,
        scope: (LocalScopeId, LocalScopeMark),
        generics: &ast::Generics,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> Generics {
        let static_parameters = generics
            .static_parameters
            .as_ref()
            .map(|static_parameters| {
                static_parameters
                    .iter()
                    .map(|static_parameter| {
                        self.bind_parameter(
                            module,
                            scope,
                            *static_parameter,
                            parent_id,
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect()
            });
        let with_clauses = generics.with_clauses.as_ref().map(|with_clauses| {
            with_clauses
                .iter()
                .map(|with_clause| {
                    self.bind_with_clause(
                        module,
                        scope,
                        *with_clause,
                        parent_id,
                        tree,
                        symbols,
                        types,
                    )
                })
                .collect()
        });
        let where_clauses = generics.where_clauses.as_ref().map(|where_clauses| {
            where_clauses
                .iter()
                .map(|where_clause| {
                    self.bind_where_clause(
                        module,
                        scope,
                        *where_clause,
                        parent_id,
                        tree,
                        symbols,
                        types,
                    )
                })
                .collect()
        });
        Generics {
            static_parameters,
            with_clauses,
            where_clauses,
        }
    }

    /// Bind AST heritage into DIR heritage.
    pub(super) fn bind_heritage(
        &self,
        module: &Module,
        scope: (LocalScopeId, LocalScopeMark),
        heritage: &ast::Heritage,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> Heritage {
        let extends_types = heritage.extends_types.as_ref().map(|extends_types| {
            extends_types
                .iter()
                .map(|extends_type| {
                    self.bind_expression(
                        module,
                        scope,
                        *extends_type,
                        parent_id,
                        tree,
                        symbols,
                        types,
                    )
                })
                .collect()
        });
        let implements_types = heritage.implements_types.as_ref().map(|implements_types| {
            implements_types
                .iter()
                .map(|implements_type| {
                    self.bind_expression(
                        module,
                        scope,
                        *implements_type,
                        parent_id,
                        tree,
                        symbols,
                        types,
                    )
                })
                .collect()
        });
        Heritage {
            extends_types,
            implements_types,
            embedded_types: None,
        }
    }
}
