use destack_ast::{self as ast};
use destack_core::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use super::UnbindContext;
use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Unbind a DIR generic parameter to an AST generic parameter.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn unbind_generic_parameter(
        &self,
        module: &Module,
        parameter_id: dir::LocalNodeId<dir::GenericParameter>,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::GenericParameter> {
        let parameter = tree.get(parameter_id);
        let span = self.unbind_span(module, parameter_id.into());

        let ast_parameter = match parameter {
            dir::GenericParameter::Type {
                name,
                is_const,
                variance,
                constraint,
                default,
                ..
            } => {
                let name = ast_strings.intern_from(&self.repository.strings, *name);
                let variance = variance.map(|variance| match variance {
                    dir::VarianceModifier::In => ast::VarianceModifier::In,
                    dir::VarianceModifier::Out => ast::VarianceModifier::Out,
                    dir::VarianceModifier::InOut => ast::VarianceModifier::InOut,
                });
                let constraint = constraint.map(|constraint| {
                    self.unbind_type_expression(
                        module,
                        constraint,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });
                let default = default.map(|default| {
                    self.unbind_type_expression(
                        module,
                        default,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });

                ast::GenericParameter::Type {
                    name,
                    is_const: *is_const,
                    variance,
                    constraint,
                    default,
                }
            }
            dir::GenericParameter::Value {
                name,
                declared_type,
                default,
                is_comptime,
                ..
            } => {
                let name = ast_strings.intern_from(&self.repository.strings, *name);
                let declared_type = declared_type.map(|declared_type| {
                    self.unbind_type_expression(
                        module,
                        declared_type,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });
                let default = default.map(|default| {
                    self.unbind_expression(
                        module,
                        default,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });

                ast::GenericParameter::Value {
                    name,
                    declared_type,
                    default,
                    is_comptime: *is_comptime,
                }
            }
            dir::GenericParameter::Error { .. } => ast::GenericParameter::Error,
        };

        let ast_parameter_id = ast_tree.insert(ast_parameter, span);
        context.map(parameter_id.into_any(), ast_parameter_id.into_any());
        ast_parameter_id
    }

    /// Unbind a DIR function kind to an AST function kind.
    #[inline]
    pub(super) fn unbind_function_kind(
        &self,
        _context: &mut UnbindContext,
        kind: dir::FunctionKind,
    ) -> ast::FunctionKind {
        match kind {
            dir::FunctionKind::Function => ast::FunctionKind::Function,
            dir::FunctionKind::Lambda => ast::FunctionKind::Lambda,
        }
    }

    /// Unbind a DIR function cardinality to an AST function cardinality.
    #[inline]
    pub(super) fn unbind_function_cardinality(
        &self,
        _context: &mut UnbindContext,
        cardinality: dir::FunctionCardinality,
    ) -> ast::FunctionCardinality {
        match cardinality {
            dir::FunctionCardinality::Scalar => ast::FunctionCardinality::Scalar,
            dir::FunctionCardinality::Generator => ast::FunctionCardinality::Generator,
        }
    }

    /// Unbind a DIR function mode to an AST function mode.
    #[inline]
    pub(super) fn unbind_function_mode(
        &self,
        _context: &mut UnbindContext,
        mode: dir::FunctionMode,
    ) -> ast::FunctionMode {
        match mode {
            dir::FunctionMode::Getter => ast::FunctionMode::Getter,
            dir::FunctionMode::Setter => ast::FunctionMode::Setter,
            dir::FunctionMode::Constructor => ast::FunctionMode::Constructor,
            dir::FunctionMode::New => ast::FunctionMode::New,
            dir::FunctionMode::Call => ast::FunctionMode::Call,
        }
    }

    /// Unbind a DIR function signature to an AST function signature.
    pub(super) fn unbind_function_signature(
        &self,
        module: &Module,
        signature: &dir::FunctionSignature,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::FunctionSignature {
        let is_abstract = signature.is_abstract;
        let is_override = signature.is_override;
        let asynchrony = self.unbind_asynchrony(context, signature.asynchrony);
        let cardinality = self.unbind_function_cardinality(context, signature.cardinality);
        let mode = signature
            .mode
            .map(|mode| self.unbind_function_mode(context, mode));
        let kind = self.unbind_function_kind(context, signature.kind);
        let generic_parameters = signature
            .generic_parameters
            .iter()
            .map(|parameter| {
                self.unbind_generic_parameter(
                    module,
                    *parameter,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                )
            })
            .collect();
        let where_clauses = signature
            .where_clauses
            .iter()
            .map(|where_clause| {
                self.unbind_where_clause(
                    module,
                    *where_clause,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                )
            })
            .collect();
        let this_parameter = signature.this_parameter.map(|parameter| {
            self.unbind_parameter(
                module,
                parameter,
                tree,
                symbols,
                types,
                ast_tree,
                ast_strings,
                context,
            )
        });
        let parameters = signature
            .parameters
            .iter()
            .map(|parameter| {
                self.unbind_parameter(
                    module,
                    *parameter,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                )
            })
            .collect();
        let return_type = signature.return_type.map(|return_type| {
            self.unbind_type_expression(
                module,
                return_type,
                tree,
                symbols,
                types,
                ast_tree,
                ast_strings,
                context,
            )
        });
        ast::FunctionSignature {
            is_abstract,
            is_override,
            asynchrony,
            cardinality,
            mode,
            kind,
            generic_parameters,
            where_clauses,
            this_parameter,
            parameters,
            return_type,
        }
    }
}
