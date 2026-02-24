use super::member::{MemberLookupMode, MemberReceiverContext};
use crate::Compiler;
use crate::analyze::common::{AnalyzeDependencyStage, CanonicalSymbolMode, InferTablesContext};
use destack_dir::{
    Expression, GlobalSymbolId, LocalNodeId, LocalTypeId, NodeTree, StaticKey, SymbolSpace,
    SymbolTable, SymbolType, Type, TypeTable,
};
use destack_workspace::{Module, ProfileId};
use std::collections::HashSet;

impl Compiler {
    /// Return true when a member receiver should be evaluated as a type projection receiver.
    pub(crate) fn is_projection_receiver_expression(
        &self,
        tables: &mut InferTablesContext<'_>,
        receiver_id: LocalNodeId<Expression>,
    ) -> bool {
        let receiver_id = self.unwrap_parenthesized_expression(receiver_id, tables.tree);
        if !tables
            .tree
            .get(receiver_id)
            .static_arguments()
            .is_some_and(|arguments| !arguments.is_empty())
        {
            return false;
        }

        let symbol = self.resolve_projection_receiver_symbol(&mut tables.reborrow(), receiver_id);
        let Some(symbol) = symbol else {
            return false;
        };

        self.query_symbol_supports_projection_receiver(
            tables.module,
            tables.profile,
            symbol,
            tables.symbols,
        )
    }

    /// Resolve a symbol candidate for associated projection receiver inference.
    fn resolve_projection_receiver_symbol(
        &self,
        tables: &mut InferTablesContext<'_>,
        receiver_id: LocalNodeId<Expression>,
    ) -> Option<GlobalSymbolId> {
        self.resolve_direct_receiver_symbol_for_expression(
            tables.module,
            receiver_id,
            tables.profile,
            tables.tree,
            tables.symbols,
        )
        .or_else(|| {
            let receiver_type_id = self
                .resolve_declared_type_expression(
                    tables.module,
                    tables.profile,
                    receiver_id,
                    tables.tree,
                    tables.symbols,
                    tables.types,
                    true,
                    true,
                )
                .ok()?;
            self.query_type_like_receiver_symbol_for_type_id(receiver_type_id, tables.types)
        })
    }

    /// Return true when a symbol can act as a projection receiver in infer.
    fn query_symbol_supports_projection_receiver(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        symbols: &SymbolTable,
    ) -> bool {
        let symbol = self.canonical_symbol_id(
            module,
            symbols,
            profile,
            symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        let symbol = self
            .declaration_symbol_id(module, symbols, profile, symbol)
            .unwrap_or(symbol);

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
        module: &Module,
        receiver_id: LocalNodeId<Expression>,
        receiver_ty_id: Option<LocalTypeId>,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> MemberReceiverContext {
        let receiver_id = self.unwrap_parenthesized_expression(receiver_id, tree);
        let nominal_symbol = self.query_nominal_value_symbol_for_expression(
            module,
            receiver_id,
            profile,
            tree,
            symbols,
        );
        let has_static_arguments = tree
            .get(receiver_id)
            .static_arguments()
            .is_some_and(|arguments| !arguments.is_empty());
        let has_this_receiver =
            matches!(tree.get(receiver_id), Expression::This | Expression::Super)
                || receiver_ty_id.is_some_and(|receiver_ty_id| {
                    self.type_contains_this(receiver_ty_id, types, &mut HashSet::new())
                });
        let lookup_mode = if nominal_symbol.is_some() {
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
        module: &Module,
        receiver_id: LocalNodeId<Expression>,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        // resolve the direct reference symbol for the receiver
        let symbol = self.resolve_direct_receiver_symbol_for_expression(
            module,
            receiver_id,
            profile,
            tree,
            symbols,
        )?;
        let symbol = self.resolve_type_reference_symbol(module, profile, symbol, tree, symbols);
        let symbol = self
            .declaration_symbol_id(module, symbols, profile, symbol)
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
            .with_module_symbols_or_local_at_stage(
                module,
                profile,
                symbol.module_id,
                symbols,
                AnalyzeDependencyStage::Declare,
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
        module: &Module,
        receiver_id: LocalNodeId<Expression>,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        // peel parenthesized receivers to their core symbol
        let receiver_id = self.unwrap_parenthesized_expression(receiver_id, tree);

        // preserve direct receiver symbols through instantiation wrappers
        if let Expression::Instantiation { left, .. } = tree.get(receiver_id) {
            return self.resolve_direct_receiver_symbol_for_expression(
                module, *left, profile, tree, symbols,
            );
        }

        // resolve namespace member receivers before parse target-symbol lookup
        if let Expression::Member { left, name, .. } = tree.get(receiver_id)
            && let Some(symbol) = self.resolve_namespace_member_symbol(
                module,
                profile,
                receiver_id,
                *left,
                StaticKey::Name(*name),
                tree,
                symbols,
            )
        {
            return Some(self.canonical_symbol_id(
                module,
                symbols,
                profile,
                symbol,
                CanonicalSymbolMode::FollowAliases,
            ));
        }

        // resolve symbol references first and then consult the parse target symbol
        self.reference_symbol_for_expression(module, receiver_id, profile, tree, symbols)
            .or_else(|| {
                tree.get(receiver_id).target_symbol().map(|symbol| {
                    self.canonical_symbol_id(
                        module,
                        symbols,
                        profile,
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
