use std::collections::HashSet;

use destack_dir::{
    Expression, GlobalSymbolId, LocalNodeId, LocalTypeId, NodeTree, PrimitiveType, ScalarLiteral,
    SymbolType, Type, TypeExpression, TypeLiteral,
};
use destack_workspace::ModuleSource;

use super::ModuleTypeView;
use super::r#type::TypeContainmentVisitor;
use crate::{AnalyzeError, AnalyzeOptions, Compiler};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Check whether a type expression implicitly relies on managed defaults.
    pub(crate) fn type_is_implicit_managed(&self, ctx: ModuleTypeView<'_>, ty: &Type) -> bool {
        // track visited symbols to break alias cycles
        let mut visited = HashSet::new();
        self.type_is_implicit_managed_inner(ctx, ty, &mut visited)
    }

    /// Check whether a type expression relies on managed defaults.
    ///
    /// Explicit ownership wrappers (`^T`, `&T`, `*T`) are treated as non managed.
    pub(crate) fn type_contains_managed(
        &self,
        ctx: ModuleTypeView<'_>,
        type_id: LocalTypeId,
    ) -> bool {
        let mut visited_symbols = HashSet::new();
        self.type_contains_managed_with_visited_symbols(ctx, type_id, &mut visited_symbols)
    }

    /// Check whether a type expression relies on managed defaults with visited tracking.
    fn type_contains_managed_with_visited_symbols(
        &self,
        ctx: ModuleTypeView<'_>,
        type_id: LocalTypeId,
        visited_symbols: &mut HashSet<GlobalSymbolId>,
    ) -> bool {
        let mut visited_types = HashSet::new();
        let visitor = TypeContainmentVisitor::new_managed_type(
            self,
            ctx,
            &mut visited_types,
            visited_symbols,
        );
        visitor.contains(ctx.types, type_id)
    }

    /// Check whether an expression makes ownership explicit.
    pub(crate) fn expression_has_explicit_ownership(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        // walk nested expressions to find explicit ownership operators
        match tree.get(expression_id) {
            Expression::ValueOf { .. }
            | Expression::ReferenceOf { .. }
            | Expression::PointerOf { .. }
            | Expression::OwnershipCast { .. } => true,
            Expression::UnresolvedPath { path, .. }
            | Expression::LocalReference { path, .. }
            | Expression::ModuleReference { path, .. }
            | Expression::GlobalReference { path, .. } => path
                .first_segment()
                .map(|name| {
                    matches!(
                        self.repository.strings.get(name).as_ref(),
                        "Managed"
                            | "Owned"
                            | "Borrowed"
                            | "Raw"
                            | "Shared"
                            | "AsManaged"
                            | "AsOwned"
                            | "AsBorrowed"
                            | "AsRaw"
                    )
                })
                .unwrap_or(false),
            Expression::Parenthesized { expression } => {
                self.expression_has_explicit_ownership(tree, *expression)
            }
            _ => false,
        }
    }

    /// Check whether a type expression makes ownership explicit.
    pub(crate) fn type_expression_has_explicit_ownership(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<TypeExpression>,
    ) -> bool {
        // walk nested type expressions to find explicit ownership operators
        match tree.get(expression_id) {
            TypeExpression::ValueOf { .. }
            | TypeExpression::ReferenceOf { .. }
            | TypeExpression::PointerOf { .. } => true,
            TypeExpression::Parenthesized { expression } => {
                self.type_expression_has_explicit_ownership(tree, *expression)
            }
            _ => false,
        }
    }

    /// Emit a diagnostic when an implicit managed value is used without ownership control.
    pub(crate) fn check_no_implicit_managed_value(
        &self,
        ctx: ModuleTypeView<'_>,
        expression_id: LocalNodeId<Expression>,
        expected_ty_id: LocalTypeId,
        actual_ty_id: LocalTypeId,
        tree: &NodeTree,
        options: &AnalyzeOptions,
    ) {
        // only enforce for user modules with the option enabled
        if !options.no_implicit_managed || !matches!(ctx.module.source, ModuleSource::User) {
            return;
        }

        // skip when managed memory is fully disabled
        if options.no_managed {
            return;
        }

        // skip explicit ownership operators
        if self.expression_has_explicit_ownership(tree, expression_id) {
            return;
        }

        // skip error types to avoid noisy diagnostics
        if self.type_blocks_cascading_diagnostic(expected_ty_id, ctx.types)
            || self.type_blocks_cascading_diagnostic(actual_ty_id, ctx.types)
        {
            return;
        }

        // only require explicit ownership when a managed default is expected
        let expected_ty = ctx.types.get_type(expected_ty_id);
        if !self.type_is_implicit_managed(ctx, expected_ty) {
            return;
        }

        self.error(AnalyzeError::ImplicitManagedValueDisabled {
            node: expression_id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile)),
        });
    }

    /// Emit a diagnostic when an inferred managed value lacks ownership control.
    pub(crate) fn check_no_implicit_managed_inferred(
        &self,
        ctx: ModuleTypeView<'_>,
        expression_id: LocalNodeId<Expression>,
        inferred_ty_id: LocalTypeId,
        tree: &NodeTree,
        options: &AnalyzeOptions,
    ) {
        // only enforce for user modules with the option enabled
        if !options.no_implicit_managed || !matches!(ctx.module.source, ModuleSource::User) {
            return;
        }

        // skip when managed memory is fully disabled
        if options.no_managed {
            return;
        }

        // skip explicit ownership operators
        if self.expression_has_explicit_ownership(tree, expression_id) {
            return;
        }

        // skip error types to avoid noisy diagnostics
        if self.type_blocks_cascading_diagnostic(inferred_ty_id, ctx.types) {
            return;
        }

        // report inferred implicit managed values
        let inferred_ty = ctx.types.get_type(inferred_ty_id);
        if self.type_is_implicit_managed(ctx, inferred_ty) {
            self.error(AnalyzeError::ImplicitManagedValueDisabled {
                node: expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }
    }

    /// Walk a type for implicit managed defaults.
    fn type_is_implicit_managed_inner(
        &self,
        ctx: ModuleTypeView<'_>,
        ty: &Type,
        visited: &mut HashSet<GlobalSymbolId>,
    ) -> bool {
        // treat explicit ownership wrappers as non managed
        match ty {
            Type::ValueOf { .. } | Type::ReferenceOf { .. } | Type::PointerOf { .. } => false,
            Type::Readonly { target_type: right }
            | Type::KeyOf { target_type: right }
            | Type::Must { target_type: right }
            | Type::AsComptime { target_type: right }
            | Type::Not { target_type: right }
            | Type::Value { value: right } => {
                // follow the wrapped type
                let inner = ctx.types.get_type(*right);
                self.type_is_implicit_managed_inner(ctx, inner, visited)
            }
            Type::Reference { symbol, .. } => {
                // unwrap aliases before checking references
                self.symbol_is_managed_inner(ctx, *symbol, visited, true)
            }
            Type::Object { .. } | Type::Array { .. } | Type::Function { .. } => true,
            Type::TypeLiteral { value } => self.type_literal_is_managed(value),
            Type::Union { elements } | Type::Intersection { elements } => {
                elements.iter().any(|element| {
                    // report managed defaults in union or intersection members
                    let inner = ctx.types.get_type(*element);
                    self.type_is_implicit_managed_inner(ctx, inner, visited)
                })
            }
            Type::This => true,
            _ => false,
        }
    }

    /// Walk a type for any managed usage.
    pub(super) fn symbol_is_managed_inner(
        &self,
        ctx: ModuleTypeView<'_>,
        symbol: GlobalSymbolId,
        visited: &mut HashSet<GlobalSymbolId>,
        treat_explicit_wrappers_as_managed: bool,
    ) -> bool {
        // match on symbol kind to decide managed status
        match symbol.ty() {
            SymbolType::Class | SymbolType::Interface => true,
            SymbolType::Struct
            | SymbolType::Enum
            | SymbolType::Function
            | SymbolType::Extension
            | SymbolType::Void => false,
            SymbolType::TypeAlias | SymbolType::Newtype => {
                // guard against alias cycles
                if !visited.insert(symbol) {
                    return false;
                }

                // resolve the alias target type and recurse
                let resolved = self.with_alias_target_type(
                    ctx,
                    symbol,
                    |alias_view, alias_type_id, alias_ty| {
                        if treat_explicit_wrappers_as_managed {
                            self.type_is_implicit_managed_inner(alias_view, alias_ty, visited)
                        } else {
                            self.type_contains_managed_with_visited_symbols(
                                alias_view,
                                alias_type_id,
                                visited,
                            )
                        }
                    },
                );
                resolved.unwrap_or(false)
            }
        }
    }

    /// Provide the alias target type to a handler when available.
    fn with_alias_target_type<R>(
        &self,
        ctx: ModuleTypeView<'_>,
        symbol: GlobalSymbolId,
        handle: impl FnOnce(ModuleTypeView<'_>, LocalTypeId, &Type) -> R,
    ) -> Option<R> {
        self.with_module_types_or_local_for_artifact(
            ctx.compiler_context,
            ctx.module,
            ctx.profile,
            symbol.module_id,
            ctx.types,
            destack_artifact::ArtifactKey::dir_declared,
            |owner_module, owner_types| {
                let target_id = owner_types.get_alias_target_type_id(symbol)?;
                let target_ty = owner_types.get_type(target_id);
                let view = ModuleTypeView::new(
                    ctx.compiler_context,
                    owner_module,
                    ctx.profile,
                    owner_types,
                );
                Some(handle(view, target_id, target_ty))
            },
        )
        .ok()
        .flatten()
    }

    /// Decide whether a type literal implies managed defaults.
    pub(super) fn type_literal_is_managed(&self, value: &TypeLiteral) -> bool {
        // object and string literals are managed by default
        match value {
            TypeLiteral::Object => true,
            TypeLiteral::Primitive(PrimitiveType::String) => true,
            TypeLiteral::ScalarLiteral(literal) => self.scalar_literal_is_managed(literal),
            _ => false,
        }
    }

    /// Decide whether a scalar literal implies managed defaults.
    pub(crate) fn scalar_literal_is_managed(&self, literal: &ScalarLiteral) -> bool {
        // string like literals imply managed defaults
        matches!(
            literal,
            ScalarLiteral::String(_)
                | ScalarLiteral::Character(_)
                | ScalarLiteral::RegexString { .. }
        )
    }
}
