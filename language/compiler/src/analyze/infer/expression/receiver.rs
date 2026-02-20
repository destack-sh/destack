use super::member::{MemberLookupMode, MemberReceiverContext};
use crate::Compiler;
use crate::analyze::common::{AnalyzeDependencyStage, CanonicalSymbolMode};
use destack_dir::{
    Expression, GlobalSymbolId, LocalNodeId, LocalTypeId, NodeTree, SymbolSpace, SymbolTable,
    SymbolType, Type, TypeTable,
};
use destack_workspace::{Module, ProfileId};

impl Compiler {
    /// Return true when a member receiver should be evaluated as a type projection receiver.
    pub(crate) fn query_expression_is_projection_receiver_for_infer(
        &self,
        module: &Module,
        profile: ProfileId,
        receiver_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> bool {
        let receiver_id = self.unwrap_parenthesized_expression(receiver_id, tree);
        if !self.query_receiver_has_static_arguments(receiver_id, tree) {
            return false;
        }

        let symbol = self.resolve_projection_receiver_symbol_for_expression(
            module,
            profile,
            receiver_id,
            tree,
            symbols,
            types,
        );
        let Some(symbol) = symbol else {
            return false;
        };

        self.query_symbol_supports_projection_receiver(module, profile, symbol, symbols)
    }

    /// Return true when a receiver expression has explicit static arguments.
    fn query_receiver_has_static_arguments(
        &self,
        receiver_id: LocalNodeId<Expression>,
        tree: &NodeTree,
    ) -> bool {
        tree.get(receiver_id)
            .static_arguments()
            .is_some_and(|arguments| !arguments.is_empty())
    }

    /// Resolve a symbol candidate for associated projection receiver inference.
    fn resolve_projection_receiver_symbol_for_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        receiver_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<GlobalSymbolId> {
        self.resolve_direct_receiver_symbol_for_expression(
            module,
            receiver_id,
            profile,
            tree,
            symbols,
        )
        .or_else(|| {
            let receiver_type_id = self
                .resolve_declared_type_expression(
                    module,
                    profile,
                    receiver_id,
                    tree,
                    symbols,
                    types,
                    true,
                    true,
                )
                .ok()?;
            self.query_type_like_receiver_symbol_for_type_id(receiver_type_id, types)
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

    /// Select a member lookup mode from nominal receiver metadata and receiver type.
    pub(crate) fn query_member_lookup_mode_for_receiver(
        &self,
        nominal_symbol: Option<GlobalSymbolId>,
        receiver_ty: &Type,
    ) -> MemberLookupMode {
        // nominal values only expose static members
        if nominal_symbol.is_some() {
            return MemberLookupMode::Value;
        }

        self.query_member_lookup_mode_for_type(receiver_ty)
    }

    /// Select a member lookup mode from receiver type metadata only.
    pub(crate) fn query_member_lookup_mode_for_type(&self, receiver_ty: &Type) -> MemberLookupMode {
        // instance receivers should never surface static members
        if matches!(receiver_ty, Type::Reference { .. }) {
            return MemberLookupMode::Instance;
        }

        // fall back to unfiltered lookup for non-instance receivers
        MemberLookupMode::Any
    }

    /// Classify member receiver behavior for symbol and type lookup paths.
    pub(crate) fn query_member_receiver_context_for_expression(
        &self,
        module: &Module,
        receiver_id: LocalNodeId<Expression>,
        receiver_ty: &Type,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> MemberReceiverContext {
        let receiver_id = self.unwrap_parenthesized_expression(receiver_id, tree);
        let nominal_symbol = self.query_nominal_value_symbol_for_expression(
            module,
            receiver_id,
            profile,
            tree,
            symbols,
        );
        let has_static_arguments = self.query_receiver_has_static_arguments(receiver_id, tree);
        let lookup_mode = self.query_member_lookup_mode_for_receiver(nominal_symbol, receiver_ty);

        MemberReceiverContext {
            nominal_symbol,
            has_static_arguments,
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

        let space =
            self.query_symbol_space_for_global_non_blocking(module, profile, symbol, symbols);
        if space.is_some_and(|space| matches!(space, SymbolSpace::Value | SymbolSpace::TypeValue)) {
            Some(symbol)
        } else {
            None
        }
    }

    /// Resolve a canonical direct symbol for a receiver expression.
    fn resolve_direct_receiver_symbol_for_expression(
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

        // resolve symbol references first and then fallback to the parse target symbol
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
    fn query_type_like_receiver_symbol_for_type_id(
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

    /// Resolve symbol space for one global symbol without blocking on remote readiness.
    fn query_symbol_space_for_global_non_blocking(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        symbols: &SymbolTable,
    ) -> Option<SymbolSpace> {
        self.with_module_symbols_or_local_at_stage(
            module,
            profile,
            symbol.module_id,
            symbols,
            AnalyzeDependencyStage::Declare,
            |_, owner_symbols| owner_symbols.get_symbol(symbol.local_id).space,
        )
        .ok()
    }
}
