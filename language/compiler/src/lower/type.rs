use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{Generics, Heritage, Module, Mutability, NodeId, Type, TypeKind, VarianceBound};

impl<'a> Compiler<'a> {
    /// Lower a an expression into a type (without evaluating it at all).
    pub(super) fn lower_expression_to_type(
        &mut self,
        module: &Module,
        expression_id: ast::NodeId<ast::Expression>,
    ) -> NodeId<Type> {
        let expression = self.lower_expression(module, expression_id);
        let type_id = self.session.tree.insert_from_ast(
            Type::UnresolvedExpression(expression),
            module.id,
            expression_id,
        );
        self.session
            .tree
            .alias_from_source(module.id, expression_id.id, type_id);
        type_id
    }

    /// Lower mutability into a DIR mutability.
    #[inline]
    pub(super) fn lower_mutability(&self, mutability: ast::Mutability) -> Mutability {
        match mutability {
            ast::Mutability::Immutable => Mutability::Immutable,
            ast::Mutability::Mutable => Mutability::Mutable,
        }
    }

    /// Lower a TypeKind to a DIR type kind.
    pub(super) fn lower_type_kind(&mut self, kind: ast::TypeKind) -> TypeKind {
        match kind {
            ast::TypeKind::Structural => TypeKind::Structural,
            ast::TypeKind::Nominal => TypeKind::Nominal,
        }
    }

    /// Lower a VarianceBound to a DIR variance bound.
    pub(super) fn lower_variance_bound(&mut self, bound: ast::VarianceBound) -> VarianceBound {
        match bound {
            ast::VarianceBound::Implements => VarianceBound::Implements,
            ast::VarianceBound::Extends => VarianceBound::Extends,
            ast::VarianceBound::Super => VarianceBound::Super,
        }
    }

    /// Lower AST definition generics into DIR definition generics.
    pub(super) fn lower_generics(&mut self, module: &Module, generics: &ast::Generics) -> Generics {
        let static_parameters = generics
            .static_parameters
            .as_ref()
            .map(|static_parameters| {
                static_parameters
                    .iter()
                    .map(|static_parameter| self.lower_parameter(module, *static_parameter))
                    .collect()
            });
        let with_clauses = generics.with_clauses.as_ref().map(|with_clauses| {
            with_clauses
                .iter()
                .map(|with_clause| self.lower_with_clause(module, *with_clause))
                .collect()
        });
        let where_clauses = generics.where_clauses.as_ref().map(|where_clauses| {
            where_clauses
                .iter()
                .map(|where_clause| self.lower_where_clause(module, *where_clause))
                .collect()
        });
        Generics {
            static_parameters,
            with_clauses,
            where_clauses,
        }
    }

    /// Lower AST heritage into DIR heritage.
    pub(super) fn lower_heritage(&mut self, module: &Module, heritage: &ast::Heritage) -> Heritage {
        let extends_types = heritage.extends_types.as_ref().map(|extends_types| {
            extends_types
                .iter()
                .map(|extends_type| self.lower_expression_to_type(module, *extends_type))
                .collect()
        });
        let implements_types = heritage.implements_types.as_ref().map(|implements_types| {
            implements_types
                .iter()
                .map(|implements_type| self.lower_expression_to_type(module, *implements_type))
                .collect()
        });
        Heritage {
            extends_types,
            implements_types,
            embedded_types: None,
        }
    }
}
