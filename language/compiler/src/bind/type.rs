use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{
    Generics, Heritage, LocalScopeId, LocalTypeId, Module, Mutability, NodeTree, SymbolTable, Type,
    TypeKind, TypeTable, VarianceBound,
};

impl<'a> Compiler<'a> {
    /// Bind a an expression into a type (without evaluating it at all).
    pub(super) fn bind_expression_to_type(
        &self,
        module: &Module,
        scope_id: LocalScopeId,
        expression_id: ast::LocalNodeId<ast::Expression>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        let expression_id = self.bind_expression(module, scope_id, expression_id, tree, symbols);
        let type_id =
            types.insert_from(Type::UnresolvedExpression(expression_id), expression_id);
        type_id
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
        scope_id: LocalScopeId,
        generics: &ast::Generics,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
    ) -> Generics {
        let static_parameters = generics
            .static_parameters
            .as_ref()
            .map(|static_parameters| {
                static_parameters
                    .iter()
                    .map(|static_parameter| {
                        self.bind_parameter(module, scope_id, *static_parameter, tree, symbols)
                    })
                    .collect()
            });
        let with_clauses = generics.with_clauses.as_ref().map(|with_clauses| {
            with_clauses
                .iter()
                .map(|with_clause| {
                    self.bind_with_clause(module, scope_id, *with_clause, tree, symbols)
                })
                .collect()
        });
        let where_clauses = generics.where_clauses.as_ref().map(|where_clauses| {
            where_clauses
                .iter()
                .map(|where_clause| {
                    self.bind_where_clause(module, scope_id, *where_clause, tree, symbols)
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
        scope_id: LocalScopeId,
        heritage: &ast::Heritage,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
    ) -> Heritage {
        let extends_types = heritage.extends_types.as_ref().map(|extends_types| {
            extends_types
                .iter()
                .map(|extends_type| {
                    self.bind_expression(module, scope_id, *extends_type, tree, symbols)
                })
                .collect()
        });
        let implements_types = heritage.implements_types.as_ref().map(|implements_types| {
            implements_types
                .iter()
                .map(|implements_type| {
                    self.bind_expression(module, scope_id, *implements_type, tree, symbols)
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
