use super::member::{MemberLookupMode, MemberReceiverContext};
use crate::Compiler;
use crate::analyze::common::{CanonicalSymbolMode, InferContext, ModuleSymbolView, TypeContext};
use destack_dir::{
    Expression, GlobalSymbolId, LocalNodeId, LocalTypeId, StaticKey, SymbolSpace, SymbolType, Type,
    TypeExpression, TypeTable,
};
use std::collections::HashSet;

impl Compiler {
    /// Return true when a member receiver should be evaluated as a type projection receiver.
    pub(crate) fn is_projection_receiver_expression(
        &self,
        ctx: &mut InferContext<'_>,
        receiver_id: LocalNodeId<Expression>,
    ) -> bool {
        let receiver_id = self.unwrap_parenthesized_expression(receiver_id, ctx.tree);
        let Some(arguments) = ctx.tree.get(receiver_id).generic_arguments() else {
            return false;
        };

        if arguments.is_empty() {
            return false;
        }

        let symbol = self.resolve_projection_receiver_symbol(&mut ctx.reborrow(), receiver_id);
        let Some(symbol) = symbol else {
            return false;
        };

        self.query_symbol_supports_projection_receiver(ctx.module_symbol_view(), symbol)
    }

    /// Resolve a symbol candidate for associated projection receiver inference.
    fn resolve_projection_receiver_symbol(
        &self,
        ctx: &mut InferContext<'_>,
        receiver_id: LocalNodeId<Expression>,
    ) -> Option<GlobalSymbolId> {
        self.resolve_direct_receiver_symbol_for_expression(
            &ctx.type_context_reborrow(),
            receiver_id,
        )
        .or_else(|| {
            let receiver_type_id = self.query_projection_receiver_type_id(ctx, receiver_id)?;
            self.query_type_like_receiver_symbol_for_type_id(receiver_type_id, ctx.types)
        })
    }

    /// Query one already-known receiver type id for projection receiver selection.
    fn query_projection_receiver_type_id(
        &self,
        ctx: &InferContext<'_>,
        receiver_id: LocalNodeId<Expression>,
    ) -> Option<LocalTypeId> {
        let receiver_node = receiver_id.into_global_any(ctx.module.id);
        ctx.infer
            .inferred_type_for_node(receiver_node)
            .or_else(|| ctx.types.get_declared_or_inferred_type_id(receiver_node))
    }

    /// Return true when a symbol can act as a projection receiver in infer.
    fn query_symbol_supports_projection_receiver(
        &self,
        ctx: ModuleSymbolView<'_>,
        symbol: GlobalSymbolId,
    ) -> bool {
        let symbol = self.canonical_symbol_id(ctx, symbol, CanonicalSymbolMode::FollowAliases);
        let symbol = self.declaration_symbol_id(ctx, symbol).unwrap_or(symbol);

        matches!(
            symbol.ty(),
            SymbolType::Class
                | SymbolType::Struct
                | SymbolType::Interface
                | SymbolType::Enum
                | SymbolType::TypeAlias
                | SymbolType::Newtype
        )
    }

    /// Classify member receiver behavior for symbol and type lookup paths.
    pub(crate) fn query_member_receiver_context_for_expression(
        &self,
        ctx: &TypeContext<'_>,
        receiver_id: LocalNodeId<Expression>,
        receiver_ty_id: Option<LocalTypeId>,
    ) -> MemberReceiverContext {
        let receiver_id = self.unwrap_parenthesized_expression(receiver_id, ctx.tree);
        // nominal receiver classification should follow the expression form,
        // not the eventual runtime instance type
        let nominal_symbol = self.query_nominal_value_symbol_for_expression(ctx, receiver_id);
        let has_static_arguments = ctx
            .tree
            .get(receiver_id)
            .generic_arguments()
            .is_some_and(|arguments| !arguments.is_empty());
        let has_this_receiver = matches!(
            ctx.tree.get(receiver_id),
            Expression::This | Expression::Super
        ) || receiver_ty_id.is_some_and(|receiver_ty_id| {
            self.type_contains_this(receiver_ty_id, ctx.types, &mut HashSet::new())
        });

        // instance-bound receivers like `this` and `super` should not be reclassified as static
        let lookup_mode = if has_this_receiver {
            MemberLookupMode::Instance
        } else if nominal_symbol.is_some() {
            // nominal values only expose static members
            MemberLookupMode::Value
        } else {
            // runtime receivers expose instance members only
            MemberLookupMode::Instance
        };

        MemberReceiverContext {
            nominal_symbol,
            has_static_arguments,
            has_this_receiver,
            lookup_mode,
        }
    }

    /// Return a nominal symbol when the expression refers to a type value.
    fn query_nominal_value_symbol_for_expression(
        &self,
        ctx: &TypeContext<'_>,
        receiver_id: LocalNodeId<Expression>,
    ) -> Option<GlobalSymbolId> {
        // resolve the direct reference symbol for the receiver
        let symbol = self.resolve_direct_receiver_symbol_for_expression(ctx, receiver_id)?;
        self.query_nominal_value_symbol_for_symbol(ctx, symbol)
    }

