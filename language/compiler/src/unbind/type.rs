use destack_ast::{self as ast};
use destack_base::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use crate::Compiler;

impl Compiler {
    /// Unbind a DIR mutability to an AST mutability.
    #[inline]
    pub(super) fn unbind_mutability(&self, mutability: dir::Mutability) -> ast::Mutability {
        match mutability {
            dir::Mutability::Immutable => ast::Mutability::Immutable,
            dir::Mutability::Mutable => ast::Mutability::Mutable,
        }
    }

    /// Unbind a DIR variance bound to an AST variance bound.
    #[inline]
    pub(super) fn unbind_variance_bound(&self, variance: dir::VarianceBound) -> ast::VarianceBound {
        match variance {
            dir::VarianceBound::Implements => ast::VarianceBound::Implements,
            dir::VarianceBound::Extends => ast::VarianceBound::Extends,
            dir::VarianceBound::Super => ast::VarianceBound::Super,
        }
    }

    /// Unbind a DIR asynchrony to an AST asynchrony.
    #[inline]
    pub(super) fn unbind_asynchrony(&self, asynchrony: dir::Asynchrony) -> ast::Asynchrony {
        match asynchrony {
            dir::Asynchrony::Sync => ast::Asynchrony::Sync,
            dir::Asynchrony::Async => ast::Asynchrony::Async,
        }
    }

    /// Unbind a DIR type kind to an AST type kind.
    #[inline]
    pub(super) fn unbind_type_kind(&self, kind: dir::TypeKind) -> ast::TypeKind {
        match kind {
            dir::TypeKind::Structural => ast::TypeKind::Structural,
            dir::TypeKind::Nominal => ast::TypeKind::Nominal,
        }
    }

    /// Unbind DIR generics to AST generics.
    pub(super) fn unbind_generics(
        &self,
        module: &Module,
        generics: &dir::Generics,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
    ) -> ast::Generics {
        let static_parameters = generics.static_parameters.as_ref().map(|static_parameters| {
            static_parameters
                .iter()
                .map(|static_parameter| {
                    self.unbind_parameter(
                        module,
                        *static_parameter,
                        tree,
                        symbols,
                        ast_tree,
                        ast_strings,
                    )
                })
                .collect()
        });
        let where_clauses = generics.where_clauses.as_ref().map(|where_clauses| {
            where_clauses
                .iter()
                .map(|where_clause| {
                    self.unbind_where_clause(
                        module,
                        *where_clause,
                        tree,
                        symbols,
                        ast_tree,
                        ast_strings,
                    )
                })
                .collect()
        });
        ast::Generics {
            static_parameters,
            where_clauses,
        }
    }

    /// Unbind DIR heritage to AST heritage.
    pub(super) fn unbind_heritage(
        &self,
        module: &Module,
        heritage: &dir::Heritage,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
    ) -> ast::Heritage {
        let extends_types = heritage.extends_types.as_ref().map(|extends_types| {
            extends_types
                .iter()
                .map(|extends_type| {
                    self.unbind_expression(
                        module,
                        *extends_type,
                        tree,
                        symbols,
                        ast_tree,
                        ast_strings,
                    )
                })
                .collect()
        });
        let implements_types = heritage.implements_types.as_ref().map(|implements_types| {
            implements_types
                .iter()
                .map(|implements_type| {
                    self.unbind_expression(
                        module,
                        *implements_type,
                        tree,
                        symbols,
                        ast_tree,
                        ast_strings,
                    )
                })
                .collect()
        });
        ast::Heritage {
            extends_types,
            implements_types,
        }
    }
}
