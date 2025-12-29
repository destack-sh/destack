use destack_ast::{self as ast};
use destack_base::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use super::UnbindContext;
use crate::Compiler;

impl Compiler {
    /// Unbind a DIR mutability to an AST mutability.
    #[inline]
    pub(super) fn unbind_mutability(
        &self,
        _context: &mut UnbindContext,
        mutability: dir::Mutability,
    ) -> ast::Mutability {
        match mutability {
            dir::Mutability::Immutable => ast::Mutability::Immutable,
            dir::Mutability::Mutable => ast::Mutability::Mutable,
        }
    }

    /// Unbind a DIR variance bound to an AST variance bound.
    #[inline]
    pub(super) fn unbind_variance_bound(
        &self,
        _context: &mut UnbindContext,
        variance: dir::VarianceBound,
    ) -> ast::VarianceBound {
        match variance {
            dir::VarianceBound::Implements => ast::VarianceBound::Implements,
            dir::VarianceBound::Extends => ast::VarianceBound::Extends,
            dir::VarianceBound::Super => ast::VarianceBound::Super,
        }
    }

    /// Unbind a DIR type modifier to an AST type modifier.
    #[inline]
    pub(super) fn unbind_type_modifier(
        &self,
        _context: &mut UnbindContext,
        modifier: dir::TypeModifier,
    ) -> ast::TypeModifier {
        match modifier {
            dir::TypeModifier::Add => ast::TypeModifier::Add,
            dir::TypeModifier::Remove => ast::TypeModifier::Remove,
            dir::TypeModifier::None => ast::TypeModifier::None,
        }
    }

    /// Unbind DIR mapped type modifiers to AST mapped type modifiers.
    #[inline]
    pub(super) fn unbind_type_mapped_modifiers(
        &self,
        _context: &mut UnbindContext,
        modifiers: dir::TypeMappedModifiers,
    ) -> ast::TypeMappedModifiers {
        ast::TypeMappedModifiers {
            readonly: self.unbind_type_modifier(_context, modifiers.readonly),
            optional: self.unbind_type_modifier(_context, modifiers.optional),
        }
    }

    /// Unbind a DIR asynchrony to an AST asynchrony.
    #[inline]
    pub(super) fn unbind_asynchrony(
        &self,
        _context: &mut UnbindContext,
        asynchrony: dir::Asynchrony,
    ) -> ast::Asynchrony {
        match asynchrony {
            dir::Asynchrony::Sync => ast::Asynchrony::Sync,
            dir::Asynchrony::Async => ast::Asynchrony::Async,
        }
    }

    /// Unbind a DIR type kind to an AST type kind.
    #[inline]
    pub(super) fn unbind_type_kind(
        &self,
        _context: &mut UnbindContext,
        kind: dir::TypeKind,
    ) -> ast::TypeKind {
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
        context: &mut UnbindContext,
    ) -> ast::Generics {
        let static_parameters = generics
            .static_parameters
            .as_ref()
            .map(|static_parameters| {
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
                            context,
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
                        context,
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
        context: &mut UnbindContext,
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
                        context,
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
                        context,
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