    /// Resolve one nominal value symbol from an explicit symbol candidate.
    fn query_nominal_value_symbol_for_symbol(
        &self,
        ctx: &TypeContext<'_>,
        symbol: GlobalSymbolId,
    ) -> Option<GlobalSymbolId> {
        let symbol = self
            .declaration_symbol_id(ctx.module_symbol_view(), symbol)
            .unwrap_or(symbol);

        // keep only nominal symbols in value space
        if !matches!(
            symbol.ty(),
            SymbolType::Class | SymbolType::Struct | SymbolType::Enum | SymbolType::Newtype
        ) {
            return None;
        }

        // enum members are always value-like, even when symbol-space tagging reports type
        if symbol.ty() == SymbolType::Enum {
            return Some(symbol);
        }

        let space = self
            .with_module_symbols_or_local_for_artifact(
                ctx.compiler_context,
                ctx.module,
                ctx.profile,
                symbol.module_id,
                ctx.symbols,
                destack_artifact::ArtifactKey::dir_declared,
                |_, owner_symbols| owner_symbols.get_symbol(symbol.local_id).space,
            )
            .ok();
        if space.is_some_and(|space| matches!(space, SymbolSpace::Value | SymbolSpace::TypeValue)) {
            Some(symbol)
        } else {
            None
        }
    }

    /// Resolve a canonical direct symbol for a receiver expression.
    pub(crate) fn resolve_direct_receiver_symbol_for_expression(
        &self,
        ctx: &TypeContext<'_>,
        receiver_id: LocalNodeId<Expression>,
    ) -> Option<GlobalSymbolId> {
        // peel parenthesized receivers to their core symbol
        let receiver_id = self.unwrap_parenthesized_expression(receiver_id, ctx.tree);

        // preserve direct receiver symbols through instantiation wrappers
        if let Expression::Instantiation { left, .. } = ctx.tree.get(receiver_id) {
            return self.resolve_direct_receiver_symbol_for_expression(ctx, *left);
        }

        // resolve namespace member receivers before parse target-symbol lookup
        if let Expression::Member { left, name, .. } = ctx.tree.get(receiver_id)
            && let Some(name) = *name
            && let Some(symbol) = self.resolve_namespace_member_symbol(
                ctx.tree_symbol_view(),
                receiver_id,
                *left,
                StaticKey::Name(name),
            )
        {
            return Some(self.canonical_symbol_id(
                ctx.module_symbol_view(),
                symbol,
                CanonicalSymbolMode::FollowAliases,
            ));
        }

        // resolve symbol references first and then consult the parse target symbol
        self.reference_symbol_for_expression(ctx.tree_symbol_view(), receiver_id)
            .or_else(|| {
                ctx.tree.get(receiver_id).target_symbol().map(|symbol| {
                    self.canonical_symbol_id(
                        ctx.module_symbol_view(),
                        symbol,
                        CanonicalSymbolMode::FollowAliases,
                    )
                })
            })
    }

    /// Resolve one direct receiver symbol from a type expression.
    pub(crate) fn resolve_direct_receiver_symbol_for_type_expression(
        &self,
        ctx: &TypeContext<'_>,
        receiver_id: LocalNodeId<TypeExpression>,
    ) -> Option<GlobalSymbolId> {
        // peel parenthesized receivers to their core symbol
        let receiver_id = self.unwrap_parenthesized_type_expression(receiver_id, ctx.tree);

        // resolve namespace member receivers before parse target-symbol lookup
        if let TypeExpression::Member { left, name, .. } = ctx.tree.get(receiver_id)
            && let Some(symbol) = self.resolve_namespace_type_member_symbol(
                ctx.tree_symbol_view(),
                receiver_id,
                *left,
                StaticKey::Name(*name),
            )
        {
            return Some(self.canonical_symbol_id(
                ctx.module_symbol_view(),
                symbol,
                CanonicalSymbolMode::FollowAliases,
            ));
        }

        // resolve symbol references first and then consult the parse target symbol
        self.reference_symbol_for_type_expression(ctx.tree_symbol_view(), receiver_id)
            .or_else(|| {
                ctx.tree.get(receiver_id).target_symbol().map(|symbol| {
                    self.canonical_symbol_id(
                        ctx.module_symbol_view(),
                        symbol,
                        CanonicalSymbolMode::FollowAliases,
                    )
                })
            })
    }

    /// Resolve a type symbol for a potentially union or intersection receiver type.
    pub(crate) fn query_type_like_receiver_symbol_for_type_id(
        &self,
        receiver_type_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<GlobalSymbolId> {
        // use direct type-symbol unwrap when possible
        if let Some((symbol, _, _)) = self.unwrap_type_symbol(types, receiver_type_id) {
            return Some(symbol);
        }

        // otherwise scan union and intersection members for a nominal type symbol
        let element_ids = match types.get_type(receiver_type_id) {
            Type::Intersection { elements } | Type::Union { elements } => elements.clone(),
            _ => return None,
        };
        element_ids.iter().find_map(|element_id| {
            self.unwrap_type_symbol(types, *element_id)
                .map(|(symbol, _, _)| symbol)
        })
    }
}
